//! Slider styled to match the crownuikit design.
//!
//! Masonry's built-in slider can't add a shadow beneath the thumb (only fill
//! colors are exposed as properties), so this module implements a custom
//! Masonry `Widget` and a matching Xilem `View` wrapper.

use xilem::core::{MessageContext, MessageResult, Mut, View, ViewMarker};
use xilem::masonry::accesskit::{self, Node, Role};
use xilem::masonry::core::keyboard::{Key, NamedKey};
use xilem::masonry::core::{
    AccessCtx, AccessEvent, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, PaintCtx,
    PointerButton, PointerButtonEvent, PointerEvent, PointerScrollEvent, PointerUpdate,
    PropertiesMut, PropertiesRef, RegisterCtx, ScrollDelta, TextEvent, Update, UpdateCtx, Widget,
    WidgetMut,
};
use xilem::masonry::kurbo::{Point, Rect, RoundedRect, Size};
use xilem::masonry::peniko::color::palette;
use xilem::masonry::peniko::Gradient;
use xilem::masonry::util::fill;
use xilem::masonry::vello::Scene;
use xilem::{Affine, Color, Pod, ViewCtx};

use crate::animation::{Clock, Spring};

const TRACK_HEIGHT: f64 = 8.0;
const THUMB_WIDTH: f64 = 26.0;
const THUMB_HEIGHT: f64 = 20.0;
const THUMB_CORNER_RADIUS: f64 = THUMB_HEIGHT / 2.0;
const THUMB_HALF_WIDTH: f64 = THUMB_WIDTH / 2.0;
const SHADOW_BLUR_RADIUS: f64 = 0.5;
const INNER_SHADOW_STRENGTH: f32 = 1.1;
const WIDGET_VERTICAL_PADDING: f64 = 10.0;

const TRACK_COLOR: Color = Color::from_rgb8(0xE4, 0xE4, 0xE4);
const FILLED_GRADIENT_START: Color = Color::from_rgba8(0xEC, 0x48, 0x99, 0x80);
const FILLED_GRADIENT_END: Color = Color::from_rgb8(0xEC, 0x48, 0x99);

fn shadow_halo_color() -> Color {
    palette::css::BLACK.with_alpha(0.05)
}

fn inflated_pill(center: Point, inflate: f64) -> RoundedRect {
    let half_w = THUMB_HALF_WIDTH + inflate;
    let half_h = THUMB_HEIGHT / 2.0 + inflate;
    Rect::new(
        center.x - half_w,
        center.y - half_h,
        center.x + half_w,
        center.y + half_h,
    )
    .to_rounded_rect(half_h)
}

fn inner_shadow_gradient(center: Point) -> Gradient {
    Gradient::new_radial(center, THUMB_HALF_WIDTH as f32).with_stops([
        (0.0_f32, palette::css::WHITE.with_alpha(0.0)),
        (0.78_f32, palette::css::WHITE.with_alpha(0.0)),
        (
            1.0_f32,
            palette::css::WHITE_SMOKE.with_alpha(INNER_SHADOW_STRENGTH),
        ),
    ])
}

pub struct Slider {
    min: f64,
    max: f64,
    /// The authoritative value — what actions report and what external state
    /// sync uses. Always the exact target.
    value: f64,
    /// Animated value driving the paint — springs toward `value`. Only differs
    /// from `value` while a spring animation is in flight (i.e. after a
    /// track-tap or keyboard step). During pointer drag we snap this to
    /// `value` so the thumb tracks the cursor without lag.
    displayed: Spring,
    clock: Clock,
}

impl Slider {
    pub fn new(min: f64, max: f64, value: f64) -> Self {
        let clamped = value.clamp(min, max);
        Self {
            min,
            max,
            value: clamped,
            displayed: Spring::new(clamped as f32),
            clock: Clock::new(),
        }
    }

    pub fn set_value(this: &mut WidgetMut<'_, Self>, value: f64) {
        let clamped = value.clamp(this.widget.min, this.widget.max);
        if (clamped - this.widget.value).abs() > f64::EPSILON {
            this.widget.value = clamped;
            this.widget.displayed.set_target(clamped as f32);
            this.widget.clock.reset();
            this.ctx.request_anim_frame();
        }
    }

    pub fn set_range(this: &mut WidgetMut<'_, Self>, min: f64, max: f64) {
        if this.widget.min != min || this.widget.max != max {
            this.widget.min = min;
            this.widget.max = max;
            Self::set_value(this, this.widget.value);
        }
    }

    /// Update `self.value` from a pointer x position. Returns true if the
    /// value changed.
    fn value_from_position(&mut self, x: f64, width: f64) -> bool {
        let track_width = (width - THUMB_HALF_WIDTH * 2.0).max(0.0);
        if track_width <= 0.0 {
            return false;
        }
        let progress = ((x - THUMB_HALF_WIDTH) / track_width).clamp(0.0, 1.0);
        let new_value = self.min + progress * (self.max - self.min);
        if (new_value - self.value).abs() > f64::EPSILON {
            self.value = new_value.clamp(self.min, self.max);
            true
        } else {
            false
        }
    }
}

