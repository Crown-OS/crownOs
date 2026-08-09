//! Orchestrates hotkey → audio → ASR → text injection, drives the overlay, and
//! follows `input.ron` while it does.
//!
//! # One loop, two kinds of news
//!
//! The controller thread hears about two things: the chord going down or coming
//! up, and the settings changing underneath it. Both arrive on the same channel
//! as an [`Event`], which is what makes "the user switched dictation off while
//! holding the chord" an ordinary sequence of messages rather than a race
//! between two threads. The settings are only ever read here, on this thread,
//! between events — so there is no lock around them and no moment where half of
//! a change has been applied.
//!
//! Each of the four settings lands somewhere different, and when:
//!
//! * the switch and the shortcut go to the hotkey watcher immediately, because
//!   they decide whether events happen at all;
//! * the microphone is read at the start of each recording, because that is the
//!   only moment it means anything;
//! * the backend preference goes to the ASR thread, which picks it up the next
//!   time it loads the model — brought forward by unloading it now.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use calloop::channel::Sender as WakeSender;
use crownos_config::schema::Input;

use crate::asr::{self, AsrCmd};
use crate::audio;
use crate::hotkey::{self, HotkeyEvent};
use crate::inject;
use crate::settings;
use crate::state::{Phase, Shared, set_phase};

/// Recordings shorter than this are treated as an accidental tap.
const MIN_UTTERANCE: Duration = Duration::from_millis(350);
/// Model is dropped from (V)RAM after this long without dictation.
const IDLE_TTL: Duration = Duration::from_secs(300);

pub struct Options {
    /// `--cpu`: never use the GPU, whatever `input.ron` says.
    ///
    /// A flag beats a preference, because the flag is what somebody typed on
    /// the command line for this run, and a settings file they may not even
    /// have open should not quietly override it.
    pub force_cpu: bool,
}

/// Everything the controller loop waits on.
enum Event {
    /// The configured chord went down or came back up.
    Hotkey(HotkeyEvent),
    /// `input.ron` changed. Carries the whole section — see [`crate::settings`].
    Settings(Input),
}

fn wake(ui: &WakeSender<()>) {
    let _ = ui.send(());
}

pub fn spawn(shared: Shared, ui: WakeSender<()>, opts: Options) {
    let mut input = settings::load();

    // Shared with the ASR thread, which reads it whenever it loads the model.
    let prefer_gpu = Arc::new(AtomicBool::new(gpu_wanted(&input, opts.force_cpu)));
    let asr_tx = asr::spawn(prefer_gpu.clone(), IDLE_TTL);

    let (tx, rx) = mpsc::channel::<Event>();
    let hotkeys = hotkey::spawn(
        {
            let tx = tx.clone();
            move |event| {
                let _ = tx.send(Event::Hotkey(event));
            }
        },
        input.dictation_hotkey,
        input.dictation_enabled,
    );
    // Held for the life of the thread: dropping it would stop the daemon
    // following the file it is configured by.
    let subscription = settings::watch(move |new| {
        let _ = tx.send(Event::Settings(new));
    });

    announce(&input);

    std::thread::Builder::new()
        .name("controller".into())
        .spawn(move || {
            let _subscription = subscription;
            let mut rec: Option<audio::Recording> = None;

            for event in rx {
                match event {
                    Event::Settings(new) => {
                        if new == input {
                            continue;
                        }
                        // The switch and the chord first: they decide whether
                        // there will be any more hotkey events at all, and
                        // switching off mid-utterance comes back around as a
                        // `Released` that stops the recording properly.
                        hotkeys.configure(new.dictation_hotkey, new.dictation_enabled);

                        let gpu = gpu_wanted(&new, opts.force_cpu);
                        if gpu != prefer_gpu.swap(gpu, Ordering::Relaxed) {
                            // The resident model was built for the other
                            // backend, so it is dropped now rather than left to
                            // serve the next utterance from the backend the
                            // user just turned off.
                            let _ = asr_tx.send(AsrCmd::Unload);
                        }

                        input = new;
                        announce(&input);
                    }

                    Event::Hotkey(HotkeyEvent::Pressed) => {
                        if rec.is_some() {
                            continue;
                        }
                        // Preload while the user is still speaking so the
                        // model is (being) loaded by the time they release.
                        let _ = asr_tx.send(AsrCmd::Preload);
                        match audio::start(shared.clone(), input.dictation_microphone.as_deref()) {
                            Ok(r) => {
                                rec = Some(r);
                                set_phase(&shared, Phase::Listening);
                            }
                            Err(e) => {
                                log::error!("audio: {e:#}");
                                set_phase(&shared, Phase::Error);
                            }
                        }
                        wake(&ui);
                    }

                    Event::Hotkey(HotkeyEvent::Released) => {
                        let Some(r) = rec.take() else { continue };
                        let (samples, rate) = r.stop();
                        let secs = samples.len() as f32 / rate as f32;
                        if secs < MIN_UTTERANCE.as_secs_f32() {
                            log::debug!("controller: {secs:.2}s tap ignored");
                            set_phase(&shared, Phase::Hidden);
                            wake(&ui);
                            continue;
                        }
                        set_phase(&shared, Phase::Thinking);
                        wake(&ui);

                        let samples_16k = audio::resample_to_16k(&samples, rate);
                        let (reply_tx, reply_rx) = mpsc::channel();
                        let _ = asr_tx.send(AsrCmd::Transcribe {
                            samples_16k,
                            reply: reply_tx,
                        });
                        // Generous timeout: the very first run may still be
                        // downloading the model.
                        let result = reply_rx
                            .recv_timeout(Duration::from_secs(600))
                            .map_err(anyhow::Error::from)
                            .and_then(|r| r);
                        match result {
                            Ok(text) if !text.trim().is_empty() => {
                                log::info!("controller: \"{text}\"");
                                match inject::type_text(text.trim()) {
                                    Ok(()) => set_phase(&shared, Phase::Success),
                                    Err(e) => {
                                        log::error!("inject: {e:#}");
                                        set_phase(&shared, Phase::Error);
                                    }
                                }
                            }
                            Ok(_) => {
                                log::info!("controller: nothing recognized");
                                set_phase(&shared, Phase::Hidden);
                            }
                            Err(e) => {
                                log::error!("asr: {e:#}");
                                set_phase(&shared, Phase::Error);
                            }
                        }
                        wake(&ui);
                    }
                }
            }
        })
        .expect("spawn controller thread");
}

