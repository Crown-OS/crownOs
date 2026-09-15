//! Radix-style select built on masonry's layer infrastructure.
//!
//! The widget is split in two pieces that talk to each other through the
//! xilem view layer:
//!
//! - [`SelectTrigger`] — the button that always stays put. It renders the
//!   currently selected value and, on click, creates a new masonry layer
//!   containing the popup.
//! - [`SelectPopup`] — the drop-down. It's pushed onto masonry's top-level
//!   [`LayerStack`], so it can extend *both above and below* the trigger
//!   without perturbing the parent flex layout. It's positioned so the
//!   selected row's y in popup-local space lines up with the trigger's y in
//!   window space (that's the Radix "selected item is under the pointer"
//!   behaviour).
//!
//! ### Action routing
//! The popup isn't a child of the trigger — it's a top-level layer widget
//! sitting alongside the app root. Its actions therefore can't bubble up
//! through the view tree the way a child's would. We work around that by
//! *pre-reserving* the popup's [`WidgetId`] in [`SelectView::build`] and
//! calling [`ViewCtx::record_action`], which tells xilem "route any action
//! with this id to this view's `message`". The trigger later stamps the same
//! id onto the [`NewWidget`] it hands to `ctx.create_layer`.
//!
//! ### Lifecycle
//! 1. User clicks trigger → `SelectTrigger::on_pointer_event` calls
//!    `ctx.create_layer(popup, pos)` with the pre-reserved id.
//! 2. Popup opens with a spring-driven fade + slide.
//! 3. User picks a row → popup submits `PopupAction::Selected(idx)`,
//!    optimistically springs its own checkmark to the new row, and starts a
//!    close animation. When the close animation settles it self-removes via
//!    `ctx.remove_layer(ctx.widget_id())` and submits `PopupAction::Dismissed`
//!    so the trigger's `is_open` flag can be cleared.
//! 4. Xilem routes the action → `SelectView::message` → user callback →
//!    app state → rebuild → `SelectTrigger::set_selected` updates the label.

use blinc_icons::icons;
use xilem::core::{MessageContext, MessageResult, Mut, View, ViewMarker};
use xilem::masonry::accesskit::{self, Node, Role};
use xilem::masonry::core::{
    AccessCtx, AccessEvent, ArcStr, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, NewWidget,
    PaintCtx, PointerButton, PointerButtonEvent, PointerEvent, PointerUpdate, Properties,
    PropertiesMut, PropertiesRef, RegisterCtx, StyleProperty, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut, WidgetPod,
};
use xilem::masonry::kurbo::{Affine, Point, Rect, Size, Vec2};
use xilem::masonry::peniko::{Color, Mix};
use xilem::masonry::properties::ContentColor;
use xilem::masonry::util::{fill, stroke};
use xilem::masonry::vello::Scene;
use xilem::masonry::widgets::Label;
use xilem::{Pod, ViewCtx};

use crate::animation::{Clock, Spring, SpringProfile};
use crate::config::{PopoverColors, theme};
use crate::widgets::icon::{IconShape, paint_shapes, parse_shapes};

// --- MARK: Metrics ---
const TRIGGER_WIDTH: f64 = 200.0;
const TRIGGER_HEIGHT: f64 = 36.0;
const TRIGGER_RADIUS: f64 = 8.0;
const TRIGGER_H_PADDING: f64 = 12.0;
const CHEVRON_SIZE: f64 = 16.0;

const PANEL_RADIUS: f64 = 10.0;
const PANEL_PADDING: f64 = 6.0;
/// Locked to the trigger height so the selected row overlays the trigger
/// exactly when the popup is positioned by [`popup_offset_for`].
const ROW_HEIGHT: f64 = TRIGGER_HEIGHT;
const ROW_H_PADDING: f64 = 10.0;
const ROW_RADIUS: f64 = 6.0;
const HEADER_HEIGHT: f64 = 24.0;
const CHECK_SIZE: f64 = 14.0;

/// The popup starts this many pixels below its final position on open, and
/// slides *up* to rest — a Radix-style "drop into place" motion. On close it
/// reverses.
const POPUP_APPEAR_OFFSET: f32 = 6.0;

// --- MARK: Actions ---

/// Emitted by [`SelectView`] via the user callback. Kept public so callers
/// can name the action type if they need to plumb it through their own
/// message pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectChanged(pub usize);

