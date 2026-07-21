//! # Sidebar layout
//!
//! Shadcn-style declarative sidebar for crownuikit.
//!
//! ## Low-level design (SOLID)
//!
//! ### Single-responsibility
//! - [`item::SidebarItem`] — a single nav row. Owns its own hover / selected
//!   springs and paints the "raised pill" background. Nothing else.
//! - [`collapse::SidebarCollapseWidget`] — a general-purpose container that
//!   animates the visible height of one child. Ignorant of what the child is.
//! - View constructors ([`sidebar`], [`sidebar_item`], [`sidebar_subitem`],
//!   [`sidebar_group`], [`sidebar_brand`], [`sidebar_separator`]) — pure
//!   composition. They wire the underlying widgets into shapes the app
//!   author actually wants to write.
//!
//! ### Open / closed
//! Each view returns a `#[must_use]` builder value. New surface options
//! (badges, right-aligned counts, disabled variants) can be added as
//! chainable methods without changing the widget's private state.
//!
//! ### Interface segregation
//! The widgets don't know about each other. `SidebarItem` doesn't know it
//! lives inside a group; `SidebarCollapseWidget` doesn't know it's holding
//! sidebar rows. Callers compose the pieces they need.
//!
//! ### Dependency inversion
//! - Colors are read from [`crate::config::theme`] where applicable, not
//!   hardcoded per widget — swap the theme and the whole kit follows.
//! - Icons arrive as `&'static str` SVG bodies. Any icon set works
//!   (`blinc_icons`, custom sprites, …) — no compile-time coupling.
//! - Selection state is external. The sidebar paints whatever the app
//!   tells it, so a router, tab enum, or hash map can all drive it.
//!
//! ### Liskov substitution
//! `sidebar_item`, `sidebar_subitem`, `sidebar_brand`, and the group header
//! all return the same [`item::SidebarItemView`] type with different
//! `SidebarItemKind` inside. They're freely interchangeable inside a
//! sidebar's flex column.
//!
//! ## Example
//!
//! ```ignore
//! use blinc_icons::icons;
//! use crownuikit::layouts::sidebar::{
//!     sidebar, sidebar_brand, sidebar_group, sidebar_item, sidebar_separator,
//!     sidebar_subitem,
//! };
//! use xilem::view::flex_col;
//! use xilem::masonry::properties::types::AsUnit;
//!
//! sidebar(flex_col((
//!     sidebar_brand("Untitled UI", icons::INFINITY, |_s: &mut State| ()),
//!     sidebar_item("Dashboard", icons::HOME, s.tab == Tab::Dashboard,
//!         |s: &mut State| s.tab = Tab::Dashboard),
//!     sidebar_separator(),
//!     sidebar_group("Bank accounts", icons::LANDMARK, s.accounts_open,
//!         |s: &mut State| s.accounts_open = !s.accounts_open,
//!         flex_col((
//!             sidebar_subitem("Local currency", s.tab == Tab::Local,
//!                 |s: &mut State| s.tab = Tab::Local),
//!             sidebar_subitem("Foreign currency", s.tab == Tab::Foreign,
//!                 |s: &mut State| s.tab = Tab::Foreign),
//!         )).gap(2.0.px())),
//! )).gap(2.0.px()))
//! ```

mod collapse;
mod item;

pub use collapse::{sidebar_collapse, SidebarCollapse, SidebarCollapseWidget};
pub use item::{
    sidebar_brand, sidebar_item, sidebar_subitem, SidebarActivated, SidebarItem, SidebarItemKind,
    SidebarItemView,
};

use blinc_icons::icons;
use xilem::core::{MessageContext, MessageResult, Mut, View, ViewMarker};
use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, ArcStr, BoxConstraints, ChildrenIds, LayoutCtx, NoAction, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, Widget,
};
use xilem::masonry::kurbo::{Rect, Size};
use xilem::masonry::peniko::Color;
use xilem::masonry::properties::types::AsUnit;
use xilem::masonry::properties::Padding;
use xilem::masonry::util::fill;
use xilem::masonry::vello::Scene;
use xilem::style::Style;
use xilem::view::{flex_col, sized_box};
use xilem::{Pod, ViewCtx, WidgetView};

// --- MARK: Sidebar chrome constants ---

