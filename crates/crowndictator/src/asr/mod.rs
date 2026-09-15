//! Local Parakeet TDT 0.6B v2 inference via ONNX Runtime.
//!
//! Pipeline: 16 kHz mono f32 → nemo128 mel preprocessor → conformer encoder
//! → greedy token-and-duration transducer (TDT) decode with the
//! decoder+joint network → SentencePiece detokenization.
//!
//! The [`spawn`] engine thread owns the sessions, preloads on demand and
//! drops everything (freeing VRAM/RAM) after an idle timeout.

mod download;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use ort::ep::CUDAExecutionProvider;
use ort::inputs;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::{Tensor, ValueType};

/// Durations head of this TDT model predicts an index into [0,1,2,3,4],
/// i.e. the argmax index *is* the frame step.
const MAX_TOKENS_PER_STEP: usize = 10;

pub struct Engine {
    preproc: Session,
    encoder: Session,
    dj: Session,
    vocab: Vec<String>,
    blank: usize,
    vocab_size: usize,
    targets_i32: bool,
    target_len_i32: bool,
    /// (num_layers, hidden) for the two LSTM states.
    s1_shape: (usize, usize),
    s2_shape: (usize, usize),
    pub gpu: bool,
}

/// ort builder errors carry the builder for recovery and are !Send, so they
/// can't cross into anyhow via `?` — strip them down to their message.
fn oerr<R>(e: ort::Error<R>) -> anyhow::Error {
    anyhow!("{e}")
}

fn session_for(path: &std::path::Path, gpu: bool) -> Result<Session> {
    let mut builder = Session::builder()
        .map_err(oerr)?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(oerr)?
        .with_intra_threads(std::thread::available_parallelism().map_or(4, |n| n.get().min(8)))
        .map_err(oerr)?;
    if gpu {
        builder = builder
            .with_execution_providers([CUDAExecutionProvider::default().build().error_on_failure()])
            .map_err(oerr)?;
    }
    builder.commit_from_file(path).map_err(oerr)
}

/// Probe whether the CUDA EP can actually initialize (driver + cuDNN present)
/// by building a session for the small preprocessor model on the GPU.
fn cuda_usable(probe_model: &std::path::Path) -> bool {
    match session_for(probe_model, true) {
        Ok(_) => true,
        Err(e) => {
            log::warn!("asr: CUDA unavailable ({e}); falling back to CPU int8");
            false
        }
    }
}

fn tensor_meta(sess: &Session, name: &str) -> Option<(ort::value::TensorElementType, Vec<i64>)> {
    sess.inputs()
        .iter()
        .find(|o| o.name() == name)
        .and_then(|o| match o.dtype() {
            ValueType::Tensor { ty, shape, .. } => Some((*ty, shape.to_vec())),
            _ => None,
        })
}