/// Popup → view internal traffic. Public because it appears in
/// [`SelectPopup::Action`], but callers should treat this as an implementation
/// detail and rely on [`SelectChanged`] instead.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupAction {
    Selected(usize),
    Dismissed,
}

// --- MARK: Layout helpers ---

fn header_h(has_header: bool) -> f64 {
    if has_header { HEADER_HEIGHT } else { 0.0 }
}

fn panel_size(has_header: bool, n_options: usize) -> Size {
    let rows_h = n_options as f64 * ROW_HEIGHT;
    let height = PANEL_PADDING * 2.0 + header_h(has_header) + rows_h;
    Size::new(TRIGGER_WIDTH, height)
}

/// Row `index`'s y (top edge) in popup-local space.
fn row_top(has_header: bool, index: usize) -> f64 {
    PANEL_PADDING + header_h(has_header) + index as f64 * ROW_HEIGHT
}

/// Vector to add to the trigger's window origin to get the popup layer's
/// origin. Chosen so the selected row's top edge lands on the trigger's top
/// edge.
fn popup_offset_for(has_header: bool, selected: Option<usize>) -> Vec2 {
    let anchor = row_top(has_header, selected.unwrap_or(0));
    Vec2::new(0.0, -anchor)
}

fn trigger_label_text(options: &[ArcStr], selected: Option<usize>) -> ArcStr {
    selected
        .and_then(|i| options.get(i).cloned())
        .unwrap_or_else(|| ArcStr::from(""))
}

fn styled_label(text: ArcStr, size: f32) -> Label {
    Label::new(text).with_style(StyleProperty::FontSize(size))
}

fn props_with_color(color: Color) -> Properties {
    Properties::new().with(ContentColor::new(color))
}

// ================================================================
// MARK: SelectPopup — top-layer widget
// ================================================================

pub struct SelectPopup {
    options: Vec<ArcStr>,
    header: Option<ArcStr>,
    selected: Option<usize>,
    hover_index: Option<usize>,
    /// 1.0 = fully visible. Starts at 0, springs to 1 on creation.
    /// On selection / dismissal, target flips to 0 and the widget removes
    /// itself when the spring settles.
    alive_progress: Spring,
    /// Fractional row index the hover highlight tracks.
    hover_pos: Spring,
    /// Opacity of the hover highlight.
    hover_alpha: Spring,
    /// Fractional row index the checkmark tracks — springs when a new option
    /// is picked so the check glides during close.
    checkmark_pos: Spring,
    /// True after a selection has been made; drives self-removal in
    /// `on_anim_frame`.
    closing: bool,

    clock: Clock,
    header_label: Option<WidgetPod<Label>>,
    option_labels: Vec<WidgetPod<Label>>,
    check_shapes: Vec<IconShape>,
}

impl SelectPopup {
    fn new(header: Option<ArcStr>, options: Vec<ArcStr>, selected: Option<usize>) -> Self {
        let colors = theme().popover;
        let header_label = header.as_ref().map(|h| {
            styled_label(h.clone(), 12.0)
                .with_props(props_with_color(colors.muted_text))
                .to_pod()
        });
        let option_labels = options
            .iter()
            .map(|o| {
                styled_label(o.clone(), 14.0)
                    .with_props(props_with_color(colors.text))
                    .to_pod()
            })
            .collect();

        let initial_row = selected.unwrap_or(0) as f32;
        Self {
            options,
            header,
            selected,
            hover_index: None,
            alive_progress: Spring::with_profile(0.0, SpringProfile::SMOOTH),
            hover_pos: Spring::with_profile(initial_row, SpringProfile::SMOOTH),
            hover_alpha: Spring::with_profile(0.0, SpringProfile::SNAPPY),
            checkmark_pos: Spring::with_profile(initial_row, SpringProfile::SMOOTH),
            closing: false,
            clock: Clock::new(),
            header_label,
            option_labels,
            check_shapes: parse_shapes(icons::CHECK),
        }
    }

    fn has_header(&self) -> bool {
        self.header.is_some()
    }

    fn row_rect(&self, index: usize) -> Rect {
        let y = row_top(self.has_header(), index);
        Rect::new(
            PANEL_PADDING,
            y,
            TRIGGER_WIDTH - PANEL_PADDING,
            y + ROW_HEIGHT,
        )
    }

