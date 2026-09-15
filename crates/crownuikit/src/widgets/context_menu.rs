//! A macOS-style context menu built from small composable pieces.
//!
//! The menu is a rounded rectangle container that stacks rows vertically.
//! Row types:
//! - [`menu_item`]: icon + label + trailing shortcut.
//! - [`menu_item_selected`]: same layout, filled with the accent color.
//! - [`menu_item_disabled`]: same layout, dimmed and non-interactive.
//! - [`menu_submenu`]: label with a trailing chevron marker.
//! - [`menu_separator`]: hairline.
//! - [`menu_header`]: section header in muted uppercase text.

use blinc_icons::icons;
use xilem::masonry::parley::style::FontWeight;
use xilem::masonry::properties::Padding;
use xilem::masonry::properties::types::AsUnit;
use xilem::style::Style;
use xilem::view::{
    CrossAxisAlignment, FlexExt, FlexSequence, FlexSpacer, MainAxisAlignment, flex_col, flex_row,
    label, sized_box,
};
use xilem::{AnyWidgetView, Color, WidgetView};

use crate::util::INTER;
use crate::widgets::icon::icon;

// --- MARK: Palette ---
const BG_COLOR: Color = Color::from_rgb8(0xF7, 0xF7, 0xF7);
const BORDER_COLOR: Color = Color::from_rgb8(0xE1, 0xE1, 0xE3);
const ACCENT_COLOR: Color = Color::from_rgb8(0x1F, 0x6B, 0xFF);
const PRIMARY_TEXT: Color = Color::from_rgb8(0x1B, 0x1B, 0x1F);
const SHORTCUT_TEXT: Color = Color::from_rgb8(0x8E, 0x8E, 0x93);
const DISABLED_TEXT: Color = Color::from_rgb8(0xBD, 0xBD, 0xC2);
const HEADER_TEXT: Color = Color::from_rgb8(0x8E, 0x8E, 0x93);
const SEPARATOR_COLOR: Color = Color::from_rgb8(0xE4, 0xE4, 0xE7);
const WHITE: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF);

// --- MARK: Metrics ---
const ROW_HEIGHT: f64 = 30.0;
const ICON_SLOT_WIDTH: f64 = 30.0;
const ICON_SIZE: f64 = 16.0;
const ROW_H_PADDING: f64 = 10.0;
const MENU_WIDTH: f64 = 340.0;
const LABEL_FONT_SIZE: f32 = 15.0;
const SHORTCUT_FONT_SIZE: f32 = 14.0;
const HEADER_FONT_SIZE: f32 = 12.0;

// --- MARK: Rows ---

