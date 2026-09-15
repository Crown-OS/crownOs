//! A single sidebar row: leading icon, label, optional trailing icon.
//!
//! Three visual states, all interpolated by two springs:
//!
//! - **Idle** — transparent background, muted foreground.
//! - **Hover** — subtle dark overlay.
//! - **Selected** — white "raised" pill with a hairline border and a soft
//!   drop-shadow underneath, drawing the eye without competing with content.
//!
//! Selection and hover each carry their own spring (`select_progress`,
//! `hover_progress`) so overlap is smooth: hovering a selected item does not
//! reset the "selected" animation and vice-versa.
//!
//! The trailing chevron rotates with `select_progress` when used as a group
//! header (see [`crate::layouts::sidebar::group`]), so the same widget can
//! back both leaf items and expandable group headers.

use xilem::core::{MessageContext, MessageResult, Mut, View, ViewMarker};
use xilem::masonry::accesskit::{self, Node, Role};
use xilem::masonry::core::{
    AccessCtx, AccessEvent, ArcStr, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, NewWidget,
    PaintCtx, PointerButton, PointerButtonEvent, PointerEvent, Properties, PropertiesMut,
    PropertiesRef, RegisterCtx, StyleProperty, TextEvent, Update, UpdateCtx, Widget, WidgetMut,
    WidgetPod,
};
use xilem::masonry::kurbo::{Rect, Size};
use xilem::masonry::peniko::Color;
use xilem::masonry::peniko::color::palette;
use xilem::masonry::properties::ContentColor;
use xilem::masonry::util::{fill, stroke};
use xilem::masonry::vello::Scene;
use xilem::masonry::widgets::Label;
use xilem::{Pod, ViewCtx};

use crate::animation::{Clock, Spring};
use crate::util::lerp_color;
use crate::widgets::icon::{IconShape, paint_shapes, parse_shapes};

// --- MARK: Metrics ---
const ROW_HEIGHT: f64 = 36.0;
const ROW_RADIUS: f64 = 8.0;
const H_PADDING: f64 = 10.0;
const ICON_SIZE: f64 = 20.0;
const ICON_GAP: f64 = 10.0;
const TRAILING_SIZE: f64 = 16.0;
const BRAND_ICON_SIZE: f64 = 22.0;
const BRAND_HEIGHT: f64 = 44.0;
/// Horizontal indent applied to sub-items (rendered with no leading icon).
const SUBITEM_INDENT: f64 = 20.0;
// A drop-shadow used to live here. It was removed because
// `draw_blurred_rounded_rect` paints beyond the widget's layout bounds,
// and Masonry only invalidates the widget's own rect on hover — so when a
// sibling row repainted, it cleared parts of the selected pill's shadow
// and left visible artifacts until a full re-render. The "raised" look now
// comes from the hairline border alone.

// --- MARK: Palette ---
const IDLE_TEXT: Color = Color::from_rgb8(0x3F, 0x3F, 0x46);
const SELECTED_TEXT: Color = Color::from_rgb8(0x0A, 0x0A, 0x0A);
const SUBITEM_TEXT: Color = Color::from_rgb8(0x5A, 0x5A, 0x63);
const BRAND_TEXT: Color = Color::from_rgb8(0x0A, 0x0A, 0x0A);
const IDLE_ICON: Color = Color::from_rgb8(0x71, 0x71, 0x7A);
const SELECTED_ICON: Color = Color::from_rgb8(0x1F, 0x1F, 0x22);
/// Idle background — fully transparent so the sidebar's fill shows through.
const IDLE_BG: Color = Color::from_rgba8(0, 0, 0, 0);
/// Hover overlay — a whisper of black on the sidebar's warm gray.
const HOVER_BG: Color = Color::from_rgba8(0x0A, 0x0A, 0x0A, 12);
/// Selected background — the "raised" pill.
const SELECTED_BG: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF);
const SELECTED_BORDER: Color = Color::from_rgba8(0x0A, 0x0A, 0x0A, 40);

// --- MARK: Kind ---

/// How a [`SidebarItem`] presents itself. The widget shares one paint path
/// across all three so that switching kind at rebuild is a one-liner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarItemKind {
    /// Top-level nav row with a leading icon (Dashboard, Orders, etc.).
    Item,
    /// Indented nav row with no leading icon (Local currency, Beneficiaries).
    Subitem,
    /// Top-level group header — leading icon plus trailing chevron whose
    /// rotation is driven by the same `select_progress` spring so an "open"
    /// group visually reads as active.
    GroupHeader,
    /// The very top brand row — bigger icon, bolder label, no bg on click.
    Brand,
}

