#[derive(Debug, thiserror::Error)]
pub enum LanguageError {
    #[error("the dictionary encoding {0} is not known")]
    Encoding(String),
    #[error("the dictionary is not valid: {0}")]
    Parse(String),
}

const MICROSOFT: &str = "microsoft-";

pub struct Dictionary {
    inner: spellbook::Dictionary,
}

impl Dictionary {
    pub fn new(aff: &[u8], dic: &[u8]) -> Result<Self, LanguageError> {
        let encoding = encoding(aff)?;
        let (aff, _, _) = encoding.decode(aff);
        let (dic, _, _) = encoding.decode(dic);
        let inner = spellbook::Dictionary::new(&aff, &dic)
            .map_err(|error| LanguageError::Parse(error.to_string()))?;
        Ok(Self { inner })
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
