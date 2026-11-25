use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use iced::{Settings, keyboard, window};
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
use presswork_language::{Dictionaries, Dictionary};
use presswork_preview::Action as PreviewAction;
use presswork_sync::TextDocument;
use presswork_text::{ChangeSet, Edit, Position, Suggestion, SuggestionKind};
use presswork_toolchain::{Digest, Platform, Release, ReleaseFile, Version};
use uuid::Uuid;

use crate::app::{App, Message, send};
use crate::chat_panel::Message as ChatPanelMessage;
use crate::find_bar::Message as FindMessage;
use crate::library_view::{Message as LibraryMessage, PickedFile, Target};
use crate::live_session::Event as LiveEvent;
use crate::proofreading::Message as ProofreadingMessage;
use crate::publish_view::Message as PublishMessage;
use crate::server_dialog::{Connection, Message as ServerMessage, Shelf};
use crate::task_list::Message as TaskListMessage;
use crate::toolchain_panel::Message as ToolchainMessage;
use crate::typeset_view::Message as TypesetMessage;
use crate::workspace::Workspace;
use crate::{log_panel, menu, unsaved};

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 800.0;
const PREVIEW: (f32, f32) = (720.0, 900.0);
const NARROW: (f32, f32) = (760.0, 560.0);
const IMAGE_UPLOAD: Duration = Duration::from_millis(300);
const TOOLTIP_WAIT: Duration = Duration::from_millis(600);

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
    render_scene(app, name, window, size, None);
}

fn render_scene(
    app: &mut App,
    name: &str,
    window: Option<window::Id>,
    size: (f32, f32),
    hover: Option<&str>,
) {
    let directory = directory();
    std::fs::create_dir_all(&directory).unwrap();
    app.render_pages_now();
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
        if let Some(target) = hover {
            rest_on(&mut ui, target);
        }
        let _ = ui.snapshot(&theme).unwrap();
        std::thread::sleep(IMAGE_UPLOAD);
        ui.snapshot(&theme)
            .unwrap()
            .matches_image(directory.join(stem))
            .unwrap();
    }
}

fn rest_on(ui: &mut Simulator<'_, Message>, target: &str) {
    let center = ui
        .find(iced::widget::Id::from(target.to_owned()))
        .unwrap()
        .bounds()
        .center();
    let moved = iced::Event::Mouse(iced::mouse::Event::CursorMoved { position: center });
    ui.point_at(center);
    let _ = ui.simulate([moved.clone()]);
    std::thread::sleep(TOOLTIP_WAIT);
    let _ = ui.simulate([moved]);
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

    render_find(&mut app);

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
    render_spreads(&mut app);
    render_publish(&mut app);
    render_typeset(&mut app);
    render_problem_marks(&mut app);
    render_house_style(&mut app);
    render_suggestions(&mut app);
    render_server_suggestions(&mut app);
}

const LIVE_SUGGESTED: &str = "#set text(font: \"Libertinus Serif\")\n= Paper\n\n\
                              The press prints fourtwelve sheets in an hour. \
                              Each proof goes to the author.\n";

fn render_server_suggestions(app: &mut App) {
    let id = Uuid::from_u128(202);
    let document = Document {
        id,
        project_id: Uuid::from_u128(2),
        path: "chapters/paper.typ".to_owned(),
    };
    send(app, Message::Server(ServerMessage::Open(document)));
    let (sender, _receiver) = futures::channel::mpsc::unbounded();
    send(app, Message::Live(LiveEvent::Ready(sender)));
    send(app, Message::Live(LiveEvent::Connected));
    let by_bob = |kind, start: usize, end: usize| Suggestion {
        kind,
        range: Position::new(start)..Position::new(end),
        author: "Bob".to_owned(),
        time: 1_795_340_000,
    };
    let four = LIVE_SUGGESTED.find("four").unwrap();
    let marks = [
        by_bob(SuggestionKind::Deletion, four, four + "four".len()),
        by_bob(
            SuggestionKind::Insertion,
            four + "four".len(),
            four + "fourtwelve".len(),
        ),
    ];
    let mut server = TextDocument::new();
    server
        .apply(
            &ChangeSet::new(0, [Edit::insert(Position::new(0), LIVE_SUGGESTED)]).unwrap(),
            Some(&marks),
        )
        .unwrap();
    send(
        app,
        Message::Live(LiveEvent::Received(LiveMessage::Subscribed {
            document: id,
            version: server.version(),
            data: server
                .updates_since(&TextDocument::new().version())
                .unwrap(),
        })),
    );
    let author = LIVE_SUGGESTED.find("the author").unwrap() + "the author".len();
    select(app, author, author);
    send(
        app,
        Message::Editor(EditorAction::Insert(" and the editor".to_owned())),
    );
    app.compile_now();
    send(app, Message::Hover(Some(Position::new(four + 1))));
    render(app, "server-suggestions");
    send(app, Message::Hover(None));
}

