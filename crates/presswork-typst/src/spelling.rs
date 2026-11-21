use presswork_language::{Dictionaries, words};

use crate::ide::Analysis;
use crate::outline::SourceRange;

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
        let mut spelling = Spelling::default();
        for prose in self.prose() {
            for run in &prose.runs {
                let tag = run.language.tag();
                let Some(dictionary) =
                    dictionaries.find(&run.language.language, run.language.region.as_deref())
                else {
                    if !spelling.missing.contains(&tag) {
                        spelling.missing.push(tag);
                    }
                    continue;
                };
                for (offset, word) in words(prose.text(run)) {
                    if dictionary.check(word) {
                        continue;
                    }
                    let Some(source) = prose.place(run, offset..offset + word.len()) else {
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
