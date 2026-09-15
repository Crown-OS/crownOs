//! crowndictator — push-to-talk dictation for Wayland.
//!
//! Hold the shortcut, speak, release: the recording is transcribed locally
//! with NVIDIA Parakeet TDT 0.6B v2 (ONNX Runtime, CUDA when available)
//! and typed into the focused window. A small waveform pill rendered
//! through crownshell sits at the bottom of the screen while it listens
//! and thinks.
//!
//! The shortcut — along with the switch that turns the whole thing off, which
//! microphone to record from, and whether to use the GPU — comes from
//! `~/.config/crownos/input.ron`, which the settings panel's Input page writes
//! and this daemon follows live. See [`settings`].
//!
//! Debug modes:
//!   crowndictator --demo              cycle overlay states with fake audio
//!   crowndictator --transcribe f.wav  run the ASR pipeline on a wav file
//!   crowndictator --cpu               skip CUDA even if available

mod asr;
mod audio;
mod controller;
mod hotkey;
mod inject;
mod settings;
mod state;
mod wav;
mod waveform;

use anyhow::{Context, Result, anyhow};
use crownshell::{Anchor, KeyboardInteractivity, Layer, WindowConfig};
use smithay_client_toolkit::compositor::Region;
use smithay_client_toolkit::shell::WaylandSurface;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let force_cpu = args.iter().any(|a| a == "--cpu");
    let demo = args.iter().any(|a| a == "--demo");

    if let Some(i) = args.iter().position(|a| a == "--transcribe") {
        let path = args
            .get(i + 1)
            .ok_or_else(|| anyhow!("--transcribe needs a wav path"))?;
        let samples = wav::read_wav_16k(std::path::Path::new(path))?;
        log::info!("loaded {:.1}s of audio", samples.len() as f32 / 16_000.0);
        // One shot, so the backend is decided here and now: the flag, else
        // whatever the user configured for the daemon.
        let prefer_gpu = !force_cpu && settings::load().dictation_gpu;
        let mut engine = asr::Engine::load(prefer_gpu)?;
        log::info!(
            "engine backend: {}",
            if engine.gpu { "CUDA" } else { "CPU" }
        );
        println!("{}", engine.transcribe(&samples)?);
        return Ok(());
    }

    let shared = state::new_shared();
    let (wake_tx, wake_rx) = calloop::channel::channel::<()>();

    if demo {
        controller::spawn_demo(shared.clone(), wake_tx);
    } else {
        // The controller says what it is listening for once it has read
        // `input.ron` — there is no shortcut to name from out here.
        controller::spawn(shared.clone(), wake_tx, controller::Options { force_cpu });
    }

    crownshell::run(move |app| {
        app.create_window(
            WindowConfig {
                namespace: "crowndictator".into(),
                layer: Layer::Overlay,
                anchor: Anchor::BOTTOM,
                size: (waveform::WIN_W, waveform::WIN_H),
                exclusive_zone: 0,
                keyboard_interactivity: KeyboardInteractivity::None,
                blur: false,
                auto_blur_region: false,
                tick_interval: None,
            },
            waveform::Overlay::new(shared),
        );

        // The overlay is purely decorative: let clicks pass through it.
        let window = app.windows.last().expect("window just created");
        let region = Region::new(&app.compositor_state).context("create empty input region")?;
        window
            .layer
            .wl_surface()
            .set_input_region(Some(region.wl_region()));
        window.layer.commit();

        app.loop_handle
            .insert_source(wake_rx, |event, _, app| {
                if let calloop::channel::Event::Msg(()) = event {
                    let crownshell::App {
                        compositor_state,
                        qh,
                        windows,
                        text_cx,
                        ..
                    } = app;
                    for window in windows.iter_mut() {
                        window.request_frame(compositor_state, qh, text_cx);
                    }
                }
            })
            .map_err(|e| anyhow!("insert wake channel: {e}"))?;
        Ok(())
    })
}
