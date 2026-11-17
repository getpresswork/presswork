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
    pub editions: Vec<Edition>,
}

impl Work {
    #[must_use]
    pub fn standard_edition(&self) -> Option<&Edition> {
        self.editions.iter().find(|edition| edition.standard)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Edition {
    pub id: Uuid,
    pub year: Option<i32>,
    pub publisher: String,
    pub language: String,
    pub translators: Vec<String>,
    pub standard: bool,
    #[serde(default)]
    pub corrections: Vec<PageRange>,
    pub file: Option<LibraryFile>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PageMap {
    pub corrections: Vec<PageRange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NewEdition {
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub translators: Vec<String>,
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
    pub edition: NewEdition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SearchHit {
    pub work_id: Uuid,
    pub edition_id: Uuid,
    pub key: String,
    pub title: String,
    pub page: u32,
    pub label: String,
    pub snippet: String,
    pub standard: bool,
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::*;

    fn edition(id: u128, year: i32, standard: bool) -> Edition {
        Edition {
            id: Uuid::from_u128(id),
            year: Some(year),
            publisher: "Russky Vestnik".to_owned(),
            language: "ru".to_owned(),
            translators: Vec::new(),
            standard,
            corrections: Vec::new(),
            file: None,
        }
    }

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
                edition: NewEdition::default(),
            }
        );
    }

    #[test]
    fn a_new_edition_needs_no_values() {
        let new: NewEdition =
            serde_json::from_value(json!({ "year": 1922, "translators": ["Maude, Louise"] }))
                .unwrap();

        assert_eq!(
            new,
            NewEdition {
                year: Some(1922),
                publisher: String::new(),
                language: String::new(),
                translators: vec!["Maude, Louise".to_owned()],
            }
        );
    }

    #[test]
    fn a_work_tells_its_editions_and_the_pages_of_their_files() {
        let mut first = edition(1, 1869, true);
        first.file = Some(LibraryFile {
            size: 2048,
            pages: 12,
            pages_with_text: 11,
            state: FileState::Ready,
            problem: None,
        });
        let work = Work {
            id: Uuid::nil(),
            organization_id: Uuid::nil(),
            key: "war-and-peace".to_owned(),
            title: "War and Peace".to_owned(),
            creators: vec!["Tolstoy, Leo".to_owned()],
            editions: vec![first, edition(2, 1922, false)],
        };

        let value = serde_json::to_value(&work).unwrap();

        assert_eq!(
            value["editions"][0]["file"],
            json!({ "size": 2048, "pages": 12, "pages_with_text": 11, "state": "ready" })
        );
        assert_eq!(serde_json::from_value::<Work>(value).unwrap(), work);
    }

    #[test]
    fn the_standard_edition_is_the_marked_edition() {
        let work = Work {
            id: Uuid::nil(),
            organization_id: Uuid::nil(),
            key: "war-and-peace".to_owned(),
            title: "War and Peace".to_owned(),
            creators: Vec::new(),
            editions: vec![edition(1, 1869, false), edition(2, 1922, true)],
        };

        assert_eq!(
            work.standard_edition().map(|edition| edition.id),
            Some(Uuid::from_u128(2))
        );
        let without = Work {
            editions: Vec::new(),
            ..work
        };
        assert_eq!(without.standard_edition(), None);
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
    fn a_search_hit_names_the_work_the_edition_and_the_page() {
        let hit: SearchHit = serde_json::from_value(json!({
            "work_id": Uuid::nil(),
            "edition_id": Uuid::from_u128(2),
            "key": "field-notes",
            "title": "Field Notes",
            "page": 3,
            "label": "iii",
            "snippet": "good paper",
            "standard": false,
        }))
        .unwrap();

        assert_eq!((hit.page, hit.label.as_str()), (3, "iii"));
        assert_eq!(hit.key, "field-notes");
        assert_eq!(hit.edition_id, Uuid::from_u128(2));
        assert!(!hit.standard);
    }

    #[test]
    fn an_edition_tells_the_corrections_of_its_page_map() {
        let mut corrected = edition(1, 1869, true);
        corrected.corrections = vec![PageRange {
            first_page: 15,
            style: LabelStyle::Arabic,
            prefix: String::new(),
            start: 1,
        }];

        let value = serde_json::to_value(&corrected).unwrap();

        assert_eq!(
            value["corrections"],
            json!([{ "first_page": 15, "style": "arabic", "prefix": "", "start": 1 }])
        );
        assert_eq!(serde_json::from_value::<Edition>(value).unwrap(), corrected);
        let page_map: PageMap = serde_json::from_value(json!({
            "corrections": [{ "first_page": 1, "style": "lower_roman" }]
        }))
        .unwrap();
        assert_eq!(page_map.corrections[0].style, LabelStyle::LowerRoman);
    }
}
