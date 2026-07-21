//! A pill-style toggle switch with the same API surface as
//! [`xilem::view::checkbox`], so the two views are drop-in interchangeable.
//!
//! Off state: light-gray pill with a white knob on the left.
//! On state:  purple pill with a white knob on the right.

use xilem::core::{MessageContext, MessageResult, Mut, View, ViewMarker};
use xilem::masonry::accesskit::{self, Node, Role, Toggled};
use xilem::masonry::core::keyboard::Key;
use xilem::masonry::core::{
    AccessCtx, AccessEvent, ArcStr, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, NewWidget,
    PaintCtx, PointerButton, PointerButtonEvent, PointerEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetMut, WidgetPod,
};
use xilem::masonry::kurbo::{Circle, Point, Rect, Size};
use xilem::masonry::peniko::color::palette;
use xilem::masonry::peniko::Gradient;
use xilem::masonry::util::fill;
use xilem::masonry::vello::Scene;
use xilem::masonry::widgets::Label;
use xilem::{Color, Pod, ViewCtx};

use crate::animation::{Clock, Spring};

// --- MARK: Metrics ---
const TRACK_WIDTH: f64 = 44.0;
const TRACK_HEIGHT: f64 = 24.0;
const TRACK_RADIUS: f64 = TRACK_HEIGHT / 2.0;
const KNOB_RADIUS: f64 = 10.0;
const KNOB_PADDING: f64 = 2.0;
const LABEL_GAP: f64 = 10.0;
/// How far past the knob the soft glow extends. Bigger = softer edge.
const KNOB_GLOW_RADIUS: f64 = 4.0;

// --- MARK: Palette ---
const OFF_COLOR: Color = Color::from_rgb8(0xE5, 0xE5, 0xEA);
const ON_COLOR: Color = Color::from_rgb8(0x7F, 0x5A, 0xF8);
const KNOB_COLOR: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF);

/// Soft glow surrounding the knob. Painted as a radial gradient that fades
/// from a subtle dark tint at the knob's rim to fully transparent past
/// `KNOB_GLOW_RADIUS`, softening the boundary against the track color and
/// hiding any pixelation along the curve.
fn knob_glow(center: Point) -> Gradient {
    let outer = (KNOB_RADIUS + KNOB_GLOW_RADIUS) as f32;
    // Fraction of the gradient where the knob body ends.
    let knob_edge = KNOB_RADIUS as f32 / outer;
    Gradient::new_radial(center, outer).with_stops([
        (0.0_f32, palette::css::BLACK.with_alpha(0.0)),
        (knob_edge - 0.02, palette::css::BLACK.with_alpha(0.14)),
        (knob_edge, palette::css::BLACK.with_alpha(0.08)),
        (1.0_f32, palette::css::BLACK.with_alpha(0.0)),
    ])
}

/// Straight linear interpolation between two sRGB colors in premultiplied
/// component space. Good enough for a UI accent transition; avoids pulling in
/// the full color-space machinery for what is a 1-D mix.
fn lerp_color(a: Color, b: Color, t: f64) -> Color {
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

/// A subtle inner shadow along the knob's inner rim — smooths the transition
/// from the pure-white body into the outer glow so the edge doesn't look
/// aliased.
fn knob_inner_shadow(center: Point) -> Gradient {
    Gradient::new_radial(center, KNOB_RADIUS as f32).with_stops([
        (0.0_f32, palette::css::BLACK.with_alpha(0.0)),
        (0.80_f32, palette::css::BLACK.with_alpha(0.0)),
        (1.0_f32, palette::css::BLACK.with_alpha(0.08)),
    ])
}

// --- MARK: Widget ---

/// The action emitted by a [`Toggle`] when the user activates it.
///
/// Mirrors [`masonry::widgets::CheckboxToggled`] so that callers can treat
/// `Toggle` and `Checkbox` uniformly.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ToggleToggled(pub bool);

pub struct Toggle {
    checked: bool,
    /// Animated position in [0.0, 1.0]. 0.0 = off, 1.0 = on. Drives both the
    /// knob X offset and the track color mix.
    progress: Spring,
    clock: Clock,
    label: WidgetPod<Label>,
}

impl Toggle {
    pub fn new(checked: bool, text: impl Into<ArcStr>) -> Self {
        let initial = if checked { 1.0 } else { 0.0 };
        Self {
            checked,
            progress: Spring::new(initial),
            clock: Clock::new(),
            label: WidgetPod::new(Label::new(text)),
        }
    }

    pub fn from_label(checked: bool, label: NewWidget<Label>) -> Self {
        let initial = if checked { 1.0 } else { 0.0 };
        Self {
            checked,
            progress: Spring::new(initial),
            clock: Clock::new(),
            label: label.to_pod(),
        }
    }

    pub fn set_checked(this: &mut WidgetMut<'_, Self>, checked: bool) {
        if this.widget.checked != checked {
            this.widget.checked = checked;
            this.widget
                .progress
                .set_target(if checked { 1.0 } else { 0.0 });
            // Reset the clock so the first anim tick uses a fresh dt.
            this.widget.clock.reset();
            this.ctx.request_anim_frame();
        }
    }

    pub fn set_text(this: &mut WidgetMut<'_, Self>, text: ArcStr) {
        Label::set_text(&mut Self::label_mut(this), text);
    }

    pub fn label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label)
    }
}

impl Widget for Toggle {
    type Action = ToggleToggled;

