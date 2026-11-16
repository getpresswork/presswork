use iced::alignment::Vertical;
use iced::widget::{button, checkbox, column, container, row, scrollable, space, text, text_input};
use iced::{Element, Fill, Task};
use presswork_api::{NewTask, Task as ProjectTask, TaskChange, TaskState};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, space as gap};
use uuid::Uuid;

use crate::i18n::{FluentArgs, Localizer};
use crate::server_dialog::{Connection, Problem, problem, problem_text};

const LIST_WIDTH: f32 = 640.0;

#[cfg(test)]
pub(crate) const TEXT_IDS: [&str; 5] = [
    "tasks-title",
    "tasks-loading",
    "tasks-empty",
    "tasks-placeholder",
    "tasks-add",
];

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Loaded(Result<Vec<ProjectTask>, Problem>),
    Toggle(Uuid),
    Changed(Result<ProjectTask, Problem>),
    Draft(String),
    Add,
    Added(Result<ProjectTask, Problem>),
}

#[derive(Debug)]
pub(crate) struct TaskList {
    project: Uuid,
    name: String,
    tasks: Option<Vec<ProjectTask>>,
    draft: String,
    problem: Option<Problem>,
}

impl TaskList {
    pub(crate) fn new(project: Uuid, name: String) -> Self {
        Self {
            project,
            name,
            tasks: None,
            draft: String::new(),
            problem: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn state_of(&self, task: Uuid) -> Option<TaskState> {
        self.tasks
            .as_ref()?
            .iter()
            .find(|item| item.id == task)
            .map(|item| item.state)
    }

    pub(crate) fn load(&self, connection: &Connection) -> Task<Message> {
        let Connection { api, token } = connection.clone();
        let project = self.project;
        Task::perform(
            async move { api.tasks(&token, project).await.map_err(problem) },
            Message::Loaded,
        )
    }

    pub(crate) fn update(&mut self, message: Message, connection: &Connection) -> Task<Message> {
        match message {
            Message::Loaded(Ok(tasks)) => self.tasks = Some(tasks),
            Message::Loaded(Err(problem)) | Message::Added(Err(problem)) => {
                self.problem = Some(problem);
            }
            Message::Toggle(id) => return self.toggle(id, connection),
            Message::Changed(Ok(task)) => self.replace(task),
            Message::Changed(Err(problem)) => {
                self.problem = Some(problem);
                return self.load(connection);
            }
            Message::Draft(draft) => self.draft = draft,
            Message::Add => return self.add(connection),
            Message::Added(Ok(task)) => {
                if let Some(tasks) = &mut self.tasks {
                    tasks.push(task);
                }
                self.draft.clear();
            }
        }
        Task::none()
    }

    fn toggle(&mut self, id: Uuid, connection: &Connection) -> Task<Message> {
        let Some(task) = self
            .tasks
            .as_mut()
            .and_then(|tasks| tasks.iter_mut().find(|task| task.id == id))
        else {
            return Task::none();
        };
        task.state = if task.state == TaskState::Done {
            TaskState::ToDo
        } else {
            TaskState::Done
        };
        let change = TaskChange {
            state: Some(task.state),
            ..TaskChange::default()
        };
        self.problem = None;
        let Connection { api, token } = connection.clone();
        Task::perform(
            async move { api.update_task(&token, id, &change).await.map_err(problem) },
            Message::Changed,
        )
    }

    fn replace(&mut self, task: ProjectTask) {
        if let Some(known) = self
            .tasks
            .as_mut()
            .and_then(|tasks| tasks.iter_mut().find(|known| known.id == task.id))
        {
            *known = task;
        }
    }

    fn add(&mut self, connection: &Connection) -> Task<Message> {
        let title = self.draft.trim().to_owned();
        if title.is_empty() {
            return Task::none();
        }
        self.problem = None;
        let Connection { api, token } = connection.clone();
        let project = self.project;
        let new = NewTask {
            title,
            ..NewTask::default()
        };
        Task::perform(
            async move {
                api.create_task(&token, project, &new)
                    .await
                    .map_err(problem)
            },
            Message::Added,
        )
    }

    pub(crate) fn view<'a>(&'a self, localizer: &Localizer) -> Element<'a, Message> {
        let mut args = FluentArgs::new();
        args.set("project", self.name.clone());
        let ready = !self.draft.trim().is_empty();
        let add = row![
            text_input(&localizer.text("tasks-placeholder"), &self.draft)
                .on_input(Message::Draft)
                .on_submit(Message::Add)
                .padding([gap::XS + gap::XXS, gap::S])
                .style(style::field),
            button(text(localizer.text("tasks-add")))
                .padding([gap::XS + gap::XXS, gap::M])
                .style(style::primary)
                .on_press_maybe(ready.then_some(Message::Add)),
        ]
        .spacing(gap::S)
        .align_y(Vertical::Center);
        let mut content = column![
            text(localizer.format("tasks-title", &args))
                .font(font::UI_SEMIBOLD)
                .size(font_size::TITLE),
            add,
        ]
        .spacing(gap::M);
        if let Some(problem) = &self.problem {
            content = content.push(
                text(problem_text(problem, localizer))
                    .size(font_size::SMALL)
                    .style(style::tone(Tone::Danger)),
            );
        }
        let list: Element<'a, Message> = match &self.tasks {
            None => text(localizer.text("tasks-loading"))
                .style(style::muted)
                .into(),
            Some(tasks) if tasks.is_empty() => text(localizer.text("tasks-empty"))
                .style(style::muted)
                .into(),
            Some(tasks) => scrollable(column(tasks.iter().map(line)).spacing(gap::XXS))
                .height(Fill)
                .into(),
        };
        container(content.push(list).width(LIST_WIDTH))
            .center_x(Fill)
            .padding(gap::XL)
            .height(Fill)
            .style(style::page)
            .into()
    }
}

