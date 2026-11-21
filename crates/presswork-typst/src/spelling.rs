use std::collections::HashMap;
use std::ops::Range;

use presswork_language::{Dictionaries, words};
use typst::World as _;
use typst::layout::{Frame, FrameItem};
use typst::syntax::{FileId, LinkedNode, Source, Span, SyntaxKind};
use typst_layout::PagedDocument;

use crate::ide::{Analysis, project_path};
use crate::outline::SourceRange;

type Language = (String, Option<String>);

struct Run {
    range: Range<usize>,
    spans: Vec<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Misspelling {
    pub word: String,
    pub language: String,
    pub source: SourceRange,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spelling {
    pub misspellings: Vec<Misspelling>,
    pub missing: Vec<String>,
}

impl Analysis {
    #[must_use]
    pub fn spelling(&self, dictionaries: &Dictionaries) -> Spelling {
        let Some(document) = self.document() else {
            return Spelling::default();
        };
        let languages = languages(document);
        let main = self.world().main();
        let mut files: Vec<FileId> = languages.keys().filter_map(|span| span.id()).collect();
        files.sort_by_key(|id| (*id != main, project_path(*id)));
        files.dedup();
        let mut spelling = Spelling::default();
        for id in files {
            let Ok(source) = self.world().source(id) else {
                continue;
            };
            for run in runs(&source) {
                let Some((language, region)) =
                    run.spans.iter().find_map(|span| languages.get(span))
                else {
                    continue;
                };
                let tag = region
                    .as_ref()
                    .map_or_else(|| language.clone(), |region| format!("{language}-{region}"));
                let Some(dictionary) = dictionaries.find(language, region.as_deref()) else {
                    if !spelling.missing.contains(&tag) {
                        spelling.missing.push(tag);
                    }
                    continue;
                };
                let text = &source.text()[run.range.clone()];
                for (offset, word) in words(text) {
                    if dictionary.check(word) {
                        continue;
                    }
                    let start = run.range.start + offset;
                    let range = start..start + word.len();
                    let Some(source) = place(id, main, range) else {
                        continue;
                    };
                    spelling.misspellings.push(Misspelling {
                        word: word.to_owned(),
                        language: tag.clone(),
                        source,
                    });
                }
            }
        }
        spelling
    }
}

fn place(id: FileId, main: FileId, range: Range<usize>) -> Option<SourceRange> {
    if id == main {
        return Some(SourceRange::Main(range));
    }
    Some(SourceRange::File(project_path(id)?, range))
}

fn languages(document: &PagedDocument) -> HashMap<Span, Language> {
    let mut languages = HashMap::new();
    for page in document.pages() {
        collect_languages(&page.frame, &mut languages);
    }
    languages
}

fn collect_languages(frame: &Frame, languages: &mut HashMap<Span, Language>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => collect_languages(&group.frame, languages),
            FrameItem::Text(text) => {
                for glyph in &text.glyphs {
                    languages.entry(glyph.span.0).or_insert_with(|| {
                        (
                            text.lang.as_str().to_owned(),
                            text.region.map(|region| region.as_str().to_owned()),
                        )
                    });
                }
            }
            FrameItem::Shape(..)
            | FrameItem::Image(..)
            | FrameItem::Link(..)
            | FrameItem::Tag(_) => {}
        }
    }
}

fn runs(source: &Source) -> Vec<Run> {
    let mut runs = Vec::new();
    collect_runs(&LinkedNode::new(source.root()), &mut runs);
    runs
}