const SUGGESTED: &str = "#set text(font: \"Libertinus Serif\")\n= Field notes\n\n\
                         The paper for the first proof comes from the mill. \
                         We cut each sheet by hand and fold it twice.\n";

fn render_suggestions(app: &mut App) {
    let book = folder(&[
        ("presswork.toml", "[project]\nname = \"Field Notes\"\n"),
        ("main.typ", SUGGESTED),
    ]);
    send(app, Message::SelectWorkspace(Workspace::Write));
    send(app, Message::FolderPicked(Some(book.path().to_path_buf())));
    app.suggest_as("Ann Lee");
    let first = SUGGESTED.find("first").unwrap();
    select(app, first, first + "first".len());
    send(
        app,
        Message::Editor(EditorAction::Insert("second".to_owned())),
    );
    let twice = SUGGESTED.find(" and fold it twice").unwrap() + "second".len();
    select(app, twice, twice + " and fold it twice".len());
    send(
        app,
        Message::Editor(EditorAction::Delete(presswork_editor::Deletion::Backward)),
    );
    let mill = SUGGESTED.find("mill").unwrap() + "second".len() + "mill".len();
    select(app, mill, mill);
    send(
        app,
        Message::Editor(EditorAction::Insert(" at Arches".to_owned())),
    );
    app.compile_now();
    send(app, Message::Hover(Some(Position::new(first + 2))));
    render(app, "suggestions");

    send(app, Message::Hover(None));
    select(app, first + 1, first + 1);
    send(
        app,
        Message::Complete(presswork_editor::CompletionEvent::Correct),
    );
    render(app, "suggestion-choices");
}

const STYLED_NOTES: &str = "#set text(font: \"Libertinus Serif\")\n= Field notes\n\n\
                            We organise the notes in order to find them on pages 12-19.\n\
                            The papr is cut by hand.\n";

const HOUSE_STYLE: &str = "[[rule]]\ntext = \"organise\"\nreplacement = \"organize\"\n\n\
                           [[rule]]\ntext = \"in order to\"\nreplacement = \"to\"\n\
                           message = \"Use the short form.\"\n\n\
                           [[rule]]\npattern = '(\\d+)-(\\d+)'\nreplacement = '$1–$2'\n\
                           message = \"Use an en dash between two numbers.\"\n";

fn render_house_style(app: &mut App) {
    let words = "field notes we organise the in order to find them on pages is cut by hand paper";
    let dic = format!(
        "{}\n{}\n",
        words.split(' ').count(),
        words.replace(' ', "\n")
    );
    let mut dictionaries = Dictionaries::default();
    dictionaries.insert(
        "en-US",
        Dictionary::new(b"SET UTF-8\n", dic.as_bytes()).unwrap(),
    );
    let book = folder(&[
        ("presswork.toml", "[project]\nname = \"Field Notes\"\n"),
        ("main.typ", STYLED_NOTES),
        ("presswork-style.toml", HOUSE_STYLE),
    ]);
    send(app, Message::SelectWorkspace(Workspace::Write));
    send(app, Message::FolderPicked(Some(book.path().to_path_buf())));
    send(
        app,
        Message::Proofreading(ProofreadingMessage::Loaded(Arc::new(dictionaries))),
    );
    app.compile_now();
    app.proofread_now();
    let range = STYLED_NOTES.find("12-19").unwrap() + 1;
    send(app, Message::Hover(Some(Position::new(range))));
    render(app, "house-style");

    send(app, Message::Hover(None));
    let personal = tempfile::tempdir().unwrap();
    app.set_personal_list(personal.path().join("personal-words.txt"));
    let papr = STYLED_NOTES.find("papr").unwrap();
    send(
        app,
        Message::Editor(EditorAction::MoveTo {
            position: Position::new(papr + 2),
            extend: false,
            goal: None,
        }),
    );
    app.correct_now();
    render(app, "corrections");
}

