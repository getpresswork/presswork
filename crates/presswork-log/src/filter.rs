use crate::{Level, LogEntry};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogFilter {
    pub minimum: Level,
    pub text: String,
}

impl LogFilter {
    #[must_use]
    pub fn matches(&self, entry: &LogEntry) -> bool {
        entry.level >= self.minimum && self.matches_text(entry)
    }

    fn matches_text(&self, entry: &LogEntry) -> bool {
        let needle = self.text.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        let contains = |haystack: &str| haystack.to_lowercase().contains(&needle);
        contains(&entry.message)
            || contains(&entry.target)
            || entry
                .fields
                .iter()
                .any(|(name, value)| contains(name) || contains(value))
            || entry.spans.iter().any(|span| contains(span))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(level: Level, message: &str) -> LogEntry {
        LogEntry {
            level,
            message: message.to_owned(),
            target: "presswork_typst::world".to_owned(),
            ..LogEntry::default()
        }
    }

    fn text(text: &str) -> LogFilter {
        LogFilter {
            text: text.to_owned(),
            ..LogFilter::default()
        }
    }

    #[test]
    fn default_filter_shows_information_and_more_severe_entries() {
        let filter = LogFilter::default();

        assert!(filter.matches(&entry(Level::Info, "a")));
        assert!(filter.matches(&entry(Level::Error, "a")));
        assert!(!filter.matches(&entry(Level::Debug, "a")));
    }

    #[test]
    fn text_finds_words_in_the_message_without_case() {
        let filter = text("COMPILED");

        assert!(filter.matches(&entry(Level::Info, "Project compiled")));
        assert!(!filter.matches(&entry(Level::Info, "Project opened")));
    }

    #[test]
    fn text_finds_words_in_the_area_the_fields_and_the_spans() {
        let mut entry = entry(Level::Info, "done");
        entry.fields = vec![("project".to_owned(), "Long Night".to_owned())];
        entry.spans = vec!["export".to_owned()];

        for words in ["typst", "long night", "export"] {
            assert!(text(words).matches(&entry), "{words} is not found");
        }
    }
}
