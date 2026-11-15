use std::path::PathBuf;
use std::time::Duration;

use iced::Settings;
use iced_test::Simulator;
use jiff::Timestamp;
use presswork_log::{Level, LogEntry, LogStore};
use presswork_ui::theme::Mode;
use presswork_ui::{font, font_size};

use crate::app::{App, Message, send};
use crate::log_panel;

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 800.0;
const IMAGE_UPLOAD: Duration = Duration::from_millis(300);

fn directory() -> PathBuf {
    std::env::var_os("PRESSWORK_SCREENS").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/screens"),
        PathBuf::from,
    )
}

fn settings() -> Settings {
    Settings {
        fonts: font::all(),
        default_font: font::UI,
        default_text_size: font_size::BODY.into(),
        antialiasing: true,
        ..Settings::default()
    }
}

fn render(app: &mut App, name: &str) {
    let directory = directory();
    std::fs::create_dir_all(&directory).unwrap();
    for (label, mode) in [("light", Mode::Light), ("dark", Mode::Dark)] {
        send(app, Message::SystemTheme(mode));
        let theme = app.theme();
        let stem = format!("{name}-{label}");
        for old in std::fs::read_dir(&directory).unwrap().flatten() {
            if old.file_name().to_string_lossy().starts_with(&stem) {
                std::fs::remove_file(old.path()).unwrap();
            }
        }
        let mut ui = Simulator::with_size(settings(), (WIDTH, HEIGHT), app.view());
        let _ = ui.snapshot(&theme).unwrap();
        std::thread::sleep(IMAGE_UPLOAD);
        ui.snapshot(&theme)
            .unwrap()
            .matches_image(directory.join(stem))
            .unwrap();
    }
}

fn sample_store() -> LogStore {
    let store = LogStore::with_capacity(100);
    let samples = [
        (
            Level::Info,
            "presswork::app",
            "Presswork started",
            vec![("version", "0.1.0")],
        ),
        (
            Level::Debug,
            "presswork::app",
            "Workspace selected",
            vec![("workspace", "Review")],
        ),
        (
            Level::Warn,
            "presswork_typst::world",
            "Font not found",
            vec![("family", "Libertinus Serif")],
        ),
        (
            Level::Info,
            "presswork_typst::compile",
            "Compiled",
            vec![("pages", "214"), ("duration_ms", "87")],
        ),
        (
            Level::Error,
            "presswork_sync::client",
            "Connection lost",
            vec![("access_token", "[removed]")],
        ),
    ];
    for (level, target, message, fields) in samples {
        store.push(LogEntry {
            sequence: 0,
            time: Timestamp::now(),
            level,
            target: target.to_owned(),
            message: message.to_owned(),
            fields: fields
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
            spans: Vec::new(),
        });
    }
    store
}

#[test]
#[ignore = "writes screen images for design reviews"]
fn render_screens() {
    let mut app = App::new(sample_store());
    app.compile_now();
    render(&mut app, "main-window");

    send(
        &mut app,
        Message::Logs(log_panel::Message::Show(Level::Debug)),
    );
    render(&mut app, "log-panel");
}
