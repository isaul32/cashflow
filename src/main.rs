// A pure GUI app: never open a console window, in debug builds either.
#![windows_subsystem = "windows"]

use std::borrow::Cow;

use gpui_kit::assets::{Assets, icon_assets};
use gpui_kit::component::TitleBar;
use gpui_kit::*;

mod app;
mod backup;
mod brand;
mod data;
mod i18n;
mod import;
mod mcp;
mod menu;
mod model;
mod recurring;
mod sankey;
mod settings;
mod storage;
mod update;
mod year;

// Icons used by the app that are not in the default component bundle.
icon_assets!(ExtraIcons, [CalendarDays, Repeat, Tags, X]);

/// The default component icons plus [`ExtraIcons`].
struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraIcons.load(path)? {
            return Ok(Some(bytes));
        }
        Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut all = Assets.list(path)?;
        all.extend(ExtraIcons.list(path)?);
        Ok(all)
    }
}

/// Debug builds only: when started from a terminal (`cargo run`), print to that
/// terminal like a console app would. Without a parent console this does nothing,
/// so double-clicking the exe never opens a console window.
#[cfg(all(windows, debug_assertions))]
fn attach_parent_console() {
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    // SAFETY: plain Win32 call with a constant argument; failure is harmless.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn main() {
    #[cfg(all(windows, debug_assertions))]
    attach_parent_console();

    // `cashflow --mcp` is what an AI assistant starts to use the app: no window, MCP over stdio.
    if std::env::args().skip(1).any(|a| a == "--mcp") {
        std::process::exit(mcp::run_stdio());
    }

    gpui_kit::application().with_assets(AppAssets).run(|cx| {
        gpui_kit::init(cx);
        menu::init(cx);
        // Closing the last window ends the app, also on macOS (where it would stay in the Dock).
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        // macOS: the app draws its own title bar (see CashFlowApp::render), so the window
        // gets the transparent title bar the component expects, with the traffic lights inside.
        let (base, titlebar) = if cfg!(target_os = "macos") {
            (TitleBar::window_options(), TitleBar::title_bar_options())
        } else {
            (WindowOptions::default(), TitlebarOptions::default())
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                // Developer override (CASHFLOW_SIZE=WxH), used to capture screenshots.
                std::env::var("CASHFLOW_SIZE")
                    .ok()
                    .and_then(|v| {
                        let (w, h) = v.split_once('x')?;
                        Some(size(px(w.parse().ok()?), px(h.parse().ok()?)))
                    })
                    .unwrap_or_else(|| size(px(1360.0), px(860.0))),
                cx,
            ))),
            titlebar: Some(TitlebarOptions { title: Some("CashFlow".into()), ..titlebar }),
            window_min_size: Some(size(px(800.0), px(600.0))),
            ..base
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| app::CashFlowApp::new(window, cx))
        })
        .expect("failed to open window");
        cx.activate(true);
    });
}
