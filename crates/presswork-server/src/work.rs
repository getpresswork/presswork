use jiff_sqlx::{Date as SqlDate, Timestamp as SqlTimestamp, ToSqlx};
use presswork_api::{Membership, NewTask, Patch, Project, Role, Task, TaskChange, TaskState};
use sqlx::PgPool;
use uuid::Uuid;

const MAX_PROJECT_NAME_CHARACTERS: usize = 200;
const MAX_TITLE_CHARACTERS: usize = 300;
const MAX_DESCRIPTION_CHARACTERS: usize = 100_000;
const TASK_COLUMNS: &str = "id, project_id, title, description, state, assignee_id, due_date, \
                            created_by, created_at, updated_at";

#[derive(Debug, thiserror::Error)]
pub enum WorkError {
    #[error("a project name must have 1 to 200 characters")]
    InvalidName,
    #[error("a task title must have 1 to 300 characters")]
    InvalidTitle,
    #[error("a task description must have 100,000 characters or fewer")]
    LongDescription,
    #[error("the assignee must be a member of the organization")]
    AssigneeNotMember,
    #[error("the item does not exist")]
    NotFound,
    #[error("the database failed: {0}")]
    Database(#[from] sqlx::Error),
}

type TaskRow = (
    Uuid,
    Uuid,
    String,
    String,
    String,
    Option<Uuid>,
    Option<SqlDate>,
    Option<Uuid>,
    SqlTimestamp,
    SqlTimestamp,
);

fn task(row: TaskRow) -> Task {
    let (
        id,
        project_id,
        title,
        description,
        state,
        assignee_id,
        due_date,
        created_by,
        created,
        updated,
    ) = row;
    Task {
        id,
        project_id,
        title,
        description,
        state: TaskState::parse(&state).unwrap_or(TaskState::ToDo),
        assignee_id,
        due_date: due_date.map(SqlDate::to_jiff),
        created_by,
        created_at: created.to_jiff(),
        updated_at: updated.to_jiff(),
    }
}

fn parse_project_name(name: &str) -> Result<&str, WorkError> {
    let name = name.trim();
    if (1..=MAX_PROJECT_NAME_CHARACTERS).contains(&name.chars().count()) {
        Ok(name)
    } else {
        Err(WorkError::InvalidName)
    }
}

pub(crate) fn parse_title(title: &str) -> Result<&str, WorkError> {
    let title = title.trim();
    if (1..=MAX_TITLE_CHARACTERS).contains(&title.chars().count()) {
        Ok(title)
    } else {
        Err(WorkError::InvalidTitle)
    }
}

fn check_description(description: &str) -> Result<(), WorkError> {
    if description.chars().count() > MAX_DESCRIPTION_CHARACTERS {
        return Err(WorkError::LongDescription);
    }
    Ok(())
}

async fn check_assignee(
    pool: &PgPool,
    organization_id: Uuid,
    user_id: Uuid,
) -> Result<(), WorkError> {
    let member: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM memberships WHERE organization_id = $1 AND user_id = $2)",
    )
    .bind(organization_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    if member {
        Ok(())
    } else {
        Err(WorkError::AssigneeNotMember)
    }
}

pub async fn memberships(pool: &PgPool, user_id: Uuid) -> Result<Vec<Membership>, WorkError> {
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT organizations.id, organizations.name, memberships.role FROM memberships \
         JOIN organizations ON organizations.id = memberships.organization_id \
         WHERE memberships.user_id = $1 ORDER BY organizations.name",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(organization_id, name, role)| {
            Some(Membership {
                organization_id,
                name,
                role: Role::parse(&role)?,
            })
        })
        .collect())
}

pub async fn create_project(
    pool: &PgPool,
    organization_id: Uuid,
    name: &str,
) -> Result<Project, WorkError> {
    let project = Project {
        id: Uuid::now_v7(),
        organization_id,
        name: parse_project_name(name)?.to_owned(),
    };
    sqlx::query("INSERT INTO projects (id, organization_id, name) VALUES ($1, $2, $3)")
        .bind(project.id)
        .bind(organization_id)
        .bind(&project.name)
        .execute(pool)
        .await?;
    Ok(project)
}