impl Engine {
    pub fn load(prefer_gpu: bool) -> Result<Self> {
        let t0 = Instant::now();
        let (preproc_path, vocab_path) = download::fetch_small()?;
        let gpu = prefer_gpu && cuda_usable(&preproc_path);
        let paths = download::fetch(!gpu)?;

        // Vocab lines are "<token> <id>"; SentencePiece '▁' marks word starts.
        let mut vocab_pairs: Vec<(usize, String)> = std::fs::read_to_string(&vocab_path)
            .context("read vocab.txt")?
            .lines()
            .filter_map(|line| {
                let (tok, id) = line.rsplit_once(' ')?;
                Some((id.parse().ok()?, tok.replace('\u{2581}', " ")))
            })
            .collect();
        vocab_pairs.sort_by_key(|(id, _)| *id);
        let vocab: Vec<String> = vocab_pairs.into_iter().map(|(_, t)| t).collect();
        let blank = vocab
            .iter()
            .position(|t| t == "<blk>")
            .ok_or_else(|| anyhow!("vocab has no <blk> token"))?;

        log::info!(
            "asr: loading sessions ({}), vocab={} tokens",
            if gpu { "CUDA fp32" } else { "CPU int8" },
            vocab.len()
        );
        let preproc = session_for(&paths.preprocessor, false)?;
        let encoder = session_for(&paths.encoder, gpu)?;
        let dj = session_for(&paths.decoder_joint, gpu)?;

        let (targets_ty, _) = tensor_meta(&dj, "targets")
            .ok_or_else(|| anyhow!("decoder_joint: no `targets` input"))?;
        let (tlen_ty, _) = tensor_meta(&dj, "target_length")
            .ok_or_else(|| anyhow!("decoder_joint: no `target_length` input"))?;
        let state_shape = |name: &str| -> (usize, usize) {
            match tensor_meta(&dj, name) {
                Some((_, dims)) if dims.len() == 3 && dims[0] > 0 && dims[2] > 0 => {
                    (dims[0] as usize, dims[2] as usize)
                }
                _ => (2, 640), // parakeet-tdt-0.6b-v2 decoder LSTM
            }
        };
        let s1_shape = state_shape("input_states_1");
        let s2_shape = state_shape("input_states_2");

        let vocab_size = vocab.len();
        let mut engine = Self {
            preproc,
            encoder,
            dj,
            s1_shape,
            s2_shape,
            targets_i32: matches!(targets_ty, ort::value::TensorElementType::Int32),
            target_len_i32: matches!(tlen_ty, ort::value::TensorElementType::Int32),
            vocab,
            blank,
            vocab_size,
            gpu,
        };

        // Warmup pass primes CUDA kernels / cuDNN autotuning so the first
        // real dictation isn't slow.
        engine.transcribe(&vec![0.0f32; 8000])?;
        log::info!("asr: model ready in {:.1}s", t0.elapsed().as_secs_f32());
        Ok(engine)
    }

    /// Transcribe 16 kHz mono samples.
    pub fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        let t0 = Instant::now();
        let n = samples.len();
        if n < 1600 {
            return Ok(String::new());
        }

        // 1. Mel features.
        let pre_out = self.preproc.run(inputs![
            "waveforms" => Tensor::from_array((vec![1i64, n as i64], samples.to_vec()))?,
            "waveforms_lens" => Tensor::from_array((vec![1i64], vec![n as i64]))?,
        ])?;
        let (fshape, feats) = pre_out["features"].try_extract_tensor::<f32>()?;
        let (_, flens) = pre_out["features_lens"].try_extract_tensor::<i64>()?;
        let fdims: Vec<i64> = fshape.iter().copied().collect();
        let feats = feats.to_vec();
        let flen = flens[0];
        drop(pre_out);

        // 2. Encoder.
        let enc_out = self.encoder.run(inputs![
            "audio_signal" => Tensor::from_array((fdims, feats))?,
            "length" => Tensor::from_array((vec![1i64], vec![flen]))?,
        ])?;
        let (eshape, enc) = enc_out["outputs"].try_extract_tensor::<f32>()?;
        let (_, elens) = enc_out["encoded_lengths"].try_extract_tensor::<i64>()?;
        // outputs: (1, D, T)
        let d = eshape[1] as usize;
        let t_total = eshape[2] as usize;
        let t_len = (elens[0] as usize).min(t_total);
        let enc = enc.to_vec();
        drop(enc_out);

        // 3. Greedy TDT decode.
        let mut s1 = vec![0f32; self.s1_shape.0 * self.s1_shape.1];
        let mut s2 = vec![0f32; self.s2_shape.0 * self.s2_shape.1];
        let mut tokens: Vec<usize> = Vec::new();
        let mut t = 0usize;
        let mut emitted = 0usize;

