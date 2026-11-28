use std::fmt;

use iced::widget::{button, column, container, opaque, row, scrollable, space, text, text_input};
use iced::{Element, Fill, Task};
use presswork_api::{Document, Project};
use presswork_client::{Api, ClientError};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, size, space as gap};
use uuid::Uuid;

use crate::i18n::{FluentArgs, Localizer};

const MAIN_FILE: &str = "main.typ";
const LIST_HEIGHT: f32 = 360.0;

#[cfg(test)]
pub(crate) const TEXT_IDS: [&str; 18] = [
    "menu-connect",
    "server-title",
    "server-address",
    "server-address-placeholder",
    "server-email",
    "server-password",
    "server-sign-in",
    "server-signing-in",
    "server-cancel",
    "server-close",
    "server-problem-address",
    "server-problem-unreachable",
    "server-problem-status",
    "server-browse-title",
    "server-loading",
    "server-no-projects",
    "server-project",
    "server-make-main",
];

#[derive(Clone)]
pub(crate) struct Connection {
    pub(crate) api: Api,
    pub(crate) token: String,
    pub(crate) user_name: String,
}

impl fmt::Debug for Connection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Connection")
            .field("server", &self.api.server())
            .field("token", &"[removed]")
            .field("user_name", &self.user_name)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Problem {
    Address,
    Unreachable,
    Refused(String),
    Status(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shelf {
    pub(crate) organization: String,
    pub(crate) project: Project,
    pub(crate) documents: Vec<Document>,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Address(String),
    Email(String),
    Password(String),
    SignIn,
    SignedIn(Result<Connection, Problem>),
    Loaded(Result<Vec<Shelf>, Problem>),
    Make(Uuid),
    Made(Result<Document, Problem>),
    Open(Document),
    Close,
}

#[derive(Debug)]
enum Listing {
    Loading,
    Loaded(Vec<Shelf>),
    Failed(Problem),
}

#[derive(Debug)]
struct Browse {
    connection: Connection,
    listing: Listing,
}

#[derive(Debug, Default)]
pub(crate) struct ServerDialog {
    open: bool,
    address: String,
    email: String,
    password: String,
    signing_in: bool,
    problem: Option<Problem>,
    browse: Option<Browse>,
}

impl ServerDialog {
    pub(crate) fn open(&mut self) {
        self.open = true;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    #[cfg(test)]
    pub(crate) fn is_signing_in(&self) -> bool {
        self.signing_in
    }

    pub(crate) fn project_name(&self, project: Uuid) -> Option<String> {
        let Some(Browse {
            listing: Listing::Loaded(shelves),
            ..
        }) = &self.browse
        else {
            return None;
        };
        shelves
            .iter()
            .find(|shelf| shelf.project.id == project)
            .map(|shelf| shelf.project.name.clone())
    }

    pub(crate) fn organization_of(&self, document: &Document) -> Option<(Uuid, String)> {
        let Some(Browse {
            listing: Listing::Loaded(shelves),
            ..
        }) = &self.browse
        else {
            return None;
        };
        shelves
            .iter()
            .find(|shelf| shelf.project.id == document.project_id)
            .map(|shelf| (shelf.project.organization_id, shelf.organization.clone()))
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Address(address) => self.address = address,
            Message::Email(email) => self.email = email,
            Message::Password(password) => self.password = password,
            Message::SignIn => return self.sign_in(),
            Message::SignedIn(Ok(connection)) => {
                if self.open {
                    return self.browse(connection);
                }
            }
            Message::SignedIn(Err(problem)) => {
                self.signing_in = false;
                self.problem = Some(problem);
            }
            Message::Loaded(result) => {
                if let Some(browse) = &mut self.browse {
                    browse.listing = match result {
                        Ok(shelves) => Listing::Loaded(shelves),
                        Err(problem) => Listing::Failed(problem),
                    };
                }
            }
            Message::Make(project) => return self.make(project),
            Message::Made(Ok(document)) => self.add(document),
            Message::Made(Err(problem)) => self.problem = Some(problem),
            Message::Open(_) | Message::Close => self.close(),
        }
        Task::none()
    }

    fn close(&mut self) {
        self.open = false;
        self.signing_in = false;
        self.password.clear();
        self.problem = None;
    }

    fn ready(&self) -> bool {
        !self.signing_in
            && [&self.address, &self.email, &self.password]
                .iter()
                .all(|field| !field.trim().is_empty())
    }

    fn sign_in(&mut self) -> Task<Message> {
        if !self.ready() {
            return Task::none();
        }
        let Ok(api) = Api::new(self.address.trim()) else {
            self.problem = Some(Problem::Address);
            return Task::none();
        };
        self.signing_in = true;
        self.problem = None;
        let email = self.email.trim().to_owned();
        let password = self.password.clone();
        Task::perform(
            async move {
                let signed = api
                    .sign_in(&email, &password, &device_name())
                    .await
                    .map_err(problem)?;
                Ok(Connection {
                    api,
                    token: signed.token,
                    user_name: signed.user_name,
                })
            },
            Message::SignedIn,
        )
    }

    fn browse(&mut self, connection: Connection) -> Task<Message> {
        self.signing_in = false;
        self.password.clear();
        self.problem = None;
        self.browse = Some(Browse {
            connection: connection.clone(),
            listing: Listing::Loading,
        });
        Task::perform(shelves(connection), Message::Loaded)
    }

    fn make(&mut self, project: Uuid) -> Task<Message> {
        let Some(browse) = &self.browse else {
            return Task::none();
        };
        self.problem = None;
        let Connection { api, token, .. } = browse.connection.clone();
        Task::perform(
            async move {
                api.create_document(&token, project, MAIN_FILE)
                    .await
                    .map_err(problem)
            },
            Message::Made,
        )
    }

    fn add(&mut self, document: Document) {
        let Some(Browse {
            listing: Listing::Loaded(shelves),
            ..
        }) = &mut self.browse
        else {
            return;
        };
        if let Some(shelf) = shelves
            .iter_mut()
            .find(|shelf| shelf.project.id == document.project_id)
        {
            shelf.documents.push(document);
            shelf
                .documents
                .sort_by(|first, second| first.path.cmp(&second.path));
        }
    }

    pub(crate) fn view<'a>(&'a self, localizer: &Localizer) -> Element<'a, Message> {
        let content = match &self.browse {
            Some(browse) => self.browse_view(browse, localizer),
            None => self.form_view(localizer),
        };
        let card = container(content)
            .padding(gap::L)
            .width(size::DIALOG)
            .style(style::popup);
        opaque(container(card).center(Fill).style(style::scrim))
    }

    fn form_view<'a>(&'a self, localizer: &Localizer) -> Element<'a, Message> {
        let label = |id: &str| {
            text(localizer.text(id))
                .size(font_size::SMALL)
                .style(style::muted)
        };
        let field = |value: &str, placeholder: &str| {
            text_input(placeholder, value)
                .padding([gap::XS + gap::XXS, gap::S])
                .style(style::field)
        };
        let mut form = column![
            title(localizer.text("server-title")),
            column![
                label("server-address"),
                field(&self.address, &localizer.text("server-address-placeholder"))
                    .on_input(Message::Address)
            ]
            .spacing(gap::XS),
            column![
                label("server-email"),
                field(&self.email, "").on_input(Message::Email)
            ]
            .spacing(gap::XS),
            column![
                label("server-password"),
                field(&self.password, "")
                    .secure(true)
                    .on_input(Message::Password)
                    .on_submit(Message::SignIn)
            ]
            .spacing(gap::XS),
        ]
        .spacing(gap::M);
        form = form.push(self.problem_view(localizer));
        let sign_in = if self.signing_in {
            "server-signing-in"
        } else {
            "server-sign-in"
        };
        let buttons = row![
            space::horizontal(),
            action(localizer.text("server-cancel"), style::secondary).on_press(Message::Close),
            action(localizer.text(sign_in), style::primary)
                .on_press_maybe(self.ready().then_some(Message::SignIn)),
        ]
        .spacing(gap::S);
        form.push(buttons).into()
    }

    fn browse_view<'a>(
        &'a self,
        browse: &'a Browse,
        localizer: &Localizer,
    ) -> Element<'a, Message> {
        let mut list = column![].spacing(gap::XXS);
        match &browse.listing {
            Listing::Loading => {
                list = list.push(text(localizer.text("server-loading")).style(style::muted));
            }
            Listing::Failed(problem) => {
                list = list
                    .push(text(problem_text(problem, localizer)).style(style::tone(Tone::Danger)));
            }
            Listing::Loaded(shelves) if shelves.is_empty() => {
                list = list.push(text(localizer.text("server-no-projects")).style(style::muted));
            }
            Listing::Loaded(shelves) => {
                for shelf in shelves {
                    list = list.push(shelf_view(shelf, localizer));
                }
            }
        }
        let buttons = row![
            space::horizontal(),
            action(localizer.text("server-close"), style::secondary).on_press(Message::Close),
        ];
        column![
            title(localizer.text("server-browse-title")),
            container(scrollable(list)).max_height(LIST_HEIGHT),
        ]
        .push(self.problem_view(localizer))
        .push(buttons)
        .spacing(gap::M)
        .into()
    }

    fn problem_view<'a>(&self, localizer: &Localizer) -> Option<Element<'a, Message>> {
        self.problem.as_ref().map(|problem| {
            text(problem_text(problem, localizer))
                .size(font_size::SMALL)
                .style(style::tone(Tone::Danger))
                .into()
        })
    }
}

