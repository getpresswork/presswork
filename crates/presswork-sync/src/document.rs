use std::collections::HashMap;
use std::ops::Range;

use loro::cursor::PosType;
use loro::event::Diff;
use loro::{
    ContainerTrait, ExpandType, ExportMode, LoroDoc, LoroEncodeError, LoroError, LoroText,
    LoroValue, StyleConfig, StyleConfigMap, TextDelta, VersionVector,
};
use presswork_text::{ChangeSet, Edit, Position, Suggestion, SuggestionKind, TextError};

const TEXT: &str = "text";
const SUGGESTION: &str = "suggestion";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    pub changes: ChangeSet,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("the update is not valid")]
    Update(#[source] LoroError),
    #[error("the version is not valid")]
    Version,
    #[error("the edit does not agree with the document")]
    Edit(#[source] LoroError),
    #[error("the updates cannot be exported")]
    Export(#[source] LoroEncodeError),
    #[error(transparent)]
    Text(#[from] TextError),
}

pub struct TextDocument {
    doc: LoroDoc,
    text: LoroText,
}

impl TextDocument {
    #[must_use]
    pub fn new() -> Self {
        let doc = LoroDoc::new();
        let mut styles = StyleConfigMap::new();
        styles.insert(
            SUGGESTION.into(),
            StyleConfig {
                expand: ExpandType::None,
            },
        );
        doc.config_text_style(styles);
        let text = doc.get_text(TEXT);
        Self { doc, text }
    }

    pub fn load(updates: &[u8]) -> Result<Self, SyncError> {
        let mut document = Self::new();
        document.merge(updates)?;
        Ok(document)
    }

    pub fn load_all(updates: &[Vec<u8>]) -> Result<Self, SyncError> {
        let document = Self::new();
        if !updates.is_empty() {
            document
                .doc
                .import_batch(updates)
                .map_err(SyncError::Update)?;
        }
        Ok(document)
    }

    pub fn merge(&mut self, update: &[u8]) -> Result<(), SyncError> {
        if !update.is_empty() {
            self.doc.import(update).map_err(SyncError::Update)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn text(&self) -> String {
        self.text.to_string()
    }

    #[must_use]
    pub fn version(&self) -> Vec<u8> {
        self.doc.oplog_vv().encode()
    }

    #[must_use]
    pub fn suggestions(&self) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let mut start = 0;
        for item in self.text.to_delta() {
            if let TextDelta::Insert { insert, attributes } = item {
                let end = start + insert.len();
                if let Some(suggestion) = attributes
                    .as_ref()
                    .and_then(|attributes| attributes.get(SUGGESTION))
                    .and_then(|value| marked(value, start..end))
                {
                    suggestions.push(suggestion);
                }
                start = end;
            }
        }
        suggestions
    }

    pub fn apply(
        &mut self,
        changes: &ChangeSet,
        suggestions: Option<&[Suggestion]>,
    ) -> Result<Vec<u8>, SyncError> {
        let before = self.doc.oplog_vv();
        let edits: Vec<Edit> = changes.edits().collect();
        for edit in edits.iter().rev() {
            let start = edit.range.start.offset();
            let length = edit.range.end.offset() - start;
            if length > 0 {
                self.text
                    .delete_utf8(start, length)
                    .map_err(SyncError::Edit)?;
            }
            if !edit.text.is_empty() {
                self.text
                    .insert_utf8(start, &edit.text)
                    .map_err(SyncError::Edit)?;
            }
        }
        if let Some(wanted) = suggestions {
            self.mark(&self.suggestions(), &joined(wanted))?;
        }
        self.doc.commit();
        self.export_since(&before)
    }

    pub fn import(&mut self, update: &[u8]) -> Result<Remote, SyncError> {
        let changes = self.import_changes(update)?;
        Ok(Remote {
            changes,
            suggestions: self.suggestions(),
        })
    }

    fn mark(&self, current: &[Suggestion], wanted: &[Suggestion]) -> Result<(), SyncError> {
        for old in current.iter().filter(|old| !wanted.contains(old)) {
            let range = self.unicode(old.range.start)?..self.unicode(old.range.end)?;
            self.text
                .unmark(range, SUGGESTION)
                .map_err(SyncError::Edit)?;
        }
        for new in wanted.iter().filter(|new| !current.contains(new)) {
            let range = new.range.start.offset()..new.range.end.offset();
            self.text
                .mark_utf8(range, SUGGESTION, mark_value(new))
                .map_err(SyncError::Edit)?;
        }
        Ok(())
    }

    fn unicode(&self, position: Position) -> Result<usize, SyncError> {
        let offset = position.offset();
        self.text
            .convert_pos(offset, PosType::Bytes, PosType::Unicode)
            .ok_or_else(|| {
                SyncError::Text(TextError::InvalidRange {
                    start: offset,
                    end: offset,
                    len: self.text.len_utf8(),
                })
            })
    }

    fn import_changes(&mut self, update: &[u8]) -> Result<ChangeSet, SyncError> {
        let old = self.text();
        if update.is_empty() {
            return Ok(ChangeSet::empty(old.len()));
        }
        let before = self.doc.state_frontiers();
        self.doc.import(update).map_err(SyncError::Update)?;
        let after = self.doc.state_frontiers();
        if before == after {
            return Ok(ChangeSet::empty(old.len()));
        }
        let diff = self.doc.diff(&before, &after).map_err(SyncError::Update)?;
        let id = self.text.id();
        let delta = diff.iter().find_map(|(container, diff)| match diff {
            Diff::Text(delta) if *container == id => Some(delta),
            _ => None,
        });
        match delta {
            Some(delta) => changes(&old, delta),
            None => Ok(ChangeSet::empty(old.len())),
        }
    }

    pub fn updates_since(&self, version: &[u8]) -> Result<Vec<u8>, SyncError> {
        let version = VersionVector::decode(version).map_err(|_| SyncError::Version)?;
        self.export_since(&version)
    }

    fn export_since(&self, version: &VersionVector) -> Result<Vec<u8>, SyncError> {
        if version.includes_vv(&self.doc.oplog_vv()) {
            return Ok(Vec::new());
        }
        self.doc
            .export(ExportMode::updates(version))
            .map_err(SyncError::Export)
    }
}

impl Default for TextDocument {
    fn default() -> Self {
        Self::new()
    }
}

fn joined(suggestions: &[Suggestion]) -> Vec<Suggestion> {
    let mut joined: Vec<Suggestion> = Vec::with_capacity(suggestions.len());
    for suggestion in suggestions
        .iter()
        .filter(|suggestion| suggestion.range.start < suggestion.range.end)
    {
        match joined.last_mut() {
            Some(last)
                if last.range.end == suggestion.range.start
                    && last.kind == suggestion.kind
                    && last.author == suggestion.author
                    && last.time == suggestion.time =>
            {
                last.range.end = suggestion.range.end;
            }
            _ => joined.push(suggestion.clone()),
        }
    }
    joined
}

fn mark_value(suggestion: &Suggestion) -> LoroValue {
    let kind = match suggestion.kind {
        SuggestionKind::Insertion => "insertion",
        SuggestionKind::Deletion => "deletion",
    };
    LoroValue::from(HashMap::from([
        ("kind", LoroValue::from(kind)),
        ("author", LoroValue::from(suggestion.author.as_str())),
        ("time", LoroValue::from(suggestion.time)),
    ]))
}

fn marked(value: &LoroValue, range: Range<usize>) -> Option<Suggestion> {
    let map = value.as_map()?;
    let kind = match map.get("kind")?.as_string()?.as_str() {
        "insertion" => SuggestionKind::Insertion,
        "deletion" => SuggestionKind::Deletion,
        _ => return None,
    };
    Some(Suggestion {
        kind,
        range: Position::new(range.start)..Position::new(range.end),
        author: map.get("author")?.as_string()?.to_string(),
        time: *map.get("time")?.as_i64()?,
    })
}

fn changes(old: &str, delta: &[TextDelta]) -> Result<ChangeSet, SyncError> {
    let mut rest = old.chars();
    let mut byte = 0;
    let mut advance = |count: usize, byte: &mut usize| {
        for character in rest.by_ref().take(count) {
            *byte += character.len_utf8();
        }
    };
    let mut edits = Vec::new();
    let mut pending: Option<Edit> = None;
    for item in delta {
        match item {
            TextDelta::Retain { retain, .. } => {
                edits.extend(pending.take());
                advance(*retain, &mut byte);
            }
            TextDelta::Insert { insert, .. } => {
                let at = Position::new(byte);
                pending
                    .get_or_insert_with(|| Edit::insert(at, String::new()))
                    .text
                    .push_str(insert);
            }
            TextDelta::Delete { delete } => {
                let at = Position::new(byte);
                advance(*delete, &mut byte);
                pending
                    .get_or_insert_with(|| Edit::insert(at, String::new()))
                    .range
                    .end = Position::new(byte);
            }
        }
    }
    edits.extend(pending);
    Ok(ChangeSet::new(old.len(), edits)?)
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use presswork_text::{Buffer, ChangeSet, Edit, Position, SuggestionKind};
    use proptest::prelude::*;
    use proptest::sample::Index;

    use super::*;

    fn at(offset: usize) -> Position {
        Position::new(offset)
    }

    fn insert(document: &TextDocument, offset: usize, text: &str) -> ChangeSet {
        ChangeSet::new(document.text().len(), [Edit::insert(at(offset), text)]).unwrap()
    }

    fn delete(document: &TextDocument, start: usize, end: usize) -> ChangeSet {
        ChangeSet::new(document.text().len(), [Edit::delete(at(start)..at(end))]).unwrap()
    }

    struct Person {
        document: TextDocument,
        buffer: Buffer,
        suggestions: Vec<Suggestion>,
    }

    impl Person {
        fn new(document: TextDocument) -> Self {
            let buffer = Buffer::from(document.text().as_str());
            let suggestions = document.suggestions();
            Self {
                document,
                buffer,
                suggestions,
            }
        }

        fn edit(&mut self, changes: &ChangeSet) -> Vec<u8> {
            let moved = self
                .suggestions
                .iter()
                .filter_map(|suggestion| suggestion.map(changes))
                .collect();
            self.suggest(changes, moved)
        }

        fn suggest(&mut self, changes: &ChangeSet, suggestions: Vec<Suggestion>) -> Vec<u8> {
            self.buffer.apply(changes).unwrap();
            self.suggestions = suggestions;
            self.document
                .apply(changes, Some(&self.suggestions))
                .unwrap()
        }

        fn receive(&mut self, update: &[u8]) {
            let remote = self.document.import(update).unwrap();
            self.buffer.apply(&remote.changes).unwrap();
            self.suggestions = remote.suggestions;
        }

        fn text(&self) -> String {
            let text = self.document.text();
            assert_eq!(
                self.buffer.to_string(),
                text,
                "the buffer left the document"
            );
            text
        }
    }

    fn shared(text: &str) -> (Person, Person) {
        let mut first = TextDocument::new();
        let start = first
            .apply(
                &ChangeSet::new(0, [Edit::insert(at(0), text)]).unwrap(),
                None,
            )
            .unwrap();
        let mut second = TextDocument::new();
        second.import(&start).unwrap();
        (Person::new(first), Person::new(second))
    }

    #[test]
    fn a_new_document_is_empty() {
        assert_eq!(TextDocument::new().text(), "");
    }

    #[test]
    fn a_local_edit_changes_the_text() {
        let mut document = TextDocument::new();

        document
            .apply(
                &ChangeSet::new(0, [Edit::insert(at(0), "Field Notes")]).unwrap(),
                None,
            )
            .unwrap();

        assert_eq!(document.text(), "Field Notes");
    }

    #[test]
    fn an_update_brings_a_local_edit_to_the_other_person() {
        let (mut ann, mut bob) = shared("The press stopped.");

        let update = ann.edit(&insert(&ann.document, 4, "old "));
        bob.receive(&update);

        assert_eq!(bob.text(), "The old press stopped.");
    }

    #[test]
    fn concurrent_edits_of_two_persons_converge() {
        let (mut ann, mut bob) = shared("The press stopped.");

        let from_ann = ann.edit(&insert(&ann.document, 0, "Late: "));
        let from_bob = bob.edit(&delete(&bob.document, 9, 17));
        ann.receive(&from_bob);
        bob.receive(&from_ann);

        assert_eq!(ann.text(), "Late: The press.");
        assert_eq!(bob.text(), ann.text());
    }

    #[test]
    fn remote_changes_use_utf8_positions() {
        let (mut ann, mut bob) = shared("Größe – Maß");

        let end = ann.document.text().len();
        let update = ann.edit(&insert(&ann.document, end, " ✓"));
        bob.receive(&update);
        let middle = "Größe".len();
        let update = bob.edit(&insert(&bob.document, middle, " und Form"));
        ann.receive(&update);

        assert_eq!(ann.text(), "Größe und Form – Maß ✓");
        assert_eq!(bob.text(), ann.text());
    }

    #[test]
    fn a_remote_change_set_has_one_edit_for_each_changed_area() {
        let (mut ann, mut bob) = shared("one two three");
        let changes = ChangeSet::new(
            13,
            [
                Edit::replace(at(0)..at(3), "1"),
                Edit::replace(at(8)..at(13), "3"),
            ],
        )
        .unwrap();

        let update = ann.edit(&changes);
        let received = bob.document.import(&update).unwrap().changes;

        assert_eq!(received.edits().count(), 2);
        bob.buffer.apply(&received).unwrap();
        assert_eq!(bob.text(), "1 two 3");
    }

    #[test]
    fn updates_since_a_version_bring_only_the_missing_changes() {
        let (mut ann, mut bob) = shared("a");
        let version = bob.document.version();
        ann.edit(&insert(&ann.document, 1, "b"));
        ann.edit(&insert(&ann.document, 2, "c"));

        let missing = ann.document.updates_since(&version).unwrap();
        bob.receive(&missing);

        assert_eq!(bob.text(), "abc");
        assert!(bob.document.import(&missing).unwrap().changes.is_empty());
    }

    #[test]
    fn an_empty_change_set_gives_an_update_that_changes_nothing() {
        let (mut ann, mut bob) = shared("Proof");

        let update = ann.edit(&ChangeSet::empty(5));
        bob.receive(&update);

        assert_eq!(bob.text(), "Proof");
    }

    fn suggestion(kind: SuggestionKind, range: Range<usize>, author: &str) -> Suggestion {
        Suggestion {
            kind,
            range: at(range.start)..at(range.end),
            author: author.to_owned(),
            time: 1_763_800_000,
        }
    }

    fn insertion(range: Range<usize>) -> Suggestion {
        suggestion(SuggestionKind::Insertion, range, "Ann")
    }

    fn deletion(range: Range<usize>) -> Suggestion {
        suggestion(SuggestionKind::Deletion, range, "Ann")
    }

    #[test]
    fn a_suggestion_goes_to_the_other_person_as_a_mark() {
        let (mut ann, mut bob) = shared("The press stopped.");

        let update = ann.suggest(&insert(&ann.document, 4, "old "), vec![insertion(4..8)]);
        bob.receive(&update);

        assert_eq!(bob.text(), "The old press stopped.");
        assert_eq!(bob.suggestions, [insertion(4..8)]);
    }

    #[test]
    fn a_mark_does_not_grow_at_its_edges() {
        let (mut ann, mut bob) = shared("The press stopped.");
        bob.receive(&ann.suggest(&ChangeSet::empty(18), vec![deletion(4..9)]));

        let edges =
            ChangeSet::new(18, [Edit::insert(at(4), "<"), Edit::insert(at(9), ">")]).unwrap();
        ann.receive(&bob.edit(&edges));

        assert_eq!(ann.text(), "The <press> stopped.");
        assert_eq!(ann.suggestions, [deletion(5..10)]);
        assert_eq!(bob.document.suggestions(), bob.suggestions);
    }

    #[test]
    fn text_typed_in_a_suggestion_is_part_of_it() {
        let (mut ann, mut bob) = shared("The press stopped.");
        bob.receive(&ann.suggest(&ChangeSet::empty(18), vec![deletion(4..9)]));

        ann.receive(&bob.edit(&insert(&bob.document, 6, "e")));

        assert_eq!(ann.text(), "The preess stopped.");
        assert_eq!(ann.suggestions, [deletion(4..10)]);
        assert_eq!(bob.document.suggestions(), bob.suggestions);
    }

    #[test]
    fn a_removed_suggestion_removes_its_mark() {
        let text = "Größe – Maß";
        let (mut ann, mut bob) = shared(text);
        let start = "Größe – ".len();
        let unchanged = ChangeSet::empty(text.len());
        bob.receive(&ann.suggest(&unchanged, vec![deletion(start..text.len())]));
        assert_eq!(bob.suggestions, [deletion(start..text.len())]);

        bob.receive(&ann.suggest(&unchanged, vec![]));

        assert_eq!(bob.suggestions, []);
        assert_eq!(bob.text(), text);
    }

    #[test]
    fn a_document_loads_with_its_suggestions() {
        let (mut ann, _) = shared("Proof");
        ann.suggest(&ChangeSet::empty(5), vec![deletion(1..3)]);

        let all = ann
            .document
            .updates_since(&TextDocument::new().version())
            .unwrap();

        assert_eq!(
            TextDocument::load(&all).unwrap().suggestions(),
            [deletion(1..3)]
        );
    }

    #[test]
    fn concurrent_suggestions_on_the_same_text_converge() {
        let (mut ann, mut bob) = shared("The press stopped.");
        let unchanged = ChangeSet::empty(18);
        let from_ann = ann.suggest(&unchanged, vec![deletion(4..12)]);
        let by_bob = suggestion(SuggestionKind::Deletion, 8..17, "Bob");
        let from_bob = bob.suggest(&unchanged, vec![by_bob]);

        ann.receive(&from_bob);
        bob.receive(&from_ann);

        assert_ne!(ann.suggestions, []);
        assert_eq!(ann.suggestions, bob.suggestions);
        assert!(
            ann.suggestions
                .windows(2)
                .all(|pair| pair[0].range.end <= pair[1].range.start)
        );
    }

    #[test]
    fn only_a_changed_list_of_suggestions_gives_an_update() {
        let (mut ann, _) = shared("Proof");

        let changed = ann.suggest(&ChangeSet::empty(5), vec![deletion(1..3)]);
        let unchanged = ann.suggest(&ChangeSet::empty(5), vec![deletion(1..3)]);

        assert_ne!(changed.len(), 0);
        assert!(unchanged.is_empty(), "{} bytes", unchanged.len());
    }

    #[test]
    fn equal_suggestions_side_by_side_are_one_mark() {
        let (mut ann, _) = shared("Proof");
        let side_by_side = vec![deletion(1..2), deletion(2..3)];

        ann.suggest(&ChangeSet::empty(5), side_by_side.clone());
        let again = ann.suggest(&ChangeSet::empty(5), side_by_side);

        assert!(again.is_empty(), "{} bytes", again.len());
        assert_eq!(ann.document.suggestions(), [deletion(1..3)]);
    }

    #[test]
    fn updates_since_a_version_with_all_changes_are_empty() {
        let (ann, bob) = shared("Proof");

        let missing = ann.document.updates_since(&bob.document.version()).unwrap();

        assert!(missing.is_empty(), "{} bytes", missing.len());
    }

    #[test]
    fn a_document_loads_from_all_of_its_updates() {
        let (mut ann, _) = shared("Proof");
        ann.edit(&insert(&ann.document, 5, "s"));

        let all = ann
            .document
            .updates_since(&TextDocument::new().version())
            .unwrap();

        assert_eq!(TextDocument::load(&all).unwrap().text(), "Proofs");
    }

    #[test]
    fn a_document_loads_from_many_stored_updates() {
        let (mut ann, _) = shared("Proof");
        let mut stored = vec![
            ann.document
                .updates_since(&TextDocument::new().version())
                .unwrap(),
        ];
        stored.push(ann.edit(&insert(&ann.document, 5, "s")));
        stored.push(ann.edit(&insert(&ann.document, 0, "The ")));

        let loaded = TextDocument::load_all(&stored).unwrap();

        assert_eq!(loaded.text(), "The Proofs");
        assert_eq!(loaded.version(), ann.document.version());
    }

    #[test]
    fn merge_applies_an_update_without_a_change_set() {
        let (mut ann, bob) = shared("Proof");
        let everything = bob
            .document
            .updates_since(&TextDocument::new().version())
            .unwrap();
        let mut server = TextDocument::load(&everything).unwrap();

        server
            .merge(&ann.edit(&insert(&ann.document, 5, "s")))
            .unwrap();

        assert_eq!(server.text(), "Proofs");
        assert!(matches!(
            server.merge(b"garbage"),
            Err(SyncError::Update(_))
        ));
        assert_eq!(server.text(), "Proofs");
    }

    #[test]
    fn an_update_that_is_not_valid_gives_an_error() {
        let mut document = TextDocument::new();

        assert!(matches!(
            document.import(b"not an update"),
            Err(SyncError::Update(_))
        ));
        assert_eq!(document.text(), "");
    }

    #[test]
    fn a_version_that_is_not_valid_gives_an_error() {
        assert!(matches!(
            TextDocument::new().updates_since(b"\xff\xff\xff"),
            Err(SyncError::Version)
        ));
    }

    fn change(text: &str, (place, length, words): &(Index, usize, String)) -> ChangeSet {
        let boundaries: Vec<usize> = text
            .char_indices()
            .map(|(offset, _)| offset)
            .chain([text.len()])
            .collect();
        let first = place.index(boundaries.len());
        let end = (first + length).min(boundaries.len() - 1);
        ChangeSet::new(
            text.len(),
            [Edit::replace(
                at(boundaries[first])..at(boundaries[end]),
                words.clone(),
            )],
        )
        .unwrap()
    }

    fn edits() -> impl Strategy<Value = Vec<(Index, usize, String)>> {
        prop::collection::vec((any::<Index>(), 0usize..4, "[a-zé–✓ ]{0,3}"), 0..8)
    }

    proptest! {
        #[test]
        fn random_concurrent_edits_converge(ann_edits in edits(), bob_edits in edits()) {
            let (mut ann, mut bob) = shared("Presswork – Größe");
            let mut from_ann = Vec::new();
            for edit in &ann_edits {
                let changes = change(&ann.document.text(), edit);
                from_ann.push(ann.edit(&changes));
            }
            let mut from_bob = Vec::new();
            for edit in &bob_edits {
                let changes = change(&bob.document.text(), edit);
                from_bob.push(bob.edit(&changes));
            }

            for update in &from_bob {
                ann.receive(update);
            }
            for update in &from_ann {
                bob.receive(update);
            }

            prop_assert_eq!(ann.text(), bob.text());
        }

        #[test]
        fn marks_move_as_the_suggestions_of_the_editor(bob_edits in edits()) {
            let text = "Presswork – Größe und Maß";
            let (mut ann, mut bob) = shared(text);
            let marked = "Presswork – ".len().."Presswork – Größe".len();
            bob.receive(&ann.suggest(&ChangeSet::empty(text.len()), vec![deletion(marked)]));

            for edit in &bob_edits {
                let changes = change(&bob.document.text(), edit);
                ann.receive(&bob.edit(&changes));
            }

            prop_assert_eq!(bob.document.suggestions(), bob.suggestions.clone());
            prop_assert_eq!(ann.suggestions, bob.suggestions);
        }
    }
}
