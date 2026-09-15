//! The dictation overlay: a small dark pill pinned to the bottom of the
//! screen showing a live waveform, in the style of Wispr Flow.
//!
//! Layout inside the pill, left to right:
//!   • a few idle dots (they pulse while the model is thinking)
//!   • a text-cursor tick
//!   • the waveform strip — one bar per slot, oldest left, newest right.
//!     A silent slot collapses to a dot, so quiet audio reads as a dotted
//!     baseline rather than an empty gap.

use std::time::Instant;

use crownshell::vello::kurbo::{Affine, Circle, Point, Rect, RoundedRect, Stroke};
use crownshell::vello::peniko::{Color, Fill, Mix};
use crownshell::{Scene, SurfaceCtx, SurfaceHandler};

use crate::state::{Phase, Shared, WAVE_N, wave_snapshot};

// ---- geometry ---------------------------------------------------------
const PAD_X: f32 = 14.0;
const PILL_H: f32 = 54.0;
const PILL_R: f32 = 16.0;
/// Distance from the bottom screen edge to the bottom of the pill.
const BOTTOM_PAD: f32 = 22.0;

const DOTS: usize = 4;
const DOT_R: f32 = 1.5;
const DOT_PITCH: f32 = 7.0;
const DOTS_W: f32 = (DOTS - 1) as f32 * DOT_PITCH + DOT_R * 2.0;

const CURSOR_W: f32 = 2.0;
/// Slightly taller than the loudest bar, as in the reference.
const CURSOR_H: f32 = 28.0;

const BARS: usize = 28;
const BAR_W: f32 = 2.6;
const BAR_GAP: f32 = 2.4;
const BAR_PITCH: f32 = BAR_W + BAR_GAP;
const BAR_MAX: f32 = 26.0;
const STRIP_W: f32 = BARS as f32 * BAR_PITCH - BAR_GAP;

/// Space between the dot group, the cursor and the waveform strip.
const GAP: f32 = 9.0;
const PILL_W: f32 = PAD_X * 2.0 + DOTS_W + GAP + CURSOR_W + GAP + STRIP_W;

/// Surface size — the pill plus room for its shadow and the slide-in.
pub const WIN_W: u32 = PILL_W as u32 + 40;
pub const WIN_H: u32 = 100;

/// How long the success/error flash stays before the pill retracts.
const RESULT_FLASH_SECS: f32 = 0.55;

pub struct Overlay {
    shared: Shared,
    t0: Instant,
    last: Instant,
    /// 0 = fully retracted, 1 = fully shown.
    presence: f32,
    /// Smoothed per-bar heights, 0..1.
    bars: [f32; BARS],
    was_visible: bool,
    animating: bool,
}

impl Overlay {
    pub fn new(shared: Shared) -> Self {
        Self {
            shared,
            t0: Instant::now(),
            last: Instant::now(),
            presence: 0.0,
            bars: [0.0; BARS],
            was_visible: false,
            animating: false,
        }
    }
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba8(r, g, b, a)
}

/// Peak of the waveform history that falls into bar `i`.
fn bar_peak(wave: &[f32; WAVE_N], i: usize) -> f32 {
    let seg = WAVE_N as f32 / BARS as f32;
    let a = (i as f32 * seg) as usize;
    let b = (((i + 1) as f32 * seg).ceil() as usize).clamp(a + 1, WAVE_N);
    wave[a..b].iter().copied().fold(0.0, f32::max)
}

