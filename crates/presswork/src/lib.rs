mod app;
mod i18n;
mod log_panel;
mod logging;
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
        (
            App::new(store.clone()),
            iced::system::theme().map(app::Message::SystemTheme),
        )
    };
    iced::application(boot, App::update, App::view)
        .settings(iced::Settings {
            fonts: font::all(),
            default_font: font::UI,
            default_text_size: font_size::BODY.into(),
            antialiasing: true,
            ..iced::Settings::default()
        })
        .subscription(App::subscription)
        .theme(App::theme)
        .title(App::title)
        .window_size((1280.0, 800.0))
        .centered()
        .run()
}