// --- MARK: Action ---

/// Emitted when the user clicks a [`SidebarItem`]. Carries no payload — the
/// meaning of "activated" is up to the view function that wraps the widget
/// (item selection, group toggle, etc.), so the widget stays context-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarActivated;

// --- MARK: Widget ---

pub struct SidebarItem {
    kind: SidebarItemKind,
    selected: bool,
    icon_shapes: Option<Vec<IconShape>>,
    trailing_shapes: Option<Vec<IconShape>>,
    label: WidgetPod<Label>,
    select_progress: Spring,
    hover_progress: Spring,
    clock: Clock,
}

impl SidebarItem {
    pub fn new(
        kind: SidebarItemKind,
        label: impl Into<ArcStr>,
        icon_svg: Option<&'static str>,
        trailing_svg: Option<&'static str>,
        selected: bool,
    ) -> Self {
        let text = label.into();
        let font_size = match kind {
            SidebarItemKind::Brand => 15.0,
            _ => 14.0,
        };
        let color = match kind {
            SidebarItemKind::Brand => BRAND_TEXT,
            SidebarItemKind::Subitem => SUBITEM_TEXT,
            _ => IDLE_TEXT,
        };
        let label_widget = Label::new(text).with_style(StyleProperty::FontSize(font_size));
        let label_pod = NewWidget::new_with_props(
            label_widget,
            Properties::new().with(ContentColor::new(color)),
        )
        .to_pod();

        Self {
            kind,
            selected,
            icon_shapes: icon_svg.map(parse_shapes),
            trailing_shapes: trailing_svg.map(parse_shapes),
            label: label_pod,
            select_progress: Spring::new(if selected { 1.0 } else { 0.0 }),
            hover_progress: Spring::new(0.0),
            clock: Clock::new(),
        }
    }

    pub fn set_selected(this: &mut WidgetMut<'_, Self>, selected: bool) {
        if this.widget.selected == selected {
            return;
        }
        this.widget.selected = selected;
        this.widget
            .select_progress
            .set_target(if selected { 1.0 } else { 0.0 });
        this.widget.clock.reset();
        this.ctx.request_anim_frame();
        // Text color depends on selection — retint the child label.
        let target_color = target_text_color(this.widget.kind, selected);
        let mut label = Self::label_mut(this);
        label.insert_prop(ContentColor::new(target_color));
    }

    pub fn set_label(this: &mut WidgetMut<'_, Self>, label: ArcStr) {
        Label::set_text(&mut Self::label_mut(this), label);
    }

    fn label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label)
    }

    fn icon_x(&self) -> f64 {
        match self.kind {
            SidebarItemKind::Brand => H_PADDING,
            SidebarItemKind::Subitem => H_PADDING + SUBITEM_INDENT,
            _ => H_PADDING,
        }
    }

    fn label_x(&self) -> f64 {
        match self.kind {
            SidebarItemKind::Subitem => H_PADDING + SUBITEM_INDENT,
            SidebarItemKind::Brand => H_PADDING + BRAND_ICON_SIZE + ICON_GAP,
            _ => {
                if self.icon_shapes.is_some() {
                    H_PADDING + ICON_SIZE + ICON_GAP
                } else {
                    H_PADDING
                }
            }
        }
    }

    fn height(&self) -> f64 {
        match self.kind {
            SidebarItemKind::Brand => BRAND_HEIGHT,
            _ => ROW_HEIGHT,
        }
    }
}

fn target_text_color(kind: SidebarItemKind, selected: bool) -> Color {
    match kind {
        SidebarItemKind::Brand => BRAND_TEXT,
        SidebarItemKind::Subitem => {
            if selected {
                SELECTED_TEXT
            } else {
                SUBITEM_TEXT
            }
        }
        _ => {
            if selected {
                SELECTED_TEXT
            } else {
                IDLE_TEXT
            }
        }
    }
}

