//! Microphone capture (cpal) and offline resampling to 16 kHz mono.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::state::{Shared, push_wave};

/// Samples-per-envelope-point pushed to the visualizer. ~21 ms at 48 kHz,
/// so the overlay's 56-slot history spans a little over a second.
const VIZ_BLOCK: usize = 1024;
/// Hard cap on the recording (5 min at 48 kHz ≈ 55 MB of f32).
const MAX_SAMPLES: usize = 48_000 * 300;

pub struct Recording {
    stream: cpal::Stream,
    buf: Arc<Mutex<Vec<f32>>>,
    rate: u32,
}

impl Recording {
    /// Stop capturing and return (mono samples, sample rate).
    pub fn stop(self) -> (Vec<f32>, u32) {
        drop(self.stream);
        let samples = std::mem::take(&mut *self.buf.lock().unwrap());
        (samples, self.rate)
    }
}

/// Open an input device and start capturing mono audio.
/// Envelope/level data is streamed into `shared` for the overlay.
///
/// `wanted` is the device name from `input.ron`, or `None` for the system
/// default — see [`open`] for what happens when the named one isn't there.
pub fn start(shared: Shared, wanted: Option<&str>) -> Result<Recording> {
    let host = cpal::default_host();
    let device = open(&host, wanted)?;
    let config = device
        .default_input_config()
        .context("query default input config")?;
    let channels = config.channels() as usize;
    let rate = config.sample_rate().0;
    log::info!(
        "audio: capturing from '{}' @ {} Hz, {} ch, {:?}",
        device.name().unwrap_or_default(),
        rate,
        channels,
        config.sample_format()
    );

    let buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(rate as usize * 8)));
    let cb_buf = buf.clone();
    // Envelope accumulator carried across callbacks.
    let mut acc = 0.0f32;
    let mut acc_n = 0usize;

    let mut on_mono = move |mono: &mut dyn Iterator<Item = f32>| {
        let mut buf = cb_buf.lock().unwrap();
        for s in mono {
            if buf.len() < MAX_SAMPLES {
                buf.push(s);
            }
            acc += s * s;
            acc_n += 1;
            if acc_n >= VIZ_BLOCK {
                let rms = (acc / acc_n as f32).sqrt();
                // Perceptual-ish curve; speech RMS is usually well below 1.
                push_wave(&shared, (rms * 9.0).powf(0.7).min(1.0));
                acc = 0.0;
                acc_n = 0;
            }
        }
    };

    let err_cb = |e| log::error!("audio: stream error: {e}");
    let stream_config: cpal::StreamConfig = config.clone().into();

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                on_mono(&mut data.chunks(channels).map(|f| {
                    f.iter().sum::<f32>() / channels as f32
                }));
            },
            err_cb,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _| {
                on_mono(&mut data.chunks(channels).map(|f| {
                    f.iter().map(|&v| v as f32 / 32768.0).sum::<f32>() / channels as f32
                }));
            },
            err_cb,
            None,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            &stream_config,
            move |data: &[u16], _| {
                on_mono(&mut data.chunks(channels).map(|f| {
                    f.iter()
                        .map(|&v| (v as f32 - 32768.0) / 32768.0)
                        .sum::<f32>()
                        / channels as f32
                }));
            },
            err_cb,
            None,
        )?,
        fmt => return Err(anyhow!("unsupported sample format {fmt:?}")),
    };
    stream.play().context("start input stream")?;

    Ok(Recording { stream, buf, rate })
}

/// The input device to record from: the one named in `input.ron`, or the
/// system default.
///
/// A configured name that matches nothing falls back to the default rather than
/// failing. The device is a USB microphone that is currently unplugged far more
/// often than it is a typo, and refusing to record at all would be a strange
/// way to react to a headset being in the other room — the settings panel
/// already says so on the Input page, and the name stays in the file so it is
/// used again the moment the device is back.
fn open(host: &cpal::Host, wanted: Option<&str>) -> Result<cpal::Device> {
    let Some(wanted) = wanted else {
        return host
            .default_input_device()
            .ok_or_else(|| anyhow!("no default audio input device"));
    };

    let found = host
        .input_devices()
        .context("enumerate audio input devices")?
        .find(|device| device.name().is_ok_and(|name| name == wanted));

    match found {
        Some(device) => Ok(device),
        None => {
            log::warn!("audio: '{wanted}' is not connected; using the default input device");
            host.default_input_device()
                .ok_or_else(|| anyhow!("'{wanted}' is not connected and there is no default input device"))
        }
    }
}

/// Resample mono audio to 16 kHz with a windowed-sinc kernel.
pub fn resample_to_16k(samples: &[f32], rate: u32) -> Vec<f32> {
    const TARGET: f64 = 16_000.0;
    if rate == 16_000 || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = rate as f64 / TARGET; // input samples per output sample
    let out_len = (samples.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);

    // Low-pass cutoff at the lower Nyquist, slightly under to reduce aliasing.
    let cutoff = (TARGET / rate as f64).min(1.0) * 0.92;
    let half_taps = if ratio > 1.0 { (12.0 * ratio).ceil() as isize } else { 12 };

    for i in 0..out_len {
        let center = i as f64 * ratio;
        let c0 = center.floor() as isize;
        let mut sum = 0.0f64;
        let mut wsum = 0.0f64;
        for k in (c0 - half_taps)..=(c0 + half_taps) {
            if k < 0 || k as usize >= samples.len() {
                continue;
            }
            let x = (k as f64 - center) * cutoff;
            let sinc = if x.abs() < 1e-9 {
                1.0
            } else {
                (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
            };
            // Hann window over the kernel extent.
            let w = 0.5
                + 0.5
                    * (std::f64::consts::PI * (k as f64 - center) / (half_taps as f64 + 1.0))
                        .cos();
            let coeff = sinc * w * cutoff;
            sum += samples[k as usize] as f64 * coeff;
            wsum += coeff;
        }
        out.push(if wsum.abs() > 1e-12 { (sum / wsum) as f32 } else { 0.0 });
    }
    out
}
