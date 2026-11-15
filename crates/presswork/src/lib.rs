mod app;
mod i18n;
mod log_panel;
mod logging;
mod workspace;

use app::App;
use presswork_log::LogStore;

pub fn run() -> iced::Result {
    let logging = logging::start();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Presswork started");
    let store = logging.as_ref().map_or_else(
        || LogStore::with_capacity(1),
        |logging| logging.store.clone(),
    );
    iced::application(move || App::new(store.clone()), App::update, App::view)
        .subscription(App::subscription)
        .title(App::title)
        .window_size((1280.0, 800.0))
        .centered()
        .run()
}