fn shelf_view<'a>(shelf: &'a Shelf, localizer: &Localizer) -> Element<'a, Message> {
    let mut args = FluentArgs::new();
    args.set("organization", shelf.organization.clone());
    args.set("project", shelf.project.name.clone());
    let mut rows = column![
        container(
            text(localizer.format("server-project", &args))
                .font(font::UI_SEMIBOLD)
                .size(font_size::SMALL)
        )
        .padding([gap::XS, gap::XXS])
    ];
    for document in &shelf.documents {
        rows = rows.push(entry(
            Icon::FileText,
            document.path.clone(),
            Message::Open(document.clone()),
        ));
    }
    if shelf.documents.is_empty() {
        rows = rows.push(entry(
            Icon::FilePlus,
            localizer.text("server-make-main"),
            Message::Make(shelf.project.id),
        ));
    }
    rows.into()
}

fn entry<'a>(symbol: Icon, words: String, message: Message) -> Element<'a, Message> {
    button(
        row![icon(symbol).size(size::ICON_SMALL), text(words)]
            .spacing(gap::S)
            .align_y(iced::alignment::Vertical::Center),
    )
    .width(Fill)
    .padding([gap::XS + gap::XXS, gap::S])
    .style(style::menu_item)
    .on_press(message)
    .into()
}

