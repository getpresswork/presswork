use crate::change::ChangeSet;
use crate::selection::Selection;
use crate::suggestion::Suggestion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub changes: ChangeSet,
    pub selection: Option<Selection>,
    pub suggestions: Option<Vec<Suggestion>>,
}

impl Transaction {
    #[must_use]
    pub fn new(changes: ChangeSet) -> Self {
        Self {
            changes,
            selection: None,
            suggestions: None,
        }
    }

    #[must_use]
    pub fn with_selection(mut self, selection: Selection) -> Self {
        self.selection = Some(selection);
        self
    }

    #[must_use]
    pub fn with_suggestions(mut self, suggestions: Vec<Suggestion>) -> Self {
        self.suggestions = Some(suggestions);
        self
    }
}
