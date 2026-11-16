use std::collections::HashMap;
use std::fmt;
use std::ops::RangeInclusive;
use std::path::Path;
use std::sync::Arc;

use axum::body::Bytes;
use object_store::local::LocalFileSystem;
use object_store::memory::InMemory;
use object_store::path::Path as BlobPath;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload};
use presswork_api::{LibraryFile, NewWork, SearchHit, Work};
use presswork_library::{KeyError, parse_key};
use presswork_ocr::{ReadError, text_layer};
use presswork_search::{PageIndex, SearchError};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
const MAX_TITLE_CHARACTERS: usize = 500;
const MAX_CREATOR_CHARACTERS: usize = 200;
const MAX_CREATORS: usize = 20;
const YEARS: RangeInclusive<i32> = -9999..=9999;
const SEARCH_LIMIT: usize = 20;
const UNIQUE_VIOLATION: &str = "23505";

type WorkRow = (
    Uuid,
    Uuid,
    String,
    String,
    Vec<String>,
    Option<i32>,
    Option<i64>,
    Option<i32>,
    Option<i32>,
);

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error(transparent)]
    InvalidKey(#[from] KeyError),
    #[error("a title must have 1 to 500 characters")]
    InvalidTitle,
    #[error("a work can have 20 creators or fewer, and each name must have 1 to 200 characters")]
    InvalidCreators,
    #[error("the year must be from -9999 to 9999")]
    InvalidYear,
    #[error("the organization already has a work with this citation key")]
    KeyTaken,
    #[error("the file is not a PDF file that Presswork can read")]
    NotPdf,
    #[error("the PDF file is encrypted. Upload a PDF file without a password")]
    Encrypted,
    #[error("the work does not exist")]
    NotFound,
    #[error("the blob store failed: {0}")]
    Blobs(#[from] object_store::Error),
    #[error("the search index failed: {0}")]
    Search(#[from] SearchError),
    #[error("the data folder is not available: {0}")]
    Folder(#[from] std::io::Error),
    #[error("a background job stopped: {0}")]
    Job(#[from] tokio::task::JoinError),
    #[error("the database failed: {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ValidWork {
    key: String,
    title: String,
    creators: Vec<String>,
    year: Option<i32>,
}

pub(crate) fn parse_work(work: &NewWork) -> Result<ValidWork, LibraryError> {
    let key = parse_key(&work.key)?.to_owned();
    let title = work.title.trim();
    if !(1..=MAX_TITLE_CHARACTERS).contains(&title.chars().count()) {
        return Err(LibraryError::InvalidTitle);
    }
    let creators: Vec<String> = work
        .creators
        .iter()
        .map(|creator| creator.trim().to_owned())
        .collect();
    let valid_creators = creators.len() <= MAX_CREATORS
        && creators
            .iter()
            .all(|creator| (1..=MAX_CREATOR_CHARACTERS).contains(&creator.chars().count()));
    if !valid_creators {
        return Err(LibraryError::InvalidCreators);
    }
    if work.year.is_some_and(|year| !YEARS.contains(&year)) {
        return Err(LibraryError::InvalidYear);
    }
    Ok(ValidWork {
        key,
        title: title.to_owned(),
        creators,
        year: work.year,
    })
}

pub async fn create_work(
    pool: &PgPool,
    organization_id: Uuid,
    created_by: Uuid,
    new: &NewWork,
) -> Result<Work, LibraryError> {
    let valid = parse_work(new)?;
    let id = Uuid::now_v7();
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "INSERT INTO works (id, organization_id, key, title, creators, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(organization_id)
    .bind(&valid.key)
    .bind(&valid.title)
    .bind(&valid.creators)
    .bind(created_by)
    .execute(&mut *transaction)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(database) if database.code().as_deref() == Some(UNIQUE_VIOLATION) => {
            LibraryError::KeyTaken
        }
        _ => LibraryError::Database(error),
    })?;
    sqlx::query("INSERT INTO editions (id, work_id, year) VALUES ($1, $2, $3)")
        .bind(Uuid::now_v7())
        .bind(id)
        .bind(valid.year)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(Work {
        id,
        organization_id,
        key: valid.key,
        title: valid.title,
        creators: valid.creators,
        year: valid.year,
        file: None,
    })
}

pub async fn works(pool: &PgPool, organization_id: Uuid) -> Result<Vec<Work>, LibraryError> {
    let rows: Vec<WorkRow> = sqlx::query_as(
        "SELECT works.id, works.organization_id, works.key, works.title, works.creators, \
         editions.year, library_files.size, library_files.pages, library_files.pages_with_text \
         FROM works JOIN editions ON editions.work_id = works.id \
         LEFT JOIN library_files ON library_files.edition_id = editions.id \
         WHERE works.organization_id = $1 ORDER BY lower(works.title), works.key",
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(work_from).collect())
}

pub async fn work(pool: &PgPool, work_id: Uuid) -> Result<Option<Work>, LibraryError> {
    let row: Option<WorkRow> = sqlx::query_as(
        "SELECT works.id, works.organization_id, works.key, works.title, works.creators, \
         editions.year, library_files.size, library_files.pages, library_files.pages_with_text \
         FROM works JOIN editions ON editions.work_id = works.id \
         LEFT JOIN library_files ON library_files.edition_id = editions.id \
         WHERE works.id = $1",
    )
    .bind(work_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(work_from))
}

pub async fn work_organization(pool: &PgPool, work_id: Uuid) -> Result<Option<Uuid>, LibraryError> {
    Ok(
        sqlx::query_scalar("SELECT organization_id FROM works WHERE id = $1")
            .bind(work_id)
            .fetch_optional(pool)
            .await?,
    )
}

fn work_from(row: WorkRow) -> Work {
    let (id, organization_id, key, title, creators, year, size, pages, pages_with_text) = row;
    let file = match (size, pages, pages_with_text) {
        (Some(size), Some(pages), Some(pages_with_text)) => Some(LibraryFile {
            size: u64::try_from(size).unwrap_or_default(),
            pages: u32::try_from(pages).unwrap_or_default(),
            pages_with_text: u32::try_from(pages_with_text).unwrap_or_default(),
        }),
        _ => None,
    };
    Work {
        id,
        organization_id,
        key,
        title,
        creators,
        year,
        file,
    }
}

struct NewFile<'a> {
    id: Uuid,
    size: i64,
    digest: &'a [u8],
    pages: &'a [String],
}

async fn attach_file(
    pool: &PgPool,
    work_id: Uuid,
    created_by: Uuid,
    file: &NewFile<'_>,
) -> Result<Option<Uuid>, LibraryError> {
    let mut transaction = pool.begin().await?;
    let edition: Uuid = sqlx::query_scalar(
        "SELECT id FROM editions WHERE work_id = $1 ORDER BY created_at, id LIMIT 1",
    )
    .bind(work_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(LibraryError::NotFound)?;
    let replaced: Option<Uuid> =
        sqlx::query_scalar("DELETE FROM library_files WHERE edition_id = $1 RETURNING id")
            .bind(edition)
            .fetch_optional(&mut *transaction)
            .await?;
    let with_text = file
        .pages
        .iter()
        .filter(|page| !page.trim().is_empty())
        .count();
    sqlx::query(
        "INSERT INTO library_files (id, edition_id, size, sha256, pages, pages_with_text, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(file.id)
    .bind(edition)
    .bind(file.size)
    .bind(file.digest)
    .bind(count(file.pages.len()))
    .bind(count(with_text))
    .bind(created_by)
    .execute(&mut *transaction)
    .await?;
    let numbers: Vec<i32> = (1..=count(file.pages.len())).collect();
    sqlx::query(
        "INSERT INTO library_pages (file_id, number, text) \
         SELECT $1, page.number, page.text FROM UNNEST($2::INTEGER[], $3::TEXT[]) AS page (number, text)",
    )
    .bind(file.id)
    .bind(&numbers)
    .bind(file.pages)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(replaced)
}

fn count(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn blob_path(file: Uuid) -> BlobPath {
    BlobPath::from(format!("library/{file}.pdf"))
}

fn readable(pages: Vec<String>) -> Vec<String> {
    pages
        .into_iter()
        .map(|page| page.replace('\0', ""))
        .collect()
}

#[derive(Clone)]
pub struct Library {
    blobs: Arc<dyn ObjectStore>,
    index: Arc<PageIndex>,
}

impl fmt::Debug for Library {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Library").finish_non_exhaustive()
    }
}

impl Library {
    pub fn open(data: &Path) -> Result<Self, LibraryError> {
        let blobs = data.join("blobs");
        std::fs::create_dir_all(&blobs)?;
        Ok(Self {
            blobs: Arc::new(LocalFileSystem::new_with_prefix(blobs)?),
            index: Arc::new(PageIndex::open(&data.join("index"))?),
        })
    }

    pub fn in_memory() -> Result<Self, LibraryError> {
        Ok(Self {
            blobs: Arc::new(InMemory::new()),
            index: Arc::new(PageIndex::in_memory()?),
        })
    }

    pub async fn add_file(
        &self,
        pool: &PgPool,
        organization: Uuid,
        work_id: Uuid,
        created_by: Uuid,
        file: Bytes,
    ) -> Result<Work, LibraryError> {
        let reader = file.clone();
        let pages = tokio::task::spawn_blocking(move || text_layer(reader))
            .await
            .map_err(|_| LibraryError::NotPdf)?
            .map_err(|error| match error {
                ReadError::Invalid => LibraryError::NotPdf,
                ReadError::Encrypted => LibraryError::Encrypted,
            })?;
        let pages = readable(pages);
        let digest = Sha256::digest(&file);
        let new = NewFile {
            id: Uuid::now_v7(),
            size: i64::try_from(file.len()).unwrap_or(i64::MAX),
            digest: digest.as_slice(),
            pages: &pages,
        };
        let location = blob_path(new.id);
        self.blobs.put(&location, PutPayload::from(file)).await?;
        let replaced = match attach_file(pool, work_id, created_by, &new).await {
            Ok(replaced) => replaced,
            Err(error) => {
                if let Err(error) = self.blobs.delete(&location).await {
                    tracing::warn!(%error, "A library file without a record was not deleted");
                }
                return Err(error);
            }
        };
        if let Some(old) = replaced
            && let Err(error) = self.blobs.delete(&blob_path(old)).await
        {
            tracing::warn!(%error, "An old library file was not deleted");
        }
        let index = Arc::clone(&self.index);
        let indexed = tokio::task::spawn_blocking(move || index.put(organization, work_id, &pages))
            .await
            .map_err(LibraryError::from)
            .and_then(|result| result.map_err(LibraryError::from));
        if let Err(error) = indexed {
            tracing::error!(%error, "The pages of a library file are not in the search index");
        }
        work(pool, work_id).await?.ok_or(LibraryError::NotFound)
    }

    pub async fn search(
        &self,
        pool: &PgPool,
        organization: Uuid,
        words: &str,
    ) -> Result<Vec<SearchHit>, LibraryError> {
        let index = Arc::clone(&self.index);
        let words = words.to_owned();
        let hits =
            tokio::task::spawn_blocking(move || index.search(organization, &words, SEARCH_LIMIT))
                .await??;
        let ids: Vec<Uuid> = hits.iter().map(|hit| hit.work).collect();
        let names: HashMap<Uuid, (String, String)> = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT id, key, title FROM works WHERE id = ANY($1)",
        )
        .bind(&ids)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|(id, key, title)| (id, (key, title)))
        .collect();
        Ok(hits
            .into_iter()
            .filter_map(|hit| {
                let (key, title) = names.get(&hit.work)?;
                Some(SearchHit {
                    work_id: hit.work,
                    key: key.clone(),
                    title: title.clone(),
                    page: hit.page,
                    snippet: hit.snippet,
                })
            })
            .collect())
    }

    pub async fn rebuild_if_empty(&self, pool: &PgPool) -> Result<usize, LibraryError> {
        if !self.index.is_empty() {
            return Ok(0);
        }
        let rows: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT works.organization_id, works.id, library_pages.text FROM library_pages \
             JOIN library_files ON library_files.id = library_pages.file_id \
             JOIN editions ON editions.id = library_files.edition_id \
             JOIN works ON works.id = editions.work_id \
             ORDER BY works.id, library_pages.number",
        )
        .fetch_all(pool)
        .await?;
        let mut works: Vec<(Uuid, Uuid, Vec<String>)> = Vec::new();
        for (organization, work, text) in rows {
            match works.last_mut() {
                Some((_, last, pages)) if *last == work => pages.push(text),
                _ => works.push((organization, work, vec![text])),
            }
        }
        let count = works.len();
        let index = Arc::clone(&self.index);
        tokio::task::spawn_blocking(move || {
            works
                .iter()
                .try_for_each(|(organization, work, pages)| index.put(*organization, *work, pages))
        })
        .await??;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use presswork_api::Role;

    use super::*;
    use crate::accounts::{User, create_user};
    use crate::links::create_organization;
    use crate::testing::test_database;

    const PDF: &[u8] = include_bytes!("../../presswork-ocr/fixtures/typst.pdf");

    fn new_work(key: &str, title: &str) -> NewWork {
        NewWork {
            key: key.to_owned(),
            title: title.to_owned(),
            creators: vec!["Gutenberg, Johannes".to_owned()],
            year: Some(1455),
        }
    }

    async fn member(pool: &PgPool, organization: Uuid, email: &str) -> User {
        let user = create_user(pool, email, "Someone", "correct horse battery")
            .await
            .unwrap();
        sqlx::query("INSERT INTO memberships (organization_id, user_id, role) VALUES ($1, $2, $3)")
            .bind(organization)
            .bind(user.id)
            .bind(Role::Member.as_str())
            .execute(pool)
            .await
            .unwrap();
        user
    }

    #[test]
    fn a_new_work_gets_trimmed_values() {
        let work = NewWork {
            key: " field-notes ".to_owned(),
            title: "  Field Notes ".to_owned(),
            creators: vec![" Gutenberg, Johannes ".to_owned()],
            year: Some(-500),
        };

        assert_eq!(
            parse_work(&work).unwrap(),
            ValidWork {
                key: "field-notes".to_owned(),
                title: "Field Notes".to_owned(),
                creators: vec!["Gutenberg, Johannes".to_owned()],
                year: Some(-500),
            }
        );
    }

    #[test]
    fn a_new_work_needs_valid_values() {
        let cases = [
            (new_work("Field Notes", "Field Notes"), "key"),
            (new_work("field-notes", " "), "title"),
            (new_work("field-notes", &"x".repeat(501)), "title"),
            (
                NewWork {
                    creators: vec![String::new()],
                    ..new_work("field-notes", "Field Notes")
                },
                "creators",
            ),
            (
                NewWork {
                    creators: vec!["Someone".to_owned(); 21],
                    ..new_work("field-notes", "Field Notes")
                },
                "creators",
            ),
            (
                NewWork {
                    year: Some(10_000),
                    ..new_work("field-notes", "Field Notes")
                },
                "year",
            ),
        ];

        for (work, part) in cases {
            let error = parse_work(&work).unwrap_err();
            let expected = match part {
                "key" => matches!(error, LibraryError::InvalidKey(_)),
                "title" => matches!(error, LibraryError::InvalidTitle),
                "creators" => matches!(error, LibraryError::InvalidCreators),
                _ => matches!(error, LibraryError::InvalidYear),
            };
            assert!(expected, "{part}: {error:?}");
        }
    }

    #[test]
    fn page_texts_lose_null_characters() {
        assert_eq!(
            readable(vec!["Good\0 paper".to_owned()]),
            ["Good paper".to_owned()]
        );
    }

    #[tokio::test]
    async fn a_work_gets_its_file_and_the_search_finds_its_pages() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let ann = member(pool, press.id, "ann@example.com").await;
        let library = Library::in_memory().unwrap();
        let work = create_work(
            pool,
            press.id,
            ann.id,
            &new_work("field-notes", "Field Notes"),
        )
        .await
        .unwrap();

        let stored = library
            .add_file(pool, press.id, work.id, ann.id, Bytes::from_static(PDF))
            .await
            .unwrap();

        assert_eq!(
            stored.file,
            Some(LibraryFile {
                size: u64::try_from(PDF.len()).unwrap(),
                pages: 2,
                pages_with_text: 2,
            })
        );
        let hits = library.search(pool, press.id, "grain").await.unwrap();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!((hits[0].key.as_str(), hits[0].page), ("field-notes", 1));
        assert_eq!(works(pool, press.id).await.unwrap(), [stored]);
    }

    #[tokio::test]
    async fn a_new_file_takes_the_place_of_the_old_file() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let ann = member(pool, press.id, "ann@example.com").await;
        let library = Library::in_memory().unwrap();
        let work = create_work(
            pool,
            press.id,
            ann.id,
            &new_work("field-notes", "Field Notes"),
        )
        .await
        .unwrap();

        for _ in 0..2 {
            library
                .add_file(pool, press.id, work.id, ann.id, Bytes::from_static(PDF))
                .await
                .unwrap();
        }

        let files: i64 = sqlx::query_scalar("SELECT count(*) FROM library_files")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(files, 1);
        assert_eq!(
            library.search(pool, press.id, "grain").await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn a_file_that_is_not_a_pdf_file_is_refused() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let ann = member(pool, press.id, "ann@example.com").await;
        let library = Library::in_memory().unwrap();
        let work = create_work(
            pool,
            press.id,
            ann.id,
            &new_work("field-notes", "Field Notes"),
        )
        .await
        .unwrap();

        let result = library
            .add_file(
                pool,
                press.id,
                work.id,
                ann.id,
                Bytes::from_static(b"Good paper"),
            )
            .await;

        assert!(matches!(result, Err(LibraryError::NotPdf)), "{result:?}");
        assert_eq!(
            work_organization(pool, work.id).await.unwrap(),
            Some(press.id)
        );
    }

    #[tokio::test]
    async fn a_key_is_unique_in_its_organization() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let atlas = create_organization(pool, "Atlas Press").await.unwrap();
        let ann = member(pool, press.id, "ann@example.com").await;
        create_work(
            pool,
            press.id,
            ann.id,
            &new_work("field-notes", "Field Notes"),
        )
        .await
        .unwrap();

        let again = create_work(pool, press.id, ann.id, &new_work("field-notes", "Other")).await;
        let elsewhere = create_work(
            pool,
            atlas.id,
            ann.id,
            &new_work("field-notes", "Field Notes"),
        )
        .await;

        assert!(matches!(again, Err(LibraryError::KeyTaken)), "{again:?}");
        assert!(elsewhere.is_ok());
    }

    #[tokio::test]
    async fn an_empty_index_is_made_again_from_the_database() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let ann = member(pool, press.id, "ann@example.com").await;
        let work = create_work(
            pool,
            press.id,
            ann.id,
            &new_work("field-notes", "Field Notes"),
        )
        .await
        .unwrap();
        Library::in_memory()
            .unwrap()
            .add_file(pool, press.id, work.id, ann.id, Bytes::from_static(PDF))
            .await
            .unwrap();
        let library = Library::in_memory().unwrap();

        assert_eq!(library.rebuild_if_empty(pool).await.unwrap(), 1);
        assert_eq!(library.rebuild_if_empty(pool).await.unwrap(), 0);
        assert_eq!(
            library.search(pool, press.id, "dry").await.unwrap()[0].page,
            2
        );
    }

    #[tokio::test]
    async fn a_library_in_a_folder_keeps_its_index() {
        let folder = tempfile::tempdir().unwrap();

        let library = Library::open(folder.path()).unwrap();

        assert!(folder.path().join("blobs").is_dir());
        assert!(folder.path().join("index").is_dir());
        assert!(library.index.is_empty());
    }
}