pub async fn rename_project(
    pool: &PgPool,
    project_id: Uuid,
    name: &str,
) -> Result<Project, WorkError> {
    let name = parse_project_name(name)?;
    let organization_id: Option<Uuid> =
        sqlx::query_scalar("UPDATE projects SET name = $2 WHERE id = $1 RETURNING organization_id")
            .bind(project_id)
            .bind(name)
            .fetch_optional(pool)
            .await?;
    let organization_id = organization_id.ok_or(WorkError::NotFound)?;
    Ok(Project {
        id: project_id,
        organization_id,
        name: name.to_owned(),
    })
}

pub async fn projects(pool: &PgPool, organization_id: Uuid) -> Result<Vec<Project>, WorkError> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, name FROM projects WHERE organization_id = $1 ORDER BY name, id",
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name)| Project {
            id,
            organization_id,
            name,
        })
        .collect())
}

pub async fn project_organization(
    pool: &PgPool,
    project_id: Uuid,
) -> Result<Option<Uuid>, WorkError> {
    Ok(
        sqlx::query_scalar("SELECT organization_id FROM projects WHERE id = $1")
            .bind(project_id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn create_task(
    pool: &PgPool,
    project_id: Uuid,
    created_by: Uuid,
    new: &NewTask,
) -> Result<Task, WorkError> {
    let title = parse_title(&new.title)?;
    check_description(&new.description)?;
    let organization_id = project_organization(pool, project_id)
        .await?
        .ok_or(WorkError::NotFound)?;
    if let Some(assignee) = new.assignee_id {
        check_assignee(pool, organization_id, assignee).await?;
    }
    let row: TaskRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO tasks (id, project_id, title, description, state, assignee_id, due_date, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {TASK_COLUMNS}"
    )))
    .bind(Uuid::now_v7())
    .bind(project_id)
    .bind(title)
    .bind(&new.description)
    .bind(TaskState::ToDo.as_str())
    .bind(new.assignee_id)
    .bind(new.due_date.map(ToSqlx::to_sqlx))
    .bind(created_by)
    .fetch_one(pool)
    .await?;
    Ok(task(row))
}

pub async fn tasks(pool: &PgPool, project_id: Uuid) -> Result<Vec<Task>, WorkError> {
    let rows: Vec<TaskRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {TASK_COLUMNS} FROM tasks WHERE project_id = $1 ORDER BY created_at, id"
    )))
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(task).collect())
}

pub async fn task_owner(
    pool: &PgPool,
    task_id: Uuid,
) -> Result<Option<(Uuid, Option<Uuid>)>, WorkError> {
    Ok(sqlx::query_as(
        "SELECT projects.organization_id, tasks.created_by FROM tasks \
         JOIN projects ON projects.id = tasks.project_id WHERE tasks.id = $1",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?)
}

pub async fn update_task(
    pool: &PgPool,
    task_id: Uuid,
    change: &TaskChange,
) -> Result<Task, WorkError> {
    let title = change.title.as_deref().map(parse_title).transpose()?;
    if let Some(description) = &change.description {
        check_description(description)?;
    }
    let (organization_id, _) = task_owner(pool, task_id)
        .await?
        .ok_or(WorkError::NotFound)?;
    if let Patch::Set(assignee) = change.assignee_id {
        check_assignee(pool, organization_id, assignee).await?;
    }
    let (assignee_changes, assignee) = patch_parts(change.assignee_id);
    let (due_date_changes, due_date) = patch_parts(change.due_date.map(ToSqlx::to_sqlx));
    let row: Option<TaskRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE tasks SET \
         title = COALESCE($2, title), \
         description = COALESCE($3, description), \
         state = COALESCE($4, state), \
         assignee_id = CASE WHEN $5 THEN $6 ELSE assignee_id END, \
         due_date = CASE WHEN $7 THEN $8 ELSE due_date END, \
         updated_at = now() \
         WHERE id = $1 RETURNING {TASK_COLUMNS}"
    )))
    .bind(task_id)
    .bind(title)
    .bind(change.description.as_deref())
    .bind(change.state.map(TaskState::as_str))
    .bind(assignee_changes)
    .bind(assignee)
    .bind(due_date_changes)
    .bind(due_date)
    .fetch_optional(pool)
    .await?;
    row.map(task).ok_or(WorkError::NotFound)
}

