//! Messages between the native window and its serialized hardware worker.
#[derive(Debug, Clone)]
pub enum Command {
    Refresh,
    SetGain { channel: u8, db: f32 },
    SetInputMute { channel: u8, muted: bool },
    SetPhantom { channel: u8, enabled: bool },
    SetLinked(bool),
    StartSmartgain { channel_mask: u8 },
    StopSmartgain,
    SetOutputVolume(f32),
    SetOutputMute(bool),
    SetMonitorBlend(f32),
    SetMonitorPan(f32),
    SetLoopbackGain { source: u8, gain_db: f32 },
    SetLoopbackPan { source: u8, pan: f32 },
    SetLoopbackMute { source: u8, muted: bool },
    SetSampleRate(u32),
    SetBufferSize(u32),
}

#[derive(Debug)]
pub struct Request {
    pub generation: u64,
    pub command: Command,
}

#[derive(Debug, Clone, Default)]
pub struct Capabilities {
    pub gain: bool,
    pub input_mute: bool,
    pub phantom: bool,
    pub link: bool,
    pub smartgain: bool,
    pub output_volume: bool,
    pub output_mute: bool,
    pub monitor_blend: bool,
    pub monitor_pan: bool,
    pub loopback: bool,
    pub sample_rate: bool,
    pub buffer_size: bool,
    pub meters: bool,
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub gain_db: f32,
    pub muted: bool,
    pub phantom: bool,
    /// Normalized peak 0..1; supplied only if meters capability is true.
    pub peak: f32,
}
impl Default for InputState {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            muted: false,
            phantom: false,
            peak: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoopbackState {
    pub gain_db: f32,
    pub pan: f32,
    pub muted: bool,
    pub known: bool,
}
impl Default for LoopbackState {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            pan: 0.0,
            muted: false,
            known: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub generation: u64,
    pub connected: bool,
    pub device_name: String,
    pub status: String,
    pub error: bool,
    pub driver_version: String,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub supported_sample_rates: Vec<u32>,
    pub supported_buffers: Vec<u32>,
    pub inputs: [InputState; 2],
    /// Main playback-source meter positions, before any assumed output attenuation.
    pub playback_peaks: [f32; 2],
    pub linked: bool,
    pub output_db: f32,
    pub output_muted: bool,
    /// 0=input, 100=playback.
    pub monitor_blend: f32,
    /// -100=left, 100=right.
    pub monitor_pan: f32,
    pub loopback: [LoopbackState; 6],
    pub smartgain_status: String,
    pub smartgain_running: bool,
    pub loopback_provenance: String,
    pub capabilities: Capabilities,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            generation: 0,
            connected: false,
            device_name: "EVO 4".into(),
            status: "Connecting to your interface…".into(),
            error: false,
            driver_version: String::new(),
            sample_rate: 0,
            buffer_size: 0,
            supported_sample_rates: Vec::new(),
            supported_buffers: Vec::new(),
            inputs: std::array::from_fn(|_| InputState::default()),
            playback_peaks: [0.0; 2],
            linked: false,
            output_db: -60.0,
            output_muted: false,
            monitor_blend: 50.0,
            monitor_pan: 0.0,
            loopback: std::array::from_fn(|_| LoopbackState::default()),
            smartgain_status: "Automatic input level".into(),
            capabilities: Capabilities::default(),
            smartgain_running: false,
            loopback_provenance: "Routing unavailable".into(),
        }
    }
}
