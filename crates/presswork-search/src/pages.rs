use std::path::Path;
use std::sync::{Mutex, PoisonError};

use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::directory::error::OpenDirectoryError;
use tantivy::query::{BooleanQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::{Field, IndexRecordOption, STORED, STRING, Schema, TEXT, Value};
use tantivy::snippet::SnippetGenerator;
use tantivy::{
    DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, Searcher, TantivyDocument, Term, doc,
};
use uuid::Uuid;

const WRITER_MEMORY: usize = 50_000_000;
const SNIPPET_CHARACTERS: usize = 200;
const SCHEMA_VERSION: &str = "v2";

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("the search index failed: {0}")]
    Index(#[from] tantivy::TantivyError),
    #[error("the folder of the search index is not available: {0}")]
    Folder(#[from] std::io::Error),
    #[error("the folder of the search index cannot be opened: {0}")]
    Directory(#[from] OpenDirectoryError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageHit {
    pub work: Uuid,
    pub edition: Uuid,
    pub page: u32,
    pub snippet: String,
}

#[derive(Debug, Clone, Copy)]
struct Fields {
    organization: Field,
    work: Field,
    edition: Field,
    page: Field,
    text: Field,
}

impl Fields {
    fn schema() -> (Schema, Self) {
        let mut builder = Schema::builder();
        let fields = Self {
            organization: builder.add_text_field("organization", STRING),
            work: builder.add_text_field("work", STRING | STORED),
            edition: builder.add_text_field("edition", STRING | STORED),
            page: builder.add_u64_field("page", STORED),
            text: builder.add_text_field("text", TEXT | STORED),
        };
        (builder.build(), fields)
    }
}

pub struct PageIndex {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    fields: Fields,
}

impl PageIndex {
    pub fn in_memory() -> Result<Self, SearchError> {
        let (schema, fields) = Fields::schema();
        Self::with(Index::create_in_ram(schema), fields)
    }

    pub fn open(folder: &Path) -> Result<Self, SearchError> {
        let folder = folder.join(SCHEMA_VERSION);
        std::fs::create_dir_all(&folder)?;
        let (schema, fields) = Fields::schema();
        let directory = MmapDirectory::open(&folder)?;
        Self::with(Index::open_or_create(directory, schema)?, fields)
    }

    fn with(index: Index, fields: Fields) -> Result<Self, SearchError> {
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let writer = index.writer_with_num_threads(1, WRITER_MEMORY)?;
        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
        })
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.reader.searcher().num_docs() == 0
    }

    pub fn put(
        &self,
        organization: Uuid,
        work: Uuid,
        edition: Uuid,
        pages: &[String],
    ) -> Result<(), SearchError> {
        let mut writer = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
        let edition = edition.to_string();
        writer.delete_term(Term::from_field_text(self.fields.edition, &edition));
        for (number, text) in (1_u64..).zip(pages) {
            if text.trim().is_empty() {
                continue;
            }
            writer.add_document(doc!(
                self.fields.organization => organization.to_string(),
                self.fields.work => work.to_string(),
                self.fields.edition => edition.as_str(),
                self.fields.page => number,
                self.fields.text => text.as_str(),
            ))?;
        }
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    pub fn search(
        &self,
        organization: Uuid,
        words: &str,
        limit: usize,
    ) -> Result<Vec<PageHit>, SearchError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let searcher = self.reader.searcher();
        let mut parser = QueryParser::for_index(&self.index, vec![self.fields.text]);
        parser.set_conjunction_by_default();
        let (words, _) = parser.parse_query_lenient(words);
        let mut snippets = SnippetGenerator::create(&searcher, &*words, self.fields.text)?;
        snippets.set_max_num_chars(SNIPPET_CHARACTERS);
        let organization = TermQuery::new(
            Term::from_field_text(self.fields.organization, &organization.to_string()),
            IndexRecordOption::Basic,
        );
        let query = BooleanQuery::new(vec![
            (Occur::Must, words),
            (Occur::Must, Box::new(organization)),
        ]);
        let found = searcher.search(&query, &TopDocs::with_limit(limit).order_by_score())?;
        let mut hits = Vec::with_capacity(found.len());
        for (_, address) in found {
            if let Some(hit) = self.hit(&searcher, &snippets, address)? {
                hits.push(hit);
            }
        }
        Ok(hits)
    }

    fn hit(
        &self,
        searcher: &Searcher,
        snippets: &SnippetGenerator,
        address: DocAddress,
    ) -> Result<Option<PageHit>, SearchError> {
        let document: TantivyDocument = searcher.doc(address)?;
        let id = |field: Field| {
            document
                .get_first(field)
                .and_then(|value| value.as_str())
                .and_then(|text| Uuid::parse_str(text).ok())
        };
        let page = document
            .get_first(self.fields.page)
            .and_then(|value| value.as_u64())
            .and_then(|number| u32::try_from(number).ok());
        let (Some(work), Some(edition), Some(page)) =
            (id(self.fields.work), id(self.fields.edition), page)
        else {
            return Ok(None);
        };
        let snippet = snippets.snippet_from_doc(&document).fragment().to_owned();
        Ok(Some(PageHit {
            work,
            edition,
            page,
            snippet,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRESS: Uuid = Uuid::from_u128(1);
    const OTHER_PRESS: Uuid = Uuid::from_u128(2);
    const NOTES: Uuid = Uuid::from_u128(10);
    const ATLAS: Uuid = Uuid::from_u128(11);
    const NOTES_1455: Uuid = Uuid::from_u128(20);
    const NOTES_1922: Uuid = Uuid::from_u128(21);
    const ATLAS_1683: Uuid = Uuid::from_u128(22);

    fn pages(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    fn places(hits: &[PageHit]) -> Vec<(Uuid, u32)> {
        hits.iter().map(|hit| (hit.edition, hit.page)).collect()
    }

    fn notes() -> Vec<String> {
        pages(&[
            "Good books start with good paper.",
            "The ink must dry before you fold the sheets.",
        ])
    }

    #[test]
    fn a_search_finds_the_page_with_the_words() {
        let index = PageIndex::in_memory().unwrap();
        index.put(PRESS, NOTES, NOTES_1455, &notes()).unwrap();

        let hits = index.search(PRESS, "Ink", 10).unwrap();

        assert_eq!(places(&hits), [(NOTES_1455, 2)]);
        assert_eq!(hits[0].work, NOTES);
        assert!(hits[0].snippet.contains("ink must dry"), "{hits:?}");
    }

    #[test]
    fn all_words_must_be_on_the_page() {
        let index = PageIndex::in_memory().unwrap();
        index.put(PRESS, NOTES, NOTES_1455, &notes()).unwrap();

        assert_eq!(places(&index.search(PRESS, "good ink", 10).unwrap()), []);
        assert_eq!(
            places(&index.search(PRESS, "good paper", 10).unwrap()),
            [(NOTES_1455, 1)]
        );
    }

    #[test]
    fn a_search_finds_only_the_pages_of_its_organization() {
        let index = PageIndex::in_memory().unwrap();
        index.put(PRESS, NOTES, NOTES_1455, &notes()).unwrap();
        index
            .put(
                OTHER_PRESS,
                ATLAS,
                ATLAS_1683,
                &pages(&["A map of the paper mills."]),
            )
            .unwrap();

        assert_eq!(
            places(&index.search(PRESS, "paper", 10).unwrap()),
            [(NOTES_1455, 1)]
        );
        assert_eq!(
            places(&index.search(OTHER_PRESS, "paper", 10).unwrap()),
            [(ATLAS_1683, 1)]
        );
    }

    #[test]
    fn new_pages_of_an_edition_take_the_place_of_its_old_pages() {
        let index = PageIndex::in_memory().unwrap();
        index.put(PRESS, NOTES, NOTES_1455, &notes()).unwrap();

        index
            .put(PRESS, NOTES, NOTES_1455, &pages(&["", "Paper and ink"]))
            .unwrap();

        assert_eq!(places(&index.search(PRESS, "dry", 10).unwrap()), []);
        assert_eq!(
            places(&index.search(PRESS, "paper", 10).unwrap()),
            [(NOTES_1455, 2)]
        );
    }

    #[test]
    fn two_editions_of_a_work_keep_their_own_pages() {
        let index = PageIndex::in_memory().unwrap();
        index.put(PRESS, NOTES, NOTES_1455, &notes()).unwrap();
        index
            .put(PRESS, NOTES, NOTES_1922, &pages(&["Paper, ink, and type."]))
            .unwrap();

        index
            .put(PRESS, NOTES, NOTES_1922, &pages(&["Paper and type."]))
            .unwrap();

        let mut found = places(&index.search(PRESS, "paper", 10).unwrap());
        found.sort();
        assert_eq!(found, [(NOTES_1455, 1), (NOTES_1922, 1)]);
        assert_eq!(
            places(&index.search(PRESS, "ink", 10).unwrap()),
            [(NOTES_1455, 2)]
        );
    }

    #[test]
    fn an_index_is_empty_until_it_gets_pages() {
        let index = PageIndex::in_memory().unwrap();
        assert!(index.is_empty());

        index.put(PRESS, NOTES, NOTES_1455, &notes()).unwrap();

        assert!(!index.is_empty());
    }

    #[test]
    fn signs_of_the_query_language_do_not_stop_a_search() {
        let index = PageIndex::in_memory().unwrap();
        index.put(PRESS, NOTES, NOTES_1455, &notes()).unwrap();

        assert_eq!(
            places(&index.search(PRESS, "paper) AND: (", 10).unwrap()),
            [(NOTES_1455, 1)]
        );
        assert_eq!(places(&index.search(PRESS, "   ", 10).unwrap()), []);
        assert_eq!(places(&index.search(PRESS, "paper", 0).unwrap()), []);
    }

    #[test]
    fn an_index_in_a_folder_keeps_its_pages_in_a_folder_for_its_schema() {
        let folder = tempfile::tempdir().unwrap();
        PageIndex::open(folder.path())
            .unwrap()
            .put(PRESS, NOTES, NOTES_1455, &notes())
            .unwrap();

        let index = PageIndex::open(folder.path()).unwrap();

        assert!(folder.path().join(SCHEMA_VERSION).is_dir());
        assert!(!index.is_empty());
        assert_eq!(
            places(&index.search(PRESS, "paper", 10).unwrap()),
            [(NOTES_1455, 1)]
        );
    }
}
