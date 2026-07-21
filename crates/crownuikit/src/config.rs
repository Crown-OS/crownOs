//! Global theme configuration.
//!
//! Widgets pull their gradient stops from a single [`Theme`] value so the look
//! stays consistent across the kit and can be overridden at startup without
//! touching individual widget files.
//!
//! ```ignore
//! use crownuikit::config::{set_theme, GradientStops, Theme};
//! use xilem::Color;
//!
//! set_theme(Theme {
//!     accent: GradientStops {
//!         start: Color::from_rgb8(0xFF, 0x6A, 0x00),
//!         end:   Color::from_rgb8(0xEE, 0x0F, 0x5A),
//!     },
//!     ..Theme::DEFAULT
//! });
//! ```
//!
//! Reads clone the current theme (cheap — it's a handful of `Color` values),
//! so paint code can call [`theme()`] freely without holding a lock.
//!
//! Updates take an exclusive lock. Prefer calling [`set_theme`] once during
//! app startup rather than per-frame.
//!
//! Note: gradient orientation is per-widget (toggle track = vertical, slider
//! fill = horizontal) — the theme only supplies the color stops.
//! Widgets choose how to project them.
//!
//! To add a new themable surface, extend [`Theme`] with another field and read
//! it from the widget's `paint`.

use std::sync::{LazyLock, RwLock};

use xilem::Color;

/// Two-stop gradient endpoints. Widgets decide the geometry (linear vs. radial,
/// direction, etc.) and how to interpolate between the two.
#[derive(Clone, Copy, Debug)]
pub struct GradientStops {
    pub start: Color,
    pub end: Color,
}

impl GradientStops {
    pub const fn new(start: Color, end: Color) -> Self {
        Self { start, end }
    }
}

/// Flat color slots for popover-shaped surfaces — dropdown panels and their
/// trigger buttons. Kept as a single struct because the trigger and panel
/// always ship together as one visual unit; splitting them would force callers
/// to override two fields to keep them coherent.
#[derive(Clone, Copy, Debug)]
pub struct PopoverColors {
    /// Panel background.
    pub bg: Color,
    /// Panel + trigger border stroke.
    pub border: Color,
    /// Primary text (option labels, selected value in trigger).
    pub text: Color,
    /// De-emphasized text (section headers, chevron).
    pub muted_text: Color,
    /// Hover row background inside the panel.
    pub hover_bg: Color,
    /// Trigger button background.
    pub trigger_bg: Color,
}

impl PopoverColors {
    /// shadcn "dark" defaults — matches the palette the Select widget shipped
    /// with before the theme wiring landed.
    pub const DEFAULT: PopoverColors = PopoverColors {
        bg: Color::from_rgb8(0x18, 0x18, 0x1B),
        border: Color::from_rgb8(0x27, 0x27, 0x2A),
        text: Color::from_rgb8(0xFA, 0xFA, 0xFA),
        muted_text: Color::from_rgb8(0x71, 0x71, 0x7A),
        hover_bg: Color::from_rgb8(0x27, 0x27, 0x2A),
        trigger_bg: Color::from_rgb8(0x0A, 0x0A, 0x0A),
    };
}

/// Global look-and-feel palette. Extend with additional named surfaces as new
/// widgets need themable gradients.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    /// Off-state background of pill toggles. Interpreted as a top→bottom
    /// vertical gradient.
    pub toggle_off: GradientStops,
    /// Primary accent gradient. Used for the toggle's on state (top→bottom)
    /// and the slider's filled track (left→right), so both widgets share the
    /// same active color story.
    pub accent: GradientStops,
    /// Palette for dropdown-style surfaces (Select trigger + panel).
    pub popover: PopoverColors,
}

impl Theme {
    /// Default palette baked into the kit. Kept as an associated `const` so
    /// callers can spread it into a partial override: `Theme { accent: ...,
    /// ..Theme::DEFAULT }`.
    pub const DEFAULT: Theme = Theme {
        toggle_off: GradientStops::new(
            Color::from_rgb8(0xEC, 0xEC, 0xEC),
            Color::from_rgb8(0xD6, 0xD6, 0xD6),
        ),
        accent: GradientStops::new(
            Color::from_rgb8(0x8F, 0x6D, 0xFB),
            Color::from_rgb8(0x6D, 0x48, 0xE8),
        ),
        popover: PopoverColors::DEFAULT,
    };
}

impl Default for Theme {
    fn default() -> Self {
        Self::DEFAULT
    }
}

static THEME: LazyLock<RwLock<Theme>> = LazyLock::new(|| RwLock::new(Theme::DEFAULT));

/// Snapshot the current global theme. Cheap enough to call from paint code.
pub fn theme() -> Theme {
    *THEME.read().expect("crownuikit theme lock poisoned")
}

/// Replace the global theme. Intended for one-shot configuration during app
/// startup; calling it mid-frame will not force a repaint on its own.
pub fn set_theme(new: Theme) {
    *THEME.write().expect("crownuikit theme lock poisoned") = new;
}