        while t < t_len {
            let frame: Vec<f32> = (0..d).map(|di| enc[di * t_total + t]).collect();
            let last = *tokens.last().unwrap_or(&self.blank) as i64;

            let mut dj_in = inputs![
                "encoder_outputs" => Tensor::from_array((vec![1i64, d as i64, 1], frame))?,
                "input_states_1" => Tensor::from_array((
                    vec![self.s1_shape.0 as i64, 1, self.s1_shape.1 as i64], s1.clone()))?,
                "input_states_2" => Tensor::from_array((
                    vec![self.s2_shape.0 as i64, 1, self.s2_shape.1 as i64], s2.clone()))?,
            ];
            if self.targets_i32 {
                dj_in.push((
                    "targets".into(),
                    Tensor::from_array((vec![1i64, 1], vec![last as i32]))?.into(),
                ));
            } else {
                dj_in.push((
                    "targets".into(),
                    Tensor::from_array((vec![1i64, 1], vec![last]))?.into(),
                ));
            }
            if self.target_len_i32 {
                dj_in.push((
                    "target_length".into(),
                    Tensor::from_array((vec![1i64], vec![1i32]))?.into(),
                ));
            } else {
                dj_in.push((
                    "target_length".into(),
                    Tensor::from_array((vec![1i64], vec![1i64]))?.into(),
                ));
            }

            let out = self.dj.run(dj_in)?;
            let (_, logits) = out["outputs"].try_extract_tensor::<f32>()?;
            debug_assert!(logits.len() > self.vocab_size);

            let token = argmax(&logits[..self.vocab_size]);
            let step = argmax(&logits[self.vocab_size..]);

            if token != self.blank {
                let (_, ns1) = out["output_states_1"].try_extract_tensor::<f32>()?;
                let (_, ns2) = out["output_states_2"].try_extract_tensor::<f32>()?;
                s1.copy_from_slice(ns1);
                s2.copy_from_slice(ns2);
                tokens.push(token);
                emitted += 1;
            }

            if step > 0 {
                t += step;
                emitted = 0;
            } else if token == self.blank || emitted >= MAX_TOKENS_PER_STEP {
                t += 1;
                emitted = 0;
            }
        }

        // 4. Detokenize.
        let mut text = String::new();
        for tok in &tokens {
            text.push_str(&self.vocab[*tok]);
        }
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        log::info!(
            "asr: transcribed {:.1}s of audio in {:.2}s ({} tokens)",
            n as f32 / 16_000.0,
            t0.elapsed().as_secs_f32(),
            tokens.len()
        );
        Ok(text)
    }
}

fn argmax(xs: &[f32]) -> usize {
    let mut best = 0;
    for (i, &v) in xs.iter().enumerate() {
        if v > xs[best] {
            best = i;
        }
    }
    best
}

pub enum AsrCmd {
    /// Start loading the model if it isn't resident yet.
    Preload,
    /// Drop the model if it is resident.
    ///
    /// Only the backend preference uses this: the model is chosen for CPU or
    /// GPU when it loads, so the way to honour that setting changing is to make
    /// the next command load it again.
    Unload,
    Transcribe {
        samples_16k: Vec<f32>,
        reply: Sender<Result<String>>,
    },
}

/// Spawn the engine thread. The model loads lazily on the first command and
/// is dropped after `idle_ttl` without work (deload), freeing GPU memory.
///
/// `prefer_gpu` is shared rather than copied because it comes from
/// `input.ron` and can change while the daemon runs. It is read at each load,
/// so a change takes effect on the next one — which [`AsrCmd::Unload`] is how
/// the controller brings forward.
pub fn spawn(prefer_gpu: Arc<AtomicBool>, idle_ttl: Duration) -> Sender<AsrCmd> {
    let (tx, rx): (Sender<AsrCmd>, Receiver<AsrCmd>) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("asr-engine".into())
        .spawn(move || {
            let mut engine: Option<Engine> = None;
            let ensure = |engine: &mut Option<Engine>| -> Result<()> {
                if engine.is_none() {
                    *engine = Some(Engine::load(prefer_gpu.load(Ordering::Relaxed))?);
                }
                Ok(())
            };
            loop {
                match rx.recv_timeout(idle_ttl) {
                    Ok(AsrCmd::Preload) => {
                        if let Err(e) = ensure(&mut engine) {
                            log::error!("asr: preload failed: {e:#}");
                        }
                    }
                    Ok(AsrCmd::Unload) => {
                        if engine.take().is_some() {
                            log::info!("asr: backend changed, model deloaded");
                        }
                    }
                    Ok(AsrCmd::Transcribe { samples_16k, reply }) => {
                        let result = ensure(&mut engine).and_then(|_| {
                            engine
                                .as_mut()
                                .expect("engine loaded")
                                .transcribe(&samples_16k)
                        });
                        if result.is_err() {
                            // A wedged session shouldn't poison later requests.
                            engine = None;
                        }
                        let _ = reply.send(result);
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if engine.take().is_some() {
                            log::info!("asr: idle for {idle_ttl:?}, model deloaded");
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        })
        .expect("spawn asr thread");
    tx
}
