use std::path::PathBuf;
use std::time::Duration;

use iced::Settings;
use iced_test::Simulator;
use jiff::Timestamp;
use presswork_log::{Level, LogEntry, LogStore};
use presswork_ui::theme::Mode;
use presswork_ui::{font, font_size};

use presswork_editor::{Action as EditorAction, Motion};
use presswork_preview::Action as PreviewAction;
use presswork_text::Position;
use presswork_toolchain::{Digest, Platform, Release, ReleaseFile, Version};

use crate::app::{App, Message, send};
use crate::log_panel;
use crate::toolchain_panel::Message as ToolchainMessage;
use crate::workspace::Workspace;

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
    let toolchains = tempfile::tempdir().unwrap();
    let installed = toolchains.path().join("typst/0.14.2");
    std::fs::create_dir_all(&installed).unwrap();
    std::fs::write(installed.join(Platform::current().executable()), "").unwrap();
    let mut app = App::with_toolchains(sample_store(), Some(toolchains.path().to_path_buf()));
    app.compile_now();
    render(&mut app, "main-window");

    send(
        &mut app,
        Message::Logs(log_panel::Message::Show(Level::Debug)),
    );
    render(&mut app, "log-panel");

    send(&mut app, Message::Logs(log_panel::Message::Close));
    send(
        &mut app,
        Message::Editor(presswork_editor::Action::Insert("#nothing\n".to_owned())),
    );
    app.compile_now();
    render(&mut app, "write-error");

    let book = folder(&[
        ("presswork.toml", "[project]\nname = \"Field Notes\"\n"),
        ("main.typ", BOOK),
    ]);
    send(
        &mut app,
        Message::FolderPicked(Some(book.path().to_path_buf())),
    );
    send(
        &mut app,
        Message::Editor(EditorAction::Move {
            motion: Motion::DocumentEnd,
            extend: false,
        }),
    );
    send(
        &mut app,
        Message::Editor(EditorAction::Insert(
            "\nThe last proof goes to the printer.\n".to_owned(),
        )),
    );
    app.compile_now();
    let sentence = "Good books start with good paper.";
    let start = BOOK.find(sentence).unwrap();
    select(&mut app, start, start + sentence.len());
    render(&mut app, "project");

    send(&mut app, Message::Preview(PreviewAction::ZoomIn));
    send(&mut app, Message::Preview(PreviewAction::ZoomIn));
    render(&mut app, "preview-zoom");
    send(&mut app, Message::Preview(PreviewAction::FitWidth));

    let notes = folder(&[("notes.txt", "")]);
    send(
        &mut app,
        Message::FolderPicked(Some(notes.path().to_path_buf())),
    );
    render(&mut app, "notice");

    send(&mut app, Message::DismissNotice);
    send(&mut app, Message::SelectWorkspace(Workspace::Plan));
    render(&mut app, "plan-workspace");

    send(&mut app, Message::SelectWorkspace(Workspace::Write));
    send(
        &mut app,
        Message::Toolchains(ToolchainMessage::Use(Version::new(0, 14, 1))),
    );
    send(&mut app, Message::Toolchains(ToolchainMessage::Toggle));
    send(
        &mut app,
        Message::Toolchains(ToolchainMessage::Loaded(Ok(sample_releases()))),
    );
    send(
        &mut app,
        Message::Toolchains(ToolchainMessage::Progress(Version::new(0, 14, 1), 45)),
    );
    render(&mut app, "toolchains");
}

fn sample_releases() -> Vec<Release> {
    let name = Platform::current().release_file().unwrap();
    [
        ("0.15.1", 22_463_684),
        ("0.15.0", 22_310_112),
        ("0.14.2", 21_904_208),
        ("0.14.1", 21_887_540),
        ("0.14.0", 21_850_016),
        ("0.13.1", 20_731_392),
        ("0.12.0", 19_986_844),
    ]
    .into_iter()
    .map(|(version, size)| Release {
        version: Version::parse(version).unwrap(),
        files: vec![ReleaseFile {
            name: name.clone(),
            url: String::new(),
            size,
            digest: Digest::from_hex(&"0".repeat(64)),
        }],
    })
    .collect()
}

const BOOK: &str = r#"#set page(paper: "a5", margin: (x: 1.8cm, y: 2cm), numbering: "1")
#set text(font: "Libertinus Serif", size: 10.5pt, lang: "en")
#set par(justify: true)
#set heading(numbering: "1.1")

#align(center)[
  #text(size: 20pt, weight: "bold")[Field Notes]
  #v(-4pt)
  #text(size: 11pt, style: "italic")[A small handbook for printers]
]

= Paper and ink <paper>

Good books start with good paper. The weight, the grain, and the color of the paper change how the type looks on the page. Test each paper with the ink of your press before you print the full edition.

== Weights

#table(
  columns: 3,
  [*Use*], [*Weight*], [*Feel*],
  [Text], [90 g/m²], [Soft],
  [Cover], [250 g/m²], [Firm],
  [Plates], [150 g/m²], [Smooth],
)

== Ink

The ink must dry before you fold the sheets. @paper tells why the paper comes first.#footnote[Most inks dry in 24 hours.]

$ "coverage" = "ink area" / "page area" $
"#;

fn folder(files: &[(&str, &str)]) -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    for (path, text) in files {
        std::fs::write(folder.path().join(path), text).unwrap();
    }
    folder
}

fn select(app: &mut App, start: usize, end: usize) {
    for (offset, extend) in [(start, false), (end, true)] {
        send(
            app,
            Message::Editor(EditorAction::MoveTo {
                position: Position::new(offset),
                extend,
                goal: None,
            }),
        );
    }
}
