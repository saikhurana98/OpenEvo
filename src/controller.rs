//! Serialized device actor. Device reads, writes and reconnects never block UI.
use crate::driver::{Device, Result};
use crate::model::{Capabilities, Command, Request, Snapshot};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender};
use std::time::{Duration, Instant};

pub fn read_state(device: &Device, previous: &Snapshot) -> Result<Snapshot> {
    let mut next = previous.clone();
    next.sample_rate = device.sample_rate()?;
    next.output_db = device.output_db()?;
    next.output_muted = device.output_muted()?;
    next.monitor_blend = device.monitor_blend()?;
    for channel in 0..2u8 {
        let input = &mut next.inputs[channel as usize];
        input.gain_db = device.gain(channel)?;
        input.muted = device.input_muted(channel)?;
        input.phantom = device.phantom(channel)?;
    }
    next.connected = true;
    next.driver_version = "Audient API 5.70 · verified Windows x64".into();
    next.capabilities = Capabilities {
        gain: true,
        input_mute: true,
        phantom: true,
        output_volume: true,
        output_mute: true,
        monitor_blend: true,
        loopback: true,
        ..Capabilities::default()
    };
    if !previous.connected || previous.capabilities.link {
        if let Ok(linked) = device.linked() {
            next.linked = linked;
            next.capabilities.link = true;
        }
    }
    if !previous.connected || previous.capabilities.monitor_pan {
        if let Ok(pan) = device.monitor_pan() {
            next.monitor_pan = pan;
            next.capabilities.monitor_pan = true;
        }
    }
    if (!previous.connected || previous.capabilities.smartgain)
        && let Ok(status) = device.smartgain_state()
    {
        next.smartgain_running = matches!(status, 2 | 3);
        next.smartgain_status = match status {
            0 => "Ready · select inputs to set their gain",
            1 => "Select inputs for Smartgain",
            2 => "Starting Smartgain…",
            3 => "Listening · perform at your normal level",
            4 => "Smartgain complete",
            5 => "Smartgain could not set a level. Try again.",
            _ => unreachable!(),
        }
        .into();
        next.capabilities.smartgain = true;
    }
    if (!previous.connected || previous.capabilities.meters)
        && let Ok(peaks) = device.source_peaks()
    {
        next.inputs[0].peak = meter_position(peaks[0]);
        next.inputs[1].peak = meter_position(peaks[1]);
        next.playback_peaks = [meter_position(peaks[2]), meter_position(peaks[3])];
        next.capabilities.meters = true;
    }
    if (!previous.connected || previous.capabilities.sample_rate)
        && let Ok(rates) = device.supported_sample_rates()
    {
        next.supported_sample_rates = rates;
        next.capabilities.sample_rate = true;
    }
    if (!previous.connected || previous.capabilities.buffer_size)
        && let Ok((size, allowed)) = device.buffer_info()
    {
        next.buffer_size = size;
        next.supported_buffers = allowed;
        next.capabilities.buffer_size = true;
    }
    Ok(next)
}

fn meter_position(linear: f32) -> f32 {
    if !linear.is_finite() || linear <= 0.001 {
        0.0
    } else {
        ((20.0 * linear.log10() + 60.0) / 60.0).clamp(0.0, 1.0)
    }
}

fn execute(device: &Device, command: Command) -> Result<()> {
    match command {
        Command::Refresh => Ok(()),
        Command::SetGain { channel, db } => device.set_gain(channel, db),
        Command::SetLinked(linked) => {
            device.set_linked(linked)?;
            if device.linked()? != linked {
                return Err("Stereo link readback did not match".into());
            }
            Ok(())
        }
        Command::StartSmartgain { channel_mask } => device.start_smartgain(channel_mask),
        Command::StopSmartgain => device.stop_smartgain(),
        Command::SetInputMute { channel, muted } => {
            device.set_input_muted(channel, muted)?;
            if device.input_muted(channel)? != muted {
                return Err("Input mute readback did not match".into());
            }
            Ok(())
        }
        Command::SetPhantom { channel, enabled } => {
            device.set_phantom(channel, enabled)?;
            if device.phantom(channel)? != enabled {
                return Err("48V readback did not match".into());
            }
            Ok(())
        }
        Command::SetOutputVolume(db) => device.set_output_db(db),
        Command::SetOutputMute(muted) => {
            device.set_output_muted(muted)?;
            if device.output_muted()? != muted {
                return Err("Output mute readback did not match".into());
            }
            Ok(())
        }
        Command::SetMonitorBlend(value) => device.set_monitor_blend(value),
        Command::SetMonitorPan(value) => device.set_monitor_pan(value),
        Command::SetSampleRate(rate) => device.set_sample_rate(rate),
        Command::SetBufferSize(size) => device.set_buffer_size(size),
        _ => Err("This control is awaiting device verification".into()),
    }
}

