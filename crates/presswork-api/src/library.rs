use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Work {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub key: String,
    pub title: String,
    pub creators: Vec<String>,
    pub year: Option<i32>,
    pub file: Option<LibraryFile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LibraryFile {
    pub size: u64,
    pub pages: u32,
    pub pages_with_text: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NewWork {
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub creators: Vec<String>,
    #[serde(default)]
    pub year: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SearchHit {
    pub work_id: Uuid,
    pub key: String,
    pub title: String,
    pub page: u32,
    pub snippet: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn a_new_work_needs_only_a_key_and_a_title() {
        let new: NewWork =
            serde_json::from_value(json!({ "key": "field-notes", "title": "Field Notes" }))
                .unwrap();

        assert_eq!(
            new,
            NewWork {
                key: "field-notes".to_owned(),
                title: "Field Notes".to_owned(),
                creators: Vec::new(),
                year: None,
            }
        );
    }

    #[test]
    fn a_work_tells_the_pages_of_its_file() {
        let work = Work {
            id: Uuid::nil(),
            organization_id: Uuid::nil(),
            key: "field-notes".to_owned(),
            title: "Field Notes".to_owned(),
            creators: vec!["Gutenberg, Johannes".to_owned()],
            year: Some(1455),
            file: Some(LibraryFile {
                size: 2048,
                pages: 12,
                pages_with_text: 11,
            }),
        };

        let value = serde_json::to_value(&work).unwrap();

        assert_eq!(
            value["file"],
            json!({ "size": 2048, "pages": 12, "pages_with_text": 11 })
        );
        assert_eq!(serde_json::from_value::<Work>(value).unwrap(), work);
    }

    #[test]
    fn a_search_hit_names_the_work_and_the_page() {
        let hit: SearchHit = serde_json::from_value(json!({
            "work_id": Uuid::nil(),
            "key": "field-notes",
            "title": "Field Notes",
            "page": 3,
            "snippet": "good paper",
        }))
        .unwrap();

        assert_eq!(hit.page, 3);
        assert_eq!(hit.key, "field-notes");
    }
}
