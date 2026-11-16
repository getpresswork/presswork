mod app;
mod chat_panel;
mod chrome;
mod compiler;
mod files;
mod i18n;
mod live_session;
mod log_panel;
mod logging;
mod menu;
#[cfg(test)]
mod screens;
mod server_dialog;
#[cfg(test)]
mod testing;
mod toolchain_panel;
mod unsaved;
mod workspace;

use app::App;
use presswork_log::LogStore;
use presswork_ui::{font, font_size};

pub fn run() -> iced::Result {
    let logging = logging::start();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Presswork started");
    let store = logging.as_ref().map_or_else(
        || LogStore::with_capacity(1),
        |logging| logging.store.clone(),
    );
    let boot = move || {
        let mut app = App::new(store.clone());
        let window = app.open_main_window();
        let compile = app.request_compile();
        let theme = iced::system::theme().map(app::Message::SystemTheme);
        (app, iced::Task::batch([window, theme, compile]))
    };
    iced::daemon(boot, App::update, App::view_window)
        .settings(iced::Settings {
            fonts: font::all(),
            default_font: font::UI,
            default_text_size: font_size::BODY.into(),
            antialiasing: true,
            ..iced::Settings::default()
        })
        .subscription(App::subscription)
        .theme(App::window_theme)
        .title(App::window_title)
        .run()
}
