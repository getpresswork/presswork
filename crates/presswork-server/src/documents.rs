use presswork_api::Document;
use sqlx::PgPool;
use uuid::Uuid;

const MAX_PATH_CHARACTERS: usize = 500;
const UNIQUE_VIOLATION: &str = "23505";
const FORBIDDEN_CHARACTERS: &[char] = &['\\', ':', '*', '?', '"', '<', '>', '|'];

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error(
        "a path must be relative, have 1 to 500 characters, and use / between its parts. \
         The characters \\ : * ? \" < > | are not possible"
    )]
    InvalidPath,
    #[error("the project has a document with this path")]
    PathTaken,
    #[error("the database failed: {0}")]
    Database(#[from] sqlx::Error),
}

pub(crate) fn parse_path(path: &str) -> Result<&str, DocumentError> {
    let valid = (1..=MAX_PATH_CHARACTERS).contains(&path.chars().count())
        && !path.contains(FORBIDDEN_CHARACTERS)
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".." && part.trim() == part);
    if valid {
        Ok(path)
    } else {
        Err(DocumentError::InvalidPath)
    }
}

pub async fn create_document(
    pool: &PgPool,
    project_id: Uuid,
    created_by: Uuid,
    path: &str,
) -> Result<Document, DocumentError> {
    let document = Document {
        id: Uuid::now_v7(),
        project_id,
        path: parse_path(path)?.to_owned(),
    };
    sqlx::query("INSERT INTO documents (id, project_id, path, created_by) VALUES ($1, $2, $3, $4)")
        .bind(document.id)
        .bind(project_id)
        .bind(&document.path)
        .bind(created_by)
        .execute(pool)
        .await
        .map_err(|error| match &error {
            sqlx::Error::Database(database)
                if database.code().as_deref() == Some(UNIQUE_VIOLATION) =>
            {
                DocumentError::PathTaken
            }
            _ => DocumentError::Database(error),
        })?;
    Ok(document)
}

pub async fn documents(pool: &PgPool, project_id: Uuid) -> Result<Vec<Document>, DocumentError> {
    let rows: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, path FROM documents WHERE project_id = $1 ORDER BY path")
            .bind(project_id)
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, path)| Document {
            id,
            project_id,
            path,
        })
        .collect())
}

pub async fn document_organization(
    pool: &PgPool,
    document_id: Uuid,
) -> Result<Option<Uuid>, DocumentError> {
    Ok(sqlx::query_scalar(
        "SELECT projects.organization_id FROM documents \
         JOIN projects ON projects.id = documents.project_id WHERE documents.id = $1",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?)
}

pub async fn append_update(
    pool: &PgPool,
    document_id: Uuid,
    author_id: Uuid,
    data: &[u8],
) -> Result<(), DocumentError> {
    sqlx::query("INSERT INTO document_updates (document_id, data, author_id) VALUES ($1, $2, $3)")
        .bind(document_id)
        .bind(data)
        .bind(author_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn updates(pool: &PgPool, document_id: Uuid) -> Result<Vec<Vec<u8>>, DocumentError> {
    Ok(sqlx::query_scalar(
        "SELECT data FROM document_updates WHERE document_id = $1 ORDER BY sequence",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?)
}

#[cfg(test)]
mod tests {
    use presswork_api::Role;

    use super::*;
    use crate::accounts::{User, create_user};
    use crate::links::create_organization;
    use crate::testing::test_database;
    use crate::work::create_project;

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
    fn a_path_is_relative_with_slashes_between_its_parts() {
        for path in ["main.typ", "chapters/one.typ", "Größe/ünd.typ"] {
            assert_eq!(parse_path(path).unwrap(), path);
        }
        for path in [
            "",
            "/main.typ",
            "chapters//one.typ",
            "chapters/",
            "../main.typ",
            "chapters/./one.typ",
            "chapters\\one.typ",
            "c:main.typ",
            "what?.typ",
            " main.typ",
            "main.typ ",
            "line\nbreak.typ",
            &"x".repeat(501),
        ] {
            assert!(
                matches!(parse_path(path), Err(DocumentError::InvalidPath)),
                "{path:?}"
            );
        }
    }

    #[tokio::test]
    async fn documents_belong_to_their_project() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let ann = member(pool, press.id, "ann@example.com").await;
        let notes = create_project(pool, press.id, "Field Notes").await.unwrap();
        let atlas = create_project(pool, press.id, "Atlas").await.unwrap();

        let main = create_document(pool, notes.id, ann.id, "main.typ")
            .await
            .unwrap();
        let chapter = create_document(pool, notes.id, ann.id, "chapters/one.typ")
            .await
            .unwrap();

        assert_eq!(main.project_id, notes.id);
        assert_eq!(main.path, "main.typ");
        assert_eq!(documents(pool, notes.id).await.unwrap(), [chapter, main]);
        assert_eq!(documents(pool, atlas.id).await.unwrap(), []);
        assert_eq!(
            document_organization(pool, notes.id).await.unwrap(),
            None,
            "a project ID is not a document ID"
        );
    }

    #[tokio::test]
    async fn a_document_knows_its_organization() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let ann = member(pool, press.id, "ann@example.com").await;
        let notes = create_project(pool, press.id, "Field Notes").await.unwrap();
        let main = create_document(pool, notes.id, ann.id, "main.typ")
            .await
            .unwrap();

        assert_eq!(
            document_organization(pool, main.id).await.unwrap(),
            Some(press.id)
        );
    }

    #[tokio::test]
    async fn a_path_is_unique_in_its_project() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let ann = member(pool, press.id, "ann@example.com").await;
        let notes = create_project(pool, press.id, "Field Notes").await.unwrap();
        let atlas = create_project(pool, press.id, "Atlas").await.unwrap();
        create_document(pool, notes.id, ann.id, "main.typ")
            .await
            .unwrap();

        let again = create_document(pool, notes.id, ann.id, "main.typ").await;
        let elsewhere = create_document(pool, atlas.id, ann.id, "main.typ").await;

        assert!(matches!(again, Err(DocumentError::PathTaken)));
        assert!(elsewhere.is_ok());
    }

    #[tokio::test]
    async fn updates_come_back_in_their_sequence() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let ann = member(pool, press.id, "ann@example.com").await;
        let notes = create_project(pool, press.id, "Field Notes").await.unwrap();
        let main = create_document(pool, notes.id, ann.id, "main.typ")
            .await
            .unwrap();

        for data in [b"first".as_slice(), b"second", b"third"] {
            append_update(pool, main.id, ann.id, data).await.unwrap();
        }

        assert_eq!(
            updates(pool, main.id).await.unwrap(),
            [b"first".to_vec(), b"second".to_vec(), b"third".to_vec()]
        );
    }
}
