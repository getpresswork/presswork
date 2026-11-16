use jiff::Timestamp;
use jiff::civil::Date;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    ToDo,
    InProgress,
    Done,
}

impl TaskState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToDo => "to_do",
            Self::InProgress => "in_progress",
            Self::Done => "done",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "to_do" => Some(Self::ToDo),
            "in_progress" => Some(Self::InProgress),
            "done" => Some(Self::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Membership {
    pub organization_id: Uuid,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Project {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NewProject {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ProjectChange {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub description: String,
    pub state: TaskState,
    pub assignee_id: Option<Uuid>,
    #[schema(value_type = Option<String>, format = Date)]
    pub due_date: Option<Date>,
    pub created_by: Option<Uuid>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: Timestamp,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NewTask {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub assignee_id: Option<Uuid>,
    #[serde(default)]
    #[schema(value_type = Option<String>, format = Date)]
    pub due_date: Option<Date>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TaskChange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<TaskState>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    #[schema(value_type = Option<Uuid>, nullable)]
    pub assignee_id: Patch<Uuid>,
    #[serde(default, skip_serializing_if = "Patch::is_keep")]
    #[schema(value_type = Option<String>, format = Date, nullable)]
    pub due_date: Patch<Date>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Patch<T> {
    #[default]
    Keep,
    Clear,
    Set(T),
}

impl<T> Patch<T> {
    #[must_use]
    pub fn is_keep(&self) -> bool {
        matches!(self, Self::Keep)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Set(value),
            None => Self::Clear,
        })
    }
}

impl<T: Serialize> Serialize for Patch<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Set(value) => value.serialize(serializer),
            Self::Keep | Self::Clear => serializer.serialize_none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_states_have_snake_case_names() {
        for (state, name) in [
            (TaskState::ToDo, "to_do"),
            (TaskState::InProgress, "in_progress"),
            (TaskState::Done, "done"),
        ] {
            assert_eq!(serde_json::to_value(state).unwrap(), name);
            assert_eq!(state.as_str(), name);
            assert_eq!(TaskState::parse(name), Some(state));
        }
        assert_eq!(TaskState::parse("closed"), None);
    }

    #[test]
    fn a_task_change_tells_an_absent_field_from_a_cleared_field() {
        let id = Uuid::nil();

        let absent: TaskChange = serde_json::from_str("{}").unwrap();
        let cleared: TaskChange =
            serde_json::from_str(r#"{ "assignee_id": null, "due_date": null }"#).unwrap();
        let set: TaskChange = serde_json::from_str(&format!(
            r#"{{ "assignee_id": "{id}", "due_date": "2026-11-23" }}"#
        ))
        .unwrap();

        assert_eq!(absent.assignee_id, Patch::Keep);
        assert_eq!(absent.due_date, Patch::Keep);
        assert_eq!(cleared.assignee_id, Patch::Clear);
        assert_eq!(cleared.due_date, Patch::Clear);
        assert_eq!(set.assignee_id, Patch::Set(id));
        assert_eq!(set.due_date, Patch::Set(jiff::civil::date(2026, 11, 23)));
    }

    #[test]
    fn a_task_change_writes_only_the_fields_it_changes() {
        let change = TaskChange {
            state: Some(TaskState::Done),
            assignee_id: Patch::Clear,
            ..TaskChange::default()
        };

        assert_eq!(
            serde_json::to_value(&change).unwrap(),
            serde_json::json!({ "state": "done", "assignee_id": null })
        );
    }
}
