use xilem::masonry::kurbo::{Point, Rect, RoundedRect};

pub fn inflated_pill(center: Point, inflate: f64, height: f64, width: f64) -> RoundedRect {
    let half_w = (width / 2.0) + inflate;
    let half_h = height / 2.0 + inflate;
    Rect::new(
        center.x - half_w,
        center.y - half_h,
        center.x + half_w,
        center.y + half_h,
    )
    .to_rounded_rect(half_h)
}
