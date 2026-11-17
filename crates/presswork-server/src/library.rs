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
use presswork_api::{
    Edition, FileState, LabelStyle, LibraryFile, NewEdition, NewWork, PageRange, SearchHit, Work,
};
use presswork_library::{KeyError, page_labels, parse_key};
use presswork_ocr::{OcrEngine, OcrError, ReadError, languages_for, page_count, read_pages};
use presswork_search::{PageIndex, SearchError};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::jobs;

pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
const MAX_TITLE_CHARACTERS: usize = 500;
const MAX_NAME_CHARACTERS: usize = 200;
const MAX_NAMES: usize = 20;
const MAX_PUBLISHER_CHARACTERS: usize = 200;
const MAX_LANGUAGE_CHARACTERS: usize = 35;
const YEARS: RangeInclusive<i32> = -9999..=9999;
const SEARCH_LIMIT: usize = 20;
const UNIQUE_VIOLATION: &str = "23505";
const MAX_CORRECTIONS: usize = 100;
const MAX_PDF_RANGES: usize = 1000;
const MAX_PREFIX_CHARACTERS: usize = 20;
const MAX_START: u32 = 100_000;
const PDF_SOURCE: &str = "pdf";
const MEMBER_SOURCE: &str = "member";
pub(crate) const OCR_FAILED: &str =
    "OCR did not complete. The pages without a text layer have no text.";

type WorkRow = (Uuid, Uuid, String, String, Vec<String>);

