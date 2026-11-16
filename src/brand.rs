//! Brand colors shared with the website: a teal accent on warm stone neutrals.
//!
//! The theme library ships its own light and dark palettes. This module overrides the
//! colors the app uses, every time the theme changes (light, dark or system appearance).

use gpui_kit::component::{ActiveTheme as _, Theme};
use gpui_kit::*;

fn hsla(rgb: u32) -> Hsla {
    gpui_kit::rgb(rgb).into()
}

/// Page background behind the cards.
pub fn page(dark: bool) -> Hsla {
    hsla(if dark { 0x141312 } else { 0xfafaf9 })
}

struct Palette {
    background: u32,
    foreground: u32,
    muted: u32,
    muted_foreground: u32,
    border: u32,
    primary: u32,
    primary_hover: u32,
    primary_active: u32,
    ring: u32,
    sidebar: u32,
    sidebar_accent: u32,
    success: u32,
    danger: u32,
}

const LIGHT: Palette = Palette {
    background: 0xffffff,
    foreground: 0x1c1917,
    muted: 0xf5f5f4,
    muted_foreground: 0x6b6560,
    border: 0xe7e5e4,
    primary: 0x0f766e,
    primary_hover: 0x115e59,
    primary_active: 0x134e4a,
    ring: 0x0f766e,
    sidebar: 0xf5f5f4,
    sidebar_accent: 0xebe9e7,
    // Darker than the usual green and red so that text on white stays readable.
    success: 0x15803d,
    danger: 0xdc2626,
};

const DARK: Palette = Palette {
    background: 0x1c1a19,
    foreground: 0xf5f5f4,
    muted: 0x262321,
    muted_foreground: 0xa8a29e,
    border: 0x2e2b29,
    primary: 0x0f766e,
    primary_hover: 0x138178,
    primary_active: 0x0f766e,
    ring: 0x5eead4,
    sidebar: 0x171514,
    sidebar_accent: 0x2c2927,
    success: 0x4ade80,
    danger: 0xf87171,
};

/// Applies the brand colors to the current theme. Cheap to call again: it does nothing when
/// the colors are already in place.
pub fn apply(cx: &mut App) {
    let dark = cx.theme().is_dark();
    let p = if dark { &DARK } else { &LIGHT };
    if cx.theme().colors.primary == hsla(p.primary) {
        return;
    }
    Theme::update(cx, |theme| {
        let c = &mut theme.colors;
        c.background = hsla(p.background);
        c.foreground = hsla(p.foreground);
        c.muted = hsla(p.muted);
        c.muted_foreground = hsla(p.muted_foreground);
        c.border = hsla(p.border);
        c.accent = hsla(p.muted);
        c.accent_foreground = hsla(p.foreground);
        c.popover = hsla(p.background);
        c.popover_foreground = hsla(p.foreground);
        c.ring = hsla(p.ring);
        c.primary = hsla(p.primary);
        c.primary_foreground = hsla(0xffffff);
        c.primary_hover = hsla(p.primary_hover);
        c.primary_active = hsla(p.primary_active);
        c.button_primary = hsla(p.primary);
        c.button_primary_foreground = hsla(0xffffff);
        c.button_primary_hover = hsla(p.primary_hover);
        c.button_primary_active = hsla(p.primary_active);
        c.sidebar = hsla(p.sidebar);
        c.sidebar_foreground = hsla(p.foreground);
        c.sidebar_border = hsla(p.border);
        c.sidebar_accent = hsla(p.sidebar_accent);
        c.sidebar_accent_foreground = hsla(p.foreground);
        c.success = hsla(p.success);
        c.danger = hsla(p.danger);
    });
}
