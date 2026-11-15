mod app;
mod i18n;
mod workspace;

use app::App;

pub fn run() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .window_size((1280.0, 800.0))
        .centered()
        .run()
}
