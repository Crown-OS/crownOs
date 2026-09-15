//! Shared state between the controller/audio threads and the overlay UI.

use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Number of waveform envelope samples kept for visualization.
/// Two per rendered bar, so each bar shows a peak over its slice.
pub const WAVE_N: usize = 56;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// Overlay is fully retracted; UI idles (no frame callbacks).
    Hidden,
    /// Hotkey held, mic open.
    Listening,
    /// Utterance captured, model inference running.
    Thinking,
    /// Text injected successfully; the waveform settles then retracts.
    Success,
    /// Something failed; the waveform flashes red then retracts.
    Error,
}

pub struct VizState {
    pub phase: Phase,
    pub phase_since: Instant,
    /// Raw microphone RMS level, 0..1 (unsmoothed; the overlay smooths it).
    pub level: f32,
    /// Ring buffer of recent waveform envelope samples.
    pub wave: [f32; WAVE_N],
    pub wave_pos: usize,
}

pub type Shared = Arc<Mutex<VizState>>;

pub fn new_shared() -> Shared {
    Arc::new(Mutex::new(VizState {
        phase: Phase::Hidden,
        phase_since: Instant::now(),
        level: 0.0,
        wave: [0.0; WAVE_N],
        wave_pos: 0,
    }))
}

pub fn set_phase(shared: &Shared, phase: Phase) {
    let mut s = shared.lock().unwrap();
    if s.phase != phase {
        s.phase = phase;
        s.phase_since = Instant::now();
    }
    if phase == Phase::Listening {
        s.level = 0.0;
        s.wave = [0.0; WAVE_N];
    }
}

/// Push one waveform envelope sample (audio thread).
pub fn push_wave(shared: &Shared, v: f32) {
    let mut s = shared.lock().unwrap();
    let pos = s.wave_pos;
    s.wave[pos] = v;
    s.wave_pos = (pos + 1) % WAVE_N;
    s.level = v;
}

/// Copy the wave ring out in chronological order (oldest → newest).
pub fn wave_snapshot(s: &VizState) -> [f32; WAVE_N] {
    let mut out = [0.0; WAVE_N];
    for i in 0..WAVE_N {
        out[i] = s.wave[(s.wave_pos + i) % WAVE_N];
    }
    out
}
