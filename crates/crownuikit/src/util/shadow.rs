use xilem::{
    masonry::{kurbo::Point, peniko::Gradient},
    palette,
};

pub fn inner_shadow_gradient(center: Point, strength: f32, width: f32) -> Gradient {
    Gradient::new_radial(center, width).with_stops([
        (0.0_f32, palette::css::WHITE.with_alpha(0.0)),
        (0.78_f32, palette::css::WHITE.with_alpha(0.0)),
        (1.0_f32, palette::css::WHITE_SMOKE.with_alpha(strength)),
    ])
}