fn route_edit(
    command: &Command,
    routing: &crate::routing::SavedRouting,
) -> Option<(u8, crate::routing::Route)> {
    let source = match command {
        Command::SetLoopbackGain { source, .. }
        | Command::SetLoopbackPan { source, .. }
        | Command::SetLoopbackMute { source, .. } => *source,
        _ => return None,
    };
    let mut route = routing
        .sources
        .get(source as usize)
        .copied()
        .flatten()
        .unwrap_or_default();
    match command {
        Command::SetLoopbackGain { gain_db, .. } => route.gain_db = *gain_db,
        Command::SetLoopbackPan { pan, .. } => route.pan = *pan,
        Command::SetLoopbackMute { muted, .. } => route.muted = *muted,
        _ => unreachable!(),
    }
    Some((source, route))
}

pub fn run(commands: Receiver<Request>, snapshots: SyncSender<Snapshot>) {
    let mut state = Snapshot::default();
    let mut routing = crate::routing::load();
    for (index, route) in routing.sources.iter().enumerate() {
        if let Some(route) = route {
            state.loopback[index] = crate::model::LoopbackState {
                gain_db: route.gain_db,
                pan: route.pan,
                muted: route.muted,
                known: true,
            };
        }
    }
    state.loopback_provenance = "Last saved values · hardware routing cannot be read back".into();
    let mut device: Option<Device> = None;
    let mut reconnect = Instant::now();
    let mut last_poll = Instant::now() - Duration::from_secs(1);
    let mut last_meter = Instant::now();
    let mut last_error: Option<(Instant, String)> = None;
    loop {
        if device.is_none() && Instant::now() >= reconnect {
            state.generation = state.generation.wrapping_add(1);
            // Discard all intent queued against the previous connection.
            // Connecting and polling are read-only; no settings replay occurs.
            while commands.try_recv().is_ok() {}
            match Device::open().and_then(|d| read_state(&d, &state).map(|s| (d, s))) {
                Ok((d, s)) => {
                    device = Some(d);
                    state = s;
                    state.status = "Connected · settings read from EVO 4".into();
                    state.loopback_provenance =
                        "Last saved values · hardware routing cannot be read back".into();
                    state.error = false;
                    last_error = None;
                }
                Err(error) => {
                    state.connected = false;
                    state.capabilities = Capabilities::default();
                    state.status = error;
                    state.error = true;
                }
            }
            reconnect = Instant::now() + Duration::from_secs(2);
            let _ = snapshots.try_send(state.clone());
        }
        match commands.recv_timeout(Duration::from_millis(30)) {
            Ok(request) => {
                if request.generation != state.generation {
                    continue;
                }
                let command = request.command;
                let refresh = matches!(command, Command::Refresh);
                if let Some(ref d) = device {
                    let result = if let Some((source, route)) = route_edit(&command, &routing) {
                        match d.set_loopback(source, route) {
                            Ok(()) => {
                                state.loopback[source as usize] = crate::model::LoopbackState {
                                    gain_db: route.gain_db,
                                    pan: route.pan,
                                    muted: route.muted,
                                    known: true,
                                };
                                routing.sources[source as usize] = Some(route);
                                state.loopback_provenance="Last applied by this app · hardware routing cannot be read back".into();
                                crate::routing::save(&routing).map_err(|e| {
                                    format!("Routing applied, but could not save it: {e}")
                                })
                            }
                            Err(error) => {
                                if let Some(row) = state.loopback.get_mut(source as usize) {
                                    row.known = false;
                                }
                                state.loopback_provenance =
                                    "Routing uncertain after failed write · apply the source again"
                                        .into();
                                Err(error)
                            }
                        }
                    } else {
                        execute(d, command)
                    };
                    match result {
                        Ok(()) => {
                            state.status = "Settings applied".into();
                            state.error = false;
                            last_error = None;
                        }
                        Err(error) => {
                            state.status = error.clone();
                            state.error = true;
                            last_error = Some((Instant::now(), error));
                        }
                    }
                    if refresh {
                        state.connected = false;
                    }
                    // Read back after every action; failed writes never become
                    // optimistic authoritative values in the window.
                    last_poll = Instant::now() - Duration::from_secs(1);
                } else {
                    state.status = "Connect your EVO 4 before changing settings".into();
                    state.error = true;
                    let _ = snapshots.try_send(state.clone());
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if let Some(ref d) = device {
            if state.capabilities.meters && last_meter.elapsed() >= Duration::from_millis(33) {
                match d.source_peaks() {
                    Ok(peaks) => {
                        state.inputs[0].peak = meter_position(peaks[0]);
                        state.inputs[1].peak = meter_position(peaks[1]);
                        state.playback_peaks = [meter_position(peaks[2]), meter_position(peaks[3])];
                    }
                    Err(_) => {
                        state.capabilities.meters = false;
                        state.inputs[0].peak = 0.0;
                        state.inputs[1].peak = 0.0;
                        state.playback_peaks = [0.0; 2];
                    }
                }
                let _ = snapshots.try_send(state.clone());
                last_meter = Instant::now();
            }
            if last_poll.elapsed() >= Duration::from_millis(200) {
                match read_state(d, &state) {
                    Ok(s) => {
                        state = s;
                        if let Some((when, ref error)) = last_error {
                            if when.elapsed() < Duration::from_secs(8) {
                                state.status = error.clone();
                                state.error = true;
                            } else {
                                last_error = None;
                                state.status = "Connected · settings read from EVO 4".into();
                                state.error = false;
                            }
                        }
                    }
                    Err(error) => {
                        state.connected = false;
                        state.error = true;
                        state.status = format!("Connection lost: {error}");
                        state.capabilities = Capabilities::default();
                        state.loopback_provenance =
                            "Connection lost · cached routing is not current hardware state".into();
                        device = None;
                        while commands.try_recv().is_ok() {}
                        reconnect = Instant::now() + Duration::from_secs(2);
                    }
                }
                let _ = snapshots.try_send(state.clone());
                last_poll = Instant::now();
            }
        }
    }
}

pub fn diagnostic() -> Result<String> {
    let device = Device::open()?;
    let state = read_state(&device, &Snapshot::default())?;
    Ok(
        serde_json::json!({"connected":state.connected,"sample_rate":state.sample_rate,
        "output_db":state.output_db,"output_muted":state.output_muted,
        "input_gain_db":[state.inputs[0].gain_db,state.inputs[1].gain_db],
        "input_muted":[state.inputs[0].muted,state.inputs[1].muted],
        "phantom":[state.inputs[0].phantom,state.inputs[1].phantom],
        "monitor_blend":state.monitor_blend,"monitor_pan":state.monitor_pan,
        "linked":state.linked,"smartgain_status":state.smartgain_status,
        "buffer_size":state.buffer_size,"supported_sample_rates":state.supported_sample_rates,
        "supported_buffers":state.supported_buffers,
        "input_peak_display":[state.inputs[0].peak,state.inputs[1].peak],
        "playback_peak_display":state.playback_peaks,
        "capabilities":{"link":state.capabilities.link,"smartgain":state.capabilities.smartgain,
        "meters":state.capabilities.meters,"monitor_pan":state.capabilities.monitor_pan,
        "sample_rate":state.capabilities.sample_rate,"buffer_size":state.capabilities.buffer_size},
        "loopback_state":"write-only; no hardware readback"})
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_route_edits_preserve_previous_fields() {
        let mut saved = crate::routing::SavedRouting {
            schema: 2,
            ..Default::default()
        };
        let commands = [
            Command::SetLoopbackGain {
                source: 2,
                gain_db: -14.5,
            },
            Command::SetLoopbackPan {
                source: 2,
                pan: 25.0,
            },
            Command::SetLoopbackMute {
                source: 2,
                muted: true,
            },
        ];
        for command in commands {
            let (source, route) = route_edit(&command, &saved).unwrap();
            saved.sources[source as usize] = Some(route);
        }
        let route = saved.sources[2].unwrap();
        assert_eq!(route.gain_db, -14.5);
        assert_eq!(route.pan, 25.0);
        assert!(route.muted);
        assert!(saved.sources[0].is_none());
    }
    #[test]
    fn meter_scale_has_useful_decibel_range() {
        assert_eq!(meter_position(0.0), 0.0);
        assert_eq!(meter_position(f32::NAN), 0.0);
        assert!((meter_position(0.1) - 2.0 / 3.0).abs() < 0.001);
        assert_eq!(meter_position(1.0), 1.0);
    }
}
