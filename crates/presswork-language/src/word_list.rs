use std::collections::HashSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WordList {
    words: HashSet<String>,
}

impl WordList {
    #[must_use]
    pub fn parse(text: &str) -> Self {
        Self {
            words: text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_owned)
                .collect(),
        }
    }

    #[must_use]
    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(word) || self.words.contains(&word.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_list_has_one_word_on_each_line() {
        let words = WordList::parse("# Names of the project\nPresswork\n\n  typst  \n");

        assert!(words.contains("Presswork"));
        assert!(words.contains("typst"));
        assert!(!words.contains("Names"));
        assert!(!words.contains("pressworks"));
    }

    #[test]
    fn a_word_in_small_letters_also_permits_capitals() {
        let words = WordList::parse("typst\nPresswork\n");

        assert!(words.contains("Typst"));
        assert!(words.contains("TYPST"));
        assert!(!words.contains("presswork"));
    }
}
