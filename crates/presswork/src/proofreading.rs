use std::fs;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use futures::channel::oneshot;
use iced::Task;
use presswork_editor::{Mark, MarkKind};
use presswork_language::{Dictionaries, Dictionary, HouseStyle, RuleProblem, StyleError, WordList};
use presswork_project::{HOUSE_STYLE, WORD_LIST};
use presswork_text::Position;
use presswork_typst::{Analysis, Misspelling, SourceRange, Spelling, StyleProblem};

use crate::chrome::shortcut;
use crate::i18n::{FluentArgs, Localizer};
use crate::toolchain_panel::sentence;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Loaded(Arc<Dictionaries>),
    Checked {
        revision: u64,
        findings: Arc<Findings>,
    },
}

#[derive(Debug, Default)]
pub(crate) struct Findings {
    spelling: Spelling,
    style: Vec<StyleProblem>,
    house_style: Arc<HouseStyle>,
}

enum Finding<'a> {
    Spelling(&'a Misspelling),
    Style(&'a StyleProblem),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Correction {
    Replacement {
        range: Range<usize>,
        text: String,
    },
    Spelling {
        range: Range<usize>,
        word: String,
        language: String,
    },
}

#[derive(Debug, Default)]
pub(crate) struct Proofreader {
    dictionaries: Arc<Dictionaries>,
    words: WordList,
    words_text: String,
    style: Arc<HouseStyle>,
    style_text: String,
    checked: Option<(u64, Arc<Findings>)>,
    running: bool,
    queued: Option<(Analysis, u64)>,
}

impl Proofreader {
    pub(crate) fn read_project(&mut self, root: &Path) -> Option<StyleError> {
        let words = fs::read_to_string(root.join(WORD_LIST)).unwrap_or_default();
        if words != self.words_text {
            self.words = WordList::parse(&words);
            self.words_text = words;
        }
        let style = fs::read_to_string(root.join(HOUSE_STYLE)).unwrap_or_default();
        if style == self.style_text {
            return None;
        }
        self.checked = None;
        let parsed = HouseStyle::parse(&style);
        self.style_text = style;
        match parsed {
            Ok(style) => {
                self.style = Arc::new(style);
                None
            }
            Err(error) => {
                self.style = Arc::default();
                Some(error)
            }
        }
    }

    pub(crate) fn add_word(&mut self, root: &Path, word: &str) -> std::io::Result<()> {
        let path = root.join(WORD_LIST);
        let mut text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error),
        };
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        if !text.is_empty() && !text.ends_with('\n') {
            text.push_str(newline);
        }
        text.push_str(word);
        text.push_str(newline);
        fs::write(&path, &text)?;
        self.words = WordList::parse(&text);
        self.words_text = text;
        Ok(())
    }

    pub(crate) fn close_project(&mut self) {
        self.words = WordList::default();
        self.words_text.clear();
        self.style = Arc::default();
        self.style_text.clear();
        self.checked = None;
    }

    pub(crate) fn check(&mut self, analysis: Analysis, revision: u64) -> Task<Message> {
        if self.dictionaries.is_empty() && self.style.is_empty() {
            return Task::none();
        }
        if self.running {
            self.queued = Some((analysis, revision));
            return Task::none();
        }
        self.running = true;
        let dictionaries = Arc::clone(&self.dictionaries);
        let style = Arc::clone(&self.style);
        Task::perform(
            background(move || findings(&analysis, &dictionaries, style)),
            move |findings| Message::Checked {
                revision,
                findings: Arc::new(findings.unwrap_or_default()),
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
            Message::Checked { revision, findings } => {
                self.running = false;
                self.checked = Some((revision, findings));
                match self.queued.take() {
                    Some((analysis, revision)) => self.check(analysis, revision),
                    None => Task::none(),
                }
            }
        }
    }

    pub(crate) fn marks(&self, revision: u64) -> Vec<Mark> {
        self.found(revision)
            .map(|(range, finding)| Mark {
                range: Position::new(range.start)..Position::new(range.end),
                kind: match finding {
                    Finding::Spelling(_) => MarkKind::Spelling,
                    Finding::Style(_) => MarkKind::Style,
                },
            })
            .collect()
    }

    pub(crate) fn tooltip(
        &self,
        localizer: &Localizer,
        revision: u64,
        offset: usize,
    ) -> Option<String> {
        let (_, findings) = self
            .checked
            .as_ref()
            .filter(|(checked, _)| *checked == revision)?;
        let texts: Vec<String> = self
            .found(revision)
            .filter(|(range, _)| range.contains(&offset))
            .map(|(_, finding)| finding_text(localizer, findings, &finding))
            .collect();
        (!texts.is_empty()).then(|| texts.join("\n"))
    }

    pub(crate) fn correction_at(&self, revision: u64, offset: usize) -> Option<Correction> {
        self.found(revision)
            .filter(|(range, _)| range.start <= offset && offset <= range.end)
            .find_map(|(range, finding)| match finding {
                Finding::Spelling(misspelling) => Some(Correction::Spelling {
                    range,
                    word: misspelling.word.clone(),
                    language: misspelling.language.clone(),
                }),
                Finding::Style(problem) => Some(Correction::Replacement {
                    range,
                    text: problem.replacement.clone()?,
                }),
            })
    }

    pub(crate) fn dictionaries(&self) -> Arc<Dictionaries> {
        Arc::clone(&self.dictionaries)
    }

    fn found(&self, revision: u64) -> impl Iterator<Item = (Range<usize>, Finding<'_>)> {
        let findings = self
            .checked
            .as_ref()
            .filter(|(checked, _)| *checked == revision)
            .map(|(_, findings)| findings.as_ref());
        let misspellings = findings
            .into_iter()
            .flat_map(|findings| &findings.spelling.misspellings)
            .filter(|misspelling| !self.words.contains(&misspelling.word))
            .filter_map(|misspelling| {
                Some((
                    main_range(&misspelling.source)?,
                    Finding::Spelling(misspelling),
                ))
            });
        let style = findings
            .into_iter()
            .flat_map(|findings| &findings.style)
            .filter_map(|problem| Some((main_range(&problem.source)?, Finding::Style(problem))));
        misspellings.chain(style)
    }

    #[cfg(test)]
    pub(crate) fn check_now(&mut self, analysis: &Analysis, revision: u64) {
        let findings = findings(analysis, &self.dictionaries, Arc::clone(&self.style));
        let _ = self.update(Message::Checked {
            revision,
            findings: Arc::new(findings),
        });
    }
}