fn title<'a>(words: String) -> Element<'a, Message> {
    text(words)
        .font(font::UI_SEMIBOLD)
        .size(font_size::LARGE)
        .into()
}

fn action<'a>(
    words: String,
    look: fn(&iced::Theme, button::Status) -> button::Style,
) -> button::Button<'a, Message> {
    button(text(words))
        .padding([gap::XS + gap::XXS, gap::M])
        .style(look)
}

async fn shelves(connection: Connection) -> Result<Vec<Shelf>, Problem> {
    let Connection { api, token, .. } = connection;
    let mut shelves = Vec::new();
    for membership in api.organizations(&token).await.map_err(problem)? {
        let projects = api
            .projects(&token, membership.organization_id)
            .await
            .map_err(problem)?;
        for project in projects {
            let documents = api.documents(&token, project.id).await.map_err(problem)?;
            shelves.push(Shelf {
                organization: membership.name.clone(),
                project,
                documents,
            });
        }
    }
    Ok(shelves)
}

pub(crate) fn problem(error: ClientError) -> Problem {
    match error {
        ClientError::Address => Problem::Address,
        ClientError::Refused { message, .. } => Problem::Refused(message),
        ClientError::Status(status) => Problem::Status(status),
        _ => Problem::Unreachable,
    }
}

pub(crate) fn problem_text(problem: &Problem, localizer: &Localizer) -> String {
    match problem {
        Problem::Address => localizer.text("server-problem-address"),
        Problem::Unreachable => localizer.text("server-problem-unreachable"),
        Problem::Refused(message) => message.clone(),
        Problem::Status(status) => {
            let mut args = FluentArgs::new();
            args.set("status", *status);
            localizer.format("server-problem-status", &args)
        }
    }
}