fn row_body<State: 'static, Action: 'static>(
    icon_svg: Option<&'static str>,
    text: &'static str,
    shortcut: &'static str,
    text_color: Color,
    shortcut_color: Color,
    show_chevron: bool,
) -> impl WidgetView<State, Action> + use<State, Action> {
    let icon_slot: Box<AnyWidgetView<State, Action>> = match icon_svg {
        Some(svg) => Box::new(
            sized_box(icon(svg).size(ICON_SIZE).color(text_color))
                .width(ICON_SLOT_WIDTH.px())
                .height(ICON_SIZE.px()),
        ),
        None => Box::new(
            sized_box(label(""))
                .width(ICON_SLOT_WIDTH.px())
                .height(ICON_SIZE.px()),
        ),
    };

    let label_view = label(text)
        .text_size(LABEL_FONT_SIZE)
        .font(INTER)
        .color(text_color);

    let trailing: Box<AnyWidgetView<State, Action>> = if show_chevron {
        Box::new(
            sized_box(icon(icons::CHEVRON_RIGHT).size(14.0).color(text_color))
                .width(16.0.px())
                .height(16.0.px()),
        )
    } else if shortcut.is_empty() {
        Box::new(label(""))
    } else {
        Box::new(
            label(shortcut)
                .text_size(SHORTCUT_FONT_SIZE)
                .font(INTER)
                .color(shortcut_color),
        )
    };

    flex_row((
        icon_slot,
        label_view.flex(CrossAxisAlignment::Center),
        FlexSpacer::Flex(1.0),
        trailing,
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Start)
    .gap(0.0.px())
}

/// Standard menu row.
pub fn menu_item<State: 'static, Action: 'static>(
    icon_svg: Option<&'static str>,
    text: &'static str,
    shortcut: &'static str,
) -> impl WidgetView<State, Action> + use<State, Action> {
    sized_box(row_body(
        icon_svg,
        text,
        shortcut,
        PRIMARY_TEXT,
        SHORTCUT_TEXT,
        false,
    ))
    .expand_width()
    .height(ROW_HEIGHT.px())
    .padding(Padding::horizontal(ROW_H_PADDING))
}

/// Selected/hovered menu row — filled with the accent color, white text.
pub fn menu_item_selected<State: 'static, Action: 'static>(
    icon_svg: Option<&'static str>,
    text: &'static str,
    shortcut: &'static str,
) -> impl WidgetView<State, Action> + use<State, Action> {
    sized_box(row_body(icon_svg, text, shortcut, WHITE, WHITE, false))
        .expand_width()
        .height(ROW_HEIGHT.px())
        .padding(Padding::horizontal(ROW_H_PADDING))
        .background_color(ACCENT_COLOR)
        .corner_radius(5.0)
}

/// Disabled row — dimmed, non-actionable.
pub fn menu_item_disabled<State: 'static, Action: 'static>(
    icon_svg: Option<&'static str>,
    text: &'static str,
    shortcut: &'static str,
) -> impl WidgetView<State, Action> + use<State, Action> {
    sized_box(row_body(
        icon_svg,
        text,
        shortcut,
        DISABLED_TEXT,
        DISABLED_TEXT,
        false,
    ))
    .expand_width()
    .height(ROW_HEIGHT.px())
    .padding(Padding::horizontal(ROW_H_PADDING))
}

/// Submenu row — trailing chevron in place of a shortcut.
pub fn menu_submenu<State: 'static, Action: 'static>(
    icon_svg: Option<&'static str>,
    text: &'static str,
) -> impl WidgetView<State, Action> + use<State, Action> {
    sized_box(row_body(
        icon_svg,
        text,
        "",
        PRIMARY_TEXT,
        SHORTCUT_TEXT,
        true,
    ))
    .expand_width()
    .height(ROW_HEIGHT.px())
    .padding(Padding::horizontal(ROW_H_PADDING))
}

/// A hairline separator between logical sections.
pub fn menu_separator<State: 'static, Action: 'static>()
-> impl WidgetView<State, Action> + use<State, Action> {
    sized_box(label(""))
        .expand_width()
        .height(1.0.px())
        .background_color(SEPARATOR_COLOR)
}

/// Section header — small muted text, indented to align with row labels.
pub fn menu_header<State: 'static, Action: 'static>(
    text: &'static str,
) -> impl WidgetView<State, Action> + use<State, Action> {
    let header_padding = Padding {
        top: 4.0,
        bottom: 4.0,
        left: ROW_H_PADDING + ICON_SLOT_WIDTH,
        right: ROW_H_PADDING,
    };
    sized_box(
        label(text)
            .text_size(HEADER_FONT_SIZE)
            .weight(FontWeight::MEDIUM)
            .font(INTER)
            .color(HEADER_TEXT),
    )
    .expand_width()
    .height(22.0.px())
    .padding(header_padding)
}

// --- MARK: Container ---

/// Wrap a set of rows into a rounded menu container matching the reference design.
pub fn context_menu<State: 'static, Action: 'static, Seq>(
    items: Seq,
) -> impl WidgetView<State, Action> + use<State, Action, Seq>
where
    Seq: FlexSequence<State, Action> + Send + Sync + 'static,
{
    sized_box(
        flex_col(items)
            .gap(2.0.px())
            .cross_axis_alignment(CrossAxisAlignment::Fill),
    )
    .width(MENU_WIDTH.px())
    .padding(Padding::vertical(6.0))
    .background_color(BG_COLOR)
    .border(BORDER_COLOR, 1.0)
    .corner_radius(12.0)
}

// --- MARK: Preview builder ---

/// A ready-made preview matching the reference image.
pub fn preview_menu<State: 'static, Action: 'static>()
-> impl WidgetView<State, Action> + use<State, Action> {
    context_menu((
        menu_item::<State, Action>(Some(icons::UNDO), "Undo", "⌘ Z"),
        menu_item::<State, Action>(Some(icons::REDO), "Redo", "⇧ ⌘ Z"),
        menu_separator::<State, Action>(),
        menu_item::<State, Action>(Some(icons::CLIPBOARD), "Paste", "⌘ V"),
        menu_item_selected::<State, Action>(
            Some(icons::CLIPBOARD),
            "Paste and Match Style",
            "⌥ ⇧ ⌘ V",
        ),
        menu_item::<State, Action>(Some(icons::TRASH_2), "Delete", "⌫"),
        menu_item::<State, Action>(Some(icons::SQUARE_ARROW_UP), "Select All", "⌘ A"),
        menu_item::<State, Action>(None, "Paste as Quotation", "⇧ ⌘ V"),
        menu_item_disabled::<State, Action>(None, "Add Link", "⌘ K"),
        menu_separator::<State, Action>(),
        menu_submenu::<State, Action>(Some(icons::FILES), "Find"),
        menu_separator::<State, Action>(),
        menu_header::<State, Action>("Header"),
        menu_item::<State, Action>(None, "Dictation", "D"),
        menu_item::<State, Action>(Some(icons::SMILE), "Emoji", "E"),
    ))
}
