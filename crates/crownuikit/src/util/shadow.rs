//! Shared gradient helpers for shadow and highlight rings on circular and
//! pill-shaped elements.
//!
//! Every crownuikit widget that has a raised "thumb" (slider thumb, toggle
//! knob, popover panel, etc.) uses the same two visual primitives to sell the
//! sense of depth:
//!
//! - **Outer glow** — a symmetric drop shadow radiating from the shape's rim.
//!   Doubles as an anti-aliasing softener at small sizes; the smooth alpha
//!   ramp hides pixelation along the curve better than a hard stroke does.
//! - **Inner ring** — a radial gradient that darkens (shadow) or brightens
//!   (highlight) the rim from *inside* the shape, smoothing the transition
//!   between the shape's fill and its outer glow.
//!
//! Keeping both as one function each — parameterized on `color`/`strength`
//! rather than baked into each widget — keeps the look consistent across
//! widgets and makes tweaks propagate everywhere at once.

use xilem::masonry::kurbo::Point;
use xilem::masonry::peniko::{Color, Gradient};

/// Radial gradient that fades from `color` at `inner_radius` outward to fully
/// transparent at `outer_radius`, with a subtle "core" at the rim itself.
///
/// Paint over a rect whose bounds match `outer_radius` (i.e. an inflated
/// version of the source shape) to get a smooth, symmetric drop-shadow that
/// blurs the rim.
///
/// - `center`: shape centroid
/// - `inner_radius`: distance from `center` to the shape's rim (in px)
/// - `outer_radius`: how far the glow extends past the rim
/// - `rim_strength`: peak alpha at the rim (typical 0.10 – 0.20)
/// - `halo_strength`: gentle alpha just past the rim (typical 0.05 – 0.10)
pub fn outer_glow(
    center: Point,
    inner_radius: f32,
    outer_radius: f32,
    color: Color,
    rim_strength: f32,
    halo_strength: f32,
) -> Gradient {
    let edge = (inner_radius / outer_radius).clamp(0.0, 1.0);
    Gradient::new_radial(center, outer_radius).with_stops([
        (0.0_f32, color.with_alpha(0.0)),
        ((edge - 0.02).max(0.0), color.with_alpha(rim_strength)),
        (edge, color.with_alpha(halo_strength)),
        (1.0_f32, color.with_alpha(0.0)),
    ])
}

/// Radial gradient that stays transparent through most of a shape and then
/// ramps up to `color` at `strength` alpha right at the rim.
///
/// Painted *inside* the shape (over its fill), this simulates either an inner
/// shadow (use `palette::css::BLACK`) or an inner rim highlight (use
/// `palette::css::WHITE`). The soft ramp also smooths the pixel transition
/// between the shape's fill and its outer glow, killing any visible aliasing
/// at the boundary.
///
/// - `center`: shape centroid
/// - `radius`: distance from `center` to the shape's rim
/// - `color`: the ring color (BLACK for shadow, WHITE for highlight)
/// - `strength`: alpha at the rim
/// - `start`: fraction of `radius` where the ring begins fading in (typical
///   `0.75` – `0.85`; higher = tighter ring)
pub fn inner_ring(center: Point, radius: f32, color: Color, strength: f32, start: f32) -> Gradient {
    Gradient::new_radial(center, radius).with_stops([
        (0.0_f32, color.with_alpha(0.0)),
        (start, color.with_alpha(0.0)),
        (1.0_f32, color.with_alpha(strength)),
    ])
}