    fn hit_option(&self, local: Point) -> Option<usize> {
        (0..self.options.len()).find(|&i| self.row_rect(i).contains(local))
    }
}

impl Widget for SelectPopup {
    type Action = PopupAction;

    fn accepts_focus(&self) -> bool {
        true
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if ctx.is_disabled() || self.closing {
            return;
        }
        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                state,
                ..
            }) => {
                let local = ctx.local_position(state.position);
                if let Some(idx) = self.hit_option(local) {
                    // Submit the action *first* so xilem sees the state
                    // change quickly, then run our own close animation.
                    ctx.submit_action::<PopupAction>(PopupAction::Selected(idx));
                    // Optimistically move our own checkmark so the animation
                    // shows the new selection sliding into place while the
                    // popup fades.
                    self.selected = Some(idx);
                    self.checkmark_pos.set_target(idx as f32);
                    self.closing = true;
                    // Kick the spring with a starting velocity so the first
                    // frame after the click shows *visible* motion — a
                    // critically-damped spring re-targeted from rest ramps
                    // up too slowly to feel snappy on dismiss.
                    self.alive_progress.set_target_with_velocity(0.0, -6.0);
                    self.hover_alpha.set_target_with_velocity(0.0, -6.0);
                    self.clock.reset();
                    ctx.request_anim_frame();
                }
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                let local = ctx.local_position(current.position);
                let new_hover = self.hit_option(local);
                if new_hover != self.hover_index {
                    self.hover_index = new_hover;
                    if let Some(idx) = new_hover {
                        self.hover_pos.set_target(idx as f32);
                        self.hover_alpha.set_target(1.0);
                    } else {
                        self.hover_alpha.set_target(0.0);
                    }
                    self.clock.reset();
                    ctx.request_anim_frame();
                }
            }
            PointerEvent::Leave(_) => {
                if self.hover_index.take().is_some() {
                    self.hover_alpha.set_target(0.0);
                    self.clock.reset();
                    ctx.request_anim_frame();
                }
            }
            _ => {}
        }
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        // First real update after being added to the tree — kick the open
        // animation off. Seed the transform to the fully-hidden position so
        // the very first paint doesn't flash at rest. The velocity kick
        // matches the close path so both directions ramp up at the same
        // rate on the first frame.
        if matches!(event, Update::WidgetAdded) {
            ctx.set_transform(Affine::translate((0.0, POPUP_APPEAR_OFFSET as f64)));
            self.alive_progress.set_target_with_velocity(1.0, 6.0);
            self.clock.reset();
            ctx.request_anim_frame();
        }
        if matches!(
            event,
            Update::FocusChanged(_) | Update::HoveredChanged(_) | Update::ActiveChanged(_)
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
        self.alive_progress.step(dt);
        self.hover_pos.step(dt);
        self.hover_alpha.step(dt);
        self.checkmark_pos.step(dt);

        // Drive the widget's transform so background *and* child labels
        // slide together. At alive_progress=1 the popup sits at rest;
        // at alive_progress=0 it's `POPUP_APPEAR_OFFSET` below rest.
        let t = self.alive_progress.position.clamp(0.0, 1.0) as f64;
        let slide = (POPUP_APPEAR_OFFSET as f64) * (1.0 - t);
        ctx.set_transform(Affine::translate((0.0, slide)));

        // If we're closing and the fade has settled, submit the dismissal
        // signal + self-remove from the layer stack.
        if self.closing && self.alive_progress.at_rest() && self.alive_progress.target == 0.0 {
            ctx.submit_action::<PopupAction>(PopupAction::Dismissed);
            let id = ctx.widget_id();
            ctx.remove_layer(id);
            return;
        }

        let all_rest = self.alive_progress.at_rest()
            && self.hover_pos.at_rest()
            && self.hover_alpha.at_rest()
            && self.checkmark_pos.at_rest();
        if all_rest {
            self.alive_progress.snap_to_target();
            self.hover_pos.snap_to_target();
            self.hover_alpha.snap_to_target();
            self.checkmark_pos.snap_to_target();
            self.clock.reset();
        } else {
            ctx.request_anim_frame();
        }
        ctx.request_paint_only();
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(header) = self.header_label.as_mut() {
            ctx.register_child(header);
        }
        for label in &mut self.option_labels {
            ctx.register_child(label);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _bc: &BoxConstraints,
    ) -> Size {
        let size = panel_size(self.has_header(), self.options.len());

        // Header label.
        let mut y_cursor = PANEL_PADDING;
        if let Some(header) = self.header_label.as_mut() {
            let header_bc = BoxConstraints::new(
                Size::ZERO,
                Size::new(TRIGGER_WIDTH - PANEL_PADDING * 2.0, HEADER_HEIGHT),
            );
            let hs = ctx.run_layout(header, &header_bc);
            let hx = PANEL_PADDING + ROW_H_PADDING;
            let hy = y_cursor + (HEADER_HEIGHT - hs.height) / 2.0;
            ctx.place_child(header, (hx, hy).into());
            y_cursor += HEADER_HEIGHT;
        }
        // Option labels.
        for label in &mut self.option_labels {
            let row_bc = BoxConstraints::new(
                Size::ZERO,
                Size::new(
                    TRIGGER_WIDTH - PANEL_PADDING * 2.0 - ROW_H_PADDING * 2.0 - CHECK_SIZE,
                    ROW_HEIGHT,
                ),
            );
            let ls = ctx.run_layout(label, &row_bc);
            let lx = PANEL_PADDING + ROW_H_PADDING;
            let ly = y_cursor + (ROW_HEIGHT - ls.height) / 2.0;
            ctx.place_child(label, (lx, ly).into());
            y_cursor += ROW_HEIGHT;
        }
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let colors: PopoverColors = theme().popover;
        let t = self.alive_progress.position.clamp(0.0, 1.0);
        if t <= 0.001 {
            return;
        }
        // Alpha tracks progress directly. Previously it eased faster than
        // motion (`t * 1.6`) so the panel was legible early on open, but
        // that also meant the *close* stayed fully opaque until t < 0.625 —
        // which reads as "stuck" for the first ~100 ms of dismissal.
        // A linear map keeps open and close visually symmetric.
        let alpha = t;
        // The Y-slide is handled by the widget's transform (see on_anim_frame),
        // which moves child labels along with the background.

        let size = panel_size(self.has_header(), self.options.len());
        let panel_rect = Rect::new(0.0, 0.0, size.width, size.height);
        let panel_shape = panel_rect.to_rounded_rect(PANEL_RADIUS);

        scene.push_layer(Mix::Normal, alpha, Affine::IDENTITY, &panel_shape);

        fill(scene, &panel_shape, colors.bg);
        stroke(scene, &panel_shape, colors.border, 1.0);

        // Hover highlight.
        let hover_a = self.hover_alpha.position.clamp(0.0, 1.0);
        if hover_a > 0.001 {
            let hover_y = PANEL_PADDING
                + header_h(self.has_header())
                + self.hover_pos.position as f64 * ROW_HEIGHT;
            let hover_rect = Rect::new(
                PANEL_PADDING,
                hover_y,
                TRIGGER_WIDTH - PANEL_PADDING,
                hover_y + ROW_HEIGHT,
            );
            fill(
                scene,
                &hover_rect.to_rounded_rect(ROW_RADIUS),
                colors.hover_bg.with_alpha(hover_a),
            );
        }

        // Checkmark on the selected row.
        if self.selected.is_some() {
            let check_y = PANEL_PADDING
                + header_h(self.has_header())
                + self.checkmark_pos.position as f64 * ROW_HEIGHT;
            let row_right = TRIGGER_WIDTH - PANEL_PADDING;
            let check_x = row_right - ROW_H_PADDING - CHECK_SIZE;
            let check_top = check_y + (ROW_HEIGHT - CHECK_SIZE) / 2.0;
            paint_shapes(
                scene,
                &self.check_shapes,
                Rect::new(
                    check_x,
                    check_top,
                    check_x + CHECK_SIZE,
                    check_top + CHECK_SIZE,
                ),
                colors.text,
                2.0,
            );
        }

        scene.pop_layer();
    }

    fn accessibility_role(&self) -> Role {
        Role::ListBox
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        if let Some(idx) = self.selected {
            if let Some(text) = self.options.get(idx) {
                node.set_value(text.to_string());
            }
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        let mut ids = ChildrenIds::new();
        if let Some(header) = self.header_label.as_ref() {
            ids.push(header.id());
        }
        for label in &self.option_labels {
            ids.push(label.id());
        }
        ids
    }
}

// ================================================================
// MARK: SelectTrigger — base-layer widget (the button)
// ================================================================

pub struct SelectTrigger {
    options: Vec<ArcStr>,
    header: Option<ArcStr>,
    selected: Option<usize>,
    is_open: bool,
    /// Pre-reserved id for the popup layer we'll spawn on open. The view
    /// registers this id with `record_action` at build time so popup actions
    /// route back to `SelectView::message`.
    popup_id: WidgetId,

    trigger_label: WidgetPod<Label>,
    chevron_shapes: Vec<IconShape>,
}

impl SelectTrigger {
    pub fn new(
        header: Option<ArcStr>,
        options: Vec<ArcStr>,
        selected: Option<usize>,
        popup_id: WidgetId,
    ) -> Self {
        let colors = theme().popover;
        let trigger_text = trigger_label_text(&options, selected);
        let trigger_label = styled_label(trigger_text, 14.0)
            .with_props(props_with_color(colors.text))
            .to_pod();
        Self {
            options,
            header,
            selected,
            is_open: false,
            popup_id,
            trigger_label,
            chevron_shapes: parse_shapes(icons::CHEVRON_DOWN),
        }
    }

    fn has_header(&self) -> bool {
        self.header.is_some()
    }

    fn open_popup(&mut self, ctx: &mut EventCtx<'_>) {
        if self.is_open {
            return;
        }
        // Position the popup so the selected row lines up with the trigger.
        // `to_window` walks the transform chain from us to the window root.
        let trigger_origin = ctx.to_window(Point::ZERO);
        let offset = popup_offset_for(self.has_header(), self.selected);
        let pos = trigger_origin + offset;

        let popup = SelectPopup::new(self.header.clone(), self.options.clone(), self.selected);
        let new_widget = NewWidget::new_with_id(popup, self.popup_id);
        ctx.create_layer(new_widget, pos);
        self.is_open = true;
    }

    fn close_popup_immediately(&mut self, ctx: &mut EventCtx<'_>) {
        if !self.is_open {
            return;
        }
        // Trigger-initiated close is instant (no animation). Popup-initiated
        // dismissals go through the animation path in `SelectPopup` and reach
        // us via `mark_popup_closed`.
        ctx.remove_layer(self.popup_id);
        self.is_open = false;
    }

    // ---- WidgetMut API ----

    pub fn set_selected(this: &mut WidgetMut<'_, Self>, selected: Option<usize>) {
        if this.widget.selected == selected {
            return;
        }
        this.widget.selected = selected;
        let text = trigger_label_text(&this.widget.options, selected);
        Label::set_text(&mut Self::trigger_label_mut(this), text);
        this.ctx.request_render();
    }

    /// Called by the view when the popup reports it has finished dismissing
    /// itself. Just resets `is_open` — the layer is already gone.
    pub fn mark_popup_closed(this: &mut WidgetMut<'_, Self>) {
        this.widget.is_open = false;
    }

    pub fn is_popup_open(this: &WidgetMut<'_, Self>) -> bool {
        this.widget.is_open
    }

    pub fn trigger_label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.trigger_label)
    }
}

