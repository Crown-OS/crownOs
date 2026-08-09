//! Model file download/caching via the Hugging Face hub.

use std::path::PathBuf;

use anyhow::{Context, Result};
use hf_hub::api::sync::Api;

pub const REPO: &str = "istupakov/parakeet-tdt-0.6b-v2-onnx";

pub struct ModelPaths {
    pub preprocessor: PathBuf,
    pub encoder: PathBuf,
    pub decoder_joint: PathBuf,
}

fn repo() -> Result<hf_hub::api::sync::ApiRepo> {
    let api = Api::new().context("init hf-hub api")?;
    Ok(api.model(REPO.to_string()))
}

/// Fetch just the small files (preprocessor + vocab). Used both as the
/// CUDA probe model and to avoid pulling the big encoder before we know
/// which precision variant we want.
pub fn fetch_small() -> Result<(PathBuf, PathBuf)> {
    let repo = repo()?;
    let preprocessor = repo.get("nemo128.onnx").context("fetch nemo128.onnx")?;
    let vocab = repo.get("vocab.txt").context("fetch vocab.txt")?;
    Ok((preprocessor, vocab))
}

/// Fetch everything. `int8` selects the quantized variant (for CPU);
/// otherwise the fp32 variant (for GPU) is downloaded.
pub fn fetch(int8: bool) -> Result<ModelPaths> {
    let (preprocessor, _vocab) = fetch_small()?;
    let repo = repo()?;
    let (encoder, decoder_joint) = if int8 {
        log::info!("asr: fetching int8 model (first run downloads ~700 MB)");
        (
            repo.get("encoder-model.int8.onnx")?,
            repo.get("decoder_joint-model.int8.onnx")?,
        )
    } else {
        log::info!("asr: fetching fp32 model (first run downloads ~2.5 GB)");
        let enc = repo.get("encoder-model.onnx")?;
        // External weight data must sit next to the encoder graph.
        repo.get("encoder-model.onnx.data")?;
        (enc, repo.get("decoder_joint-model.onnx")?)
    };
    Ok(ModelPaths {
        preprocessor,
        encoder,
        decoder_joint,
    })
}