type EditionRow = (
    Uuid,
    Uuid,
    Option<i32>,
    String,
    String,
    Vec<String>,
    bool,
    Option<i64>,
    Option<i32>,
    Option<i32>,
    Option<String>,
    Option<String>,
);

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error(transparent)]
    InvalidKey(#[from] KeyError),
    #[error("a title must have 1 to 500 characters")]
    InvalidTitle,
    #[error("a work can have 20 creators or fewer, and each name must have 1 to 200 characters")]
    InvalidCreators,
    #[error(
        "an edition can have 20 translators or fewer, and each name must have 1 to 200 characters"
    )]
    InvalidTranslators,
    #[error("the name of a publisher must have 200 characters or fewer")]
    InvalidPublisher,
    #[error("a language must have 35 characters or fewer")]
    InvalidLanguage,
    #[error("the year must be from -9999 to 9999")]
    InvalidYear,
    #[error("the organization already has a work with this citation key")]
    KeyTaken,
    #[error("the file is not a PDF file that Presswork can read")]
    NotPdf,
    #[error("the PDF file is encrypted. Upload a PDF file without a password")]
    Encrypted,
    #[error("the work or the edition does not exist")]
    NotFound,
    #[error("the edition has no file with text yet. Upload its PDF file first")]
    NoFile,
    #[error(
        "a page map can have 100 corrections or fewer. Each correction needs its own PDF page of the file, a start number from 1 to 100000, and a prefix of 20 characters or fewer"
    )]
    InvalidPageMap,
    #[error("the blob store failed: {0}")]
    Blobs(#[from] object_store::Error),
    #[error("the search index failed: {0}")]
    Search(#[from] SearchError),
    #[error("the data folder is not available: {0}")]
    Folder(#[from] std::io::Error),
    #[error("a background job stopped: {0}")]
    Job(#[from] tokio::task::JoinError),
    #[error("OCR failed: {0}")]
    Ocr(#[from] OcrError),
    #[error("the server has no OCR engine")]
    NoOcr,
    #[error("the database failed: {0}")]
    Database(#[from] sqlx::Error),
}

impl LibraryError {
    pub(crate) fn is_permanent(&self) -> bool {
        match self {
            Self::NotPdf | Self::Encrypted | Self::NoOcr => true,
            Self::Ocr(error) => error.is_permanent(),
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ValidWork {
    key: String,
    title: String,
    creators: Vec<String>,
    edition: ValidEdition,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ValidEdition {
    year: Option<i32>,
    publisher: String,
    language: String,
    translators: Vec<String>,
}

fn names(names: &[String]) -> Option<Vec<String>> {
    let names: Vec<String> = names.iter().map(|name| name.trim().to_owned()).collect();
    let valid = names.len() <= MAX_NAMES
        && names
            .iter()
            .all(|name| (1..=MAX_NAME_CHARACTERS).contains(&name.chars().count()));
    valid.then_some(names)
}

pub(crate) fn parse_edition(edition: &NewEdition) -> Result<ValidEdition, LibraryError> {
    if edition.year.is_some_and(|year| !YEARS.contains(&year)) {
        return Err(LibraryError::InvalidYear);
    }
    let publisher = edition.publisher.trim();
    if publisher.chars().count() > MAX_PUBLISHER_CHARACTERS {
        return Err(LibraryError::InvalidPublisher);
    }
    let language = edition.language.trim();
    if language.chars().count() > MAX_LANGUAGE_CHARACTERS {
        return Err(LibraryError::InvalidLanguage);
    }
    Ok(ValidEdition {
        year: edition.year,
        publisher: publisher.to_owned(),
        language: language.to_owned(),
        translators: names(&edition.translators).ok_or(LibraryError::InvalidTranslators)?,
    })
}

pub(crate) fn parse_work(work: &NewWork) -> Result<ValidWork, LibraryError> {
    let key = parse_key(&work.key)?.to_owned();
    let title = work.title.trim();
    if !(1..=MAX_TITLE_CHARACTERS).contains(&title.chars().count()) {
        return Err(LibraryError::InvalidTitle);
    }
    Ok(ValidWork {
        key,
        title: title.to_owned(),
        creators: names(&work.creators).ok_or(LibraryError::InvalidCreators)?,
        edition: parse_edition(&work.edition)?,
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
    let edition = insert_edition(&mut transaction, id, valid.edition, true).await?;
    transaction.commit().await?;
    Ok(Work {
        id,
        organization_id,
        key: valid.key,
        title: valid.title,
        creators: valid.creators,
        editions: vec![edition],
    })
}

async fn insert_edition(
    connection: &mut PgConnection,
    work_id: Uuid,
    edition: ValidEdition,
    standard: bool,
) -> Result<Edition, sqlx::Error> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO editions (id, work_id, year, publisher, language, translators, standard) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(work_id)
    .bind(edition.year)
    .bind(&edition.publisher)
    .bind(&edition.language)
    .bind(&edition.translators)
    .bind(standard)
    .execute(connection)
    .await?;
    Ok(Edition {
        id,
        year: edition.year,
        publisher: edition.publisher,
        language: edition.language,
        translators: edition.translators,
        standard,
        corrections: Vec::new(),
        file: None,
    })
}

pub async fn add_edition(
    pool: &PgPool,
    work_id: Uuid,
    new: &NewEdition,
) -> Result<Edition, LibraryError> {
    let valid = parse_edition(new)?;
    let mut connection = pool.acquire().await?;
    Ok(insert_edition(&mut connection, work_id, valid, false).await?)
}

pub async fn set_standard(pool: &PgPool, edition_id: Uuid) -> Result<Work, LibraryError> {
    let mut transaction = pool.begin().await?;
    let work_id: Uuid = sqlx::query_scalar("SELECT work_id FROM editions WHERE id = $1")
        .bind(edition_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(LibraryError::NotFound)?;
    sqlx::query("UPDATE editions SET standard = false WHERE work_id = $1 AND standard")
        .bind(work_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("UPDATE editions SET standard = true WHERE id = $1")
        .bind(edition_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    work(pool, work_id).await?.ok_or(LibraryError::NotFound)
}

pub async fn works(pool: &PgPool, organization_id: Uuid) -> Result<Vec<Work>, LibraryError> {
    let rows: Vec<WorkRow> = sqlx::query_as(
        "SELECT id, organization_id, key, title, creators FROM works \
         WHERE organization_id = $1 ORDER BY lower(title), key",
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await?;
    with_editions(pool, rows).await
}

pub async fn work(pool: &PgPool, work_id: Uuid) -> Result<Option<Work>, LibraryError> {
    let row: Option<WorkRow> =
        sqlx::query_as("SELECT id, organization_id, key, title, creators FROM works WHERE id = $1")
            .bind(work_id)
            .fetch_optional(pool)
            .await?;
    Ok(with_editions(pool, row.into_iter().collect()).await?.pop())
}

async fn with_editions(pool: &PgPool, rows: Vec<WorkRow>) -> Result<Vec<Work>, LibraryError> {
    let ids: Vec<Uuid> = rows.iter().map(|row| row.0).collect();
    let editions: Vec<EditionRow> = sqlx::query_as(
        "SELECT editions.id, editions.work_id, editions.year, editions.publisher, \
         editions.language, editions.translators, editions.standard, library_files.size, \
         library_files.pages, library_files.pages_with_text, library_files.state, \
         library_files.problem \
         FROM editions LEFT JOIN library_files ON library_files.edition_id = editions.id \
         WHERE editions.work_id = ANY($1) ORDER BY editions.created_at, editions.id",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    let edition_ids: Vec<Uuid> = editions.iter().map(|row| row.0).collect();
    let mut corrections: HashMap<Uuid, Vec<PageRange>> = HashMap::new();
    let rows_of_ranges: Vec<(Uuid, i32, String, String, i32)> = sqlx::query_as(
        "SELECT library_files.edition_id, page_ranges.first_page, page_ranges.style, \
         page_ranges.prefix, page_ranges.start FROM page_ranges \
         JOIN library_files ON library_files.id = page_ranges.file_id \
         WHERE page_ranges.source = 'member' AND library_files.edition_id = ANY($1) \
         ORDER BY page_ranges.first_page",
    )
    .bind(&edition_ids)
    .fetch_all(pool)
    .await?;
    for (edition, first_page, style, prefix, start) in rows_of_ranges {
        corrections
            .entry(edition)
            .or_default()
            .push(range_from((first_page, style, prefix, start)));
    }
    let mut by_work: HashMap<Uuid, Vec<Edition>> = HashMap::new();
    for row in editions {
        let (work_id, mut edition) = edition_from(row);
        edition.corrections = corrections.remove(&edition.id).unwrap_or_default();
        by_work.entry(work_id).or_default().push(edition);
    }
    Ok(rows
        .into_iter()
        .map(|(id, organization_id, key, title, creators)| Work {
            id,
            organization_id,
            key,
            title,
            creators,
            editions: by_work.remove(&id).unwrap_or_default(),
        })
        .collect())
}

pub async fn edition_owner(
    pool: &PgPool,
    edition_id: Uuid,
) -> Result<Option<(Uuid, Uuid)>, LibraryError> {
    Ok(sqlx::query_as(
        "SELECT works.organization_id, works.id FROM editions \
         JOIN works ON works.id = editions.work_id WHERE editions.id = $1",
    )
    .bind(edition_id)
    .fetch_optional(pool)
    .await?)
}

async fn file_owner(
    pool: &PgPool,
    file_id: Uuid,
) -> Result<Option<(Uuid, Uuid, Uuid)>, LibraryError> {
    Ok(sqlx::query_as(
        "SELECT works.organization_id, works.id, editions.id FROM library_files \
         JOIN editions ON editions.id = library_files.edition_id \
         JOIN works ON works.id = editions.work_id WHERE library_files.id = $1",
    )
    .bind(file_id)
    .fetch_optional(pool)
    .await?)
}

pub(crate) async fn mark_failed(
    pool: &PgPool,
    file_id: Uuid,
    problem: &str,
) -> Result<Option<Work>, LibraryError> {
    sqlx::query("UPDATE library_files SET state = 'failed', problem = $2 WHERE id = $1")
        .bind(file_id)
        .bind(problem)
        .execute(pool)
        .await?;
    match file_owner(pool, file_id).await? {
        Some((_, work_id, _)) => work(pool, work_id).await,
        None => Ok(None),
    }
}

pub(crate) async fn end_ocr(pool: &PgPool, file_id: Uuid) -> Result<Option<Work>, LibraryError> {
    sqlx::query(
        "UPDATE library_files SET state = 'ready', problem = $2 \
         WHERE id = $1 AND state = 'recognizing'",
    )
    .bind(file_id)
    .bind(OCR_FAILED)
    .execute(pool)
    .await?;
    match file_owner(pool, file_id).await? {
        Some((_, work_id, _)) => work(pool, work_id).await,
        None => Ok(None),
    }
}

fn range_from((first_page, style, prefix, start): (i32, String, String, i32)) -> PageRange {
    PageRange {
        first_page: u32::try_from(first_page).unwrap_or(1),
        style: LabelStyle::parse(&style).unwrap_or(LabelStyle::Arabic),
        prefix,
        start: u32::try_from(start).unwrap_or(1),
    }
}

async fn ranges_of(
    connection: &mut PgConnection,
    file_id: Uuid,
    source: &str,
) -> Result<Vec<PageRange>, sqlx::Error> {
    let rows: Vec<(i32, String, String, i32)> = sqlx::query_as(
        "SELECT first_page, style, prefix, start FROM page_ranges \
         WHERE file_id = $1 AND source = $2 ORDER BY first_page",
    )
    .bind(file_id)
    .bind(source)
    .fetch_all(connection)
    .await?;
    Ok(rows.into_iter().map(range_from).collect())
}

async fn replace_ranges(
    connection: &mut PgConnection,
    file_id: Uuid,
    source: &str,
    ranges: &[PageRange],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM page_ranges WHERE file_id = $1 AND source = $2")
        .bind(file_id)
        .bind(source)
        .execute(&mut *connection)
        .await?;
    let first_pages: Vec<i32> = ranges
        .iter()
        .map(|range| i32::try_from(range.first_page).unwrap_or(i32::MAX))
        .collect();
    let styles: Vec<&str> = ranges.iter().map(|range| range.style.as_str()).collect();
    let prefixes: Vec<&str> = ranges.iter().map(|range| range.prefix.as_str()).collect();
    let starts: Vec<i32> = ranges
        .iter()
        .map(|range| i32::try_from(range.start).unwrap_or(i32::MAX))
        .collect();
    sqlx::query(
        "INSERT INTO page_ranges (file_id, source, first_page, style, prefix, start) \
         SELECT $1, $2, range.first_page, range.style, range.prefix, range.start \
         FROM UNNEST($3::INTEGER[], $4::TEXT[], $5::TEXT[], $6::INTEGER[]) \
         AS range (first_page, style, prefix, start)",
    )
    .bind(file_id)
    .bind(source)
    .bind(&first_pages)
    .bind(&styles)
    .bind(&prefixes)
    .bind(&starts)
    .execute(connection)
    .await?;
    Ok(())
}

fn pdf_ranges(ranges: Vec<PageRange>) -> Vec<PageRange> {
    ranges
        .into_iter()
        .take(MAX_PDF_RANGES)
        .map(|range| PageRange {
            prefix: range
                .prefix
                .chars()
                .filter(|character| !character.is_control())
                .take(MAX_PREFIX_CHARACTERS)
                .collect(),
            start: range.start.min(MAX_START),
            ..range
        })
        .collect()
}

async fn store_pages(
    pool: &PgPool,
    file_id: Uuid,
    pages: &[String],
    pdf: Option<&[PageRange]>,
    state: FileState,
) -> Result<(), LibraryError> {
    let mut transaction = pool.begin().await?;
    let with_text = pages.iter().filter(|page| !page.trim().is_empty()).count();
    let updated = sqlx::query(
        "UPDATE library_files SET state = $2, problem = NULL, pages = $3, \
         pages_with_text = $4 WHERE id = $1",
    )
    .bind(file_id)
    .bind(state.as_str())
    .bind(count(pages.len()))
    .bind(count(with_text))
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(LibraryError::NotFound);
    }
    if let Some(pdf) = pdf {
        replace_ranges(&mut transaction, file_id, PDF_SOURCE, pdf).await?;
    }
    let pdf = ranges_of(&mut transaction, file_id, PDF_SOURCE).await?;
    let corrections = ranges_of(&mut transaction, file_id, MEMBER_SOURCE).await?;
    let labels = page_labels(
        &pdf,
        &corrections,
        u32::try_from(pages.len()).unwrap_or(u32::MAX),
    );
    sqlx::query("DELETE FROM library_pages WHERE file_id = $1")
        .bind(file_id)
        .execute(&mut *transaction)
        .await?;
    let numbers: Vec<i32> = (1..=count(pages.len())).collect();
    sqlx::query(
        "INSERT INTO library_pages (file_id, number, text, label) \
         SELECT $1, page.number, page.text, page.label \
         FROM UNNEST($2::INTEGER[], $3::TEXT[], $4::TEXT[]) AS page (number, text, label)",
    )
    .bind(file_id)
    .bind(&numbers)
    .bind(pages)
    .bind(&labels)
    .execute(&mut *transaction)
    .await?;
    if state == FileState::Recognizing {
        jobs::add(&mut transaction, jobs::OCR, file_id).await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn work_organization(pool: &PgPool, work_id: Uuid) -> Result<Option<Uuid>, LibraryError> {
    Ok(
        sqlx::query_scalar("SELECT organization_id FROM works WHERE id = $1")
            .bind(work_id)
            .fetch_optional(pool)
            .await?,
    )
}

fn edition_from(row: EditionRow) -> (Uuid, Edition) {
    let (
        id,
        work_id,
        year,
        publisher,
        language,
        translators,
        standard,
        size,
        pages,
        pages_with_text,
        state,
        problem,
    ) = row;
    let file = match (size, pages, pages_with_text, state) {
        (Some(size), Some(pages), Some(pages_with_text), Some(state)) => Some(LibraryFile {
            size: u64::try_from(size).unwrap_or_default(),
            pages: u32::try_from(pages).unwrap_or_default(),
            pages_with_text: u32::try_from(pages_with_text).unwrap_or_default(),
            state: FileState::parse(&state).unwrap_or(FileState::Failed),
            problem,
        }),
        _ => None,
    };
    let edition = Edition {
        id,
        year,
        publisher,
        language,
        translators,
        standard,
        corrections: Vec::new(),
        file,
    };
    (work_id, edition)
}

pub(crate) fn parse_page_map(corrections: &[PageRange], pages: u32) -> Result<(), LibraryError> {
    let mut first_pages: Vec<u32> = corrections.iter().map(|range| range.first_page).collect();
    first_pages.sort_unstable();
    first_pages.dedup();
    let valid = corrections.len() <= MAX_CORRECTIONS
        && first_pages.len() == corrections.len()
        && corrections.iter().all(|range| {
            (1..=pages).contains(&range.first_page)
                && (1..=MAX_START).contains(&range.start)
                && range.prefix.chars().count() <= MAX_PREFIX_CHARACTERS
                && !range.prefix.chars().any(char::is_control)
        });
    if valid {
        Ok(())
    } else {
        Err(LibraryError::InvalidPageMap)
    }
}

pub async fn set_page_map(
    pool: &PgPool,
    edition_id: Uuid,
    corrections: &[PageRange],
) -> Result<Work, LibraryError> {
    let mut transaction = pool.begin().await?;
    let (file_id, pages, work_id): (Uuid, i32, Uuid) = sqlx::query_as(
        "SELECT library_files.id, library_files.pages, editions.work_id FROM library_files \
         JOIN editions ON editions.id = library_files.edition_id \
         WHERE library_files.edition_id = $1 AND library_files.state IN ('ready', 'recognizing') \
         FOR UPDATE OF library_files",
    )
    .bind(edition_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(LibraryError::NoFile)?;
    let pages = u32::try_from(pages).unwrap_or_default();
    parse_page_map(corrections, pages)?;
    replace_ranges(&mut transaction, file_id, MEMBER_SOURCE, corrections).await?;
    let pdf = ranges_of(&mut transaction, file_id, PDF_SOURCE).await?;
    let labels = page_labels(&pdf, corrections, pages);
    let numbers: Vec<i32> = (1..=count(labels.len())).collect();
    sqlx::query(
        "UPDATE library_pages SET label = page.label \
         FROM UNNEST($2::INTEGER[], $3::TEXT[]) AS page (number, label) \
         WHERE library_pages.file_id = $1 AND library_pages.number = page.number",
    )
    .bind(file_id)
    .bind(&numbers)
    .bind(&labels)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    work(pool, work_id).await?.ok_or(LibraryError::NotFound)
}

struct NewFile<'a> {
    id: Uuid,
    size: i64,
    digest: &'a [u8],
    pages: u32,
}

async fn attach_file(
    pool: &PgPool,
    edition: Uuid,
    created_by: Uuid,
    file: &NewFile<'_>,
) -> Result<Option<Uuid>, LibraryError> {
    let mut transaction = pool.begin().await?;
    let replaced: Option<Uuid> =
        sqlx::query_scalar("DELETE FROM library_files WHERE edition_id = $1 RETURNING id")
            .bind(edition)
            .fetch_optional(&mut *transaction)
            .await?;
    sqlx::query(
        "INSERT INTO library_files \
         (id, edition_id, size, sha256, pages, pages_with_text, state, created_by) \
         VALUES ($1, $2, $3, $4, $5, 0, 'reading', $6)",
    )
    .bind(file.id)
    .bind(edition)
    .bind(file.size)
    .bind(file.digest)
    .bind(i32::try_from(file.pages).unwrap_or(i32::MAX))
    .bind(created_by)
    .execute(&mut *transaction)
    .await?;
    jobs::add(&mut transaction, jobs::READ_TEXT, file.id).await?;
    transaction.commit().await?;
    Ok(replaced)
}

fn read_error(error: ReadError) -> LibraryError {
    match error {
        ReadError::Invalid => LibraryError::NotPdf,
        ReadError::Encrypted => LibraryError::Encrypted,
    }
}

fn count(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn blob_path(file: Uuid) -> BlobPath {
    BlobPath::from(format!("library/{file}.pdf"))
}

fn ocr_path(file: Uuid) -> BlobPath {
    BlobPath::from(format!("library/{file}.ocr.pdf"))
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
    ocr: Option<Arc<dyn OcrEngine>>,
    ocr_languages: Vec<String>,
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
            ocr: None,
            ocr_languages: Vec::new(),
        })
    }

    pub fn in_memory() -> Result<Self, LibraryError> {
        Ok(Self {
            blobs: Arc::new(InMemory::new()),
            index: Arc::new(PageIndex::in_memory()?),
            ocr: None,
            ocr_languages: Vec::new(),
        })
    }

    #[must_use]
    pub fn with_ocr(self, engine: Arc<dyn OcrEngine>, fallback: Vec<String>) -> Self {
        Self {
            ocr: Some(engine),
            ocr_languages: fallback,
            ..self
        }
    }

    pub async fn add_file(
        &self,
        pool: &PgPool,
        (organization, work_id, edition): (Uuid, Uuid, Uuid),
        created_by: Uuid,
        file: Bytes,
    ) -> Result<Work, LibraryError> {
        let reader = file.clone();
        let pages = tokio::task::spawn_blocking(move || page_count(reader))
            .await
            .map_err(|_| LibraryError::NotPdf)?
            .map_err(read_error)?;
        let digest = Sha256::digest(&file);
        let new = NewFile {
            id: Uuid::now_v7(),
            size: i64::try_from(file.len()).unwrap_or(i64::MAX),
            digest: digest.as_slice(),
            pages,
        };
        let location = blob_path(new.id);
        self.blobs.put(&location, PutPayload::from(file)).await?;
        let replaced = match attach_file(pool, edition, created_by, &new).await {
            Ok(replaced) => replaced,
            Err(error) => {
                if let Err(error) = self.blobs.delete(&location).await {
                    tracing::warn!(%error, "A library file without a record was not deleted");
                }
                return Err(error);
            }
        };
        if let Some(old) = replaced {
            for location in [blob_path(old), ocr_path(old)] {
                match self.blobs.delete(&location).await {
                    Ok(()) | Err(object_store::Error::NotFound { .. }) => {}
                    Err(error) => tracing::warn!(%error, "An old library file was not deleted"),
                }
            }
        }
        let owner = (organization, work_id, edition);
        if let Err(error) = self.index_pages(owner, Vec::new()).await {
            tracing::error!(%error, "The old pages of a library file stay in the search index");
        }
        work(pool, work_id).await?.ok_or(LibraryError::NotFound)
    }

    pub(crate) async fn read_text(
        &self,
        pool: &PgPool,
        file_id: Uuid,
    ) -> Result<Option<Work>, LibraryError> {
        let Some(owner) = file_owner(pool, file_id).await? else {
            return Ok(None);
        };
        let bytes = self.blobs.get(&blob_path(file_id)).await?.bytes().await?;
        let read = tokio::task::spawn_blocking(move || read_pages(bytes))
            .await
            .map_err(|_| LibraryError::NotPdf)?
            .map_err(read_error)?;
        let pages = readable(read.texts);
        let state = if self.ocr.is_some() && pages.iter().any(|page| page.trim().is_empty()) {
            FileState::Recognizing
        } else {
            FileState::Ready
        };
        let ranges = pdf_ranges(read.labels);
        store_pages(pool, file_id, &pages, Some(&ranges), state).await?;
        self.index_pages(owner, pages).await?;
        work(pool, owner.1).await
    }

    pub(crate) async fn recognize(
        &self,
        pool: &PgPool,
        file_id: Uuid,
    ) -> Result<Option<Work>, LibraryError> {
        let Some(owner) = file_owner(pool, file_id).await? else {
            return Ok(None);
        };
        let engine = self.ocr.clone().ok_or(LibraryError::NoOcr)?;
        let language: String = sqlx::query_scalar("SELECT language FROM editions WHERE id = $1")
            .bind(owner.2)
            .fetch_one(pool)
            .await?;
        let languages = languages_for(&language, engine.languages(), &self.ocr_languages);
        let bytes = self.blobs.get(&blob_path(file_id)).await?.bytes().await?;
        let recognized =
            tokio::task::spawn_blocking(move || engine.recognize(&bytes, &languages)).await??;
        let pages = readable(recognized.texts);
        store_pages(pool, file_id, &pages, None, FileState::Ready).await?;
        if let Some(copy) = recognized.pdf
            && let Err(error) = self
                .blobs
                .put(&ocr_path(file_id), PutPayload::from(copy))
                .await
        {
            tracing::warn!(%error, "The copy of a library file with its OCR text was not kept");
        }
        self.index_pages(owner, pages).await?;
        work(pool, owner.1).await
    }

    async fn index_pages(
        &self,
        (organization, work_id, edition): (Uuid, Uuid, Uuid),
        pages: Vec<String>,
    ) -> Result<(), LibraryError> {
        let index = Arc::clone(&self.index);
        tokio::task::spawn_blocking(move || index.put(organization, work_id, edition, &pages))
            .await??;
        Ok(())
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
        let work_ids: Vec<Uuid> = hits.iter().map(|hit| hit.work).collect();
        let names: HashMap<Uuid, (String, String)> = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT id, key, title FROM works WHERE id = ANY($1)",
        )
        .bind(&work_ids)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|(id, key, title)| (id, (key, title)))
        .collect();
        let edition_ids: Vec<Uuid> = hits.iter().map(|hit| hit.edition).collect();
        let standards: HashMap<Uuid, bool> = sqlx::query_as::<_, (Uuid, bool)>(
            "SELECT id, standard FROM editions WHERE id = ANY($1)",
        )
        .bind(&edition_ids)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect();
        let numbers: Vec<i32> = hits
            .iter()
            .map(|hit| i32::try_from(hit.page).unwrap_or(i32::MAX))
            .collect();
        let labels: HashMap<(Uuid, i32), String> = sqlx::query_as::<_, (Uuid, i32, String)>(
            "SELECT library_files.edition_id, library_pages.number, library_pages.label \
                 FROM library_pages \
                 JOIN library_files ON library_files.id = library_pages.file_id \
                 JOIN UNNEST($1::UUID[], $2::INTEGER[]) AS hit (edition, number) \
                 ON hit.edition = library_files.edition_id AND hit.number = library_pages.number",
        )
        .bind(&edition_ids)
        .bind(&numbers)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|(edition, number, label)| ((edition, number), label))
        .collect();
        Ok(hits
            .into_iter()
            .filter_map(|hit| {
                let (key, title) = names.get(&hit.work)?;
                let number = i32::try_from(hit.page).ok()?;
                let label = labels
                    .get(&(hit.edition, number))
                    .filter(|label| !label.is_empty())
                    .cloned()
                    .unwrap_or_else(|| hit.page.to_string());
                Some(SearchHit {
                    work_id: hit.work,
                    edition_id: hit.edition,
                    key: key.clone(),
                    title: title.clone(),
                    page: hit.page,
                    label,
                    snippet: hit.snippet,
                    standard: *standards.get(&hit.edition)?,
                })
            })
            .collect())
    }

    pub async fn rebuild_if_empty(&self, pool: &PgPool) -> Result<usize, LibraryError> {
        if !self.index.is_empty() {
            return Ok(0);
        }
        let rows: Vec<(Uuid, Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT works.organization_id, works.id, editions.id, library_pages.text \
             FROM library_pages \
             JOIN library_files ON library_files.id = library_pages.file_id \
             JOIN editions ON editions.id = library_files.edition_id \
             JOIN works ON works.id = editions.work_id \
             ORDER BY editions.id, library_pages.number",
        )
        .fetch_all(pool)
        .await?;
        let mut editions: Vec<((Uuid, Uuid, Uuid), Vec<String>)> = Vec::new();
        for (organization, work, edition, text) in rows {
            match editions.last_mut() {
                Some(((_, _, last), pages)) if *last == edition => pages.push(text),
                _ => editions.push(((organization, work, edition), vec![text])),
            }
        }
        let count = editions.len();
        let index = Arc::clone(&self.index);
        tokio::task::spawn_blocking(move || {
            editions
                .iter()
                .try_for_each(|((organization, work, edition), pages)| {
                    index.put(*organization, *work, *edition, pages)
                })
        })
        .await??;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use presswork_api::Role;
    use presswork_ocr::Recognized;

    use super::*;
    use crate::accounts::{User, create_user};
    use crate::links::create_organization;
    use crate::routes::AppState;
    use crate::testing::test_database;

    const PDF: &[u8] = include_bytes!("../../presswork-ocr/fixtures/typst.pdf");

    fn new_work(key: &str, title: &str) -> NewWork {
        NewWork {
            key: key.to_owned(),
            title: title.to_owned(),
            creators: vec!["Gutenberg, Johannes".to_owned()],
            edition: NewEdition {
                year: Some(1455),
                ..NewEdition::default()
            },
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
            edition: NewEdition {
                year: Some(-500),
                publisher: " Field Press ".to_owned(),
                language: " en ".to_owned(),
                translators: vec![" Maude, Louise ".to_owned()],
            },
        };

        assert_eq!(
            parse_work(&work).unwrap(),
            ValidWork {
                key: "field-notes".to_owned(),
                title: "Field Notes".to_owned(),
                creators: vec!["Gutenberg, Johannes".to_owned()],
                edition: ValidEdition {
                    year: Some(-500),
                    publisher: "Field Press".to_owned(),
                    language: "en".to_owned(),
                    translators: vec!["Maude, Louise".to_owned()],
                },
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
                    edition: NewEdition {
                        year: Some(10_000),
                        ..NewEdition::default()
                    },
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
    fn a_new_edition_needs_valid_values() {
        let edition = |change: fn(&mut NewEdition)| {
            let mut edition = NewEdition::default();
            change(&mut edition);
            parse_edition(&edition)
        };

        assert!(matches!(
            edition(|edition| edition.year = Some(-10_000)),
            Err(LibraryError::InvalidYear)
        ));
        assert!(matches!(
            edition(|edition| edition.publisher = "x".repeat(201)),
            Err(LibraryError::InvalidPublisher)
        ));
        assert!(matches!(
            edition(|edition| edition.language = "x".repeat(36)),
            Err(LibraryError::InvalidLanguage)
        ));
        assert!(matches!(
            edition(|edition| edition.translators = vec![" ".to_owned()]),
            Err(LibraryError::InvalidTranslators)
        ));
        assert!(edition(|_| {}).is_ok());
    }

    fn correction(first_page: u32, start: u32, prefix: &str) -> PageRange {
        PageRange {
            first_page,
            style: LabelStyle::Arabic,
            prefix: prefix.to_owned(),
            start,
        }
    }

    #[test]
    fn a_page_map_needs_valid_corrections() {
        assert!(parse_page_map(&[correction(1, 1, ""), correction(5, 3, "A-")], 5).is_ok());
        for corrections in [
            vec![correction(6, 1, "")],
            vec![correction(0, 1, "")],
            vec![correction(1, 0, "")],
            vec![correction(1, 100_001, "")],
            vec![correction(1, 1, &"x".repeat(21))],
            vec![correction(1, 1, "A\0")],
            vec![correction(1, 1, "A\n")],
            vec![correction(2, 1, ""), correction(2, 4, "")],
        ] {
            assert!(
                matches!(
                    parse_page_map(&corrections, 5),
                    Err(LibraryError::InvalidPageMap)
                ),
                "{corrections:?}"
            );
        }
        let many: Vec<PageRange> = (1..=101).map(|page| correction(page, 1, "")).collect();
        assert!(parse_page_map(&many[..100], 200).is_ok());
        assert!(matches!(
            parse_page_map(&many, 200),
            Err(LibraryError::InvalidPageMap)
        ));
    }

    #[test]
    fn page_ranges_of_a_pdf_file_lose_control_characters() {
        assert_eq!(
            pdf_ranges(vec![correction(1, 1, "A\0-\u{7}")]),
            [correction(1, 1, "A-")]
        );
    }

    #[test]
    fn only_problems_of_the_file_stop_a_job_at_once() {
        assert!(LibraryError::NotPdf.is_permanent());
        assert!(LibraryError::Encrypted.is_permanent());
        assert!(LibraryError::NoOcr.is_permanent());
        assert!(LibraryError::Ocr(OcrError::File("signed".to_owned())).is_permanent());
        assert!(
            !LibraryError::Ocr(OcrError::Engine("Tesseract stopped".to_owned())).is_permanent()
        );
        assert!(!LibraryError::NotFound.is_permanent());
    }

    #[test]
    fn page_texts_lose_null_characters() {
        assert_eq!(
            readable(vec!["Good\0 paper".to_owned()]),
            ["Good paper".to_owned()]
        );
    }

    async fn run_all(state: &AppState) {
        while jobs::run_one(state).await.unwrap() {}
    }

    struct Press {
        state: AppState,
        ann: Uuid,
        work: Work,
        owner: (Uuid, Uuid, Uuid),
    }

    async fn press_with(pool: &PgPool, library: Library, new: &NewWork) -> Press {
        let press = create_organization(pool, "Field Press").await.unwrap();
        let ann = member(pool, press.id, "ann@example.com").await;
        let work = create_work(pool, press.id, ann.id, new).await.unwrap();
        let owner = (press.id, work.id, work.editions[0].id);
        Press {
            state: AppState::new(pool.clone(), library),
            ann: ann.id,
            work,
            owner,
        }
    }

    async fn press_with_work(pool: &PgPool) -> Press {
        press_with(
            pool,
            Library::in_memory().unwrap(),
            &new_work("field-notes", "Field Notes"),
        )
        .await
    }

    fn state_of(work: &Work) -> Option<(FileState, u32, Option<&str>)> {
        work.editions
            .first()?
            .file
            .as_ref()
            .map(|file| (file.state, file.pages_with_text, file.problem.as_deref()))
    }

    struct FakeEngine {
        installed: Vec<String>,
        texts: Result<Vec<String>, String>,
        calls: std::sync::Mutex<Vec<Vec<String>>>,
    }

    impl FakeEngine {
        fn new(texts: Result<Vec<String>, String>) -> Arc<Self> {
            Arc::new(Self {
                installed: vec!["deu".to_owned(), "eng".to_owned()],
                texts,
                calls: std::sync::Mutex::default(),
            })
        }

        fn reading(texts: &[&str]) -> Arc<Self> {
            Self::new(Ok(texts.iter().map(|&text| text.to_owned()).collect()))
        }

        fn calls(&self) -> Vec<Vec<String>> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl OcrEngine for FakeEngine {
        fn languages(&self) -> &[String] {
            &self.installed
        }

        fn recognize(&self, pdf: &[u8], languages: &[String]) -> Result<Recognized, OcrError> {
            self.calls.lock().unwrap().push(languages.to_vec());
            match &self.texts {
                Ok(texts) => Ok(Recognized {
                    pdf: Some(pdf.to_vec()),
                    texts: texts.clone(),
                }),
                Err(reason) => Err(OcrError::File(reason.clone())),
            }
        }
    }

    fn scan() -> Bytes {
        Bytes::from(presswork_ocr::testing::pdf(&[
            String::new(),
            presswork_ocr::testing::lines(&["The ink must dry."]),
        ]))
    }

    async fn press_with_ocr(pool: &PgPool, engine: &Arc<FakeEngine>, language: &str) -> Press {
        let mut new = new_work("field-notes", "Field Notes");
        new.edition.language = language.to_owned();
        let library = Library::in_memory()
            .unwrap()
            .with_ocr(engine.clone(), vec!["eng".to_owned()]);
        press_with(pool, library, &new).await
    }

    async fn ocr_jobs(pool: &PgPool) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE kind = 'ocr'")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn ocr_gives_the_text_of_a_scanned_page_in_the_language_of_the_edition() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let engine = FakeEngine::reading(&["Good paper gives a good page.", "The ink must dry."]);
        let press = press_with_ocr(pool, &engine, "de-AT").await;
        let (organization, _, edition) = press.owner;
        let library = &press.state.library;
        library
            .add_file(pool, press.owner, press.ann, scan())
            .await
            .unwrap();

        assert!(jobs::run_one(&press.state).await.unwrap());
        let recognizing = works(pool, organization).await.unwrap();
        assert_eq!(
            state_of(&recognizing[0]),
            Some((FileState::Recognizing, 1, None))
        );
        assert_eq!(engine.calls(), Vec::<Vec<String>>::new());
        set_page_map(pool, edition, &[correction(2, 7, "")])
            .await
            .unwrap();

        run_all(&press.state).await;

        let ready = works(pool, organization).await.unwrap();
        assert_eq!(state_of(&ready[0]), Some((FileState::Ready, 2, None)));
        assert_eq!(engine.calls(), [vec!["deu".to_owned()]]);
        let hits = library.search(pool, organization, "paper").await.unwrap();
        assert_eq!(
            hits.iter()
                .map(|hit| (hit.page, hit.label.as_str()))
                .collect::<Vec<_>>(),
            [(1, "1")]
        );
        let dry = library.search(pool, organization, "dry").await.unwrap();
        assert_eq!(dry[0].label, "7");
        let file: Uuid = sqlx::query_scalar("SELECT id FROM library_files")
            .fetch_one(pool)
            .await
            .unwrap();
        assert!(library.blobs.head(&ocr_path(file)).await.is_ok());
        assert!(library.blobs.head(&blob_path(file)).await.is_ok());

        library
            .add_file(pool, press.owner, press.ann, Bytes::from_static(PDF))
            .await
            .unwrap();
        assert!(library.blobs.head(&ocr_path(file)).await.is_err());
    }

    #[tokio::test]
    async fn a_file_keeps_the_text_of_its_text_layer_when_ocr_fails() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let engine = FakeEngine::new(Err("the file has a digital signature".to_owned()));
        let press = press_with_ocr(pool, &engine, "").await;
        let (organization, _, _) = press.owner;
        press
            .state
            .library
            .add_file(pool, press.owner, press.ann, scan())
            .await
            .unwrap();

        run_all(&press.state).await;

        let ready = works(pool, organization).await.unwrap();
        assert_eq!(
            state_of(&ready[0]),
            Some((FileState::Ready, 1, Some(OCR_FAILED)))
        );
        assert_eq!(engine.calls(), [vec!["eng".to_owned()]]);
        let hits = press
            .state
            .library
            .search(pool, organization, "dry")
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[tokio::test]
    async fn without_an_ocr_engine_a_scanned_page_has_no_text() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = press_with_work(pool).await;
        let (organization, _, _) = press.owner;
        press
            .state
            .library
            .add_file(pool, press.owner, press.ann, scan())
            .await
            .unwrap();

        run_all(&press.state).await;

        let ready = works(pool, organization).await.unwrap();
        assert_eq!(state_of(&ready[0]), Some((FileState::Ready, 1, None)));
        assert_eq!(ocr_jobs(pool).await, 0);
    }

    #[tokio::test]
    async fn a_file_with_text_on_all_pages_gets_no_ocr() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let engine = FakeEngine::reading(&[]);
        let press = press_with_ocr(pool, &engine, "en").await;
        let (organization, _, _) = press.owner;
        press
            .state
            .library
            .add_file(pool, press.owner, press.ann, Bytes::from_static(PDF))
            .await
            .unwrap();

        run_all(&press.state).await;

        let ready = works(pool, organization).await.unwrap();
        assert_eq!(state_of(&ready[0]), Some((FileState::Ready, 2, None)));
        assert_eq!(engine.calls(), Vec::<Vec<String>>::new());
        assert_eq!(ocr_jobs(pool).await, 0);
    }

    #[tokio::test]
    async fn a_job_reads_the_pages_and_then_the_search_finds_them() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = press_with_work(pool).await;
        let (organization, _, _) = press.owner;

        let stored = press
            .state
            .library
            .add_file(pool, press.owner, press.ann, Bytes::from_static(PDF))
            .await
            .unwrap();

        assert_eq!(
            stored.editions[0].file,
            Some(LibraryFile {
                size: u64::try_from(PDF.len()).unwrap(),
                pages: 2,
                pages_with_text: 0,
                state: FileState::Reading,
                problem: None,
            })
        );
        let library = &press.state.library;
        assert_eq!(
            library.search(pool, organization, "grain").await.unwrap(),
            []
        );
        run_all(&press.state).await;
        let ready = works(pool, organization).await.unwrap();
        assert_eq!(state_of(&ready[0]), Some((FileState::Ready, 2, None)));
        let hits = library.search(pool, organization, "grain").await.unwrap();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            (hits[0].key.as_str(), hits[0].page, hits[0].standard),
            ("field-notes", 1, true)
        );
    }

    #[tokio::test]
    async fn corrections_give_the_printed_page_numbers_of_the_search() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = press_with_work(pool).await;
        let (organization, _, edition) = press.owner;
        press
            .state
            .library
            .add_file(pool, press.owner, press.ann, Bytes::from_static(PDF))
            .await
            .unwrap();
        run_all(&press.state).await;
        let label = || async {
            press
                .state
                .library
                .search(pool, organization, "dry")
                .await
                .unwrap()[0]
                .label
                .clone()
        };
        assert_eq!(label().await, "2");

        let corrected = set_page_map(
            pool,
            edition,
            &[PageRange {
                style: LabelStyle::LowerRoman,
                ..correction(2, 1, "")
            }],
        )
        .await
        .unwrap();

        assert_eq!(corrected.editions[0].corrections.len(), 1);
        assert_eq!(label().await, "i");
        set_page_map(pool, edition, &[]).await.unwrap();
        assert_eq!(label().await, "2");
    }

    #[tokio::test]
    async fn a_page_map_needs_a_file_with_text() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = press_with_work(pool).await;
        let (_, _, edition) = press.owner;

        let early = set_page_map(pool, edition, &[correction(1, 1, "")]).await;
        press
            .state
            .library
            .add_file(pool, press.owner, press.ann, Bytes::from_static(PDF))
            .await
            .unwrap();
        run_all(&press.state).await;
        let outside = set_page_map(pool, edition, &[correction(3, 1, "")]).await;

        assert!(matches!(early, Err(LibraryError::NoFile)), "{early:?}");
        assert!(
            matches!(outside, Err(LibraryError::InvalidPageMap)),
            "{outside:?}"
        );
    }

    #[tokio::test]
    async fn a_work_gets_more_editions_and_one_standard_edition() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = press_with_work(pool).await;
        assert!(press.work.editions[0].standard);

        let added = add_edition(
            pool,
            press.work.id,
            &NewEdition {
                year: Some(1922),
                publisher: "Oxford".to_owned(),
                language: "en".to_owned(),
                translators: vec!["Maude, Louise".to_owned()],
            },
        )
        .await
        .unwrap();
        assert!(!added.standard);
        let changed = set_standard(pool, added.id).await.unwrap();

        let standards: Vec<(Uuid, bool)> = changed
            .editions
            .iter()
            .map(|edition| (edition.id, edition.standard))
            .collect();
        assert_eq!(
            standards,
            [(press.work.editions[0].id, false), (added.id, true)]
        );
        assert_eq!(changed.standard_edition().unwrap().publisher, "Oxford");
        assert_eq!(works(pool, press.owner.0).await.unwrap(), [changed]);
        assert_eq!(
            edition_owner(pool, added.id).await.unwrap(),
            Some((press.owner.0, press.work.id))
        );
    }

    #[tokio::test]
    async fn each_edition_has_its_own_file_and_pages() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = press_with_work(pool).await;
        let second = add_edition(pool, press.work.id, &NewEdition::default())
            .await
            .unwrap();

        for owner in [press.owner, (press.owner.0, press.work.id, second.id)] {
            press
                .state
                .library
                .add_file(pool, owner, press.ann, Bytes::from_static(PDF))
                .await
                .unwrap();
        }
        run_all(&press.state).await;

        let mut hits: Vec<(Uuid, bool)> = press
            .state
            .library
            .search(pool, press.owner.0, "grain")
            .await
            .unwrap()
            .into_iter()
            .map(|hit| (hit.edition_id, hit.standard))
            .collect();
        hits.sort();
        let mut expected = vec![(press.owner.2, true), (second.id, false)];
        expected.sort();
        assert_eq!(hits, expected);
    }

    #[tokio::test]
    async fn a_new_file_takes_the_place_of_the_old_file() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = press_with_work(pool).await;

        for _ in 0..2 {
            press
                .state
                .library
                .add_file(pool, press.owner, press.ann, Bytes::from_static(PDF))
                .await
                .unwrap();
        }
        run_all(&press.state).await;

        let files: i64 = sqlx::query_scalar("SELECT count(*) FROM library_files")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(files, 1);
        assert_eq!(
            state_of(&works(pool, press.owner.0).await.unwrap()[0]),
            Some((FileState::Ready, 2, None))
        );
        assert_eq!(
            press
                .state
                .library
                .search(pool, press.owner.0, "grain")
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn a_member_who_joined_hears_when_the_pages_are_ready() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = press_with_work(pool).await;
        let (outbox, mut inbox) = tokio::sync::mpsc::unbounded_channel();
        press
            .state
            .hub
            .join(press.owner.0, 1, outbox, || async { Ok(Vec::new()) })
            .await
            .unwrap();
        let _ = inbox.try_recv();

        press
            .state
            .library
            .add_file(pool, press.owner, press.ann, Bytes::from_static(PDF))
            .await
            .unwrap();
        run_all(&press.state).await;

        let told = inbox.try_recv().unwrap();
        assert!(
            matches!(
                &told,
                crate::hub::Outgoing::Message(presswork_api::live::ServerMessage::WorkChanged { work: changed })
                    if state_of(changed) == Some((FileState::Ready, 2, None))
            ),
            "{told:?}"
        );
    }

    #[tokio::test]
    async fn a_file_that_the_job_cannot_read_fails_after_three_attempts() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = press_with_work(pool).await;
        press
            .state
            .library
            .add_file(pool, press.owner, press.ann, Bytes::from_static(PDF))
            .await
            .unwrap();
        let file: Uuid = sqlx::query_scalar("SELECT id FROM library_files")
            .fetch_one(pool)
            .await
            .unwrap();
        press
            .state
            .library
            .blobs
            .delete(&blob_path(file))
            .await
            .unwrap();

        for _ in 0..3 {
            assert!(jobs::run_one(&press.state).await.unwrap());
            sqlx::query("UPDATE jobs SET run_after = now()")
                .execute(pool)
                .await
                .unwrap();
        }

        assert!(!jobs::run_one(&press.state).await.unwrap());
        assert_eq!(
            state_of(&works(pool, press.owner.0).await.unwrap()[0]),
            Some((
                FileState::Failed,
                0,
                Some("Presswork cannot read the pages of the file. Upload the file again.")
            ))
        );
    }

    #[tokio::test]
    async fn a_file_that_is_not_a_pdf_file_is_refused() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = press_with_work(pool).await;

        let result = press
            .state
            .library
            .add_file(
                pool,
                press.owner,
                press.ann,
                Bytes::from_static(b"Good paper"),
            )
            .await;

        assert!(matches!(result, Err(LibraryError::NotPdf)), "{result:?}");
        assert_eq!(
            work_organization(pool, press.work.id).await.unwrap(),
            Some(press.owner.0)
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
        let press = press_with_work(pool).await;
        press
            .state
            .library
            .add_file(pool, press.owner, press.ann, Bytes::from_static(PDF))
            .await
            .unwrap();
        run_all(&press.state).await;
        let library = Library::in_memory().unwrap();

        assert_eq!(library.rebuild_if_empty(pool).await.unwrap(), 1);
        assert_eq!(library.rebuild_if_empty(pool).await.unwrap(), 0);
        assert_eq!(
            library.search(pool, press.owner.0, "dry").await.unwrap()[0].page,
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
