#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gpui_kit::*;

mod app;
mod import;
mod model;
mod sankey;
mod settings;
mod storage;

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1360.0), px(860.0)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Budget".into()),
                ..Default::default()
            }),
            window_min_size: Some(size(px(1040.0), px(660.0))),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| app::BudgetApp::new(window, cx))
        })
        .expect("failed to open window");
        cx.activate(true);
    });
}