fn patch_parts<T>(patch: Patch<T>) -> (bool, Option<T>) {
    match patch {
        Patch::Keep => (false, None),
        Patch::Clear => (true, None),
        Patch::Set(value) => (true, Some(value)),
    }
}

pub async fn delete_task(pool: &PgPool, task_id: Uuid) -> Result<bool, WorkError> {
    let result = sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use presswork_api::Role;

    use super::*;
    use crate::accounts::{User, create_user};
    use crate::links::create_organization;
    use crate::testing::test_database;

    async fn member(pool: &PgPool, organization: Uuid, email: &str, role: Role) -> User {
        let user = create_user(pool, email, "Someone", "correct horse battery")
            .await
            .unwrap();
        sqlx::query("INSERT INTO memberships (organization_id, user_id, role) VALUES ($1, $2, $3)")
            .bind(organization)
            .bind(user.id)
            .bind(role.as_str())
            .execute(pool)
            .await
            .unwrap();
        user
    }

    fn new_task(title: &str) -> NewTask {
        NewTask {
            title: title.to_owned(),
            ..NewTask::default()
        }
    }

    #[test]
    fn a_title_needs_1_to_300_characters() {
        assert!(matches!(parse_title("  "), Err(WorkError::InvalidTitle)));
        assert!(matches!(
            parse_title(&"x".repeat(301)),
            Err(WorkError::InvalidTitle)
        ));
        assert_eq!(parse_title(" Proof the cover ").unwrap(), "Proof the cover");
    }

    #[tokio::test]
    async fn a_user_sees_their_organizations_with_their_roles() {
        let Some(database) = test_database().await else {
            return;
        };
        let press = create_organization(&database.pool, "Field Press")
            .await
            .unwrap();
        let guild = create_organization(&database.pool, "Binders Guild")
            .await
            .unwrap();
        let ann = member(
            &database.pool,
            press.id,
            "ann@example.com",
            Role::Administrator,
        )
        .await;
        sqlx::query(
            "INSERT INTO memberships (organization_id, user_id, role) VALUES ($1, $2, 'member')",
        )
        .bind(guild.id)
        .bind(ann.id)
        .execute(&database.pool)
        .await
        .unwrap();

        let memberships = memberships(&database.pool, ann.id).await.unwrap();

        let summary: Vec<(&str, Role)> = memberships
            .iter()
            .map(|membership| (membership.name.as_str(), membership.role))
            .collect();
        assert_eq!(
            summary,
            [
                ("Binders Guild", Role::Member),
                ("Field Press", Role::Administrator)
            ]
        );
    }

    #[tokio::test]
    async fn projects_belong_to_their_organization() {
        let Some(database) = test_database().await else {
            return;
        };
        let press = create_organization(&database.pool, "Field Press")
            .await
            .unwrap();
        let guild = create_organization(&database.pool, "Binders Guild")
            .await
            .unwrap();

        let project = create_project(&database.pool, press.id, "Field Notes")
            .await
            .unwrap();
        let renamed = rename_project(&database.pool, project.id, "Field Notes, 2nd edition")
            .await
            .unwrap();

        assert_eq!(
            projects(&database.pool, press.id).await.unwrap(),
            std::slice::from_ref(&renamed)
        );
        assert_eq!(projects(&database.pool, guild.id).await.unwrap(), []);
        assert_eq!(renamed.name, "Field Notes, 2nd edition");
        assert_eq!(
            project_organization(&database.pool, project.id)
                .await
                .unwrap(),
            Some(press.id)
        );
    }

    #[tokio::test]
    async fn a_new_task_starts_to_do_with_its_fields() {
        let Some(database) = test_database().await else {
            return;
        };
        let press = create_organization(&database.pool, "Field Press")
            .await
            .unwrap();
        let ann = member(&database.pool, press.id, "ann@example.com", Role::Member).await;
        let project = create_project(&database.pool, press.id, "Field Notes")
            .await
            .unwrap();

        let task = create_task(
            &database.pool,
            project.id,
            ann.id,
            &NewTask {
                title: "Proof the cover".to_owned(),
                description: "Check the **spine** width.".to_owned(),
                assignee_id: Some(ann.id),
                due_date: Some(date(2026, 11, 23)),
            },
        )
        .await
        .unwrap();

        assert_eq!(task.state, TaskState::ToDo);
        assert_eq!(task.created_by, Some(ann.id));
        assert_eq!(task.assignee_id, Some(ann.id));
        assert_eq!(task.due_date, Some(date(2026, 11, 23)));
        assert_eq!(tasks(&database.pool, project.id).await.unwrap(), [task]);
    }

    #[tokio::test]
    async fn a_task_change_sets_only_the_given_fields() {
        let Some(database) = test_database().await else {
            return;
        };
        let press = create_organization(&database.pool, "Field Press")
            .await
            .unwrap();
        let ann = member(&database.pool, press.id, "ann@example.com", Role::Member).await;
        let project = create_project(&database.pool, press.id, "Field Notes")
            .await
            .unwrap();
        let task = create_task(
            &database.pool,
            project.id,
            ann.id,
            &NewTask {
                due_date: Some(date(2026, 11, 23)),
                ..new_task("Proof the cover")
            },
        )
        .await
        .unwrap();

        let changed = update_task(
            &database.pool,
            task.id,
            &TaskChange {
                state: Some(TaskState::Done),
                assignee_id: Patch::Set(ann.id),
                due_date: Patch::Clear,
                ..TaskChange::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(changed.title, "Proof the cover");
        assert_eq!(changed.state, TaskState::Done);
        assert_eq!(changed.assignee_id, Some(ann.id));
        assert_eq!(changed.due_date, None);
        assert!(changed.updated_at >= task.updated_at);
    }

    #[tokio::test]
    async fn an_assignee_must_be_a_member_of_the_organization() {
        let Some(database) = test_database().await else {
            return;
        };
        let press = create_organization(&database.pool, "Field Press")
            .await
            .unwrap();
        let ann = member(&database.pool, press.id, "ann@example.com", Role::Member).await;
        let outsider = create_user(
            &database.pool,
            "bob@example.com",
            "Bob",
            "correct horse battery",
        )
        .await
        .unwrap();
        let project = create_project(&database.pool, press.id, "Field Notes")
            .await
            .unwrap();
        let task = create_task(
            &database.pool,
            project.id,
            ann.id,
            &new_task("Proof the cover"),
        )
        .await
        .unwrap();

        let created = create_task(
            &database.pool,
            project.id,
            ann.id,
            &NewTask {
                assignee_id: Some(outsider.id),
                ..new_task("Print the proofs")
            },
        )
        .await;
        let changed = update_task(
            &database.pool,
            task.id,
            &TaskChange {
                assignee_id: Patch::Set(outsider.id),
                ..TaskChange::default()
            },
        )
        .await;

        assert!(matches!(created, Err(WorkError::AssigneeNotMember)));
        assert!(matches!(changed, Err(WorkError::AssigneeNotMember)));
    }

    #[tokio::test]
    async fn a_task_knows_its_organization_and_creator_until_it_is_deleted() {
        let Some(database) = test_database().await else {
            return;
        };
        let press = create_organization(&database.pool, "Field Press")
            .await
            .unwrap();
        let ann = member(&database.pool, press.id, "ann@example.com", Role::Member).await;
        let project = create_project(&database.pool, press.id, "Field Notes")
            .await
            .unwrap();
        let task = create_task(
            &database.pool,
            project.id,
            ann.id,
            &new_task("Proof the cover"),
        )
        .await
        .unwrap();

        assert_eq!(
            task_owner(&database.pool, task.id).await.unwrap(),
            Some((press.id, Some(ann.id)))
        );
        assert!(delete_task(&database.pool, task.id).await.unwrap());
        assert_eq!(task_owner(&database.pool, task.id).await.unwrap(), None);
        assert!(!delete_task(&database.pool, task.id).await.unwrap());
    }
}
