mod app;
mod i18n;
mod logging;
mod workspace;

use app::App;

pub fn run() -> iced::Result {
    let _logging = logging::start();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Presswork started");
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .window_size((1280.0, 800.0))
        .centered()
        .run()
}
