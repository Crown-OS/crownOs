//! Color mixing helpers shared across widgets.

use xilem::masonry::peniko::Color;

/// Straight linear interpolation between two sRGB colors in premultiplied
/// component space.
///
/// Not perceptually uniform — but good enough for 1-D UI accent transitions
/// (toggle track color, hover fades, etc.) and avoids pulling in the full
/// `color` crate machinery for a single mix.
pub fn lerp(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0) as f32;
    let [ar, ag, ab, aa] = a.components;
    let [br, bg, bb, ba] = b.components;
    Color::new([
        ar + (br - ar) * t,
        ag + (bg - ag) * t,
        ab + (bb - ab) * t,
        aa + (ba - aa) * t,
    ])
}
