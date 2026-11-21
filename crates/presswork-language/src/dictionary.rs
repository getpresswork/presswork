#[derive(Debug, thiserror::Error)]
pub enum LanguageError {
    #[error("the dictionary encoding {0} is not known")]
    Encoding(String),
    #[error("the dictionary is not valid: {0}")]
    Parse(String),
}

use spellbook::ParseDictionaryErrorSource;

const MICROSOFT: &str = "microsoft-";
const MOST_SKIPPED_LINES: usize = 50;

fn without_line(text: &str, number: usize) -> String {
    text.split_inclusive('\n')
        .enumerate()
        .filter(|(index, _)| index + 1 != number)
        .map(|(_, line)| line)
        .collect()
}

pub struct Dictionary {
    inner: spellbook::Dictionary,
}

impl Dictionary {
    pub fn new(aff: &[u8], dic: &[u8]) -> Result<Self, LanguageError> {
        let encoding = encoding(aff)?;
        let (aff, _, _) = encoding.decode(aff);
        let mut dic = encoding.decode(dic).0.into_owned();
        let mut skipped = 0;
        loop {
            match spellbook::Dictionary::new(&aff, &dic) {
                Ok(inner) => return Ok(Self { inner }),
                Err(error)
                    if skipped < MOST_SKIPPED_LINES
                        && error.source == ParseDictionaryErrorSource::Dic =>
                {
                    let Some(line) = error.line_number else {
                        return Err(LanguageError::Parse(error.to_string()));
                    };
                    dic = without_line(&dic, line);
                    skipped += 1;
                }
                Err(error) => return Err(LanguageError::Parse(error.to_string())),
            }
        }
    }

    #[must_use]
    pub fn check(&self, word: &str) -> bool {
        self.inner.check(word)
    }

    #[must_use]
    pub fn suggest(&self, word: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        self.inner.suggest(word, &mut suggestions);
        suggestions
    }

    pub fn add(&mut self, word: &str) {
        if !word.contains('/') {
            let _ = self.inner.add(word);
        }
    }
}

fn encoding(aff: &[u8]) -> Result<&'static encoding_rs::Encoding, LanguageError> {
    let Some(label) = aff
        .split(|byte| *byte == b'\n')
        .find_map(|line| line.strip_prefix(b"SET "))
    else {
        return Ok(encoding_rs::WINDOWS_1252);
    };
    let label = String::from_utf8_lossy(label).trim().to_owned();
    let known = |label: &str| encoding_rs::Encoding::for_label(label.as_bytes());
    known(&label)
        .or_else(|| {
            label
                .to_ascii_lowercase()
                .strip_prefix(MICROSOFT)
                .and_then(known)
        })
        .ok_or(LanguageError::Encoding(label))
}

#[derive(Default)]
pub struct Dictionaries {
    entries: Vec<(String, Dictionary)>,
}

impl Dictionaries {
    pub fn insert(&mut self, tag: &str, dictionary: Dictionary) {
        match self
            .entries
            .iter_mut()
            .find(|(known, _)| known.eq_ignore_ascii_case(tag))
        {
            Some(entry) => entry.1 = dictionary,
            None => self.entries.push((tag.to_owned(), dictionary)),
        }
    }

    #[must_use]
    pub fn find(&self, language: &str, region: Option<&str>) -> Option<&Dictionary> {
        let exact = region.and_then(|region| {
            self.entries.iter().find(|(tag, _)| {
                tag.split_once('-').is_some_and(|(first, second)| {
                    first.eq_ignore_ascii_case(language) && second.eq_ignore_ascii_case(region)
                })
            })
        });
        exact
            .or_else(|| {
                self.entries.iter().find(|(tag, _)| {
                    tag.split('-')
                        .next()
                        .is_some_and(|first| first.eq_ignore_ascii_case(language))
                })
            })
            .map(|(_, dictionary)| dictionary)
    }