pub(crate) async fn suggestions(
    dictionaries: Arc<Dictionaries>,
    word: String,
    language: String,
) -> Vec<String> {
    background(move || {
        let (language, region) = match language.split_once('-') {
            Some((language, region)) => (language, Some(region)),
            None => (language.as_str(), None),
        };
        dictionaries
            .find(language, region)
            .map_or_default(|dictionary| dictionary.suggest(&word))
    })
    .await
    .unwrap_or_default()
}

fn findings(
    analysis: &Analysis,
    dictionaries: &Dictionaries,
    house_style: Arc<HouseStyle>,
) -> Findings {
    let spelling = if dictionaries.is_empty() {
        Spelling::default()
    } else {
        analysis.spelling(dictionaries)
    };
    Findings {
        spelling,
        style: analysis.style(&house_style),
        house_style,
    }
}

fn finding_text(localizer: &Localizer, findings: &Findings, finding: &Finding<'_>) -> String {
    let mut args = FluentArgs::new();
    args.set("shortcut", shortcut(localizer, '.'));
    match finding {
        Finding::Spelling(misspelling) => {
            args.set("word", misspelling.word.clone());
            args.set("language", misspelling.language.clone());
            format!(
                "{}\n{}",
                localizer.format("proofreading-spelling", &args),
                localizer.format("proofreading-suggestions", &args)
            )
        }
        Finding::Style(problem) => {
            args.set("text", problem.text.clone());
            let message = findings.house_style.message(problem.rule).map_or_else(
                || localizer.format("proofreading-style", &args),
                str::to_owned,
            );
            match &problem.replacement {
                Some(replacement) => {
                    args.set("replacement", replacement.clone());
                    let hint = localizer.format("proofreading-replacement", &args);
                    format!("{message}\n{hint}")
                }
                None => message,
            }
        }
    }
}

fn main_range(source: &SourceRange) -> Option<Range<usize>> {
    match source {
        SourceRange::Main(range) => Some(range.clone()),
        SourceRange::File(..) => None,
    }
}

