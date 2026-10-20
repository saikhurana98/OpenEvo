use crate::{
    AppWindow, InputView, LoopbackView,
    model::{Command, Request, Snapshot},
};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::mpsc::{Receiver, SyncSender, TrySendError},
    time::{Duration, Instant},
};

type Queue = Rc<RefCell<VecDeque<Command>>>;

fn fitted_initial_size(work_width: f32, work_height: f32, scale: f32) -> slint::LogicalSize {
    let scale = scale.max(0.5);
    slint::LogicalSize::new(
        (work_width / scale - 40.0).clamp(400.0, 1080.0),
        (work_height / scale - 72.0).clamp(400.0, 790.0),
    )
}

fn fit_initial_window(ui: &AppWindow) {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct Rect {
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
        }
        #[link(name = "user32")]
        unsafe extern "system" {
            fn SystemParametersInfoW(action: u32, param: u32, data: *mut Rect, flags: u32) -> i32;
        }
        let mut work = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if unsafe { SystemParametersInfoW(0x30, 0, &mut work, 0) } != 0 {
            let scale = std::env::var("SLINT_SCALE_FACTOR")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite() && *v > 0.0)
                .unwrap_or(ui.window().scale_factor());
            let size = fitted_initial_size(
                (work.right - work.left) as f32,
                (work.bottom - work.top) as f32,
                scale,
            );
            ui.window().set_size(slint::PhysicalSize::new(
                (size.width * scale).round() as u32,
                (size.height * scale).round() as u32,
            ));
            ui.window()
                .set_position(slint::PhysicalPosition::new(work.left + 16, work.top + 16));
        }
    }
}

// Continuous controls replace older unsent values for the same target. Discrete
// actions retain order. Both queues are bounded, including if the driver hangs.
fn same_target(a: &Command, b: &Command) -> bool {
    match (a, b) {
        (Command::SetGain { channel: a, .. }, Command::SetGain { channel: b, .. }) => a == b,
        (
            Command::SetLoopbackGain { source: a, .. },
            Command::SetLoopbackGain { source: b, .. },
        )
        | (Command::SetLoopbackPan { source: a, .. }, Command::SetLoopbackPan { source: b, .. }) => {
            a == b
        }
        (Command::SetOutputVolume(_), Command::SetOutputVolume(_))
        | (Command::SetMonitorBlend(_), Command::SetMonitorBlend(_))
        | (Command::SetMonitorPan(_), Command::SetMonitorPan(_)) => true,
        _ => false,
    }
}

fn queue_command(queue: &mut VecDeque<Command>, command: Command) -> bool {
    if let Some(index) = queue.iter().position(|old| same_target(old, &command)) {
        queue.remove(index);
    }
    if queue.len() >= 64 {
        return false;
    }
    queue.push_back(command);
    true
}

fn enqueue(queue: &Queue, weak: &slint::Weak<AppWindow>, command: Command) {
    if !queue_command(&mut queue.borrow_mut(), command) {
        if let Some(ui) = weak.upgrade() {
            ui.set_error(true);
            ui.set_status(
                "The interface is busy. Wait for the pending changes, then try again.".into(),
            );
        }
    }
}

