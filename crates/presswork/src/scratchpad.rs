use std::fs;
use std::path::PathBuf;

use presswork_editor::Editor;

pub(crate) const SAMPLE: &str = include_str!("../samples/scratch.typ");
const FILE: &str = "scratchpad.typ";

#[derive(Debug, Default)]
pub(crate) struct Scratchpad {
    file: Option<PathBuf>,
    kept: Option<(u64, u64)>,
}

impl Scratchpad {
    pub(crate) fn new(file: Option<PathBuf>) -> Self {
        Self { file, kept: None }
    }

    pub(crate) fn text(&self) -> String {
        self.file
            .as_deref()
            .and_then(|file| fs::read_to_string(file).ok())
            .unwrap_or_else(|| SAMPLE.to_owned())
    }

    pub(crate) fn keep(&mut self, editor: &Editor) {
        let Some(file) = &self.file else {
            return;
        };
        let mark = (editor.id(), editor.revision());
        if self.kept == Some(mark) {
            return;
        }
        self.kept = Some(mark);
        let written = file
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(file, editor.document().buffer().to_string()));
        if let Err(error) = written {
            tracing::warn!(%error, path = %file.display(), "The scratchpad was not saved");
        }
    }
}

pub(crate) fn scratchpad_file() -> Option<PathBuf> {
    directories::ProjectDirs::from("io.github", "getpresswork", "Presswork")
        .map(|folders| folders.data_dir().join(FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_scratchpad_shows_the_sample() {
        let folder = tempfile::tempdir().unwrap();

        let scratchpad = Scratchpad::new(Some(folder.path().join(FILE)));

        assert_eq!(scratchpad.text(), SAMPLE);
    }

    #[test]
    fn the_scratchpad_keeps_the_text_of_the_editor() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("data").join(FILE);
        let mut scratchpad = Scratchpad::new(Some(file.clone()));

        scratchpad.keep(&Editor::new("= Idea"));

        assert_eq!(fs::read_to_string(&file).unwrap(), "= Idea");
        assert_eq!(Scratchpad::new(Some(file)).text(), "= Idea");
    }

    #[test]
    fn an_editor_without_changes_is_not_written_again() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join(FILE);
        let mut scratchpad = Scratchpad::new(Some(file.clone()));
        let editor = Editor::new("= Idea");
        scratchpad.keep(&editor);
        fs::remove_file(&file).unwrap();

        scratchpad.keep(&editor);

        assert!(!file.exists());
    }

    #[test]
    fn without_a_file_nothing_is_written() {
        let mut scratchpad = Scratchpad::default();

        scratchpad.keep(&Editor::new("= Idea"));

        assert_eq!(scratchpad.text(), SAMPLE);
    }
}