pub(crate) fn style_error_text(localizer: &Localizer, error: &StyleError) -> String {
    let mut args = FluentArgs::new();
    args.set("file", HOUSE_STYLE);
    let id = match error {
        StyleError::File(problem) => {
            args.set("problem", sentence(problem));
            "style-error-file"
        }
        StyleError::Rule { rule, problem } => {
            args.set("rule", *rule);
            match problem {
                RuleProblem::Missing => "style-error-missing",
                RuleProblem::Both => "style-error-both",
                RuleProblem::Empty => "style-error-empty",
                RuleProblem::Pattern(problem) => {
                    args.set("problem", problem.clone());
                    "style-error-pattern"
                }
            }
        }
    };
    localizer.format(id, &args)
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
        .name("proofreading".to_owned())
        .spawn(move || {
            let _ = sender.send(work());
        });
    if let Err(error) = spawned {
        tracing::error!(%error, "The proofreading thread did not start");
    }
    receiver.await.ok()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use presswork_typst::{Datetime, Fonts, World, analyze};

    use super::*;

    const TEXT: &str = "The cat sat on the mta.";

    fn english() -> Arc<Dictionaries> {
        let mut dictionaries = Dictionaries::default();
        dictionaries.insert(
            "en-US",
            Dictionary::new(b"SET UTF-8\n", b"5\nthe\ncat\nsat\non\nmat\n").unwrap(),
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

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let folder = tempfile::tempdir().unwrap();
        for (path, text) in files {
            fs::write(folder.path().join(path), text).unwrap();
        }
        folder
    }

    fn proofreader(folder: &tempfile::TempDir, dictionaries: Arc<Dictionaries>) -> Proofreader {
        let mut proofreader = Proofreader::default();
        let _ = proofreader.update(Message::Loaded(dictionaries));
        assert_eq!(proofreader.read_project(folder.path()), None);
        proofreader
    }

    fn mark(range: Range<usize>, kind: MarkKind) -> Mark {
        Mark {
            range: Position::new(range.start)..Position::new(range.end),
            kind,
        }
    }

    #[test]
    fn a_misspelled_word_of_the_checked_text_has_a_mark_and_a_tooltip() {
        let folder = project(&[]);
        let mut proofreader = proofreader(&folder, english());
        let localizer = Localizer::english();

        proofreader.check_now(&analysis(TEXT), 7);

        assert_eq!(proofreader.marks(7), [mark(19..22, MarkKind::Spelling)]);
        assert_eq!(proofreader.marks(8), []);
        assert_eq!(
            proofreader.tooltip(&localizer, 7, 21),
            Some(format!(
                "The word “mta” is not in the dictionary for en.\n\
                 Push {} to show the corrections.",
                shortcut(&localizer, '.')
            ))
        );
        assert_eq!(proofreader.tooltip(&localizer, 7, 22), None);
        assert_eq!(proofreader.tooltip(&localizer, 8, 21), None);
    }

    #[test]
    fn the_correction_at_the_cursor_is_a_replacement_or_a_misspelled_word() {
        let styled = project(&[(HOUSE_STYLE, STYLE)]);
        let mut style = proofreader(&styled, Arc::default());
        style.check_now(&analysis(TEXT), 7);
        let plain = project(&[]);
        let mut spelling = proofreader(&plain, english());
        spelling.check_now(&analysis(TEXT), 7);

        let replacement = Some(Correction::Replacement {
            range: 19..22,
            text: "mat".to_owned(),
        });
        assert_eq!(style.correction_at(7, 20), replacement);
        assert_eq!(style.correction_at(7, 22), replacement);
        assert_eq!(style.correction_at(7, 5), None);
        assert_eq!(style.correction_at(8, 20), None);
        assert_eq!(
            spelling.correction_at(7, 19),
            Some(Correction::Spelling {
                range: 19..22,
                word: "mta".to_owned(),
                language: "en".to_owned(),
            })
        );
    }

    #[test]
    fn suggestions_come_from_the_dictionary_of_the_language() {
        let found = |language: &str| {
            futures::executor::block_on(suggestions(
                english(),
                "mta".to_owned(),
                language.to_owned(),
            ))
        };

        assert!(found("en").contains(&"mat".to_owned()), "{:?}", found("en"));
        assert!(found("en-US").contains(&"mat".to_owned()));
        assert_eq!(found("de"), Vec::<String>::new());
    }

    #[test]
    fn an_added_word_goes_into_the_list_file_and_loses_its_mark() {
        let folder = project(&[]);
        let mut proofreader = proofreader(&folder, english());
        proofreader.check_now(&analysis(TEXT), 7);

        proofreader.add_word(folder.path(), "mta").unwrap();

        assert_eq!(
            fs::read_to_string(folder.path().join(WORD_LIST)).unwrap(),
            "mta\n"
        );
        assert_eq!(proofreader.marks(7), []);
        assert_eq!(proofreader.read_project(folder.path()), None);
    }

    #[test]
    fn an_added_word_keeps_the_lines_of_the_list_file() {
        let folder = project(&[(WORD_LIST, "# Names\r\nTypst")]);
        let mut proofreader = proofreader(&folder, english());

        proofreader.add_word(folder.path(), "mta").unwrap();

        assert_eq!(
            fs::read_to_string(folder.path().join(WORD_LIST)).unwrap(),
            "# Names\r\nTypst\r\nmta\r\n"
        );
    }

    #[test]
    fn a_word_of_the_project_list_has_no_mark() {
        let folder = project(&[(WORD_LIST, "mta\n")]);
        let mut proofreader = proofreader(&folder, english());

        proofreader.check_now(&analysis(TEXT), 7);

        assert_eq!(proofreader.marks(7), []);
    }

    const STYLE: &str = "[[rule]]\ntext = \"cat\"\nmessage = \"Name the animal.\"\n\n\
        [[rule]]\ntext = \"mta\"\nreplacement = \"mat\"\n";

    #[test]
    fn house_style_problems_have_marks_and_tooltips_also_without_dictionaries() {
        let folder = project(&[(HOUSE_STYLE, STYLE)]);
        let mut proofreader = proofreader(&folder, Arc::default());
        let localizer = Localizer::english();

        proofreader.check_now(&analysis(TEXT), 7);

        assert_eq!(
            proofreader.marks(7),
            [mark(4..7, MarkKind::Style), mark(19..22, MarkKind::Style)]
        );
        assert_eq!(
            proofreader.tooltip(&localizer, 7, 4).as_deref(),
            Some("Name the animal.")
        );
        assert_eq!(
            proofreader.tooltip(&localizer, 7, 19),
            Some(format!(
                "The house style does not permit “mta”.\n\
                 Push {} to change the text to “mat”.",
                shortcut(&localizer, '.')
            ))
        );
    }

    #[test]
    fn the_house_style_file_is_read_again_when_it_changes() {
        let folder = project(&[(HOUSE_STYLE, "[[rule]]\ntext = \"cat\"\n")]);
        let mut proofreader = proofreader(&folder, Arc::default());
        let file = folder.path().join(HOUSE_STYLE);

        fs::write(&file, "[[rule]]\ntext = \"sat\"\n").unwrap();
        assert_eq!(proofreader.read_project(folder.path()), None);
        proofreader.check_now(&analysis(TEXT), 7);
        assert_eq!(proofreader.marks(7), [mark(8..11, MarkKind::Style)]);

        fs::write(&file, "[[rule]]\nmessage = \"No.\"\n").unwrap();
        assert_eq!(
            proofreader.read_project(folder.path()),
            Some(StyleError::Rule {
                rule: 1,
                problem: RuleProblem::Missing
            })
        );
        assert_eq!(proofreader.read_project(folder.path()), None);
        assert_eq!(proofreader.marks(7), []);
    }

    #[test]
    fn each_style_error_has_english_text() {
        let localizer = Localizer::english();
        let rule = |problem| StyleError::Rule { rule: 2, problem };
        let errors = [
            StyleError::File("expected `]]`".to_owned()),
            rule(RuleProblem::Missing),
            rule(RuleProblem::Both),
            rule(RuleProblem::Empty),
            rule(RuleProblem::Pattern("unclosed group".to_owned())),
        ];

        for error in &errors {
            let text = style_error_text(&localizer, error);
            assert!(text.contains("presswork-style.toml"), "{text}");
        }
        assert_eq!(
            style_error_text(&localizer, &errors[4]),
            "The pattern of rule 2 in presswork-style.toml is not valid: unclosed group."
        );
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
