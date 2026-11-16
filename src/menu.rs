//! The macOS menu bar (the strip next to the Apple logo): an application menu with
//! Hide and Quit, and a Window menu. Without it the strip shows only the app name.
//!
//! Other platforms have no global menu bar, so everything here does nothing there.
//! The texts follow the language of the app, see [`set`].

use gpui_kit::*;

use crate::i18n::{Locale, Msg};

actions!(cashflow, [Quit, Hide, HideOthers, ShowAll, Minimize, Zoom]);

const ENABLED: bool = cfg!(target_os = "macos");

/// Registers the menu actions and their shortcuts. Call once at startup.
pub fn init(cx: &mut App) {
    if !ENABLED {
        return;
    }
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &Minimize, cx| with_active_window(cx, |window| window.minimize_window()));
    cx.on_action(|_: &Zoom, cx| with_active_window(cx, |window| window.zoom_window()));
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]);
}

fn with_active_window(cx: &mut App, f: impl FnOnce(&mut Window)) {
    if let Some(handle) = cx.active_window() {
        handle.update(cx, |_, window, _| f(window)).ok();
    }
}

/// Builds the menus in the language of `lc`. Call again when the language changes.
pub fn set(cx: &App, lc: &Locale) {
    if !ENABLED {
        return;
    }
    cx.set_menus([
        Menu::new("CashFlow").items([
            MenuItem::os_submenu(lc.t(Msg::MenuServices), SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action(lc.t(Msg::MenuHide), Hide),
            MenuItem::action(lc.t(Msg::MenuHideOthers), HideOthers),
            MenuItem::action(lc.t(Msg::MenuShowAll), ShowAll),
            MenuItem::separator(),
            MenuItem::action(lc.t(Msg::MenuQuit), Quit),
        ]),
        Menu::new(lc.t(Msg::MenuWindow)).items([
            MenuItem::action(lc.t(Msg::MenuMinimize), Minimize),
            MenuItem::action(lc.t(Msg::MenuZoom), Zoom),
        ]),
    ]);
}