const TYPESET: &str = "#set page(width: 105mm, height: 74mm, margin: 8mm)\n\
                       #set text(font: \"Libertinus Serif\", size: 10pt, costs: (widow: 0%, orphan: 0%))\n\
                       #set par(justify: true)\n\
                       #lorem(70)\n\n\
                       A typesetter considers incomprehensibilities thoroughly. The last line of this \
                       paragraph is short \\ it.\n";

fn render_typeset(app: &mut App) {
    let book = folder(&[
        ("presswork.toml", "[project]\nname = \"Field Notes\"\n"),
        ("main.typ", TYPESET),
    ]);
    send(app, Message::FolderPicked(Some(book.path().to_path_buf())));
    app.compile_now();
    send(app, Message::SelectWorkspace(Workspace::Typeset));
    send(app, Message::Typeset(TypesetMessage::Show(0)));
    render(app, "typeset-qa");
}

fn render_problem_marks(app: &mut App) {
    let text = format!(
        "{TYPESET}#pagebreak()\n#text(fill: rgb(\"#d6249f\"))[A note in the color of the brand.]\n"
    );
    let book = folder(&[
        (
            "presswork.toml",
            "[project]\nname = \"Field Notes\"\n\n[[export]]\nname = \"Print\"\ncolors = \"cmyk\"\n",
        ),
        ("main.typ", &text),
    ]);
    send(app, Message::FolderPicked(Some(book.path().to_path_buf())));
    app.compile_now();
    send(app, Message::SelectWorkspace(Workspace::Write));
    render_scene(
        app,
        "problem-marks",
        None,
        (WIDTH, HEIGHT),
        Some("problem-1-0"),
    );
}

const EXPORTS: &str = "[project]\nname = \"Field Notes\"\n\n\
                       [[export]]\nname = \"Print\"\nstandards = [\"a-2b\"]\nimage-ppi = 300\n\
                       page-size = \"148mm x 210mm\"\ncolors = \"cmyk\"\n\
                       output = \"out/field-notes-print.pdf\"\n\n\
                       [[export]]\nname = \"Screen\"\nstandards = [\"ua-1\"]\n\
                       output = \"out/field-notes.pdf\"\n\n\
                       [[export]]\nname = \"Pages\"\nformat = \"png\"\nppi = 144\n\
                       output = \"out/pages/page-{0p}.png\"\n";

const MAP: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"80\">\
                   <rect width=\"120\" height=\"80\" fill=\"#E8E1D3\"/>\
                   <path d=\"M10 60 Q40 10 70 40 T110 20\" stroke=\"#8C2F39\" \
                   stroke-width=\"3\" fill=\"none\"/></svg>";

fn render_publish(app: &mut App) {
    let text = "#set page(paper: \"a5\")\n#set text(font: \"Libertinus Serif\")\n\
                #show heading: set text(fill: rgb(\"#8c2f39\"))\n\
                = Field notes\n\n#image(\"meadow.png\", width: 5cm, alt: \"A meadow at dawn\")\n\n\
                #image(\"map.svg\", width: 4cm)\n";
    let book = folder(&[
        ("presswork.toml", EXPORTS),
        ("main.typ", text),
        ("map.svg", MAP),
    ]);
    std::fs::write(book.path().join("meadow.png"), gradient(300, 200)).unwrap();
    send(app, Message::FolderPicked(Some(book.path().to_path_buf())));
    send(app, Message::SelectWorkspace(Workspace::Publish));
    app.export_now(0);
    send(app, Message::Publish(PublishMessage::Export(2)));
    render(app, "publish");
}

