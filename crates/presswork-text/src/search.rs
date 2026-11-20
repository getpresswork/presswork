use std::ops::Range;

use regex::{Regex, RegexBuilder};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub text: String,
    pub match_case: bool,
    pub whole_word: bool,
    pub pattern: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the search pattern is not valid: {0}")]
pub struct PatternError(String);

#[derive(Debug, Clone)]
pub struct Search {
    regex: Regex,
    whole_word: bool,
    pattern: bool,
}

impl Search {
    pub fn new(query: &Query) -> Result<Option<Self>, PatternError> {
        if query.text.is_empty() {
            return Ok(None);
        }
        let source = if query.pattern {
            query.text.clone()
        } else {
            regex::escape(&query.text)
        };
        let regex = RegexBuilder::new(&source)
            .case_insensitive(!query.match_case)
            .multi_line(true)
            .build()
            .map_err(|error| PatternError(error.to_string()))?;
        Ok(Some(Self {
            regex,
            whole_word: query.whole_word,
            pattern: query.pattern,
        }))
    }

    #[must_use]
    pub fn find(&self, text: &str) -> Vec<Range<usize>> {
        self.regex
            .find_iter(text)
            .map(|found| found.range())
            .filter(|range| !range.is_empty())
            .filter(|range| !self.whole_word || is_whole_word(text, range))
            .collect()
    }

    #[must_use]
    pub fn replacement(&self, text: &str, found: Range<usize>, with: &str) -> String {
        if !self.pattern {
            return with.to_owned();
        }
        match self.regex.captures_at(text, found.start) {
            Some(captures) if captures.get(0).is_some_and(|whole| whole.range() == found) => {
                let mut expanded = String::new();
                captures.expand(with, &mut expanded);
                expanded
            }
            _ => with.to_owned(),
        }
    }
}

fn is_whole_word(text: &str, range: &Range<usize>) -> bool {
    let before = text[..range.start].chars().next_back();
    let after = text[range.end..].chars().next();
    !before.is_some_and(is_word) && !after.is_some_and(is_word)
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Query {
        Query {
            text: text.to_owned(),
            ..Query::default()
        }
    }

    fn pattern(text: &str) -> Query {
        Query {
            pattern: true,
            ..words(text)
        }
    }

    fn whole(text: &str) -> Query {
        Query {
            whole_word: true,
            ..words(text)
        }
    }

    fn found(text: &str, query: &Query) -> Vec<(usize, usize)> {
        Search::new(query)
            .unwrap()
            .unwrap()
            .find(text)
            .into_iter()
            .map(|range| (range.start, range.end))
            .collect()
    }

    #[test]
    fn a_query_finds_each_place_of_its_text() {
        assert_eq!(found("a cat and a cat", &words("cat")), [(2, 5), (12, 15)]);
    }

    #[test]
    fn the_search_ignores_case_unless_you_ask() {
        assert_eq!(found("Cat cat CAT", &words("cat")).len(), 3);

        let exact = Query {
            match_case: true,
            ..words("cat")
        };
        assert_eq!(found("Cat cat CAT", &exact), [(4, 7)]);
    }

    #[test]
    fn whole_word_skips_text_inside_words() {
        assert_eq!(found("cat catalog cat.", &whole("cat")), [(0, 3), (12, 15)]);
    }

    #[test]
    fn whole_word_also_works_next_to_signs() {
        assert_eq!(found("#set and #settings", &whole("#set")), [(0, 4)]);
    }

    #[test]
    fn whole_word_knows_the_letters_of_other_scripts() {
        assert_eq!(found("café cafés", &whole("café")), [(0, 5)]);
    }

    #[test]
    fn the_case_of_other_scripts_is_ignored_too() {
        assert_eq!(found("Éclair éclair", &words("éclair")).len(), 2);
    }

    #[test]
    fn signs_are_plain_text_without_a_pattern() {
        assert_eq!(found("a.b axb", &words("a.b")), [(0, 3)]);
    }

    #[test]
    fn a_pattern_finds_regular_expressions() {
        assert_eq!(
            found("page 12, page 3", &pattern(r"\d+")),
            [(5, 7), (14, 15)]
        );
    }

    #[test]
    fn line_anchors_match_each_line() {
        assert_eq!(found("= A\ntext\n= B", &pattern("^= ")), [(0, 2), (9, 11)]);
    }

    #[test]
    fn matches_without_characters_are_skipped() {
        assert_eq!(found("ab", &pattern("x*")), Vec::<(usize, usize)>::new());
    }

    #[test]
    fn an_invalid_pattern_is_an_error() {
        assert!(Search::new(&pattern("(")).is_err());
    }

    #[test]
    fn an_empty_query_finds_nothing() {
        assert!(Search::new(&words("")).unwrap().is_none());
    }

    #[test]
    fn a_pattern_replacement_uses_the_groups() {
        let text = "Smith, John";
        let search = Search::new(&pattern(r"(\w+), (\w+)")).unwrap().unwrap();

        assert_eq!(search.replacement(text, 0..11, "$2 $1"), "John Smith");
    }

    #[test]
    fn a_plain_replacement_keeps_dollar_signs() {
        let search = Search::new(&words("cost")).unwrap().unwrap();

        assert_eq!(search.replacement("cost", 0..4, "$5"), "$5");
    }
}
