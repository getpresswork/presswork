use std::ops::Range;

use regex::{Captures, Regex, RegexBuilder};
use serde::Deserialize;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StyleError {
    #[error("the house style file is not valid: {0}")]
    File(String),
    #[error("rule {rule} of the house style is not valid: {problem}")]
    Rule { rule: usize, problem: RuleProblem },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuleProblem {
    #[error("it has no text and no pattern")]
    Missing,
    #[error("it has a text and a pattern")]
    Both,
    #[error("its text or pattern is empty")]
    Empty,
    #[error("its pattern is not valid: {0}")]
    Pattern(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleMatch {
    pub range: Range<usize>,
    pub rule: usize,
    pub replacement: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct HouseStyle {
    rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
struct Rule {
    expression: Regex,
    kind: Kind,
    replacement: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Text { ignore_case: bool },
    Pattern,
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    rule: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    text: Option<String>,
    pattern: Option<String>,
    replacement: Option<String>,
    message: Option<String>,
}

impl HouseStyle {
    pub fn parse(text: &str) -> Result<Self, StyleError> {
        let file: File =
            toml::from_str(text).map_err(|error| StyleError::File(error.message().to_owned()))?;
        let rules = file
            .rule
            .into_iter()
            .enumerate()
            .map(|(index, entry)| {
                Rule::new(entry).map_err(|problem| StyleError::Rule {
                    rule: index + 1,
                    problem,
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { rules })
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    #[must_use]
    pub fn message(&self, rule: usize) -> Option<&str> {
        self.rules.get(rule)?.message.as_deref()
    }

    #[must_use]
    pub fn check(&self, text: &str) -> Vec<StyleMatch> {
        let mut found: Vec<StyleMatch> = self
            .rules
            .iter()
            .enumerate()
            .flat_map(|(index, rule)| rule.find(index, text))
            .collect();
        found.sort_by_key(|found| (found.range.start, found.rule));
        found
    }
}

impl Rule {
    fn new(entry: Entry) -> Result<Self, RuleProblem> {
        let (expression, kind) = match (entry.text, entry.pattern) {
            (None, None) => return Err(RuleProblem::Missing),
            (Some(_), Some(_)) => return Err(RuleProblem::Both),
            (Some(text), None) => text_expression(&text)?,
            (None, Some(pattern)) if pattern.is_empty() => return Err(RuleProblem::Empty),
            (None, Some(pattern)) => (
                Regex::new(&pattern).map_err(|error| RuleProblem::Pattern(short(&error)))?,
                Kind::Pattern,
            ),
        };
        Ok(Self {
            expression,
            kind,
            replacement: entry.replacement,
            message: entry.message,
        })
    }

    fn find<'a>(&'a self, index: usize, text: &'a str) -> impl Iterator<Item = StyleMatch> + 'a {
        self.expression
            .captures_iter(text)
            .filter_map(move |captures| {
                let whole = captures.get(0)?;
                (!whole.is_empty()).then(|| StyleMatch {
                    range: whole.range(),
                    rule: index,
                    replacement: self.replacement(&captures),
                })
            })
    }

    fn replacement(&self, captures: &Captures<'_>) -> Option<String> {
        let replacement = self.replacement.as_deref()?;
        Some(match self.kind {
            Kind::Pattern => {
                let mut expanded = String::new();
                captures.expand(replacement, &mut expanded);
                expanded
            }
            Kind::Text { ignore_case: true } => keep_capital(&captures[0], replacement),
            Kind::Text { ignore_case: false } => replacement.to_owned(),
        })
    }
}

fn text_expression(text: &str) -> Result<(Regex, Kind), RuleProblem> {
    let trimmed = text.trim();
    let (Some(first), Some(last)) = (trimmed.chars().next(), trimmed.chars().next_back()) else {
        return Err(RuleProblem::Empty);
    };
    let mut expression = trimmed
        .split_whitespace()
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join(r"\s+");
    if is_word(first) {
        expression.insert_str(0, r"\b");
    }
    if is_word(last) {
        expression.push_str(r"\b");
    }
    let ignore_case = !trimmed.chars().any(char::is_uppercase);
    let expression = RegexBuilder::new(&expression)
        .case_insensitive(ignore_case)
        .build()
        .map_err(|error| RuleProblem::Pattern(short(&error)))?;
    Ok((expression, Kind::Text { ignore_case }))
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn keep_capital(found: &str, replacement: &str) -> String {
    let mut characters = replacement.chars();
    match characters.next() {
        Some(first) if found.chars().next().is_some_and(char::is_uppercase) => {
            first.to_uppercase().chain(characters).collect()
        }
        _ => replacement.to_owned(),
    }
}

fn short(error: &regex::Error) -> String {
    let text = error.to_string();
    let line = text
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    line.trim().trim_start_matches("error: ").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(style: &str, text: &str) -> Vec<(String, Option<String>)> {
        let style = HouseStyle::parse(style).unwrap();
        style
            .check(text)
            .into_iter()
            .map(|found| (text[found.range].to_owned(), found.replacement))
            .collect()
    }

    fn problem(style: &str) -> Option<(usize, RuleProblem)> {
        match HouseStyle::parse(style) {
            Err(StyleError::Rule { rule, problem }) => Some((rule, problem)),
            _ => None,
        }
    }

    #[test]
    fn a_file_without_rules_gives_an_empty_style() {
        assert!(HouseStyle::parse("").unwrap().is_empty());
        assert!(
            !HouseStyle::parse("[[rule]]\ntext = \"a\"\n")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_text_rule_finds_whole_words() {
        let style = "[[rule]]\ntext = \"organise\"\nreplacement = \"organize\"\n";

        assert_eq!(
            found(style, "We organise the reorganised books."),
            [("organise".to_owned(), Some("organize".to_owned()))]
        );
    }

    #[test]
    fn a_text_rule_in_small_letters_keeps_the_capital_of_the_text() {
        let style = "[[rule]]\ntext = \"organise\"\nreplacement = \"organize\"\n";

        assert_eq!(
            found(style, "Organise it."),
            [("Organise".to_owned(), Some("Organize".to_owned()))]
        );
    }

    #[test]
    fn a_text_rule_with_capitals_finds_only_the_same_capitals() {
        let style = "[[rule]]\ntext = \"Javascript\"\nreplacement = \"JavaScript\"\n";

        assert_eq!(
            found(style, "Javascript or javascript"),
            [("Javascript".to_owned(), Some("JavaScript".to_owned()))]
        );
    }

    #[test]
    fn a_text_rule_can_have_more_words_on_more_lines() {
        let style = "[[rule]]\ntext = \"in order to\"\nreplacement = \"to\"\n";

        assert_eq!(
            found(style, "We read in order to learn."),
            [("in order to".to_owned(), Some("to".to_owned()))]
        );
        assert_eq!(
            found(style, "We read in order\n  to learn."),
            [("in order\n  to".to_owned(), Some("to".to_owned()))]
        );
    }

    #[test]
    fn a_pattern_rule_uses_a_regular_expression_and_its_groups() {
        let style = "[[rule]]\npattern = '(\\d+)-(\\d+)'\nreplacement = '$1–$2'\nmessage = \"Use an en dash.\"\n";
        let house = HouseStyle::parse(style).unwrap();

        assert_eq!(
            found(style, "Pages 12-19 and 3."),
            [("12-19".to_owned(), Some("12–19".to_owned()))]
        );
        assert_eq!(house.message(0), Some("Use an en dash."));
    }

    #[test]
    fn a_rule_without_a_replacement_only_finds_the_text() {
        let style =
            "[[rule]]\npattern = '\\b[1-9]\\b'\nmessage = \"Write small numbers in words.\"\n";

        assert_eq!(found(style, "We have 3 cats."), [("3".to_owned(), None)]);
    }

    #[test]
    fn a_rule_needs_one_text_or_one_valid_pattern() {
        assert_eq!(
            problem("[[rule]]\nmessage = \"No.\"\n"),
            Some((1, RuleProblem::Missing))
        );
        assert_eq!(
            problem("[[rule]]\ntext = \"a\"\npattern = \"b\"\n"),
            Some((1, RuleProblem::Both))
        );
        assert_eq!(
            problem("[[rule]]\ntext = \"a\"\n\n[[rule]]\ntext = \" \"\n"),
            Some((2, RuleProblem::Empty))
        );
        assert_eq!(
            problem("[[rule]]\npattern = \"(\"\n"),
            Some((1, RuleProblem::Pattern("unclosed group".to_owned())))
        );
        assert!(matches!(
            HouseStyle::parse("[[rule]\n"),
            Err(StyleError::File(_))
        ));
    }

    #[test]
    fn matches_come_in_the_order_of_the_text() {
        let style = "[[rule]]\ntext = \"colour\"\n\n[[rule]]\ntext = \"organise\"\n";
        let house = HouseStyle::parse(style).unwrap();

        let rules: Vec<usize> = house
            .check("organise the colour")
            .iter()
            .map(|found| found.rule)
            .collect();

        assert_eq!(rules, [1, 0]);
    }
}