fn apply(ui: &AppWindow, state: Snapshot) {
    ui.set_connected(state.connected);
    ui.set_device_name(state.device_name.into());
    ui.set_status(state.status.into());
    ui.set_error(state.error);
    ui.set_driver_version(state.driver_version.into());
    let rate = if !state.connected || state.sample_rate == 0 {
        "—".into()
    } else {
        format!("{} kHz", state.sample_rate as f32 / 1000.)
    };
    ui.set_sample_rate(rate.into());
    ui.set_buffer_size(if state.buffer_size == 0 {
        "—".into()
    } else {
        state.buffer_size.to_string().into()
    });
    let rates: Vec<slint::SharedString> = state
        .supported_sample_rates
        .iter()
        .map(|r| format!("{} kHz", *r as f32 / 1000.0).into())
        .collect();
    if ui.get_rate_options().iter().collect::<Vec<_>>() != rates {
        ui.set_rate_options(ModelRc::new(VecModel::from(rates)));
        ui.set_rate_values(ModelRc::new(VecModel::from(
            state
                .supported_sample_rates
                .iter()
                .map(|&x| x as i32)
                .collect::<Vec<_>>(),
        )));
    }
    let buffers: Vec<slint::SharedString> = state
        .supported_buffers
        .iter()
        .map(|r| r.to_string().into())
        .collect();
    if ui.get_buffer_options().iter().collect::<Vec<_>>() != buffers {
        ui.set_buffer_options(ModelRc::new(VecModel::from(buffers)));
        ui.set_buffer_values(ModelRc::new(VecModel::from(
            state
                .supported_buffers
                .iter()
                .map(|&x| x as i32)
                .collect::<Vec<_>>(),
        )));
    }
    let inputs = state
        .inputs
        .into_iter()
        .map(|v| InputView {
            gain: v.gain_db,
            muted: v.muted,
            phantom: v.phantom,
            peak: v.peak,
        })
        .collect::<Vec<_>>();
    let old_inputs = ui.get_inputs();
    if let Some(model) = old_inputs.as_any().downcast_ref::<VecModel<InputView>>() {
        for (i, value) in inputs.into_iter().enumerate() {
            if model.row_data(i).as_ref() != Some(&value) {
                model.set_row_data(i, value);
            }
        }
    } else {
        ui.set_inputs(ModelRc::new(VecModel::from(inputs)));
    }
    let names = [
        "Mic 1",
        "Mic 2",
        "Main L",
        "Main R",
        "Loopback L",
        "Loopback R",
    ];
    let selected = (ui.get_selected_source() as usize).min(5);
    let selected_gain = state.loopback[selected].gain_db;
    let selected_pan = state.loopback[selected].pan;
    let loopback = state
        .loopback
        .into_iter()
        .enumerate()
        .map(|(i, v)| LoopbackView {
            name: names[i].into(),
            gain: v.gain_db,
            pan: v.pan,
            muted: v.muted,
            known: v.known,
        })
        .collect::<Vec<_>>();
    let old_loopback = ui.get_loopback();
    if let Some(model) = old_loopback
        .as_any()
        .downcast_ref::<VecModel<LoopbackView>>()
    {
        for (i, value) in loopback.into_iter().enumerate() {
            if model.row_data(i).as_ref() != Some(&value) {
                model.set_row_data(i, value);
            }
        }
    } else {
        ui.set_loopback(ModelRc::new(VecModel::from(loopback)));
    }
    ui.set_selected_loopback_gain(selected_gain);
    ui.set_selected_loopback_pan(selected_pan);
    ui.set_linked(state.linked);
    ui.set_output_db(state.output_db);
    ui.set_playback_left(state.playback_peaks[0]);
    ui.set_playback_right(state.playback_peaks[1]);
    ui.set_output_muted(state.output_muted);
    ui.set_monitor_blend(state.monitor_blend);
    ui.set_monitor_pan(state.monitor_pan);
    ui.set_smartgain_status(state.smartgain_status.into());
    ui.set_smartgain_running(state.smartgain_running);
    ui.set_loopback_provenance(state.loopback_provenance.into());
    let c = state.capabilities;
    ui.set_cap_gain(c.gain);
    ui.set_cap_input_mute(c.input_mute);
    ui.set_cap_phantom(c.phantom);
    ui.set_cap_link(c.link);
    ui.set_cap_smartgain(c.smartgain);
    ui.set_cap_output_volume(c.output_volume);
    ui.set_cap_output_mute(c.output_mute);
    ui.set_cap_monitor_blend(c.monitor_blend);
    ui.set_cap_monitor_pan(c.monitor_pan);
    ui.set_cap_loopback(c.loopback);
    ui.set_cap_sample_rate(c.sample_rate);
    ui.set_cap_buffer_size(c.buffer_size);
    ui.set_cap_meters(c.meters);
}