/// Warm off-white background matching the design mockups.
const SIDEBAR_BG: Color = Color::from_rgb8(0xF4, 0xF3, 0xF0);
const SIDEBAR_WIDTH: f64 = 260.0;
/// Padding around the whole item list.
const SIDEBAR_PADDING: f64 = 12.0;
/// Vertical gap between rows. Kept wide enough to swallow the selected
/// pill's drop-shadow so it doesn't visibly bleed onto sibling rows.
const ITEM_GAP: f64 = 4.0;
/// Divider color.
const SEPARATOR_COLOR: Color = Color::from_rgb8(0xE2, 0xE1, 0xDD);
/// Divider height (with vertical breathing room).
const SEPARATOR_HEIGHT: f64 = 12.0;

// --- MARK: Root container ---

/// Wraps a pre-composed content view in the sidebar's chrome (fixed width,
/// warm background, generous padding). The caller composes the actual
/// items with `flex_col` — same convention `sized_box` uses in xilem.
///
/// ```ignore
/// sidebar(flex_col((
///     sidebar_item(...),
///     sidebar_item(...),
///     sidebar_separator(),
///     ...
/// )).gap(2.0.px()))
/// ```
pub fn sidebar<State, Action, V>(content: V) -> impl WidgetView<State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    // Order matters: `.width` and `.expand_height` are inherent to
    // `SizedBox`; the Style trait methods (`padding`, `background_color`)
    // return a wrapping `Prop<_>`, so any inherent-method calls must come
    // first.
    sized_box(content)
        .width(SIDEBAR_WIDTH.px())
        .expand_height()
        .padding(Padding::all(SIDEBAR_PADDING))
        .background_color(SIDEBAR_BG)
}

/// Convenience: `sidebar_column((a, b, c))` == `flex_col((a, b, c)).gap(2)`.
/// Callers can drop to `flex_col` directly for custom gaps.
pub fn sidebar_column<State, Action, Seq>(items: Seq) -> impl WidgetView<State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: xilem::view::FlexSequence<State, Action> + Send + Sync + 'static,
{
    flex_col(items).gap(ITEM_GAP.px())
}

// --- MARK: Separator ---

/// Thin horizontal divider used to visually chunk the sidebar into groups.
pub fn sidebar_separator() -> SidebarSeparator {
    SidebarSeparator
}

pub struct SidebarSeparator;

/// The Masonry-side widget backing [`sidebar_separator`]. Kept trivial —
/// just paints a hairline centered in its vertical space.
pub struct SidebarSeparatorWidget;

impl Widget for SidebarSeparatorWidget {
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
        let width = bc.max().width;
        Size::new(width, SEPARATOR_HEIGHT)
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let y = (size.height - 1.0) / 2.0;
        let rect = Rect::new(0.0, y, size.width, y + 1.0);
        fill(scene, &rect, SEPARATOR_COLOR);
    }

    fn accessibility_role(&self) -> Role {
        Role::Splitter
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

impl ViewMarker for SidebarSeparator {}
impl<State, Action> View<State, Action, ViewCtx> for SidebarSeparator
where
    State: 'static,
    Action: 'static,
{
    type Element = Pod<SidebarSeparatorWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        (ctx.create_pod(SidebarSeparatorWidget), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        _element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
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
        _element: Mut<'_, Self::Element>,
        _app_state: &mut State,
    ) -> MessageResult<Action> {
        MessageResult::Nop
    }
}

// --- MARK: Group (collapsible section) ---

/// A collapsible group: header row (with icon + chevron) plus animated
/// sub-items below it. `on_toggle` runs when the header is clicked; the app
/// is responsible for flipping the `open` bool it passed in.
///
/// The `children` view is typically a `flex_col` of `sidebar_subitem`s but
/// can be any view — the collapse is content-agnostic.
///
/// ```ignore
/// sidebar_group(
///     "Bank accounts", icons::LANDMARK,
///     state.accounts_open,
///     |s: &mut State| s.accounts_open = !s.accounts_open,
///     flex_col((
///         sidebar_subitem("Local currency", ...),
///         sidebar_subitem("Foreign currency", ...),
///     )).gap(2.0.px()),
/// )
/// ```
pub fn sidebar_group<F, State, Action, Children, L>(
    label: L,
    icon: &'static str,
    open: bool,
    on_toggle: F,
    children: Children,
) -> impl WidgetView<State, Action>
where
    F: Fn(&mut State) -> Action + Send + Sync + 'static,
    State: 'static,
    Action: 'static,
    Children: WidgetView<State, Action>,
    L: Into<ArcStr>,
{
    let header = item::group_header(label.into(), icon, icons::CHEVRON_DOWN, open, on_toggle);
    let body = sidebar_collapse(children, open);
    flex_col((header, body)).gap(ITEM_GAP.px())
}