fn gradient(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let pixels: Vec<u8> = (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                let across = u8::try_from(x * 100 / width).unwrap();
                let down = u8::try_from(y * 60 / height).unwrap();
                [110 + across, 150 + down, 120]
            })
        })
        .collect();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
    bytes
}

fn render_spreads(app: &mut App) {
    let chapters: Vec<String> = (1..=4)
        .map(|number| {
            format!(
                "= Chapter {number}\n\n{}\n",
                "Good books start with good paper. ".repeat(40)
            )
        })
        .collect();
    let text = format!(
        "#set page(paper: \"a5\", numbering: \"1\")\n#set text(font: \"Libertinus Serif\")\n{}",
        chapters.join("#pagebreak()\n")
    );
    let book = folder(&[
        ("presswork.toml", "[project]\nname = \"Spreads\"\n"),
        ("main.typ", &text),
    ]);
    send(app, Message::SelectWorkspace(Workspace::Write));
    send(app, Message::FolderPicked(Some(book.path().to_path_buf())));
    app.compile_now();
    send(app, Message::Preview(PreviewAction::Spreads));
    render(app, "spreads");
    send(app, Message::Preview(PreviewAction::Spreads));
    send(app, Message::Preview(PreviewAction::Thumbnails));
    render(app, "thumbnails");
    send(app, Message::Preview(PreviewAction::Thumbnails));
    render_guides(app);
}

fn render_guides(app: &mut App) {
    let text = "#set page(paper: \"a5\", bleed: 3mm, background: place(top, \
                rect(width: 100%, height: 4cm, fill: rgb(\"#8C2F39\"))))\n\
                #set text(font: \"Libertinus Serif\", top-edge: 11pt, bottom-edge: 0pt)\n\
                #set par(leading: 2pt, spacing: 2pt)\n\
                #v(4cm)\n= A cover band in the bleed\n\nThe colored band goes over the trim \
                line into the bleed, so the printer can cut the page without a white edge. \
                The lines of this paragraph stand on the baseline grid of the project.";
    let book = folder(&[
        (
            "presswork.toml",
            "[project]\nname = \"Guides\"\n\n[typeset]\nbaseline-grid = \"13pt\"\n",
        ),
        ("main.typ", text),
    ]);
    send(app, Message::FolderPicked(Some(book.path().to_path_buf())));
    send(app, Message::Preview(PreviewAction::Guides));
    app.compile_now();
    render(app, "guides");
}

fn render_find(app: &mut App) {
    let letter = if cfg!(target_os = "macos") { "f" } else { "h" };
    let modifiers = if cfg!(target_os = "macos") {
        keyboard::Modifiers::COMMAND | keyboard::Modifiers::ALT
    } else {
        keyboard::Modifiers::COMMAND
    };
    send(
        app,
        Message::Key(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character(letter.into()),
            modified_key: keyboard::Key::Character(letter.into()),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        }),
    );
    send(app, Message::Find(FindMessage::Query("paper".to_owned())));
    send(
        app,
        Message::Find(FindMessage::Replacement("stock".to_owned())),
    );
    render(app, "find-replace");
    send(app, Message::Find(FindMessage::Close));
    send(app, Message::Editor(EditorAction::SelectEvery));
    render(app, "many-cursors");
    send(app, Message::Editor(EditorAction::SingleCursor));
    let start = BOOK.find("== Weights\n").unwrap() + "== Weights\n".len();
    let end = BOOK.find(")\n\n== Ink").unwrap() + 2;
    let section = Position::new(start)..Position::new(end);
    send(app, Message::Editor(EditorAction::Fold(section.clone())));
    render(app, "folding");
    send(app, Message::Editor(EditorAction::Unfold(section)));
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
    let mut original = edition(21, 1683, "Moxon", 212, FileState::Recognizing, true);
    if let Some(file) = &mut original.file {
        file.pages_with_text = 12;
    }
    let works = vec![
        work(
            12,
            "atlas-of-presses",
            "Atlas of Presses",
            "Moxon, Joseph",
            vec![original, reprint, translation],
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
        user_name: "Ann".to_owned(),
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
        .apply(
            &ChangeSet::new(0, [Edit::insert(Position::new(0), BOOK)]).unwrap(),
            None,
        )
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
