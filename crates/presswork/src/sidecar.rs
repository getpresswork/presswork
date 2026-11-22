use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use presswork_text::{Position, Suggestion, SuggestionKind};
use serde::{Deserialize, Serialize};

const FOLDER: &str = ".presswork/suggestions";
const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Loaded {
    Nothing,
    Restored {
        text: String,
        suggestions: Vec<Suggestion>,
    },
    Moved(PathBuf),
}

#[derive(Serialize, Deserialize)]
struct Sidecar {
    version: u32,
    text: String,
    suggestions: Vec<Record>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Insertion,
    Deletion,
}

#[derive(Serialize, Deserialize)]
struct Record {
    kind: Kind,
    start: usize,
    end: usize,
    author: String,
    time: i64,
}

pub(crate) fn path(root: &Path, file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(".json");
    root.join(FOLDER).join(name)
}

pub(crate) fn file_text(text: &str, suggestions: &[Suggestion]) -> String {
    without(text, suggestions, SuggestionKind::Insertion)
}

pub(crate) fn without(text: &str, suggestions: &[Suggestion], kind: SuggestionKind) -> String {
    let mut ranges: Vec<(usize, usize)> = suggestions
        .iter()
        .filter(|suggestion| suggestion.kind == kind)
        .map(|suggestion| {
            (
                suggestion.range.start.offset(),
                suggestion.range.end.offset(),
            )
        })
        .collect();
    ranges.sort_unstable();
    let mut result = String::with_capacity(text.len());
    let mut at = 0;
    for (start, end) in ranges {
        if start >= at {
            result.push_str(&text[at..start]);
        }
        at = at.max(end);
    }
    result.push_str(&text[at.min(text.len())..]);
    result
}

pub(crate) fn save(
    root: &Path,
    file: &Path,
    text: &str,
    suggestions: &[Suggestion],
) -> io::Result<()> {
    let path = path(root, file);
    if suggestions.is_empty() {
        return match fs::remove_file(&path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        };
    }
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let sidecar = Sidecar {
        version: VERSION,
        text: text.to_owned(),
        suggestions: suggestions
            .iter()
            .map(|suggestion| Record {
                kind: match suggestion.kind {
                    SuggestionKind::Insertion => Kind::Insertion,
                    SuggestionKind::Deletion => Kind::Deletion,
                },
                start: suggestion.range.start.offset(),
                end: suggestion.range.end.offset(),
                author: suggestion.author.clone(),
                time: suggestion.time,
            })
            .collect(),
    };
    fs::write(&path, serde_json::to_string_pretty(&sidecar)?)
}

pub(crate) fn load(root: &Path, file: &Path, file_text: &str) -> io::Result<Loaded> {
    let path = path(root, file);
    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Loaded::Nothing),
        Err(error) => return Err(error),
    };
    match restored(&data) {
        Some((text, suggestions)) if self::file_text(&text, &suggestions) == file_text => {
            Ok(Loaded::Restored { text, suggestions })
        }
        _ => {
            let mut moved = path.clone().into_os_string();
            moved.push(".old");
            let moved = PathBuf::from(moved);
            fs::rename(&path, &moved)?;
            Ok(Loaded::Moved(moved))
        }
    }
}

fn restored(data: &str) -> Option<(String, Vec<Suggestion>)> {
    let sidecar: Sidecar = serde_json::from_str(data).ok()?;
    if sidecar.version != VERSION {
        return None;
    }
    let text = sidecar.text;
    let suggestions = sidecar
        .suggestions
        .into_iter()
        .map(|record| {
            let valid = record.start < record.end
                && text.is_char_boundary(record.start)
                && text.is_char_boundary(record.end);
            valid.then(|| Suggestion {
                kind: match record.kind {
                    Kind::Insertion => SuggestionKind::Insertion,
                    Kind::Deletion => SuggestionKind::Deletion,
                },
                range: Position::new(record.start)..Position::new(record.end),
                author: record.author,
                time: record.time,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some((text, suggestions))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use presswork_text::Position;

    use super::*;

    fn suggestion(kind: SuggestionKind, range: std::ops::Range<usize>) -> Suggestion {
        Suggestion {
            kind,
            range: Position::new(range.start)..Position::new(range.end),
            author: "Ann".to_owned(),
            time: 1_763_700_000,
        }
    }

    const TEXT: &str = "The catdog sat.";

    fn replaced() -> Vec<Suggestion> {
        vec![
            suggestion(SuggestionKind::Deletion, 4..7),
            suggestion(SuggestionKind::Insertion, 7..10),
        ]
    }

    #[test]
    fn the_file_text_has_no_open_insertions_and_keeps_the_deleted_text() {
        assert_eq!(file_text(TEXT, &replaced()), "The cat sat.");
        assert_eq!(file_text(TEXT, &[]), TEXT);
    }

    #[test]
    fn saved_suggestions_come_back_for_the_same_file() {
        let folder = tempfile::tempdir().unwrap();
        let file = Path::new("chapters/one.typ");

        save(folder.path(), file, TEXT, &replaced()).unwrap();

        assert_eq!(
            path(folder.path(), file),
            folder
                .path()
                .join(".presswork/suggestions/chapters/one.typ.json")
        );
        assert_eq!(
            load(folder.path(), file, "The cat sat.").unwrap(),
            Loaded::Restored {
                text: TEXT.to_owned(),
                suggestions: replaced()
            }
        );
    }

    #[test]
    fn suggestions_for_a_changed_file_move_away() {
        let folder = tempfile::tempdir().unwrap();
        let file = Path::new("main.typ");
        save(folder.path(), file, TEXT, &replaced()).unwrap();

        let loaded = load(folder.path(), file, "A changed text.").unwrap();

        let moved = folder
            .path()
            .join(".presswork/suggestions/main.typ.json.old");
        assert_eq!(loaded, Loaded::Moved(moved.clone()));
        assert!(moved.is_file());
        assert!(!path(folder.path(), file).exists());
    }

    #[test]
    fn a_save_without_suggestions_removes_the_file() {
        let folder = tempfile::tempdir().unwrap();
        let file = Path::new("main.typ");
        save(folder.path(), file, TEXT, &replaced()).unwrap();

        save(folder.path(), file, "The dog sat.", &[]).unwrap();

        assert!(!path(folder.path(), file).exists());
        assert_eq!(
            load(folder.path(), file, "The dog sat.").unwrap(),
            Loaded::Nothing
        );
        assert!(fs::read_dir(folder.path()).is_ok());
    }
}