impl SurfaceHandler for Overlay {
    fn paint(&mut self, scene: &mut Scene, ctx: SurfaceCtx<'_>) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.05);
        self.last = now;
        let t = (now - self.t0).as_secs_f32();

        // ---- snapshot shared state --------------------------------------
        let (phase, since, wave_raw) = {
            let s = self.shared.lock().unwrap();
            (
                s.phase,
                s.phase_since.elapsed().as_secs_f32(),
                wave_snapshot(&s),
            )
        };
        // Auto-retract once the result flash has been shown.
        if matches!(phase, Phase::Success | Phase::Error) && since > RESULT_FLASH_SECS {
            crate::state::set_phase(&self.shared, Phase::Hidden);
        }
        let visible = phase != Phase::Hidden;

        if visible && !self.was_visible {
            self.bars = [0.0; BARS];
        }
        self.was_visible = visible;

        // ---- animation ----------------------------------------------------
        let target = if visible { 1.0 } else { 0.0 };
        let rate = if visible { 16.0 } else { 11.0 };
        self.presence += (target - self.presence) * (1.0 - (-dt * rate).exp());

        if self.presence < 0.004 && !visible {
            self.animating = false;
            return; // fully retracted: leave the scene empty
        }
        self.animating = true;

        // Per-bar targets. Listening follows the mic; the other phases get
        // synthetic motion so the pill never looks frozen.
        for i in 0..BARS {
            let s = i as f32 / (BARS - 1) as f32;
            let want = match phase {
                Phase::Thinking => {
                    // A soft bump scanning left to right.
                    let pos = (t * 0.85) % 1.4 - 0.2;
                    let d = (s - pos) / 0.18;
                    0.06 + 0.9 * (-d * d).exp()
                }
                Phase::Error => 0.10 + 0.06 * (t * 9.0 + s * 6.0).sin(),
                Phase::Success | Phase::Hidden => 0.0,
                Phase::Listening => bar_peak(&wave_raw, i),
            };
            // Fast attack, slower release, so peaks pop but don't flicker.
            let k = if want > self.bars[i] { 30.0 } else { 11.0 };
            self.bars[i] += (want - self.bars[i]) * (1.0 - (-dt * k).exp());
        }

        // ---- layout --------------------------------------------------------
        let (w, h) = ctx.size;
        let (w, h) = (w as f32, h as f32);
        let ease = 1.0 - (1.0 - self.presence) * (1.0 - self.presence);
        let cx = w * 0.5;
        // Slide up into place as it fades in.
        let cy = h - BOTTOM_PAD - PILL_H * 0.5 + (1.0 - ease) * 20.0;

        let alpha = self.presence.powf(1.2);
        let full = Rect::new(0.0, 0.0, w as f64, h as f64);
        scene.push_layer(Fill::NonZero, Mix::Normal, alpha, Affine::IDENTITY, &full);

        let x0 = cx - PILL_W * 0.5;
        let pill = RoundedRect::new(
            x0 as f64,
            (cy - PILL_H * 0.5) as f64,
            (x0 + PILL_W) as f64,
            (cy + PILL_H * 0.5) as f64,
            PILL_R as f64,
        );

        // ---- pill body -------------------------------------------------------
        scene.draw_blurred_rounded_rect(
            Affine::IDENTITY,
            Rect::new(
                (x0 + 2.0) as f64,
                (cy - PILL_H * 0.5 + 4.0) as f64,
                (x0 + PILL_W - 2.0) as f64,
                (cy + PILL_H * 0.5 + 4.0) as f64,
            ),
            rgba(0, 0, 0, 120),
            PILL_R as f64,
            9.0,
        );
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            rgba(30, 30, 32, 242),
            None,
            &pill,
        );
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            rgba(255, 255, 255, 20),
            None,
            &pill,
        );

        // Phase tint: neutral while working, warm red on failure.
        let fg = match phase {
            Phase::Error => (255u8, 128u8, 116u8),
            _ => (226u8, 226u8, 231u8),
        };

        // ---- idle dots --------------------------------------------------------
        let mut x = x0 + PAD_X + DOT_R;
        for i in 0..DOTS {
            // The dots chase each other while the model is thinking.
            let a = if phase == Phase::Thinking {
                let p = (t * 2.4 - i as f32 * 0.45).sin() * 0.5 + 0.5;
                (70.0 + 150.0 * p) as u8
            } else {
                110
            };
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                rgba(fg.0, fg.1, fg.2, a),
                None,
                &Circle::new(Point::new(x as f64, cy as f64), DOT_R as f64),
            );
            x += DOT_PITCH;
        }

        // ---- cursor tick ------------------------------------------------------
        x = x0 + PAD_X + DOTS_W + GAP;
        let cursor = RoundedRect::new(
            x as f64,
            (cy - CURSOR_H * 0.5) as f64,
            (x + CURSOR_W) as f64,
            (cy + CURSOR_H * 0.5) as f64,
            (CURSOR_W * 0.5) as f64,
        );
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            rgba(fg.0, fg.1, fg.2, 210),
            None,
            &cursor,
        );

        // ---- waveform strip ----------------------------------------------------
        x += CURSOR_W + GAP;
        for (i, &v) in self.bars.iter().enumerate() {
            let v = v.clamp(0.0, 1.0);
            let bh = BAR_W + (BAR_MAX - BAR_W) * v;
            let bx = x + i as f32 * BAR_PITCH;
            // Louder bars are brighter, so the strip has depth even at rest.
            let a = (155.0 + 100.0 * v) as u8;
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                rgba(fg.0, fg.1, fg.2, a),
                None,
                &RoundedRect::new(
                    bx as f64,
                    (cy - bh * 0.5) as f64,
                    (bx + BAR_W) as f64,
                    (cy + bh * 0.5) as f64,
                    (BAR_W * 0.5) as f64,
                ),
            );
        }

        scene.pop_layer(); // global alpha
    }

    fn on_frame(&mut self, _ctx: SurfaceCtx<'_>) -> bool {
        self.animating
    }
}
