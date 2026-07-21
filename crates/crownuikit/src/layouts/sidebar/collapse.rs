//! Spring-driven collapse container.
//!
//! Wraps any single child widget and animates its visible height between
//! `0` and the child's natural size when [`open`](SidebarCollapse::open) is
//! toggled. Used by `sidebar_group` to reveal/hide its sub-items smoothly.
//!
//! The child is always laid out at its full natural size — we only clip and
//! translate what's visible. That keeps the child's own layout stable
//! throughout the animation (no re-flow at every anim frame) and lets us
//! measure the target height on frame 1.
//!
//! Design note: this is deliberately a general-purpose container. It knows
//! nothing about sidebar items specifically, which lets the same primitive
//! back other collapsible surfaces in the future (accordion, dropdown, etc.).

use xilem::core::{MessageContext, MessageResult, Mut, View, ViewMarker};
use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NoAction, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetMut, WidgetPod,
};
use xilem::masonry::kurbo::{Point, Rect, Size};
use xilem::masonry::vello::Scene;
use xilem::{Pod, ViewCtx, WidgetView};

use crate::animation::{Clock, Spring};

/// The custom widget that owns the animation state.
pub struct SidebarCollapseWidget {
    open: bool,
    progress: Spring,
    clock: Clock,
    child: WidgetPod<dyn Widget>,
}

impl SidebarCollapseWidget {
    pub fn new(open: bool, child: WidgetPod<dyn Widget>) -> Self {
        Self {
            open,
            progress: Spring::new(if open { 1.0 } else { 0.0 }),
            clock: Clock::new(),
            child,
        }
    }

    pub fn set_open(this: &mut WidgetMut<'_, Self>, open: bool) {
        if this.widget.open == open {
            return;
        }
        this.widget.open = open;
        this.widget
            .progress
            .set_target(if open { 1.0 } else { 0.0 });
        this.widget.clock.reset();
        this.ctx.request_layout();
        this.ctx.request_anim_frame();
    }

    pub fn child_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
    ) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl Widget for SidebarCollapseWidget {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        // Only accept clicks when we're open — a fully-closed panel
        // shouldn't eat pointer events from underneath.
        self.progress.position > 0.01
    }

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _interval: u64,
    ) {
        let dt = self.clock.tick();
        self.progress.step(dt);
        // Height is driven by progress, so layout — not just paint — has to
        // rerun each frame.
        ctx.request_layout();
        if !self.progress.at_rest() {
            ctx.request_anim_frame();
        } else {
            self.progress.snap_to_target();
            self.clock.reset();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        // Always give the child its natural size — a hidden child stays
        // laid out at full height so we don't lose our target dimensions
        // mid-animation.
        let child_bc =
            BoxConstraints::new(Size::new(bc.min().width, 0.0), Size::new(bc.max().width, f64::INFINITY));
        let child_size = ctx.run_layout(&mut self.child, &child_bc);
        ctx.place_child(&mut self.child, Point::ORIGIN);

        let t = self.progress.position.clamp(0.0, 1.0) as f64;
        let visible_h = child_size.height * t;

        // Clip contents to the visible band so labels stay inside the
        // collapsing area on the way down.
        ctx.set_clip_path(Rect::new(0.0, 0.0, child_size.width, visible_h));
        Size::new(child_size.width, visible_h)
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

// --- MARK: Xilem view ---

/// A view that wraps a single child and animates its expansion. The child
/// itself can be any xilem view (typically a `flex_col` of items).
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct SidebarCollapse<V, State, Action = ()> {
    inner: V,
    open: bool,
    _phantom: std::marker::PhantomData<fn() -> (State, Action)>,
}

/// Wrap `inner` so that its visible height animates between fully open and
/// fully closed as `open` changes.
pub fn sidebar_collapse<V, State, Action>(inner: V, open: bool) -> SidebarCollapse<V, State, Action>
where
    V: WidgetView<State, Action>,
    State: 'static,
    Action: 'static,
{
    SidebarCollapse {
        inner,
        open,
        _phantom: std::marker::PhantomData,
    }
}

impl<V, State, Action> ViewMarker for SidebarCollapse<V, State, Action> {}
impl<V, State, Action> View<State, Action, ViewCtx> for SidebarCollapse<V, State, Action>
where
    V: WidgetView<State, Action>,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<SidebarCollapseWidget>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = self.inner.build(ctx, app_state);
        let widget = SidebarCollapseWidget::new(self.open, child.new_widget.erased().to_pod());
        (ctx.create_pod(widget), child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if prev.open != self.open {
            SidebarCollapseWidget::set_open(&mut element, self.open);
        }
        let mut child = SidebarCollapseWidget::child_mut(&mut element);
        self.inner
            .rebuild(&prev.inner, view_state, ctx, child.downcast(), app_state);
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = SidebarCollapseWidget::child_mut(&mut element);
        self.inner.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut child = SidebarCollapseWidget::child_mut(&mut element);
        self.inner
            .message(view_state, message, child.downcast(), app_state)
    }
}
