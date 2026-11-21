mod app;
mod assist;
mod chat_panel;
mod chrome;
mod compiler;
mod files;
mod find_bar;
mod i18n;
mod icons;
mod library_view;
mod live_session;
mod log_panel;
mod logging;
mod menu;
mod outline_panel;
mod problems;
mod publish_view;
#[cfg(test)]
mod screens;
mod server_dialog;
mod spelling;
mod task_list;
#[cfg(test)]
mod testing;
mod toolchain_panel;
mod typeset_view;
mod unsaved;
mod workspace;

use app::App;
use presswork_log::LogStore;
use presswork_ui::{font, font_size};

pub(crate) const APPLICATION_ID: &str = "io.github.getpresswork.Presswork";

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
        let dictionaries = App::load_dictionaries();
        let theme = iced::system::theme().map(app::Message::SystemTheme);
        (
            app,
            iced::Task::batch([window, theme, compile, dictionaries]),
        )
    };
    iced::daemon(boot, App::update, App::view_window)
        .settings(settings())
        .subscription(App::subscription)
        .theme(App::window_theme)
        .title(App::window_title)
        .run()
}

fn settings() -> iced::Settings {
    iced::Settings {
        id: Some(APPLICATION_ID.to_owned()),
        fonts: font::all(),
        default_font: font::UI,
        default_text_size: font_size::BODY.into(),
        antialiasing: true,
        ..iced::Settings::default()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{APPLICATION_ID, settings};

    #[test]
    fn the_application_has_its_id() {
        assert_eq!(settings().id.as_deref(), Some(APPLICATION_ID));
    }

    #[test]
    fn the_packages_use_the_application_id() {
        let packaging = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging");
        let entry = packaging
            .join("linux")
            .join(format!("{APPLICATION_ID}.desktop"));
        let entry = fs::read_to_string(entry).unwrap();
        let plist = fs::read_to_string(packaging.join("macos/Info.plist")).unwrap();

        assert!(
            entry
                .lines()
                .any(|line| line == format!("Icon={APPLICATION_ID}"))
        );
        assert!(
            entry
                .lines()
                .any(|line| line == format!("StartupWMClass={APPLICATION_ID}"))
        );
        assert!(plist.contains(&format!("<string>{APPLICATION_ID}</string>")));
    }
}
