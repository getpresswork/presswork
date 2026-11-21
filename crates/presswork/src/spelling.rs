use std::path::Path;
use std::sync::Arc;

use futures::channel::oneshot;
use iced::Task;
use presswork_editor::{Mark, MarkKind};
use presswork_language::{Dictionaries, Dictionary, WordList};
use presswork_text::Position;
use presswork_typst::{Analysis, SourceRange, Spelling};

pub(crate) const WORD_LIST: &str = "presswork-words.txt";

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Loaded(Arc<Dictionaries>),
    Checked {
        revision: u64,
        spelling: Arc<Spelling>,
    },
}

#[derive(Debug, Default)]
pub(crate) struct SpellChecker {
    dictionaries: Arc<Dictionaries>,
    words: WordList,
    checked: Option<(u64, Arc<Spelling>)>,
    running: bool,
    queued: Option<(Analysis, u64)>,
}

impl SpellChecker {
    pub(crate) fn set_words(&mut self, words: WordList) {
        self.words = words;
    }

    pub(crate) fn check(&mut self, analysis: Analysis, revision: u64) -> Task<Message> {
        if self.dictionaries.is_empty() {
            return Task::none();
        }
        if self.running {
            self.queued = Some((analysis, revision));
            return Task::none();
        }
        self.running = true;
        let dictionaries = Arc::clone(&self.dictionaries);
        Task::perform(
            background(move || analysis.spelling(&dictionaries)),
            move |spelling| Message::Checked {
                revision,
                spelling: Arc::new(spelling.unwrap_or_default()),
            },
        )
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(dictionaries) => {
                self.dictionaries = dictionaries;
                self.checked = None;
                Task::none()
            }
            Message::Checked { revision, spelling } => {
                self.running = false;
                self.checked = Some((revision, spelling));
                match self.queued.take() {
                    Some((analysis, revision)) => self.check(analysis, revision),
                    None => Task::none(),
                }
            }
        }
    }

    pub(crate) fn marks(&self, revision: u64) -> Vec<Mark> {
        let Some((checked, spelling)) = &self.checked else {
            return Vec::new();
        };
        if *checked != revision {
            return Vec::new();
        }
        spelling
            .misspellings
            .iter()
            .filter(|misspelling| !self.words.contains(&misspelling.word))
            .filter_map(|misspelling| match &misspelling.source {
                SourceRange::Main(range) => Some(Mark {
                    range: Position::new(range.start)..Position::new(range.end),
                    kind: MarkKind::Spelling,
                }),
                SourceRange::File(..) => None,
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn check_now(&mut self, analysis: &Analysis, revision: u64) {
        let spelling = analysis.spelling(&self.dictionaries);
        let _ = self.update(Message::Checked {
            revision,
            spelling: Arc::new(spelling),
        });
    }
}

pub(crate) fn load(root: Option<&Path>) -> Dictionaries {
    let mut dictionaries = Dictionaries::default();
    let (Some(root), Ok(catalog)) = (root, presswork_toolchain::catalog()) else {
        return dictionaries;
    };
    let installed = presswork_toolchain::Dictionaries::new(root, &catalog);
    for source in &catalog.sources {
        let Some((aff, dic)) = installed.read(&source.tag) else {
            continue;
        };
        match Dictionary::new(&aff, &dic) {
            Ok(dictionary) => dictionaries.insert(&source.tag, dictionary),
            Err(error) => {
                tracing::warn!(%error, tag = %source.tag, "The dictionary did not load");
            }
        }
    }
    dictionaries
}

pub(crate) fn load_task(root: Option<std::path::PathBuf>) -> Task<Message> {
    Task::perform(background(move || load(root.as_deref())), |dictionaries| {
        Message::Loaded(Arc::new(dictionaries.unwrap_or_default()))
    })
}

async fn background<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (sender, receiver) = oneshot::channel();
    let spawned = std::thread::Builder::new()
        .name("spelling".to_owned())
        .spawn(move || {
            let _ = sender.send(work());
        });
    if let Err(error) = spawned {
        tracing::error!(%error, "The spelling thread did not start");
    }
    receiver.await.ok()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use presswork_typst::{Datetime, Fonts, World, analyze};

    use super::*;

    fn english() -> Arc<Dictionaries> {
        let mut dictionaries = Dictionaries::default();
        dictionaries.insert(
            "en-US",
            Dictionary::new(b"SET UTF-8\n", b"4\nthe\ncat\nsat\non\n").unwrap(),
        );
        Arc::new(dictionaries)
    }

    fn analysis(text: &str) -> Analysis {
        let world = Mutex::new(World::new(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 20),
        ));
        analyze(&world, text).analysis.unwrap()
    }

    fn checker(words: &str) -> SpellChecker {
        let mut checker = SpellChecker::default();
        let _ = checker.update(Message::Loaded(english()));
        checker.set_words(WordList::parse(words));
        checker
    }

    #[test]
    fn a_misspelled_word_of_the_checked_text_has_a_mark() {
        let mut checker = checker("");

        checker.check_now(&analysis("The cat sat on the mta."), 7);

        assert_eq!(
            checker.marks(7),
            [Mark {
                range: Position::new(19)..Position::new(22),
                kind: MarkKind::Spelling,
            }]
        );
        assert_eq!(checker.marks(8), []);
    }

    #[test]
    fn a_word_of_the_project_list_has_no_mark() {
        let mut checker = checker("mta\n");

        checker.check_now(&analysis("The cat sat on the mta."), 7);

        assert_eq!(checker.marks(7), []);
    }

    #[test]
    fn installed_dictionaries_load_in_the_order_of_the_catalog() {
        let root = tempfile::tempdir().unwrap();
        for (tag, word) in [("en-GB", "colour"), ("en-US", "color")] {
            let folder = root.path().join(tag);
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join("dictionary.aff"), "SET UTF-8\n").unwrap();
            std::fs::write(folder.join("dictionary.dic"), format!("1\n{word}\n")).unwrap();
        }

        let dictionaries = load(Some(root.path()));

        assert!(dictionaries.find("en", None).unwrap().check("color"));
        assert!(dictionaries.find("en", Some("GB")).unwrap().check("colour"));
    }
}
