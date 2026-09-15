//! A minimal Lucide icon renderer.
//!
//! Takes the raw SVG body strings shipped by the `blinc_icons` crate
//! (which use only `<path>`, `<rect>`, `<circle>`, and `<line>` primitives on a
//! `0 0 24 24` viewport) and renders them stroked with vello.

use xilem::core::{MessageContext, MessageResult, Mut, View, ViewMarker};
use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NoAction, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, Widget, WidgetMut,
};
use xilem::masonry::kurbo::{
    Affine, BezPath, Circle, Line, Point, Rect, RoundedRect, Size, Stroke,
};
use xilem::masonry::peniko::Color;
use xilem::masonry::vello::Scene;
use xilem::{Pod, ViewCtx};

const ICON_VIEW_BOX: f64 = 24.0;
const DEFAULT_ICON_SIZE: f64 = 16.0;
const DEFAULT_STROKE_WIDTH: f64 = 2.0;

/// A single primitive in a parsed Lucide icon.
pub(crate) enum IconShape {
    Path(BezPath),
    Rect(RoundedRect),
    Circle(Circle),
    Line(Line),
}

/// Render pre-parsed icon shapes into `scene`, scaled and centered inside
/// `dest`. Uses the same rounded-cap stroking that Lucide icons ship with.
pub(crate) fn paint_shapes(
    scene: &mut Scene,
    shapes: &[IconShape],
    dest: Rect,
    color: Color,
    stroke_width: f64,
) {
    let scale = dest.width().min(dest.height()) / ICON_VIEW_BOX;
    let offset_x = dest.x0 + (dest.width() - ICON_VIEW_BOX * scale) / 2.0;
    let offset_y = dest.y0 + (dest.height() - ICON_VIEW_BOX * scale) / 2.0;
    let transform = Affine::translate((offset_x, offset_y)) * Affine::scale(scale);
    let stroke = Stroke {
        width: stroke_width,
        join: xilem::masonry::kurbo::Join::Round,
        start_cap: xilem::masonry::kurbo::Cap::Round,
        end_cap: xilem::masonry::kurbo::Cap::Round,
        ..Default::default()
    };
    for shape in shapes {
        match shape {
            IconShape::Path(p) => scene.stroke(&stroke, transform, color, None, p),
            IconShape::Rect(r) => scene.stroke(&stroke, transform, color, None, r),
            IconShape::Circle(c) => scene.stroke(&stroke, transform, color, None, c),
            IconShape::Line(l) => scene.stroke(&stroke, transform, color, None, l),
        }
    }
}

pub(crate) fn parse_shapes(svg_body: &str) -> Vec<IconShape> {
    let mut shapes = Vec::new();
    let bytes = svg_body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let end = match svg_body[i..].find('>') {
            Some(e) => i + e,
            None => break,
        };
        let tag = &svg_body[i + 1..end];
        i = end + 1;
        parse_element(tag, &mut shapes);
    }
    shapes
}

fn parse_element(tag: &str, out: &mut Vec<IconShape>) {
    let name = tag.split_ascii_whitespace().next().unwrap_or("");
    match name {
        "path" => {
            if let Some(d) = attr(tag, "d") {
                if let Ok(path) = BezPath::from_svg(&d) {
                    out.push(IconShape::Path(path));
                }
            }
        }
        "rect" => {
            let x = attr_f64(tag, "x").unwrap_or(0.0);
            let y = attr_f64(tag, "y").unwrap_or(0.0);
            let w = attr_f64(tag, "width").unwrap_or(0.0);
            let h = attr_f64(tag, "height").unwrap_or(0.0);
            let rx = attr_f64(tag, "rx")
                .or_else(|| attr_f64(tag, "ry"))
                .unwrap_or(0.0);
            let rect = Rect::new(x, y, x + w, y + h).to_rounded_rect(rx);
            out.push(IconShape::Rect(rect));
        }
        "circle" => {
            let cx = attr_f64(tag, "cx").unwrap_or(0.0);
            let cy = attr_f64(tag, "cy").unwrap_or(0.0);
            let r = attr_f64(tag, "r").unwrap_or(0.0);
            out.push(IconShape::Circle(Circle::new(Point::new(cx, cy), r)));
        }
        "line" => {
            let x1 = attr_f64(tag, "x1").unwrap_or(0.0);
            let y1 = attr_f64(tag, "y1").unwrap_or(0.0);
            let x2 = attr_f64(tag, "x2").unwrap_or(0.0);
            let y2 = attr_f64(tag, "y2").unwrap_or(0.0);
            out.push(IconShape::Line(Line::new(
                Point::new(x1, y1),
                Point::new(x2, y2),
            )));
        }
        _ => {}
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=\"");
    let start = tag.find(&pattern)? + pattern.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

fn attr_f64(tag: &str, name: &str) -> Option<f64> {
    attr(tag, name).and_then(|v| v.parse().ok())
}

// --- MARK: Widget ---
pub struct Icon {
    shapes: Vec<IconShape>,
    size: f64,
    color: Color,
    stroke_width: f64,
}

impl Icon {
    pub fn new(svg_body: &'static str) -> Self {
        Self {
            shapes: parse_shapes(svg_body),
            size: DEFAULT_ICON_SIZE,
            color: Color::from_rgb8(0x1B, 0x1B, 0x1F),
            stroke_width: DEFAULT_STROKE_WIDTH,
        }
    }

    pub fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn set_color(this: &mut WidgetMut<'_, Self>, color: Color) {
        if this.widget.color != color {
            this.widget.color = color;
            this.ctx.request_render();
        }
    }

    pub fn set_svg(this: &mut WidgetMut<'_, Self>, svg_body: &'static str) {
        this.widget.shapes = parse_shapes(svg_body);
        this.ctx.request_render();
    }
}

impl Widget for Icon {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain(Size::new(self.size, self.size))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let box_size = ctx.size();
        paint_shapes(
            scene,
            &self.shapes,
            box_size.to_rect(),
            self.color,
            self.stroke_width,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

// --- MARK: Xilem view wrapper ---

pub struct IconView {
    svg: &'static str,
    size: f64,
    color: Color,
}

pub fn icon(svg: &'static str) -> IconView {
    IconView {
        svg,
        size: DEFAULT_ICON_SIZE,
        color: Color::from_rgb8(0x1B, 0x1B, 0x1F),
    }
}

impl IconView {
    pub fn size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl ViewMarker for IconView {}
impl<State, Action> View<State, Action, ViewCtx> for IconView
where
    State: 'static,
    Action: 'static,
{
    type Element = Pod<Icon>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let widget = Icon::new(self.svg)
            .with_size(self.size)
            .with_color(self.color);
        (ctx.create_pod(widget), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        if prev.svg != self.svg {
            Icon::set_svg(&mut element, self.svg);
        }
        if prev.color != self.color {
            Icon::set_color(&mut element, self.color);
        }
        // Size is only read at build; layout picks it up implicitly.
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        _message: &mut MessageContext,
        _: Mut<'_, Self::Element>,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        MessageResult::Nop
    }
}