impl Widget for SidebarItem {
    type Action = SidebarActivated;

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
                    ctx.submit_action::<Self::Action>(SidebarActivated);
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
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if ctx.is_disabled() {
            return;
        }
        if let accesskit::Action::Click = event.action {
            ctx.submit_action::<Self::Action>(SidebarActivated);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if let Update::HoveredChanged(hovered) = event {
            self.hover_progress
                .set_target(if *hovered { 1.0 } else { 0.0 });
            self.clock.reset();
            ctx.request_anim_frame();
        }
        if matches!(
            event,
            Update::FocusChanged(_) | Update::ActiveChanged(_) | Update::DisabledChanged(_)
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
        self.select_progress.step(dt);
        self.hover_progress.step(dt);
        ctx.request_paint_only();
        let sel_rest = self.select_progress.at_rest();
        let hov_rest = self.hover_progress.at_rest();
        if !sel_rest || !hov_rest {
            ctx.request_anim_frame();
        } else {
            self.select_progress.snap_to_target();
            self.hover_progress.snap_to_target();
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
        let total_h = self.height();
        let width = bc.max().width.max(bc.min().width);

        let label_x = self.label_x();
        let trailing_reserve = if self.trailing_shapes.is_some() {
            H_PADDING + TRAILING_SIZE
        } else {
            H_PADDING
        };
        let label_max_w = (width - label_x - trailing_reserve).max(0.0);
        let label_bc = BoxConstraints::new(Size::ZERO, Size::new(label_max_w, total_h));
        let label_size = ctx.run_layout(&mut self.label, &label_bc);
        let label_y = (total_h - label_size.height) / 2.0;
        ctx.place_child(&mut self.label, (label_x, label_y).into());

        Size::new(width, total_h)
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let sel = self.select_progress.position.clamp(0.0, 1.0) as f64;
        let hov = self.hover_progress.position.clamp(0.0, 1.0) as f64;

        // Inset by half a stroke width so the 1px border on the selected
        // pill sits fully inside the widget's own bounds (otherwise the
        // outer half of the stroke leaks 0.5px past the row and shows up
        // as an aliased halo).
        let bg_rect =
            Rect::new(0.5, 0.5, size.width - 0.5, size.height - 0.5).to_rounded_rect(ROW_RADIUS);

        // Brand row is a static header — no bg animation.
        if !matches!(self.kind, SidebarItemKind::Brand) {
            // Background: idle → hover → selected, all interpolated so
            // overlapping transitions blend cleanly.
            let hover_mix = lerp_color(IDLE_BG, HOVER_BG, hov);
            let bg_color = lerp_color(hover_mix, SELECTED_BG, sel);
            fill(scene, &bg_rect, bg_color);

            // Hairline border on the selected pill. Fades in with sel.
            if sel > 0.001 {
                let border_color = Color::new([
                    SELECTED_BORDER.components[0],
                    SELECTED_BORDER.components[1],
                    SELECTED_BORDER.components[2],
                    SELECTED_BORDER.components[3] * sel as f32,
                ]);
                stroke(scene, &bg_rect, border_color, 1.0);
            }
        }

        // Leading icon.
        if let Some(shapes) = &self.icon_shapes {
            let icon_dim = if matches!(self.kind, SidebarItemKind::Brand) {
                BRAND_ICON_SIZE
            } else {
                ICON_SIZE
            };
            let icon_x = self.icon_x();
            let icon_y = (size.height - icon_dim) / 2.0;
            let icon_color = match self.kind {
                SidebarItemKind::Brand => palette::css::BLACK,
                _ => lerp_color(IDLE_ICON, SELECTED_ICON, sel),
            };
            paint_shapes(
                scene,
                shapes,
                Rect::new(icon_x, icon_y, icon_x + icon_dim, icon_y + icon_dim),
                icon_color,
                1.8,
            );
        }

        // Trailing icon — typically a chevron on a group header. Painted
        // without rotation; the group's expand/collapse motion of the
        // children below is the primary "open" signal.
        if let Some(shapes) = &self.trailing_shapes {
            let icon_x = size.width - H_PADDING - TRAILING_SIZE;
            let icon_y = (size.height - TRAILING_SIZE) / 2.0;
            paint_shapes(
                scene,
                shapes,
                Rect::new(
                    icon_x,
                    icon_y,
                    icon_x + TRAILING_SIZE,
                    icon_y + TRAILING_SIZE,
                ),
                IDLE_ICON,
                1.6,
            );
        }
    }

    fn accessibility_role(&self) -> Role {
        match self.kind {
            SidebarItemKind::Brand => Role::Label,
            SidebarItemKind::GroupHeader => Role::Button,
            _ => Role::MenuItem,
        }
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        if !matches!(self.kind, SidebarItemKind::Brand) {
            node.add_action(accesskit::Action::Click);
        }
        if self.selected {
            node.set_selected(true);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.label.id()])
    }
}

// --- MARK: Xilem view ---

/// The [`View`] returned by [`sidebar_item`] and friends.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct SidebarItemView<F> {
    kind: SidebarItemKind,
    label: ArcStr,
    icon: Option<&'static str>,
    trailing: Option<&'static str>,
    selected: bool,
    disabled: bool,
    callback: F,
}