    fn accepts_focus(&self) -> bool {
        true
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if ctx.is_disabled() {
            return;
        }
        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                ..
            }) => {
                ctx.capture_pointer();
            }
            PointerEvent::Up(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                ..
            }) => {
                if ctx.is_active() && ctx.is_hovered() {
                    ctx.submit_action::<Self::Action>(ToggleToggled(!self.checked));
                }
            }
            _ => {}
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        if ctx.is_disabled() {
            return;
        }
        if let TextEvent::Keyboard(ev) = event {
            if ev.state.is_up() && matches!(&ev.key, Key::Character(c) if c == " ") {
                ctx.submit_action::<Self::Action>(ToggleToggled(!self.checked));
            }
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if ctx.is_disabled() {
            return;
        }
        if let accesskit::Action::Click = event.action {
            ctx.submit_action::<Self::Action>(ToggleToggled(!self.checked));
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(
            event,
            Update::HoveredChanged(_)
                | Update::ActiveChanged(_)
                | Update::FocusChanged(_)
                | Update::DisabledChanged(_)
        ) {
            ctx.request_paint_only();
        }
    }

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _interval: u64,
    ) {
        let dt = self.clock.tick();
        self.progress.step(dt);
        ctx.request_paint_only();
        if !self.progress.at_rest() {
            ctx.request_anim_frame();
        } else {
            // Snap to exact target so subsequent paints are pixel-stable.
            self.progress.snap_to_target();
            self.clock.reset();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.label);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let label_size = ctx.run_layout(&mut self.label, bc);
        ctx.place_child(&mut self.label, (TRACK_WIDTH + LABEL_GAP, 0.0).into());

        let width = if label_size.width > 0.0 {
            TRACK_WIDTH + LABEL_GAP + label_size.width
        } else {
            TRACK_WIDTH
        };
        let height = TRACK_HEIGHT.max(label_size.height);
        let baseline =
            ctx.child_baseline_offset(&self.label) + (height - label_size.height);
        ctx.set_baseline_offset(baseline);
        bc.constrain(Size::new(width, height))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let track_y = (size.height - TRACK_HEIGHT) / 2.0;
        let track_rect = Rect::new(0.0, track_y, TRACK_WIDTH, track_y + TRACK_HEIGHT)
            .to_rounded_rect(TRACK_RADIUS);

        // Animated progress drives both track color and knob position.
        let t = self.progress.position.clamp(0.0, 1.0) as f64;
        fill(scene, &track_rect, lerp_color(OFF_COLOR, ON_COLOR, t));

        let knob_travel = TRACK_WIDTH - KNOB_PADDING * 2.0 - KNOB_RADIUS * 2.0;
        let knob_x = KNOB_PADDING + KNOB_RADIUS + t * knob_travel;
        let knob_center = Point::new(knob_x, track_y + TRACK_HEIGHT / 2.0);

        // Soft glow — a wide, single radial gradient. The smooth alpha ramp
        // hides pixelation at the knob boundary far better than concentric
        // hard-edged fills do.
        let glow = Circle::new(knob_center, KNOB_RADIUS + KNOB_GLOW_RADIUS);
        fill(scene, &glow, &knob_glow(knob_center));

        // Knob body.
        let knob = Circle::new(knob_center, KNOB_RADIUS);
        fill(scene, &knob, KNOB_COLOR);

        // Inner rim shading — smooths the white → glow transition, so the
        // curve doesn't read as a hard stair-stepped edge.
        fill(scene, &knob, &knob_inner_shadow(knob_center));
    }

    fn accessibility_role(&self) -> Role {
        Role::Switch
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_toggled(if self.checked {
            Toggled::True
        } else {
            Toggled::False
        });
        node.add_action(accesskit::Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.label.id()])
    }
}

// --- MARK: Xilem view ---

/// A pill-style toggle switch that is API-compatible with
/// [`xilem::view::checkbox`]: same argument order, same callback signature,
/// same builder methods.
///
/// ```ignore
/// use crownuikit::widgets::toggle;
///
/// toggle("Airplane mode", state.airplane, |s: &mut State, v| s.airplane = v);
/// ```
pub fn toggle<F, State, Action>(
    label: impl Into<ArcStr>,
    checked: bool,
    callback: F,
) -> ToggleView<F>
where
    F: Fn(&mut State, bool) -> Action + Send + 'static,
{
    ToggleView {
        label: label.into(),
        callback,
        checked,
        disabled: false,
    }
}

/// The [`View`] returned by [`toggle`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct ToggleView<F> {
    label: ArcStr,
    checked: bool,
    callback: F,
    disabled: bool,
}

impl<F> ToggleView<F> {
    /// Set the disabled state of the widget.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<F> ViewMarker for ToggleView<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for ToggleView<F>
where
    F: Fn(&mut State, bool) -> Action + Send + Sync + 'static,
{
    type Element = Pod<Toggle>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|ctx| {
            let mut pod = ctx.create_pod(Toggle::new(self.checked, self.label.clone()));
            pod.new_widget.options.disabled = self.disabled;
            pod
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        if prev.disabled != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        if prev.label != self.label {
            Toggle::set_text(&mut element, self.label.clone());
        }
        if prev.checked != self.checked {
            Toggle::set_checked(&mut element, self.checked);
        }
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
        message: &mut MessageContext,
        _: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in Toggle::message"
        );
        match message.take_message::<ToggleToggled>() {
            Some(toggled) => MessageResult::Action((self.callback)(app_state, toggled.0)),
            None => MessageResult::Stale,
        }
    }
}