pub(crate) fn device_name() -> String {
    let system = match std::env::consts::OS {
        "windows" => "Windows",
        "macos" => "macOS",
        "linux" => "Linux",
        other => other,
    };
    format!("Presswork on {system}")
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use presswork_api::{Document, Project};
    use uuid::Uuid;

    use super::*;
    use crate::i18n::Localizer;
    use crate::testing::{Route, run_task, serve};

    fn filled() -> ServerDialog {
        let mut dialog = ServerDialog::default();
        dialog.open();
        let _ = dialog.update(Message::Address("https://press.example.com".to_owned()));
        let _ = dialog.update(Message::Email("ann@example.com".to_owned()));
        let _ = dialog.update(Message::Password("correct horse battery".to_owned()));
        dialog
    }

    fn clicks(dialog: &ServerDialog, label: &str) -> Vec<Message> {
        let localizer = Localizer::english();
        let mut ui = simulator(dialog.view(&localizer));
        ui.click(label).unwrap();
        ui.into_messages().collect()
    }

    fn shows(dialog: &ServerDialog, words: &str) -> bool {
        let localizer = Localizer::english();
        simulator(dialog.view(&localizer)).find(words).is_ok()
    }

    #[test]
    fn each_text_of_the_dialog_is_in_the_english_messages() {
        let localizer = Localizer::english();

        for id in TEXT_IDS {
            assert_ne!(localizer.text(id), id, "{id} has no English text");
        }
    }

    #[test]
    fn the_form_asks_for_the_address_the_email_and_the_password() {
        let mut dialog = ServerDialog::default();
        dialog.open();

        for words in [
            "Connect to a server",
            "Server address",
            "Email address",
            "Password",
            "Sign in",
            "Cancel",
        ] {
            assert!(shows(&dialog, words), "{words} is not in the dialog");
        }
    }

    #[test]
    fn sign_in_waits_until_each_field_has_text() {
        let mut dialog = ServerDialog::default();
        dialog.open();
        let _ = dialog.update(Message::Address("https://press.example.com".to_owned()));

        assert!(clicks(&dialog, "Sign in").is_empty());
        assert!(matches!(
            clicks(&filled(), "Sign in").as_slice(),
            [Message::SignIn]
        ));
    }

    #[test]
    fn an_address_without_http_is_a_problem_at_once() {
        let mut dialog = filled();
        let _ = dialog.update(Message::Address("press.example.com".to_owned()));

        let _ = dialog.update(Message::SignIn);

        assert!(shows(
            &dialog,
            "The server address must start with http:// or https://."
        ));
        assert!(!dialog.is_signing_in());
    }

    #[test]
    fn while_signing_in_the_button_waits() {
        let mut dialog = filled();

        let _ = dialog.update(Message::SignIn);

        assert!(dialog.is_signing_in());
        assert!(shows(&dialog, "Signing in…"));
        assert!(clicks(&dialog, "Signing in…").is_empty());
    }

    #[test]
    fn a_refusal_shows_the_message_of_the_server() {
        let mut dialog = filled();
        let _ = dialog.update(Message::SignIn);

        let _ = dialog.update(Message::SignedIn(Err(Problem::Refused(
            "The email address or the password is not correct.".to_owned(),
        ))));

        assert!(!dialog.is_signing_in());
        assert!(shows(
            &dialog,
            "The email address or the password is not correct."
        ));
    }

    #[test]
    fn an_unreachable_server_shows_a_problem() {
        let mut dialog = filled();
        let _ = dialog.update(Message::SignIn);

        let _ = dialog.update(Message::SignedIn(Err(Problem::Unreachable)));

        assert!(shows(
            &dialog,
            "Presswork cannot connect to the server. Make sure that the address is correct."
        ));
    }

    #[test]
    fn cancel_closes_the_dialog_and_forgets_the_password() {
        let mut dialog = filled();

        assert!(matches!(
            clicks(&dialog, "Cancel").as_slice(),
            [Message::Close]
        ));
        let _ = dialog.update(Message::Close);

        assert!(!dialog.is_open());
        dialog.open();
        assert!(!shows(&dialog, "correct horse battery"));
        assert!(clicks(&dialog, "Sign in").is_empty());
    }

    const PRESS: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0001);
    const NOTES: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0002);
    const ATLAS: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0003);
    const MAIN: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0004);

    fn connection() -> Connection {
        Connection {
            api: Api::new("https://press.example.com").unwrap(),
            token: "pws_ann".to_owned(),
            user_name: "Ann".to_owned(),
        }
    }

    fn main_document(project: Uuid) -> Document {
        Document {
            id: MAIN,
            project_id: project,
            path: "main.typ".to_owned(),
        }
    }

    fn shelves() -> Vec<Shelf> {
        let project = |id, name: &str| Project {
            id,
            organization_id: PRESS,
            name: name.to_owned(),
        };
        vec![
            Shelf {
                organization: "Field Press".to_owned(),
                project: project(NOTES, "Field Notes"),
                documents: vec![main_document(NOTES)],
            },
            Shelf {
                organization: "Field Press".to_owned(),
                project: project(ATLAS, "Atlas"),
                documents: Vec::new(),
            },
        ]
    }

    fn browsing() -> ServerDialog {
        let mut dialog = filled();
        let _ = dialog.update(Message::SignIn);
        let _ = dialog.update(Message::SignedIn(Ok(connection())));
        dialog
    }

    #[test]
    fn after_the_sign_in_the_dialog_lists_the_documents() {
        let mut dialog = browsing();
        assert!(dialog.is_open());
        assert!(shows(&dialog, "Presswork gets the projects."));

        let _ = dialog.update(Message::Loaded(Ok(shelves())));

        assert!(shows(&dialog, "Open a server document"));
        assert!(shows(&dialog, "Field Press · Field Notes"));
        assert!(shows(&dialog, "main.typ"));
    }

    #[test]
    fn a_click_on_a_document_opens_it() {
        let mut dialog = browsing();
        let _ = dialog.update(Message::Loaded(Ok(shelves())));

        let messages = clicks(&dialog, "main.typ");

        assert!(
            matches!(messages.as_slice(), [Message::Open(document)] if document.id == MAIN),
            "{messages:?}"
        );
        let _ = dialog.update(Message::Open(main_document(NOTES)));
        assert!(!dialog.is_open());
    }

    #[test]
    fn a_project_without_documents_offers_a_main_file() {
        let mut dialog = browsing();
        let _ = dialog.update(Message::Loaded(Ok(shelves())));

        assert!(matches!(
            clicks(&dialog, "Make main.typ").as_slice(),
            [Message::Make(ATLAS)]
        ));
        let _ = dialog.update(Message::Made(Ok(main_document(ATLAS))));

        assert!(!shows(&dialog, "Make main.typ"));
    }

    #[test]
    fn a_listed_document_knows_its_organization() {
        let mut dialog = browsing();
        let _ = dialog.update(Message::Loaded(Ok(shelves())));

        assert_eq!(
            dialog.organization_of(&main_document(NOTES)),
            Some((PRESS, "Field Press".to_owned()))
        );
        assert_eq!(dialog.organization_of(&main_document(Uuid::nil())), None);
    }

    #[test]
    fn a_failed_list_shows_the_problem() {
        let mut dialog = browsing();

        let _ = dialog.update(Message::Loaded(Err(Problem::Unreachable)));

        assert!(shows(
            &dialog,
            "Presswork cannot connect to the server. Make sure that the address is correct."
        ));
    }

    #[test]
    fn the_device_name_tells_the_operating_system() {
        let name = device_name();

        assert!(name.starts_with("Presswork on "), "{name}");
    }

    fn json(value: impl serde::Serialize) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    fn server() -> String {
        let session = presswork_api::Session {
            id: Uuid::from_u128(5),
            device: device_name(),
            created_at: "2026-11-28T09:00:00Z".parse().unwrap(),
            last_used_at: "2026-11-28T09:00:00Z".parse().unwrap(),
            current: true,
        };
        let shelves = shelves();
        serve(vec![
            Route::json(
                "POST /api/v1/sessions",
                &json(presswork_api::SignedIn {
                    token: "pws_ann".to_owned(),
                    session,
                    user_name: "Ann".to_owned(),
                }),
            ),
            Route::json(
                "GET /api/v1/organizations",
                &json([presswork_api::Membership {
                    organization_id: PRESS,
                    name: "Field Press".to_owned(),
                    role: presswork_api::Role::Member,
                }]),
            ),
            Route::json(
                &format!("GET /api/v1/organizations/{PRESS}/projects"),
                &json([&shelves[0].project, &shelves[1].project]),
            ),
            Route::json(
                &format!("GET /api/v1/projects/{NOTES}/documents"),
                &json([main_document(NOTES)]),
            ),
            Route::json(
                &format!("GET /api/v1/projects/{ATLAS}/documents"),
                &json(Vec::<Document>::new()),
            ),
            Route::json(
                &format!("POST /api/v1/projects/{ATLAS}/documents"),
                &json(main_document(ATLAS)),
            ),
        ])
    }

    fn on(base: &str) -> ServerDialog {
        let mut dialog = filled();
        let _ = dialog.update(Message::Address(base.to_owned()));
        dialog
    }

    fn closed_address() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    }

    #[test]
    fn a_sign_in_on_a_server_lists_its_projects_and_documents() {
        let base = server();
        let mut dialog = on(&base);

        let signed = run_task(dialog.update(Message::SignIn));
        let [Message::SignedIn(Ok(connection))] = signed.as_slice() else {
            panic!("{signed:?}");
        };
        let loaded = run_task(dialog.update(Message::SignedIn(Ok(connection.clone()))));

        assert_eq!(connection.user_name, "Ann");
        assert_eq!(connection.token, "pws_ann");
        assert!(
            matches!(loaded.as_slice(), [Message::Loaded(Ok(found))] if *found == shelves()),
            "{loaded:?}"
        );
    }

    #[test]
    fn make_main_typ_asks_the_server_for_the_file() {
        let base = server();
        let mut dialog = on(&base);
        let signed = run_task(dialog.update(Message::SignIn));
        let [Message::SignedIn(Ok(connection))] = signed.as_slice() else {
            panic!("{signed:?}");
        };
        let _ = dialog.update(Message::SignedIn(Ok(connection.clone())));
        let _ = dialog.update(Message::Loaded(Ok(shelves())));

        let made = run_task(dialog.update(Message::Make(ATLAS)));

        assert!(
            matches!(made.as_slice(), [Message::Made(Ok(document))] if *document == main_document(ATLAS)),
            "{made:?}"
        );
    }

    #[test]
    fn a_refusal_gives_the_message_of_the_server() {
        let base = serve(vec![
            Route::json(
                "POST /api/v1/sessions",
                &serde_json::json!({
                    "code": "unauthorized",
                    "message": "The email address or the password is not correct."
                }),
            )
            .status(401),
        ]);
        let mut dialog = on(&base);

        let signed = run_task(dialog.update(Message::SignIn));

        assert!(
            matches!(
                signed.as_slice(),
                [Message::SignedIn(Err(Problem::Refused(message)))]
                    if message == "The email address or the password is not correct."
            ),
            "{signed:?}"
        );
    }

    #[test]
    fn an_answer_without_a_message_gives_its_status() {
        let base = serve(vec![
            Route::new("POST /api/v1/sessions", "broken").status(500),
        ]);
        let mut dialog = on(&base);

        let signed = run_task(dialog.update(Message::SignIn));
        let _ = dialog.update(Message::SignedIn(Err(Problem::Status(500))));

        assert!(
            matches!(
                signed.as_slice(),
                [Message::SignedIn(Err(Problem::Status(500)))]
            ),
            "{signed:?}"
        );
        assert!(shows(&dialog, "The server answered with the status 500."));
    }

    #[test]
    fn a_server_that_does_not_answer_is_unreachable() {
        let mut dialog = on(&closed_address());

        let signed = run_task(dialog.update(Message::SignIn));

        assert!(
            matches!(
                signed.as_slice(),
                [Message::SignedIn(Err(Problem::Unreachable))]
            ),
            "{signed:?}"
        );
    }

    #[test]
    fn a_failed_main_file_shows_the_problem() {
        let mut dialog = browsing();
        let _ = dialog.update(Message::Loaded(Ok(shelves())));

        let _ = dialog.update(Message::Made(Err(Problem::Status(502))));

        assert!(shows(&dialog, "The server answered with the status 502."));
    }

    #[test]
    fn make_without_a_list_does_nothing() {
        let mut dialog = filled();

        let made = run_task(dialog.update(Message::Make(ATLAS)));

        assert_eq!(made.len(), 0);
    }

    #[test]
    fn a_sign_in_after_the_dialog_closed_does_not_list_projects() {
        let mut dialog = filled();
        let _ = dialog.update(Message::SignIn);
        let _ = dialog.update(Message::Close);

        let _ = dialog.update(Message::SignedIn(Ok(connection())));

        dialog.open();
        assert!(shows(&dialog, "Server address"));
    }

    #[test]
    fn a_listed_project_gives_its_name() {
        let mut dialog = browsing();
        assert_eq!(dialog.project_name(NOTES), None);

        let _ = dialog.update(Message::Loaded(Ok(shelves())));
        let _ = dialog.update(Message::Made(Ok(main_document(Uuid::nil()))));

        assert_eq!(dialog.project_name(NOTES), Some("Field Notes".to_owned()));
        assert_eq!(dialog.project_name(Uuid::nil()), None);
    }

    #[test]
    fn without_projects_the_dialog_tells_who_makes_them() {
        let mut dialog = browsing();

        let _ = dialog.update(Message::Loaded(Ok(Vec::new())));

        assert!(shows(
            &dialog,
            "Your organizations have no projects. An administrator makes the projects."
        ));
    }

    #[test]
    fn the_debug_text_of_a_connection_hides_the_token() {
        let text = format!("{:?}", connection());

        assert!(!text.contains("pws_ann"), "{text}");
        assert!(text.contains("[removed]"), "{text}");
    }
}
