use std::path::PathBuf;
use std::time::Duration;

use iced::{Settings, window};
use iced_test::Simulator;
use jiff::Timestamp;
use presswork_log::{Level, LogEntry, LogStore};
use presswork_ui::theme::Mode;
use presswork_ui::{font, font_size};

use presswork_api::live::{ChatMessage, ServerMessage as LiveMessage};
use presswork_api::{
    Document, Edition, FileState, LabelStyle, LibraryFile, Project, SearchHit, Task, TaskState,
    Work,
};
use presswork_client::Api;
use presswork_editor::{Action as EditorAction, Motion};
use presswork_preview::Action as PreviewAction;
use presswork_sync::TextDocument;
use presswork_text::{ChangeSet, Edit, Position};
use presswork_toolchain::{Digest, Platform, Release, ReleaseFile, Version};
use uuid::Uuid;

use crate::app::{App, Message, send};
use crate::chat_panel::Message as ChatPanelMessage;
use crate::library_view::{Message as LibraryMessage, PickedFile, Target};
use crate::live_session::Event as LiveEvent;
use crate::server_dialog::{Connection, Message as ServerMessage, Shelf};
use crate::task_list::Message as TaskListMessage;
use crate::toolchain_panel::Message as ToolchainMessage;
use crate::workspace::Workspace;
use crate::{log_panel, menu, unsaved};

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 800.0;
const PREVIEW: (f32, f32) = (720.0, 900.0);
const NARROW: (f32, f32) = (760.0, 560.0);
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
    render_window(app, name, None, (WIDTH, HEIGHT));
}

fn render_window(app: &mut App, name: &str, window: Option<window::Id>, size: (f32, f32)) {
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
        let view = window.map_or_else(|| app.view(), |id| app.view_window(id));
        let mut ui = Simulator::with_size(settings(), size, view);
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

    send(&mut app, Message::ToggleMenu);
    render(&mut app, "menu");
    send(&mut app, Message::CloseMenu);

    render_window(&mut app, "narrow", None, NARROW);

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

    send(&mut app, Message::MenuItem(menu::Item::Quit));
    render(&mut app, "unsaved");
    send(&mut app, Message::Unsaved(unsaved::Answer::Cancel));

    send(&mut app, Message::Preview(PreviewAction::ZoomIn));
    send(&mut app, Message::Preview(PreviewAction::ZoomIn));
    render(&mut app, "preview-zoom");
    send(&mut app, Message::Preview(PreviewAction::FitWidth));

    send(&mut app, Message::Preview(PreviewAction::Detach));
    let preview = app.preview_window().unwrap();
    render_window(&mut app, "preview-window", Some(preview), PREVIEW);
    send(&mut app, Message::WindowClosed(preview));

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
    send(&mut app, Message::Toolchains(ToolchainMessage::Toggle));

    render_server(&mut app);
    render_chat(&mut app);
    render_tasks(&mut app);
    render_library(&mut app);
}

