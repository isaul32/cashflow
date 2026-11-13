// A pure GUI app: never open a console window, in debug builds either.
#![windows_subsystem = "windows"]

use std::borrow::Cow;

use gpui_kit::assets::{Assets, icon_assets};
use gpui_kit::*;

mod app;
mod import;
mod model;
mod sankey;
mod settings;
mod storage;
mod year;

// Icons used by the app that are not in the default component bundle.
icon_assets!(ExtraIcons, [CalendarDays, Tags, X]);

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

    gpui_kit::application().with_assets(AppAssets).run(|cx| {
        gpui_kit::init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1360.0), px(860.0)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("CashFlow".into()),
                ..Default::default()
            }),
            window_min_size: Some(size(px(1040.0), px(660.0))),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| app::CashFlowApp::new(window, cx))
        })
        .expect("failed to open window");
        cx.activate(true);
    });
}