pub fn run(
    sender: SyncSender<Request>,
    snapshots: Receiver<Snapshot>,
) -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    ui.show()?;
    fit_initial_window(&ui);
    let fit_weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        if let Some(ui) = fit_weak.upgrade() {
            fit_initial_window(&ui);
        }
    });
    let queue: Queue = Rc::new(RefCell::new(VecDeque::with_capacity(64)));
    apply(&ui, Snapshot::default());
    macro_rules! connect {
        ($callback:ident, || $command:expr) => {{
            let q = queue.clone(); let weak = ui.as_weak();
            ui.$callback(move || enqueue(&q, &weak, $command));
        }};
        ($callback:ident, |$($arg:ident),*| $command:expr) => {{
            let q = queue.clone(); let weak = ui.as_weak();
            ui.$callback(move |$($arg),*| enqueue(&q, &weak, $command));
        }};
    }
    connect!(on_refresh, || Command::Refresh);
    connect!(on_gain_edited, |channel, db| Command::SetGain {
        channel: channel as u8,
        db
    });
    connect!(on_input_mute_edited, |channel, muted| {
        Command::SetInputMute {
            channel: channel as u8,
            muted,
        }
    });
    connect!(on_phantom_edited, |channel, enabled| Command::SetPhantom {
        channel: channel as u8,
        enabled
    });
    connect!(on_link_edited, |linked| Command::SetLinked(linked));
    connect!(on_smartgain_start, |mask| Command::StartSmartgain {
        channel_mask: mask as u8
    });
    connect!(on_smartgain_stop, || Command::StopSmartgain);
    connect!(on_output_edited, |db| Command::SetOutputVolume(db));
    connect!(on_output_mute_edited, |muted| Command::SetOutputMute(muted));
    connect!(on_blend_edited, |blend| Command::SetMonitorBlend(blend));
    connect!(on_monitor_pan_edited, |pan| Command::SetMonitorPan(pan));
    connect!(on_loopback_gain_edited, |source, gain_db| {
        Command::SetLoopbackGain {
            source: source as u8,
            gain_db,
        }
    });
    connect!(on_loopback_pan_edited, |source, pan| {
        Command::SetLoopbackPan {
            source: source as u8,
            pan,
        }
    });
    connect!(on_loopback_mute_edited, |source, muted| {
        Command::SetLoopbackMute {
            source: source as u8,
            muted,
        }
    });
    connect!(on_sample_rate_edited, |value| Command::SetSampleRate(
        value as u32
    ));
    connect!(on_buffer_size_edited, |value| Command::SetBufferSize(
        value as u32
    ));
    let weak = ui.as_weak();
    let timer = slint::Timer::default();
    let mut generation = 0u64;
    let mut envelopes = [crate::meters::Envelope::default(); 4];
    let mut targets = [0.0; 4];
    let mut last_frame = Instant::now();
    let mut meters_enabled = false;
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut latest = None;
            while let Ok(state) = snapshots.try_recv() {
                latest = Some(state);
            }
            if let Some(mut state) = latest {
                if state.generation != generation || !state.connected {
                    queue.borrow_mut().clear();
                    ui.invoke_reset_transient();
                    for envelope in &mut envelopes { envelope.reset(); }
                }
                generation = state.generation;
                meters_enabled = state.connected && state.capabilities.meters;
                targets = [state.inputs[0].peak, state.inputs[1].peak,
                    state.playback_peaks[0], state.playback_peaks[1]];
                if !meters_enabled {
                    targets = [0.0; 4];
                    for envelope in &mut envelopes { envelope.reset(); }
                }
                state.inputs[0].peak = envelopes[0].value();
                state.inputs[1].peak = envelopes[1].value();
                state.playback_peaks = [envelopes[2].value(), envelopes[3].value()];
                apply(&ui, state);
            }
            let elapsed = last_frame.elapsed().as_secs_f32();
            last_frame = Instant::now();
            if meters_enabled {
                for i in 0..4 { envelopes[i].advance(targets[i], elapsed); }
                let inputs = ui.get_inputs();
                if let Some(model) = inputs.as_any().downcast_ref::<VecModel<InputView>>() {
                    for (i, envelope) in envelopes.iter().take(2).enumerate() {
                        if let Some(mut row) = model.row_data(i) {
                            if (row.peak - envelope.value()).abs() > 0.0001 {
                                row.peak = envelope.value();
                                model.set_row_data(i, row);
                            }
                        }
                    }
                }
                ui.set_playback_left(envelopes[2].value());
                ui.set_playback_right(envelopes[3].value());
            }
            let mut queue = queue.borrow_mut();
            while let Some(command) = queue.pop_front() {
                match sender.try_send(Request {
                    generation,
                    command,
                }) {
                    Ok(()) => {}
                    Err(TrySendError::Full(request)) => {
                        queue.push_front(request.command);
                        break;
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        queue.clear();
                        ui.invoke_reset_transient();
                        ui.set_connected(false);
                        ui.set_cap_meters(false);
                        ui.set_playback_left(0.0);
                        ui.set_playback_right(0.0);
                        meters_enabled = false;
                        ui.set_error(true);
                        ui.set_status(
                            "Device worker stopped. Close and reopen OpenEvo.".into(),
                        );
                        break;
                    }
                }
            }
        },
    );
    ui.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initial_window_fits_1080p_at_150_percent_and_small_work_areas() {
        let size = fitted_initial_size(1920.0, 1040.0, 1.5);
        assert!(size.width * 1.5 + 40.0 <= 1920.0);
        assert!(size.height * 1.5 + 72.0 <= 1040.0);
        let small = fitted_initial_size(1280.0, 680.0, 1.5);
        assert!(small.height * 1.5 + 72.0 <= 680.0);
        assert!(small.width * 1.5 + 40.0 <= 1280.0);
    }
    fn selecting_loopback_source_is_read_only_and_tracks_authoritative_updates(ui: &AppWindow) {
        let mut state = Snapshot::default();
        state.connected = true;
        state.capabilities.loopback = true;
        for (i, row) in state.loopback.iter_mut().enumerate() {
            row.gain_db = -(i as f32) * 3.0;
            row.pan = i as f32 * 10.0;
        }
        apply(&ui, state.clone());
        let writes = Rc::new(std::cell::Cell::new(0));
        for_callback_count(&ui, writes.clone());
        for i in 0..6 {
            ui.invoke_select_source(i);
            assert_eq!(ui.get_selected_loopback_gain(), state.loopback[i as usize].gain_db);
            assert_eq!(ui.get_selected_loopback_pan(), state.loopback[i as usize].pan);
        }
        state.loopback[5].gain_db = -22.5;
        state.loopback[5].pan = -67.0;
        apply(&ui, state.clone());
        assert_eq!(ui.get_selected_loopback_gain(), -22.5);
        assert_eq!(ui.get_selected_loopback_pan(), -67.0);
        ui.invoke_select_source(0);
        assert_eq!(ui.get_selected_loopback_gain(), 0.0);
        assert_eq!(ui.get_selected_loopback_pan(), 0.0);
        assert_eq!(writes.get(), 0, "Navigation must not apply a saved route");
        fn for_callback_count(ui: &AppWindow, writes: Rc<std::cell::Cell<i32>>) {
            let gains = writes.clone();
            ui.on_loopback_gain_edited(move |_, _| gains.set(gains.get() + 1));
            let pans = writes.clone();
            ui.on_loopback_pan_edited(move |_, _| pans.set(pans.get() + 1));
            ui.on_loopback_mute_edited(move |_, _| writes.set(writes.get() + 1));
        }
    }
    #[test]
    fn native_ui_phantom_confirmation_and_source_navigation() {
        let ui = AppWindow::new().unwrap();
        let mut state = Snapshot::default();
        state.connected = true;
        state.capabilities.phantom = true;
        apply(&ui, state);
        let calls = Rc::new(std::cell::Cell::new(0));
        {
            let calls = calls.clone();
            ui.on_phantom_edited(move |_, _| calls.set(calls.get() + 1));
        }
        ui.invoke_request_phantom(0);
        assert_eq!(ui.get_phantom_confirmation_pending(), 0);
        assert_eq!(calls.get(), 0);
        ui.invoke_reset_transient();
        assert_eq!(ui.get_phantom_confirmation_pending(), -1);
        ui.invoke_request_phantom(0);
        assert_eq!(ui.get_phantom_confirmation_pending(), 0);
        assert_eq!(
            calls.get(),
            0,
            "confirmation from the prior connection must not be reused"
        );
        ui.invoke_request_phantom(0);
        assert_eq!(calls.get(), 1);
        assert_eq!(ui.get_phantom_confirmation_pending(), -1);
        selecting_loopback_source_is_read_only_and_tracks_authoritative_updates(&ui);
    }
    /// Run manually with native input automation. This creates only a labeled
    /// UI test window: no driver is loaded and no physical device is opened.
    #[test]
    #[ignore = "interactive UI-only readback regression; requires native slider input"]
    fn native_ui_readback_roundtrip() {
        use crate::model::{Capabilities, InputState, LoopbackState};
        let ui = AppWindow::new().unwrap();
        let state = Rc::new(RefCell::new(Snapshot {
            connected: true,
            generation: 100,
            device_name: "UI TEST · NO HARDWARE".into(),
            status: "UI-only simulated state · exercise each slider twice".into(),
            output_db: -20.0,
            monitor_blend: 50.0,
            inputs: [
                InputState {
                    gain_db: 10.0,
                    ..InputState::default()
                },
                InputState {
                    gain_db: 12.0,
                    ..InputState::default()
                },
            ],
            loopback: std::array::from_fn(|_| LoopbackState {
                known: true,
                ..LoopbackState::default()
            }),
            loopback_provenance: "SIMULATED · no hardware routing".into(),
            capabilities: Capabilities {
                gain: true,
                phantom: true,
                output_volume: true,
                monitor_blend: true,
                monitor_pan: true,
                loopback: true,
                ..Capabilities::default()
            },
            ..Snapshot::default()
        }));
        apply(&ui, state.borrow().clone());
        let counts = Rc::new(RefCell::new([0u8; 17]));
        let reports = Rc::new(RefCell::new(Vec::<serde_json::Value>::new()));
        let targets: std::collections::BTreeSet<usize> = std::env::var("EVO_UI_TEST_TARGETS")
            .ok()
            .map(|s| s.split(',').map(|n| n.parse::<usize>().unwrap()).collect())
            .unwrap_or_else(|| (0..17).collect());
        assert!(!targets.is_empty() && targets.iter().all(|&key| key < 17));
        let expected_cases = targets.len() * 2;
        fn observed(ui: &AppWindow, key: usize) -> f32 {
            match key {
                0 | 1 => ui.get_inputs().row_data(key).unwrap().gain,
                2 => ui.get_output_db(),
                3 => ui.get_monitor_blend(),
                4 => ui.get_monitor_pan(),
                _ => {
                    let row = ui.get_loopback().row_data((key - 5) / 2).unwrap();
                    if key % 2 == 1 { row.gain } else { row.pan }
                }
            }
        }
        fn value(state: &Snapshot, key: usize) -> f32 {
            match key {
                0 | 1 => state.inputs[key].gain_db,
                2 => state.output_db,
                3 => state.monitor_blend,
                4 => state.monitor_pan,
                _ => {
                    let row = &state.loopback[(key - 5) / 2];
                    if key % 2 == 1 { row.gain_db } else { row.pan }
                }
            }
        }
        fn assign(state: &mut Snapshot, key: usize, v: f32) {
            match key {
                0 | 1 => state.inputs[key].gain_db = v,
                2 => state.output_db = v,
                3 => state.monitor_blend = v,
                4 => state.monitor_pan = v,
                _ => {
                    let row = &mut state.loopback[(key - 5) / 2];
                    if key % 2 == 1 {
                        row.gain_db = v
                    } else {
                        row.pan = v
                    }
                }
            }
        }
        let handler = {
            let state = state.clone();
            let counts = counts.clone();
            let reports = reports.clone();
            let weak = ui.as_weak();
            Rc::new(move |key: usize, requested: f32| {
                if !targets.contains(&key) { return; }
                let ui = weak.upgrade().unwrap();
                let before = observed(&ui, key);
                assert!(
                    (before - requested).abs() < 0.01,
                    "two-way input did not reach model: {key}: {before} != {requested}"
                );
                let phase = counts.borrow()[key];
                if phase >= 2 {
                    return;
                }
                counts.borrow_mut()[key] += 1;
                let prior = value(&state.borrow(), key);
                let expected = if phase == 0 {
                    prior
                } else {
                    [
                        17.0, 19.0, -35.0, 65.0, 25.0, -25.0, 30.0, -24.0, 31.0, -23.0, 32.0,
                        -22.0, 33.0, -21.0, 34.0, -20.0, 35.0,
                    ][key]
                };
                assign(&mut state.borrow_mut(), key, expected);
                state.borrow_mut().status = format!(
                    "UI ONLY · control {key} · simulated {}",
                    if phase == 0 {
                        "failed-write rollback"
                    } else {
                        "external state change"
                    }
                );
                let snapshot = state.borrow().clone();
                let weak = weak.clone();
                let reports = reports.clone();
                slint::Timer::single_shot(Duration::from_millis(150), move || {
                    let ui = weak.upgrade().unwrap();
                    apply(&ui, snapshot);
                    let after = observed(&ui, key);
                    assert!(
                        (after - expected).abs() < 0.01,
                        "authoritative state did not reach UI model: {key}: {after} != {expected}"
                    );
                    reports.borrow_mut().push(serde_json::json!({"control":key,"phase":phase,"requested":requested,"prior":prior,"authoritative":expected,"observed":after}));
                    let evidence = serde_json::json!({"kind":"UI-only simulated state; no hardware API calls","cases":*reports.borrow()});
                    std::fs::write(
                        "evidence/dense-ui-readback.json",
                        serde_json::to_vec_pretty(&evidence).unwrap(),
                    )
                    .unwrap();
                    if reports.borrow().len() == expected_cases {
                        slint::quit_event_loop().unwrap();
                    }
                });
            })
        };
        {
            let h = handler.clone();
            ui.on_gain_edited(move |c, v| h(c as usize, v));
        }
        {
            let h = handler.clone();
            ui.on_output_edited(move |v| h(2, v));
        }
        {
            let h = handler.clone();
            ui.on_blend_edited(move |v| h(3, v));
        }
        {
            let h = handler.clone();
            ui.on_monitor_pan_edited(move |v| h(4, v));
        }
        {
            let h = handler.clone();
            ui.on_loopback_gain_edited(move |c, v| h(5 + c as usize * 2, v));
        }
        {
            let h = handler.clone();
            ui.on_loopback_pan_edited(move |c, v| h(6 + c as usize * 2, v));
        }
        slint::Timer::single_shot(Duration::from_secs(900), || {
            slint::quit_event_loop().unwrap()
        });
        ui.run().unwrap();
        assert_eq!(
            reports.borrow().len(),
            expected_cases,
            "exercise each selected control twice before closing the test window"
        );
    }
    #[test]
    fn coalesces_only_same_continuous_target() {
        assert!(same_target(
            &Command::SetGain {
                channel: 0,
                db: 1.0
            },
            &Command::SetGain {
                channel: 0,
                db: 2.0
            }
        ));
        assert!(!same_target(
            &Command::SetGain {
                channel: 0,
                db: 1.0
            },
            &Command::SetGain {
                channel: 1,
                db: 2.0
            }
        ));
        assert!(!same_target(
            &Command::SetOutputMute(true),
            &Command::SetOutputMute(false)
        ));
    }
    #[test]
    fn fader_burst_retains_latest_value_and_discrete_actions() {
        let mut queue = VecDeque::new();
        assert!(queue_command(&mut queue, Command::SetOutputMute(true)));
        for n in 0..1000 {
            assert!(queue_command(
                &mut queue,
                Command::SetOutputVolume(-(n as f32))
            ));
        }
        assert!(queue_command(&mut queue, Command::SetOutputMute(false)));
        assert_eq!(queue.len(), 3);
        assert!(matches!(queue[0], Command::SetOutputMute(true)));
        assert!(matches!(queue[1], Command::SetOutputVolume(v) if v == -999.0));
        assert!(matches!(queue[2], Command::SetOutputMute(false)));
    }
    #[test]
    fn busy_queue_is_bounded_and_reports_rejection() {
        let mut queue = VecDeque::new();
        for _ in 0..64 {
            assert!(queue_command(&mut queue, Command::Refresh));
        }
        assert!(!queue_command(&mut queue, Command::Refresh));
        assert_eq!(queue.len(), 64);
    }
}
