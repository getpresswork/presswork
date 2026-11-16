use loro::event::Diff;
use loro::{
    ContainerTrait, ExportMode, LoroDoc, LoroEncodeError, LoroError, LoroText, TextDelta,
    VersionVector,
};
use presswork_text::{ChangeSet, Edit, Position, TextError};

const TEXT: &str = "text";

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

    pub fn apply(&mut self, changes: &ChangeSet) -> Result<Vec<u8>, SyncError> {
        if changes.is_empty() {
            return Ok(Vec::new());
        }
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
        self.doc.commit();
        self.export_since(&before)
    }

    pub fn import(&mut self, update: &[u8]) -> Result<ChangeSet, SyncError> {
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
    use presswork_text::{Buffer, ChangeSet, Edit, Position};
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
    }

    impl Person {
        fn new(document: TextDocument) -> Self {
            let buffer = Buffer::from(document.text().as_str());
            Self { document, buffer }
        }

        fn edit(&mut self, changes: &ChangeSet) -> Vec<u8> {
            self.buffer.apply(changes).unwrap();
            self.document.apply(changes).unwrap()
        }

        fn receive(&mut self, update: &[u8]) {
            let changes = self.document.import(update).unwrap();
            self.buffer.apply(&changes).unwrap();
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
            .apply(&ChangeSet::new(0, [Edit::insert(at(0), text)]).unwrap())
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
            .apply(&ChangeSet::new(0, [Edit::insert(at(0), "Field Notes")]).unwrap())
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
        let received = bob.document.import(&update).unwrap();

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
        assert!(bob.document.import(&missing).unwrap().is_empty());
    }

    #[test]
    fn an_empty_change_set_gives_an_update_that_changes_nothing() {
        let (mut ann, mut bob) = shared("Proof");

        let update = ann.edit(&ChangeSet::empty(5));
        bob.receive(&update);

        assert_eq!(bob.text(), "Proof");
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
    }
}