fn render_library(app: &mut App) {
    send(app, Message::SelectWorkspace(Workspace::Research));
    let edition = |id: u128, year, publisher: &str, pages: u32, state, standard| Edition {
        id: Uuid::from_u128(id),
        year: Some(year),
        publisher: publisher.to_owned(),
        language: String::new(),
        translators: Vec::new(),
        standard,
        corrections: Vec::new(),
        file: Some(LibraryFile {
            size: u64::from(pages) * 40_000,
            pages,
            pages_with_text: if state == FileState::Ready { pages } else { 0 },
            state,
            problem: None,
        }),
    };
    let work = |id: u128, key: &str, title: &str, creator: &str, editions| Work {
        id: Uuid::from_u128(id),
        organization_id: Uuid::from_u128(1),
        key: key.to_owned(),
        title: title.to_owned(),
        creators: vec![creator.to_owned()],
        editions,
    };
    let mut reprint = edition(22, 1958, "Dover", 230, FileState::Ready, false);
    reprint.language = "en".to_owned();
    let mut translation = edition(23, 1962, "Hauswedell", 248, FileState::Reading, false);
    translation.language = "de".to_owned();
    translation.translators = vec!["Bauer, Konrad".to_owned()];
    let works = vec![
        work(
            12,
            "atlas-of-presses",
            "Atlas of Presses",
            "Moxon, Joseph",
            vec![
                edition(21, 1683, "Moxon", 212, FileState::Ready, true),
                reprint,
                translation,
            ],
        ),
        work(
            11,
            "field-notes",
            "Field Notes: A Small Handbook for Printers",
            "Gutenberg, Johannes",
            vec![edition(20, 1455, "Field Press", 2, FileState::Ready, true)],
        ),
    ];
    send(app, Message::Library(LibraryMessage::Loaded(Ok(works))));
    send(
        app,
        Message::Library(LibraryMessage::Query("fine grain".to_owned())),
    );
    let hit = |work: u128, edition: u128, title: &str, page, snippet: &str, standard| SearchHit {
        work_id: Uuid::from_u128(work),
        edition_id: Uuid::from_u128(edition),
        key: String::new(),
        title: title.to_owned(),
        page,
        label: (page + 10).to_string(),
        snippet: snippet.to_owned(),
        standard,
    };
    let hits = vec![
        hit(
            11,
            20,
            "Field Notes: A Small Handbook for Printers",
            1,
            "The fine grain of the paper gives the page its color.",
            true,
        ),
        hit(
            12,
            22,
            "Atlas of Presses",
            87,
            "A paper with a fine grain takes the ink of the press evenly.",
            false,
        ),
    ];
    send(app, Message::Library(LibraryMessage::Found(Ok(hits))));
    render(app, "server-library");
    render_page_map(app);
    render_new_work(app);
}

fn render_new_work(app: &mut App) {
    send(
        app,
        Message::Library(LibraryMessage::Picked(
            Target::Work,
            Some(PickedFile::new("mechanick-exercises.pdf", vec![0; 10])),
        )),
    );
    send(
        app,
        Message::Library(LibraryMessage::Creators("Moxon, Joseph".to_owned())),
    );
    send(
        app,
        Message::Library(LibraryMessage::Year("1683".to_owned())),
    );
    render(app, "server-library-add");
}

fn render_page_map(app: &mut App) {
    send(
        app,
        Message::Library(LibraryMessage::EditPageMap(Uuid::from_u128(21))),
    );
    for message in [
        LibraryMessage::CorrectionPage("1".to_owned()),
        LibraryMessage::CorrectionStyle(LabelStyle::LowerRoman),
        LibraryMessage::AddCorrection,
        LibraryMessage::CorrectionPage("9".to_owned()),
        LibraryMessage::CorrectionStyle(LabelStyle::Arabic),
        LibraryMessage::AddCorrection,
        LibraryMessage::CorrectionPage("15".to_owned()),
    ] {
        send(app, Message::Library(message));
    }
    render(app, "server-library-pages");
    send(app, Message::Library(LibraryMessage::Cancel));
}

