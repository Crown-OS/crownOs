//! Minimal WAV reader used by the `--transcribe` debug mode.

use std::path::Path;

use anyhow::{Context, Result, anyhow};

use crate::audio::resample_to_16k;

/// Read a PCM16/PCM32/float32 WAV, downmix to mono, resample to 16 kHz.
pub fn read_wav_16k(path: &Path) -> Result<Vec<f32>> {
    let data = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    if data.len() < 44 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err(anyhow!("not a RIFF/WAVE file"));
    }
    let mut pos = 12;
    let mut format = 0u16;
    let mut channels = 1u16;
    let mut rate = 16_000u32;
    let mut bits = 16u16;
    let mut samples: Option<Vec<f32>> = None;

    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let size = u32::from_le_bytes(data[pos + 4..pos + 8].try_into()?) as usize;
        let body = &data[pos + 8..(pos + 8 + size).min(data.len())];
        match id {
            b"fmt " if body.len() >= 16 => {
                format = u16::from_le_bytes(body[0..2].try_into()?);
                channels = u16::from_le_bytes(body[2..4].try_into()?).max(1);
                rate = u32::from_le_bytes(body[4..8].try_into()?);
                bits = u16::from_le_bytes(body[14..16].try_into()?);
            }
            b"data" => {
                let mono: Vec<f32> = match (format, bits) {
                    (1, 16) => body
                        .chunks_exact(2 * channels as usize)
                        .map(|frame| {
                            frame
                                .chunks_exact(2)
                                .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                                .sum::<f32>()
                                / channels as f32
                        })
                        .collect(),
                    (1, 32) => body
                        .chunks_exact(4 * channels as usize)
                        .map(|frame| {
                            frame
                                .chunks_exact(4)
                                .map(|c| {
                                    i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32
                                        / 2147483648.0
                                })
                                .sum::<f32>()
                                / channels as f32
                        })
                        .collect(),
                    (3, 32) => body
                        .chunks_exact(4 * channels as usize)
                        .map(|frame| {
                            frame
                                .chunks_exact(4)
                                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                                .sum::<f32>()
                                / channels as f32
                        })
                        .collect(),
                    _ => return Err(anyhow!("unsupported wav: format={format} bits={bits}")),
                };
                samples = Some(mono);
            }
            _ => {}
        }
        pos += 8 + size + (size & 1);
    }

    let samples = samples.ok_or_else(|| anyhow!("wav has no data chunk"))?;
    Ok(resample_to_16k(&samples, rate))
}
