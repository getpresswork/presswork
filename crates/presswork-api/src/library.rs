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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LibraryFile {
    pub size: u64,
    pub pages: u32,
    pub pages_with_text: u32,
    pub state: FileState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    Reading,
    Ready,
    Failed,
}

impl FileState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reading => "reading",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "reading" => Some(Self::Reading),
            "ready" => Some(Self::Ready),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LabelStyle {
    Arabic,
    UpperRoman,
    LowerRoman,
    UpperLetters,
    LowerLetters,
    None,
}

impl LabelStyle {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Arabic => "arabic",
            Self::UpperRoman => "upper_roman",
            Self::LowerRoman => "lower_roman",
            Self::UpperLetters => "upper_letters",
            Self::LowerLetters => "lower_letters",
            Self::None => "none",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "arabic" => Some(Self::Arabic),
            "upper_roman" => Some(Self::UpperRoman),
            "lower_roman" => Some(Self::LowerRoman),
            "upper_letters" => Some(Self::UpperLetters),
            "lower_letters" => Some(Self::LowerLetters),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PageRange {
    pub first_page: u32,
    pub style: LabelStyle,
    #[serde(default)]
    pub prefix: String,
    #[serde(default = "first_number")]
    pub start: u32,
}

fn first_number() -> u32 {
    1
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
                state: FileState::Ready,
                problem: None,
            }),
        };

        let value = serde_json::to_value(&work).unwrap();

        assert_eq!(
            value["file"],
            json!({ "size": 2048, "pages": 12, "pages_with_text": 11, "state": "ready" })
        );
        assert_eq!(serde_json::from_value::<Work>(value).unwrap(), work);
    }

    #[test]
    fn label_styles_have_snake_case_names() {
        for (style, name) in [
            (LabelStyle::Arabic, "arabic"),
            (LabelStyle::UpperRoman, "upper_roman"),
            (LabelStyle::LowerRoman, "lower_roman"),
            (LabelStyle::UpperLetters, "upper_letters"),
            (LabelStyle::LowerLetters, "lower_letters"),
            (LabelStyle::None, "none"),
        ] {
            assert_eq!(serde_json::to_value(style).unwrap(), name);
            assert_eq!(style.as_str(), name);
            assert_eq!(LabelStyle::parse(name), Some(style));
        }
        assert_eq!(LabelStyle::parse("greek"), None);
    }

    #[test]
    fn a_page_range_starts_at_one_without_a_prefix() {
        let range: PageRange =
            serde_json::from_value(json!({ "first_page": 15, "style": "arabic" })).unwrap();

        assert_eq!(
            range,
            PageRange {
                first_page: 15,
                style: LabelStyle::Arabic,
                prefix: String::new(),
                start: 1,
            }
        );
    }

    #[test]
    fn file_states_have_snake_case_names() {
        for (state, name) in [
            (FileState::Reading, "reading"),
            (FileState::Ready, "ready"),
            (FileState::Failed, "failed"),
        ] {
            assert_eq!(serde_json::to_value(state).unwrap(), name);
            assert_eq!(state.as_str(), name);
            assert_eq!(FileState::parse(name), Some(state));
        }
        assert_eq!(FileState::parse("lost"), None);
    }

    #[test]
    fn a_failed_file_tells_the_problem() {
        let file: LibraryFile = serde_json::from_value(json!({
            "size": 10,
            "pages": 1,
            "pages_with_text": 0,
            "state": "failed",
            "problem": "Presswork cannot read the text of the file.",
        }))
        .unwrap();

        assert_eq!(file.state, FileState::Failed);
        assert_eq!(
            file.problem.as_deref(),
            Some("Presswork cannot read the text of the file.")
        );
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