impl<F> SidebarItemView<F> {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Replace the trailing icon (a chevron, badge dot, etc.).
    pub fn trailing_icon(mut self, svg: &'static str) -> Self {
        self.trailing = Some(svg);
        self
    }
}

impl<F> ViewMarker for SidebarItemView<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for SidebarItemView<F>
where
    F: Fn(&mut State) -> Action + Send + Sync + 'static,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<SidebarItem>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        ctx.with_leaf_action_widget(|ctx| {
            let widget = SidebarItem::new(
                self.kind,
                self.label.clone(),
                self.icon,
                self.trailing,
                self.selected,
            );
            let mut pod = ctx.create_pod(widget);
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
            SidebarItem::set_label(&mut element, self.label.clone());
        }
        if prev.selected != self.selected {
            SidebarItem::set_selected(&mut element, self.selected);
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
        match message.take_message::<SidebarActivated>() {
            Some(_) => MessageResult::Action((self.callback)(app_state)),
            None => MessageResult::Stale,
        }
    }
}

// --- MARK: Public constructors (shadcn-style API) ---

/// A leaf sidebar row with a leading icon.
///
/// ```ignore
/// sidebar_item("Dashboard", icons::HOME, state.tab == Tab::Dashboard,
///     |s: &mut State| s.tab = Tab::Dashboard);
/// ```
pub fn sidebar_item<F, State, Action>(
    label: impl Into<ArcStr>,
    icon: &'static str,
    selected: bool,
    callback: F,
) -> SidebarItemView<F>
where
    F: Fn(&mut State) -> Action + Send + Sync + 'static,
{
    SidebarItemView {
        kind: SidebarItemKind::Item,
        label: label.into(),
        icon: Some(icon),
        trailing: None,
        selected,
        disabled: false,
        callback,
    }
}

/// A nested sidebar row — no leading icon, indented under a group.
pub fn sidebar_subitem<F, State, Action>(
    label: impl Into<ArcStr>,
    selected: bool,
    callback: F,
) -> SidebarItemView<F>
where
    F: Fn(&mut State) -> Action + Send + Sync + 'static,
{
    SidebarItemView {
        kind: SidebarItemKind::Subitem,
        label: label.into(),
        icon: None,
        trailing: None,
        selected,
        disabled: false,
        callback,
    }
}

/// Static top-of-sidebar brand row (bigger icon + label, no click). The
/// callback is invoked if the user does click it — useful for "go home" —
/// otherwise wire it to a no-op.
pub fn sidebar_brand<F, State, Action>(
    label: impl Into<ArcStr>,
    icon: &'static str,
    callback: F,
) -> SidebarItemView<F>
where
    F: Fn(&mut State) -> Action + Send + Sync + 'static,
{
    SidebarItemView {
        kind: SidebarItemKind::Brand,
        label: label.into(),
        icon: Some(icon),
        trailing: None,
        selected: false,
        disabled: false,
        callback,
    }
}

/// Header row for [`crate::layouts::sidebar::sidebar_group`]. Not called
/// directly by users — kept `pub(super)` so the group module can build one.
pub(super) fn group_header<F, State, Action>(
    label: impl Into<ArcStr>,
    icon: &'static str,
    trailing: &'static str,
    open: bool,
    callback: F,
) -> SidebarItemView<F>
where
    F: Fn(&mut State) -> Action + Send + Sync + 'static,
{
    SidebarItemView {
        kind: SidebarItemKind::GroupHeader,
        label: label.into(),
        icon: Some(icon),
        trailing: Some(trailing),
        // Repurpose `selected` as "expanded" so the chevron animates and the
        // label subtly tints when the group is open.
        selected: open,
        disabled: false,
        callback,
    }
}