fn line(task: &ProjectTask) -> Element<'_, Message> {
    let id = task.id;
    let mut line = row![
        checkbox(task.state == TaskState::Done)
            .label(task.title.as_str())
            .spacing(gap::S)
            .on_toggle(move |_| Message::Toggle(id)),
        space::horizontal(),
    ]
    .align_y(Vertical::Center);
    if let Some(due) = task.due_date {
        line = line.push(
            text(due.to_string())
                .size(font_size::SMALL)
                .style(style::muted),
        );
    }
    container(line).padding([gap::XS, gap::S]).into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use jiff::Timestamp;
    use jiff::civil::date;
    use presswork_api::{Task as ProjectTask, TaskState};
    use presswork_client::Api;
    use uuid::Uuid;

    use super::*;
    use crate::i18n::Localizer;
    use crate::server_dialog::{Connection, Problem};

    const NOTES: Uuid = Uuid::from_u128(2);
    const COVER: Uuid = Uuid::from_u128(5);
    const PAPER: Uuid = Uuid::from_u128(6);

    fn connection() -> Connection {
        Connection {
            api: Api::new("https://press.example.com").unwrap(),
            token: "pws_ann".to_owned(),
        }
    }

    fn task(id: Uuid, title: &str, state: TaskState) -> ProjectTask {
        ProjectTask {
            id,
            project_id: NOTES,
            title: title.to_owned(),
            description: String::new(),
            state,
            assignee_id: None,
            due_date: Some(date(2026, 11, 23)),
            created_by: None,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn loaded() -> TaskList {
        let mut list = TaskList::new(NOTES, "Field Notes".to_owned());
        let _ = list.update(
            Message::Loaded(Ok(vec![
                task(COVER, "Proof the cover", TaskState::ToDo),
                task(PAPER, "Order the paper", TaskState::Done),
            ])),
            &connection(),
        );
        list
    }

    fn shows(list: &TaskList, words: &str) -> bool {
        let localizer = Localizer::english();
        simulator(list.view(&localizer)).find(words).is_ok()
    }

    fn clicks(list: &TaskList, label: &str) -> Vec<Message> {
        let localizer = Localizer::english();
        let mut ui = simulator(list.view(&localizer));
        ui.click(label).unwrap();
        ui.into_messages().collect()
    }

    #[test]
    fn each_text_of_the_list_is_in_the_english_messages() {
        let localizer = Localizer::english();

        for id in TEXT_IDS {
            assert_ne!(localizer.text(id), id, "{id} has no English text");
        }
    }

    #[test]
    fn the_list_shows_the_tasks_of_the_project() {
        let list = loaded();

        for words in [
            "Tasks · Field Notes",
            "Proof the cover",
            "Order the paper",
            "2026-11-23",
        ] {
            assert!(shows(&list, words), "{words} is not in the list");
        }
    }

    #[test]
    fn a_click_on_a_task_completes_it_at_once() {
        let mut list = loaded();

        let messages = clicks(&list, "Proof the cover");
        assert!(
            matches!(messages.as_slice(), [Message::Toggle(COVER)]),
            "{messages:?}"
        );
        let _ = list.update(Message::Toggle(COVER), &connection());

        assert_eq!(list.state_of(COVER), Some(TaskState::Done));
        let _ = list.update(Message::Toggle(PAPER), &connection());
        assert_eq!(list.state_of(PAPER), Some(TaskState::ToDo));
    }

    #[test]
    fn a_refused_change_shows_the_problem() {
        let mut list = loaded();
        let _ = list.update(Message::Toggle(COVER), &connection());

        let _ = list.update(
            Message::Changed(Err(Problem::Refused(
                "You do not have the permission for this function.".to_owned(),
            ))),
            &connection(),
        );

        assert!(shows(
            &list,
            "You do not have the permission for this function."
        ));
    }

    #[test]
    fn add_waits_for_a_title_and_shows_the_new_task() {
        let mut list = loaded();
        assert!(clicks(&list, "Add").is_empty());

        let _ = list.update(
            Message::Draft(" Print the proofs ".to_owned()),
            &connection(),
        );
        assert!(matches!(clicks(&list, "Add").as_slice(), [Message::Add]));
        let _ = list.update(Message::Add, &connection());
        let _ = list.update(
            Message::Added(Ok(task(
                Uuid::from_u128(7),
                "Print the proofs",
                TaskState::ToDo,
            ))),
            &connection(),
        );

        assert!(shows(&list, "Print the proofs"));
        assert!(clicks(&list, "Add").is_empty());
    }

    #[test]
    fn an_empty_project_asks_for_the_first_task() {
        let mut list = TaskList::new(NOTES, "Field Notes".to_owned());
        assert!(shows(&list, "Presswork gets the tasks."));

        let _ = list.update(Message::Loaded(Ok(Vec::new())), &connection());

        assert!(shows(
            &list,
            "This project has no tasks. Add the first task."
        ));
    }
}
