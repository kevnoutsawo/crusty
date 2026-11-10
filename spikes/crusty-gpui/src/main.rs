//! Spike: Crusty as a native GPUI desktop app, on the same core crates as the TUI.

#[allow(dead_code)]
mod anim;
mod app;
#[allow(dead_code)]
mod input;
mod theme;

use gpui::{
    App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size,
};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    Application::new().run(|cx: &mut App| {
        input::bind_keys(cx);
        cx.set_global(theme::dark());

        let bounds = Bounds::centered(None, size(px(1100.), px(760.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Crusty".into()),
                        ..Default::default()
                    }),
                    window_min_size: Some(size(px(640.), px(420.))),
                    app_id: Some("crusty".into()),
                    ..Default::default()
                },
                |_, cx| cx.new(app::CrustyApp::new),
            )
            .expect("failed to open window");

        window
            .update(cx, |view, window, cx| view.focus_url(window, cx))
            .ok();
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