/// Whether the model should be loaded onto the GPU.
fn gpu_wanted(input: &Input, force_cpu: bool) -> bool {
    !force_cpu && input.dictation_gpu
}

/// Say what the daemon is now doing, so the log answers "why is nothing
/// happening" without anybody opening the config file.
fn announce(input: &Input) {
    if !input.dictation_enabled {
        log::info!("dictation is switched off in input.ron");
    } else if input.dictation_hotkey.is_empty() {
        log::warn!("dictation is on but no shortcut is set in input.ron");
    } else {
        log::info!("hold {} to dictate", input.dictation_hotkey);
    }
}

/// `--demo`: cycle the overlay through its states with synthetic audio so
/// the visuals can be tuned/verified without speaking or loading the model.
pub fn spawn_demo(shared: Shared, ui: WakeSender<()>) {
    std::thread::Builder::new()
        .name("demo".into())
        .spawn(move || {
            let t0 = std::time::Instant::now();
            loop {
                set_phase(&shared, Phase::Listening);
                wake(&ui);
                let until = std::time::Instant::now() + Duration::from_secs(5);
                while std::time::Instant::now() < until {
                    // Syllable-rate bursts under a slower breath envelope, so
                    // the waveform has speech-like peaks and pauses.
                    let t = t0.elapsed().as_secs_f32();
                    let syll = ((t * 7.0).sin() * 0.5 + 0.5).powf(0.6);
                    let breath = ((t * 0.9).sin() * 0.5 + 0.5).powf(2.0);
                    crate::state::push_wave(&shared, (syll * breath * 1.4).min(1.0));
                    std::thread::sleep(Duration::from_millis(21));
                }
                set_phase(&shared, Phase::Thinking);
                wake(&ui);
                std::thread::sleep(Duration::from_secs(2));
                set_phase(&shared, Phase::Success);
                wake(&ui);
                std::thread::sleep(Duration::from_secs(3));
            }
        })
        .expect("spawn demo thread");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The flag is the stronger statement of the two.
    #[test]
    fn the_cpu_flag_overrides_the_preference() {
        let on = Input {
            dictation_gpu: true,
            ..Input::default()
        };
        let off = Input {
            dictation_gpu: false,
            ..Input::default()
        };

        assert!(gpu_wanted(&on, false));
        assert!(!gpu_wanted(&off, false));
        assert!(!gpu_wanted(&on, true), "--cpu wins over the setting");
        assert!(!gpu_wanted(&off, true));
    }
}