fn render_server(app: &mut App) {
    send(app, Message::MenuItem(menu::Item::Connect));
    for message in [
        ServerMessage::Address("https://press.example.com".to_owned()),
        ServerMessage::Email("ann@example.com".to_owned()),
        ServerMessage::Password("correct horse battery".to_owned()),
    ] {
        send(app, Message::Server(message));
    }
    render(app, "server-sign-in");

    let connection = Connection {
        api: Api::new("https://press.example.com").unwrap(),
        token: "pws_ann".to_owned(),
    };
    send(
        app,
        Message::Server(ServerMessage::SignedIn(Ok(connection))),
    );
    let shelf = |project: u128, name: &str, paths: &[&str]| Shelf {
        organization: "Field Press".to_owned(),
        project: Project {
            id: Uuid::from_u128(project),
            organization_id: Uuid::from_u128(1),
            name: name.to_owned(),
        },
        documents: paths
            .iter()
            .enumerate()
            .map(|(index, path)| Document {
                id: Uuid::from_u128(project * 100 + index as u128),
                project_id: Uuid::from_u128(project),
                path: (*path).to_owned(),
            })
            .collect(),
    };
    let shelves = vec![
        shelf(2, "Field Notes", &["chapters/paper.typ", "main.typ"]),
        shelf(3, "Atlas of Presses", &[]),
    ];
    send(app, Message::Server(ServerMessage::Loaded(Ok(shelves))));
    render(app, "server-documents");

    let document = Document {
        id: Uuid::from_u128(201),
        project_id: Uuid::from_u128(2),
        path: "main.typ".to_owned(),
    };
    send(app, Message::Server(ServerMessage::Open(document)));
    let (sender, _receiver) = futures::channel::mpsc::unbounded();
    send(app, Message::Live(LiveEvent::Ready(sender)));
    send(app, Message::Live(LiveEvent::Connected));
    let mut server = TextDocument::new();
    server
        .apply(&ChangeSet::new(0, [Edit::insert(Position::new(0), BOOK)]).unwrap())
        .unwrap();
    send(
        app,
        Message::Live(LiveEvent::Received(LiveMessage::Subscribed {
            document: Uuid::from_u128(201),
            version: server.version(),
            data: server
                .updates_since(&TextDocument::new().version())
                .unwrap(),
        })),
    );
    app.compile_now();
    render(app, "server-live");
}

fn render_chat(app: &mut App) {
    let said = |id: u128, author: &str, minutes: i64, text: &str| ChatMessage {
        id: Uuid::from_u128(id),
        organization: Uuid::from_u128(1),
        author: Uuid::nil(),
        author_name: author.to_owned(),
        text: text.to_owned(),
        sent_at: Timestamp::from_second(1_795_340_000 + minutes * 60).unwrap(),
    };
    send(
        app,
        Message::Live(LiveEvent::Received(LiveMessage::Joined {
            organization: Uuid::from_u128(1),
            messages: vec![
                said(
                    1,
                    "Bob",
                    0,
                    "I changed the table of weights. Please check it.",
                ),
                said(2, "Ann", 3, "Looks good. I will add the plates now."),
                said(
                    3,
                    "Bob",
                    4,
                    "Thank you. The proofs go to the printer at noon.",
                ),
            ],
        })),
    );
    send(app, Message::Chat(ChatPanelMessage::Toggle));
    send(
        app,
        Message::Chat(ChatPanelMessage::Draft("Noon is fine.".to_owned())),
    );
    render(app, "server-chat");
    send(app, Message::Chat(ChatPanelMessage::Toggle));
}

fn render_tasks(app: &mut App) {
    send(app, Message::SelectWorkspace(Workspace::Plan));
    let task = |id: u128, title: &str, state: TaskState, due: Option<(i16, i8, i8)>| Task {
        id: Uuid::from_u128(id),
        project_id: Uuid::from_u128(2),
        title: title.to_owned(),
        description: String::new(),
        state,
        assignee_id: None,
        due_date: due.map(|(year, month, day)| jiff::civil::date(year, month, day)),
        created_by: None,
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
    };
    let tasks = vec![
        task(1, "Order the paper", TaskState::Done, Some((2026, 11, 18))),
        task(2, "Check the table of weights", TaskState::Done, None),
        task(
            3,
            "Add the plates",
            TaskState::InProgress,
            Some((2026, 11, 20)),
        ),
        task(4, "Proof the cover", TaskState::ToDo, Some((2026, 11, 23))),
        task(
            5,
            "Send the proofs to the printer",
            TaskState::ToDo,
            Some((2026, 11, 23)),
        ),
    ];
    send(app, Message::Tasks(TaskListMessage::Loaded(Ok(tasks))));
    render(app, "server-tasks");
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