impl Widget for SelectTrigger {
    type Action = SelectChanged;

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
        if let PointerEvent::Down(PointerButtonEvent {
            button: Some(PointerButton::Primary),
            state,
            ..
        }) = event
        {
            ctx.request_focus();
            let local = ctx.local_position(state.position);
            let trigger_rect = Rect::new(0.0, 0.0, TRIGGER_WIDTH, TRIGGER_HEIGHT);
            if trigger_rect.contains(local) {
                if self.is_open {
                    self.close_popup_immediately(ctx);
                } else {
                    self.open_popup(ctx);
                }
            }
        }
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(
            event,
            Update::FocusChanged(_) | Update::HoveredChanged(_) | Update::ActiveChanged(_)
        ) {
            ctx.request_paint_only();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.trigger_label);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _bc: &BoxConstraints,
    ) -> Size {
        let trigger_label_bc = BoxConstraints::new(
            Size::ZERO,
            Size::new(
                TRIGGER_WIDTH - TRIGGER_H_PADDING * 2.0 - CHEVRON_SIZE - 4.0,
                TRIGGER_HEIGHT,
            ),
        );
        let trigger_label_size = ctx.run_layout(&mut self.trigger_label, &trigger_label_bc);
        let trigger_label_y = (TRIGGER_HEIGHT - trigger_label_size.height) / 2.0;
        ctx.place_child(
            &mut self.trigger_label,
            (TRIGGER_H_PADDING, trigger_label_y).into(),
        );
        Size::new(TRIGGER_WIDTH, TRIGGER_HEIGHT)
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let colors = theme().popover;
        let trigger_rect =
            Rect::new(0.0, 0.0, TRIGGER_WIDTH, TRIGGER_HEIGHT).to_rounded_rect(TRIGGER_RADIUS);
        fill(scene, &trigger_rect, colors.trigger_bg);
        stroke(scene, &trigger_rect, colors.border, 1.0);

        let chevron_x = TRIGGER_WIDTH - TRIGGER_H_PADDING - CHEVRON_SIZE;
        let chevron_y = (TRIGGER_HEIGHT - CHEVRON_SIZE) / 2.0;
        paint_shapes(
            scene,
            &self.chevron_shapes,
            Rect::new(
                chevron_x,
                chevron_y,
                chevron_x + CHEVRON_SIZE,
                chevron_y + CHEVRON_SIZE,
            ),
            colors.muted_text,
            1.5,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::ComboBox
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        if let Some(idx) = self.selected {
            if let Some(text) = self.options.get(idx) {
                node.set_value(text.to_string());
            }
        }
        node.add_action(accesskit::Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        let mut ids = ChildrenIds::new();
        ids.push(self.trigger_label.id());
        ids
    }
}

// Kept as a public alias so downstream code that previously imported
// `Select` continues to compile.
pub type Select = SelectTrigger;

// ================================================================
// MARK: SelectView — xilem wrapper
// ================================================================

pub struct SelectView<F> {
    header: Option<ArcStr>,
    options: Vec<ArcStr>,
    selected: Option<usize>,
    callback: F,
    disabled: bool,
}

pub struct SelectViewState {
    popup_id: WidgetId,
}

pub fn select<F, State, Action>(
    header: Option<impl Into<ArcStr>>,
    options: impl IntoIterator<Item: Into<ArcStr>>,
    selected: Option<usize>,
    callback: F,
) -> SelectView<F>
where
    F: Fn(&mut State, usize) -> Action + Send + Sync + 'static,
{
    SelectView {
        header: header.map(Into::into),
        options: options.into_iter().map(Into::into).collect(),
        selected,
        callback,
        disabled: false,
    }
}

impl<F> SelectView<F> {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<F> ViewMarker for SelectView<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for SelectView<F>
where
    F: Fn(&mut State, usize) -> Action + Send + Sync + 'static,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<SelectTrigger>;
    type ViewState = SelectViewState;

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        // Pre-reserve the popup's widget id. When the trigger creates the
        // popup layer later it will use this exact id, so any action the
        // popup submits gets routed to this view's `message`.
        let popup_id = WidgetId::next();
        ctx.record_action(popup_id);

        let pod = ctx.with_action_widget(|ctx| {
            let widget = SelectTrigger::new(
                self.header.clone(),
                self.options.clone(),
                self.selected,
                popup_id,
            );
            let mut pod: Pod<SelectTrigger> = ctx.create_pod(widget);
            pod.new_widget.options.disabled = self.disabled;
            pod
        });
        (pod, SelectViewState { popup_id })
    }

    fn rebuild(
        &self,
        prev: &Self,
        _view_state: &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        if prev.disabled != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        if prev.selected != self.selected {
            SelectTrigger::set_selected(&mut element, self.selected);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        // Make sure a still-alive popup layer is torn down with the trigger.
        if SelectTrigger::is_popup_open(&element) {
            element.ctx.remove_layer(view_state.popup_id);
        }
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        if let Some(popup_msg) = message.take_message::<PopupAction>() {
            return match *popup_msg {
                PopupAction::Selected(idx) => {
                    MessageResult::Action((self.callback)(app_state, idx))
                }
                PopupAction::Dismissed => {
                    SelectTrigger::mark_popup_closed(&mut element);
                    MessageResult::Nop
                }
            };
        }
        MessageResult::Stale
    }
}