    pub fn add(&mut self, word: &str) {
        for (_, dictionary) in &mut self.entries {
            dictionary.add(word);
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AFF: &str = "SET UTF-8\nTRY aeiostcdgr\n\nSFX S Y 1\nSFX S 0 s .\n";
    const DIC: &str = "3\ncat/S\ndog/S\nStraße\n";

    fn english() -> Dictionary {
        Dictionary::new(AFF.as_bytes(), DIC.as_bytes()).unwrap()
    }

    #[test]
    fn a_dictionary_knows_its_words_and_their_forms() {
        let dictionary = english();

        assert!(dictionary.check("cat"));
        assert!(dictionary.check("cats"));
        assert!(dictionary.check("Straße"));
        assert!(!dictionary.check("caat"));
    }

    #[test]
    #[ignore = "set PRESSWORK_TEST_DICTIONARIES to a folder with installed dictionaries"]
    fn the_libreoffice_dictionaries_know_common_words() {
        let Some(folder) = std::env::var_os("PRESSWORK_TEST_DICTIONARIES") else {
            return;
        };
        let folder = std::path::PathBuf::from(folder);
        for (tag, words) in [
            ("en-US", ["house", "colors"]),
            ("en-GB", ["house", "colours"]),
            ("de-DE", ["Straße", "Häuser"]),
            ("de-AT", ["Jänner", "Häuser"]),
            ("de-CH", ["Strasse", "Häuser"]),
            ("fr-FR", ["maison", "être"]),
            ("es-ES", ["niño", "casa"]),
            ("it-IT", ["città", "casa"]),
            ("nl-NL", ["huis", "fiets"]),
            ("pt-BR", ["ação", "casa"]),
            ("pt-PT", ["ação", "casa"]),
            ("pl-PL", ["dziękuję", "źdźbło"]),
            ("ru-RU", ["дом", "спасибо"]),
            ("da-DK", ["hus", "tak"]),
        ] {
            let read = |name: &str| std::fs::read(folder.join(tag).join(name)).unwrap();
            let dictionary = Dictionary::new(&read("dictionary.aff"), &read("dictionary.dic"))
                .unwrap_or_else(|error| panic!("{tag}: {error}"));
            for word in words {
                assert!(dictionary.check(word), "{tag}: {word}");
            }
            assert!(!dictionary.check("qxwzvtk"), "{tag}");
        }
    }

    #[test]
    fn a_dictionary_suggests_words() {
        assert!(english().suggest("caat").contains(&"cat".to_owned()));
    }

    #[test]
    fn a_dictionary_in_latin_1_is_read_correctly() {
        let mut dic = b"1\nStra".to_vec();
        dic.extend([0xDF]);
        dic.extend(b"e\n");

        let dictionary = Dictionary::new(b"SET ISO8859-1\r\n", &dic).unwrap();

        assert!(dictionary.check("Straße"));
    }

    #[test]
    fn a_word_line_that_does_not_parse_is_skipped_as_in_hunspell() {
        let dictionary =
            Dictionary::new(b"SET UTF-8\nFLAG num\n", b"3\nhouse\n\"A/S\"\ncat\n").unwrap();

        assert!(dictionary.check("house"));
        assert!(dictionary.check("cat"));
    }

    #[test]
    fn an_unknown_encoding_is_an_error() {
        let result = Dictionary::new(b"SET NO-SUCH-ENCODING\n", b"0\n");

        assert!(
            matches!(result, Err(LanguageError::Encoding(label)) if label == "NO-SUCH-ENCODING")
        );
    }

    #[test]
    fn added_words_are_correct() {
        let mut dictionary = english();

        dictionary.add("Presswork");

        assert!(dictionary.check("Presswork"));
    }

    #[test]
    fn the_region_selects_a_dictionary_and_the_language_is_the_fallback() {
        let mut dictionaries = Dictionaries::default();
        dictionaries.insert("en-US", english());
        dictionaries.insert(
            "en-GB",
            Dictionary::new(AFF.as_bytes(), b"1\ncolour\n").unwrap(),
        );

        let british = dictionaries.find("en", Some("GB")).unwrap();
        assert!(british.check("colour"));
        assert!(dictionaries.find("en", Some("AU")).unwrap().check("cat"));
        assert!(dictionaries.find("en", None).unwrap().check("cat"));
        assert!(dictionaries.find("de", None).is_none());
    }
}