fn collect_runs(node: &LinkedNode<'_>, runs: &mut Vec<Run>) {
    if matches!(node.kind(), SyntaxKind::Raw | SyntaxKind::Equation) {
        return;
    }
    let markup = node.kind() == SyntaxKind::Markup;
    let mut current: Option<Run> = None;
    for child in node.children() {
        let prose = markup && matches!(child.kind(), SyntaxKind::Text | SyntaxKind::SmartQuote);
        if !prose {
            runs.extend(current.take());
            collect_runs(&child, runs);
            continue;
        }
        let range = child.range();
        let span = (child.kind() == SyntaxKind::Text).then(|| child.span());
        match &mut current {
            Some(run) if run.range.end == range.start => {
                run.range.end = range.end;
                run.spans.extend(span);
            }
            _ => {
                runs.extend(current.take());
                current = Some(Run {
                    range,
                    spans: span.into_iter().collect(),
                });
            }
        }
    }
    runs.extend(current);
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use presswork_language::Dictionary;
    use typst::foundations::Datetime;

    use super::*;
    use crate::fonts::Fonts;
    use crate::job::analyze;
    use crate::world::World;

    const AFF: &[u8] = b"SET UTF-8\nSFX S Y 1\nSFX S 0 s .\n";

    fn dictionaries(entries: &[(&str, &str)]) -> Dictionaries {
        let mut dictionaries = Dictionaries::default();
        for (tag, words) in entries {
            let dic = format!(
                "{}\n{}\n",
                words.split(' ').count(),
                words.replace(' ', "\n")
            );
            dictionaries.insert(tag, Dictionary::new(AFF, dic.as_bytes()).unwrap());
        }
        dictionaries
    }

    fn english() -> Dictionaries {
        dictionaries(&[("en-US", "the cat/S sat on mat can't color")])
    }

    fn spelling(folder: &Path, text: &str, dictionaries: &Dictionaries) -> Spelling {
        std::fs::write(folder.join("main.typ"), text).unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 20),
            folder.to_path_buf(),
            Path::new("main.typ"),
        ));
        analyze(&world, text)
            .analysis
            .unwrap()
            .spelling(dictionaries)
    }

    fn check(text: &str, dictionaries: &Dictionaries) -> Spelling {
        let folder = tempfile::tempdir().unwrap();
        spelling(folder.path(), text, dictionaries)
    }

    fn words(spelling: &Spelling) -> Vec<&str> {
        spelling
            .misspellings
            .iter()
            .map(|misspelling| misspelling.word.as_str())
            .collect()
    }

    #[test]
    fn a_misspelled_word_has_its_place_in_the_source() {
        let text = "The cats sat on the mta.";

        let found = check(text, &english());

        assert_eq!(
            found.misspellings,
            [Misspelling {
                word: "mta".to_owned(),
                language: "en".to_owned(),
                source: SourceRange::Main(20..23),
            }]
        );
        assert_eq!(&text[20..23], "mta");
    }

    #[test]
    fn a_contraction_is_one_word() {
        assert_eq!(
            words(&check("The cat can't sat.", &english())),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn code_math_and_raw_text_are_not_checked() {
        let found = check(
            "The cat #let xyzzy = 1 sat on `qwrt` the $abcde$ mat.\n#let unused = [qwerty]",
            &english(),
        );

        assert_eq!(words(&found), Vec::<&str>::new());
    }

    #[test]
    fn the_language_of_the_text_selects_the_dictionary() {
        let dictionaries =
            dictionaries(&[("en-US", "the cat/S color"), ("en-GB", "the cat/S colour")]);

        let british = check(
            "#set text(lang: \"en\", region: \"GB\")\nThe colour, the color.",
            &dictionaries,
        );
        let german = check("#set text(lang: \"de\")\nDas Haus.", &dictionaries);

        assert_eq!(words(&british), ["color"]);
        assert_eq!(british.misspellings[0].language, "en-GB");
        assert_eq!(words(&german), Vec::<&str>::new());
        assert_eq!(german.missing, ["de"]);
    }

    #[test]
    fn words_in_other_files_are_checked() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("chapter.typ"), "The mta.").unwrap();

        let found = spelling(folder.path(), "#include \"chapter.typ\"", &english());

        assert_eq!(
            found.misspellings[0].source,
            SourceRange::File(PathBuf::from("chapter.typ"), 4..7)
        );
    }
}