impl Widget for Slider {
    type Action = f64;

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
                state,
                ..
            }) => {
                ctx.request_focus();
                ctx.capture_pointer();
                let local = ctx.local_position(state.position);
                if self.value_from_position(local.x, ctx.size().width) {
                    // Tap-to-jump: spring toward the new value.
                    self.displayed.set_target(self.value as f32);
                    self.clock.reset();
                    ctx.request_anim_frame();
                    ctx.submit_action::<f64>(self.value);
                }
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                if ctx.is_active() {
                    let local = ctx.local_position(current.position);
                    if self.value_from_position(local.x, ctx.size().width) {
                        // Drag: snap so the thumb stays under the pointer.
                        self.displayed.snap_to_target();
                        self.displayed.set_target(self.value as f32);
                        self.displayed.snap_to_target();
                        ctx.submit_action::<f64>(self.value);
                    }
                    ctx.request_render();
                }
            }
            PointerEvent::Up(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                ..
            }) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                }
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                // Wheel / trackpad. Convert whichever axis is active into a
                // signed step and nudge the value. Horizontal takes priority
                // (trackpad two-finger scroll on a horizontal slider); mouse
                // wheel is vertical, and gets inverted so scrolling up = up.
                // Per-source sensitivity. Mouse wheel notches are coarse (one
                // notch per tick) so they need a bigger multiplier; trackpad
                // pixels arrive in a continuous stream so a smaller factor
                // keeps them controllable.
                const WHEEL_SENSITIVITY: f64 = 3.5;
                const PAGE_SENSITIVITY: f64 = 15.0;
                const TRACKPAD_SENSITIVITY: f64 = 0.3;

                let (dx, dy) = match delta {
                    // Mouse wheel: pre-invert Y so wheel-down increases the
                    // value (the shared `-dy` at the raw step cancels out).
                    ScrollDelta::LineDelta(x, y) => (
                        *x as f64 * WHEEL_SENSITIVITY,
                        -(*y as f64) * WHEEL_SENSITIVITY,
                    ),
                    ScrollDelta::PageDelta(x, y) => (
                        *x as f64 * PAGE_SENSITIVITY,
                        -(*y as f64) * PAGE_SENSITIVITY,
                    ),
                    // Trackpad pixel deltas arrive already in "natural
                    // scrolling" direction on most platforms — invert so the
                    // slider tracks the finger (swipe right = value up).
                    ScrollDelta::PixelDelta(pos) => {
                        (-pos.x * TRACKPAD_SENSITIVITY, -pos.y * TRACKPAD_SENSITIVITY)
                    }
                };
                let raw = if dx.abs() > f64::EPSILON { dx } else { -dy };
                if raw.abs() > f64::EPSILON {
                    let step = (self.max - self.min).abs().max(f64::EPSILON) / 100.0;
                    let new_value = (self.value + raw * step).clamp(self.min, self.max);
                    if (new_value - self.value).abs() > f64::EPSILON {
                        self.value = new_value;
                        self.displayed.set_target(self.value as f32);
                        self.clock.reset();
                        ctx.request_anim_frame();
                        ctx.submit_action::<f64>(self.value);
                    }
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
        if ctx.is_disabled() || !ctx.is_focus_target() {
            return;
        }
        if let TextEvent::Keyboard(key_event) = event {
            if key_event.state.is_up() {
                return;
            }
            let step = (self.max - self.min).abs().max(f64::EPSILON) / 100.0;
            let delta = if key_event.modifiers.shift() {
                step * 10.0
            } else {
                step
            };
            let mut new_value = self.value;
            match &key_event.key {
                Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowDown) => {
                    new_value -= delta;
                }
                Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowUp) => {
                    new_value += delta;
                }
                Key::Named(NamedKey::Home) => new_value = self.min,
                Key::Named(NamedKey::End) => new_value = self.max,
                _ => return,
            }
            new_value = new_value.clamp(self.min, self.max);
            if (new_value - self.value).abs() > f64::EPSILON {
                self.value = new_value;
                self.displayed.set_target(self.value as f32);
                self.clock.reset();
                ctx.request_anim_frame();
                ctx.submit_action::<f64>(self.value);
            }
        }
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _interval: u64,
    ) {
        let dt = self.clock.tick();
        self.displayed.step(dt);
        ctx.request_paint_only();
        if !self.displayed.at_rest() {
            ctx.request_anim_frame();
        } else {
            self.displayed.snap_to_target();
            self.clock.reset();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(
            event,
            Update::FocusChanged(_) | Update::HoveredChanged(_) | Update::ActiveChanged(_)
        ) {
            ctx.request_render();
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let height = THUMB_HEIGHT + WIDGET_VERTICAL_PADDING;
        let width = bc.max().width.clamp(120.0, 480.0);
        Size::new(width, height)
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let track_width = (size.width - THUMB_HALF_WIDTH * 2.0).max(0.0);
        let track_y = (size.height - TRACK_HEIGHT) / 2.0;

        // Inactive track
        let track_rect = Rect::new(
            THUMB_HALF_WIDTH,
            track_y,
            THUMB_HALF_WIDTH + track_width,
            track_y + TRACK_HEIGHT,
        );
        fill(
            scene,
            &track_rect.to_rounded_rect(TRACK_HEIGHT / 2.0),
            TRACK_COLOR,
        );

        // Filled portion
        // Use the animated `displayed` value so track fill + thumb glide
        // together when the value springs toward a new target.
        let displayed_value = self.displayed.position as f64;
        let progress = if (self.max - self.min).abs() > f64::EPSILON {
            ((displayed_value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let filled_width = progress * track_width;
        if filled_width > 0.0 {
            let filled_rect = Rect::new(
                THUMB_HALF_WIDTH,
                track_y,
                THUMB_HALF_WIDTH + filled_width,
                track_y + TRACK_HEIGHT,
            );
            // Gradient spans the full track so the visible fill reveals a
            // consistent slice of the gradient as the value changes.
            let filled_gradient = Gradient::new_linear(
                Point::new(THUMB_HALF_WIDTH, track_y),
                Point::new(THUMB_HALF_WIDTH + track_width, track_y),
            )
            .with_stops([
                (0.0_f32, FILLED_GRADIENT_START),
                (1.0_f32, FILLED_GRADIENT_END),
            ]);
            fill(
                scene,
                &filled_rect.to_rounded_rect(TRACK_HEIGHT / 2.0),
                &filled_gradient,
            );
        }

        let thumb_center = Point::new(THUMB_HALF_WIDTH + filled_width, size.height / 2.0);
        let thumb_rect = Rect::new(
            thumb_center.x - THUMB_HALF_WIDTH,
            thumb_center.y - THUMB_HEIGHT / 2.0,
            thumb_center.x + THUMB_HALF_WIDTH,
            thumb_center.y + THUMB_HEIGHT / 2.0,
        );
        let thumb_shape = thumb_rect.to_rounded_rect(THUMB_CORNER_RADIUS);

        // outer shadow of the slider thumb
        fill(
            scene,
            &inflated_pill(thumb_center, SHADOW_BLUR_RADIUS * 3.5),
            shadow_halo_color(),
        );

        // fill of the slider thumb
        fill(scene, &thumb_shape, palette::css::WHITE.with_alpha(0.85));

        // inner shadow around the slider thumb
        fill(scene, &thumb_shape, &inner_shadow_gradient(thumb_center));

        scene.draw_blurred_rounded_rect(
            Affine::IDENTITY,
            thumb_shape.rect(),
            Color::from_rgba8(0xFF, 0xFF, 0xFF, 150),
            5.0,
            5.0,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Slider
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_value(self.value.to_string());
        node.set_min_numeric_value(self.min);
        node.set_max_numeric_value(self.max);
        node.add_action(accesskit::Action::SetValue);
        node.add_action(accesskit::Action::Increment);
        node.add_action(accesskit::Action::Decrement);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

// --- MARK: Xilem view wrapper ---

pub struct SliderView<F> {
    min: f64,
    max: f64,
    value: f64,
    on_change: F,
    disabled: bool,
}

pub fn slider<State, Action, F>(min: f64, max: f64, value: f64, on_change: F) -> SliderView<F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&mut State, f64) -> Action + Send + Sync + 'static,
{
    SliderView {
        min,
        max,
        value,
        on_change,
        disabled: false,
    }
}

impl<F> SliderView<F> {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<F> ViewMarker for SliderView<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for SliderView<F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&mut State, f64) -> Action + Send + Sync + 'static,
{
    type Element = Pod<Slider>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let pod = ctx.with_action_widget(|ctx| {
            let mut pod: Pod<Slider> = ctx.create_pod(Slider::new(self.min, self.max, self.value));
            pod.new_widget.options.disabled = self.disabled;
            pod
        });
        (pod, ())
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
        if prev.min != self.min || prev.max != self.max {
            Slider::set_range(&mut element, self.min, self.max);
        }
        if (prev.value - self.value).abs() > f64::EPSILON {
            Slider::set_value(&mut element, self.value);
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
        if message.take_first().is_some() {
            return MessageResult::Stale;
        }
        match message.take_message::<f64>() {
            Some(value) => MessageResult::Action((self.on_change)(app_state, *value)),
            None => MessageResult::Stale,
        }
    }
}
