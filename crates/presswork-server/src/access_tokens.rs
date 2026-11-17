use jiff::Timestamp;
use jiff_sqlx::{Timestamp as SqlTimestamp, ToSqlx};
use presswork_api::AccessToken;
use sqlx::PgPool;
use uuid::Uuid;

use crate::sessions;

pub const TOKEN_PREFIX: &str = "pwt_";
const MAX_NAME_CHARACTERS: usize = 100;
const MAX_TOKENS: i64 = 100;

#[derive(Debug, thiserror::Error)]
pub enum AccessTokenError {
    #[error("a token name must have 1 to 100 characters and no control characters")]
    InvalidName,
    #[error("the expiry time must be in the future")]
    PastExpiry,
    #[error("a user can have 100 access tokens or fewer. Revoke a token first")]
    TooMany,
    #[error("the system cannot make a random token: {0}")]
    Random(String),
    #[error("the database failed: {0}")]
    Database(#[from] sqlx::Error),
}

type TokenRow = (
    Uuid,
    String,
    SqlTimestamp,
    Option<SqlTimestamp>,
    Option<SqlTimestamp>,
);

fn token_from((id, name, created_at, expires_at, last_used_at): TokenRow) -> AccessToken {
    AccessToken {
        id,
        name,
        created_at: created_at.to_jiff(),
        expires_at: expires_at.map(SqlTimestamp::to_jiff),
        last_used_at: last_used_at.map(SqlTimestamp::to_jiff),
    }
}

fn parse_name(name: &str) -> Result<&str, AccessTokenError> {
    let name = name.trim();
    let valid = (1..=MAX_NAME_CHARACTERS).contains(&name.chars().count())
        && !name.chars().any(char::is_control);
    if valid {
        Ok(name)
    } else {
        Err(AccessTokenError::InvalidName)
    }
}

pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    expires_at: Option<Timestamp>,
) -> Result<(String, AccessToken), AccessTokenError> {
    let name = parse_name(name)?;
    if expires_at.is_some_and(|expires_at| expires_at <= Timestamp::now()) {
        return Err(AccessTokenError::PastExpiry);
    }
    let token = sessions::random_token(TOKEN_PREFIX)
        .map_err(|error| AccessTokenError::Random(error.to_string()))?;
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM access_tokens WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&mut *transaction)
        .await?;
    if count >= MAX_TOKENS {
        return Err(AccessTokenError::TooMany);
    }
    let row: TokenRow = sqlx::query_as(
        "INSERT INTO access_tokens (id, user_id, name, token_hash, expires_at) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, name, created_at, expires_at, last_used_at",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(name)
    .bind(sessions::token_hash(&token))
    .bind(expires_at.map(ToSqlx::to_sqlx))
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok((token, token_from(row)))
}

pub async fn find(pool: &PgPool, token: &str) -> Result<Option<Uuid>, sqlx::Error> {
    let row: Option<(Uuid, Uuid, bool)> = sqlx::query_as(
        "SELECT id, user_id, \
         last_used_at IS NULL OR last_used_at < now() - interval '1 minute' \
         FROM access_tokens \
         WHERE token_hash = $1 AND (expires_at IS NULL OR expires_at > now())",
    )
    .bind(sessions::token_hash(token))
    .fetch_optional(pool)
    .await?;
    let Some((id, user_id, stale)) = row else {
        return Ok(None);
    };
    if stale {
        sqlx::query("UPDATE access_tokens SET last_used_at = now() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(Some(user_id))
}

pub async fn list(pool: &PgPool, user_id: Uuid) -> Result<Vec<AccessToken>, AccessTokenError> {
    let rows: Vec<TokenRow> = sqlx::query_as(
        "SELECT id, name, created_at, expires_at, last_used_at FROM access_tokens \
         WHERE user_id = $1 ORDER BY created_at DESC, id DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(token_from).collect())
}

pub async fn revoke(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<bool, AccessTokenError> {
    let result = sqlx::query("DELETE FROM access_tokens WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use jiff::SignedDuration;

    use super::*;
    use crate::accounts::{User, create_user};
    use crate::testing::test_database;

    async fn person(pool: &PgPool, email: &str) -> User {
        create_user(pool, email, "Someone", "correct horse battery")
            .await
            .unwrap()
    }

    fn in_days(days: i64) -> Timestamp {
        Timestamp::now() + SignedDuration::from_hours(24 * days)
    }

    #[test]
    fn a_token_name_has_1_to_100_characters() {
        assert_eq!(parse_name("  CI  ").unwrap(), "CI");
        assert!(parse_name(&"x".repeat(100)).is_ok());
        for name in ["", "   ", &"x".repeat(101), "C\u{7}I"] {
            assert!(
                matches!(parse_name(name), Err(AccessTokenError::InvalidName)),
                "{name:?}"
            );
        }
    }

    #[tokio::test]
    async fn an_access_token_finds_its_user_until_it_is_revoked() {
        let Some(database) = test_database().await else {
            return;
        };
        let ann = person(&database.pool, "ann@example.com").await;

        let (token, created) = create(&database.pool, ann.id, "CI", None).await.unwrap();

        assert!(token.starts_with(TOKEN_PREFIX));
        assert_eq!(token.len(), TOKEN_PREFIX.len() + 64);
        assert_eq!(created.name, "CI");
        assert_eq!(created.expires_at, None);
        assert_eq!(find(&database.pool, &token).await.unwrap(), Some(ann.id));
        assert_eq!(find(&database.pool, "pwt_other").await.unwrap(), None);

        assert!(revoke(&database.pool, ann.id, created.id).await.unwrap());
        assert_eq!(find(&database.pool, &token).await.unwrap(), None);
        assert!(!revoke(&database.pool, ann.id, created.id).await.unwrap());
    }

    #[tokio::test]
    async fn an_expired_access_token_is_not_valid() {
        let Some(database) = test_database().await else {
            return;
        };
        let ann = person(&database.pool, "ann@example.com").await;
        assert!(matches!(
            create(&database.pool, ann.id, "CI", Some(in_days(-1))).await,
            Err(AccessTokenError::PastExpiry)
        ));
        let (token, created) = create(&database.pool, ann.id, "CI", Some(in_days(30)))
            .await
            .unwrap();
        assert_eq!(find(&database.pool, &token).await.unwrap(), Some(ann.id));

        sqlx::query(
            "UPDATE access_tokens SET expires_at = now() - interval '1 second' WHERE id = $1",
        )
        .bind(created.id)
        .execute(&database.pool)
        .await
        .unwrap();

        assert_eq!(find(&database.pool, &token).await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_user_lists_and_revokes_only_their_own_tokens() {
        let Some(database) = test_database().await else {
            return;
        };
        let ann = person(&database.pool, "ann@example.com").await;
        let bob = person(&database.pool, "bob@example.com").await;
        let (token, _) = create(&database.pool, ann.id, "CI", None).await.unwrap();
        create(&database.pool, ann.id, "Backup", None)
            .await
            .unwrap();
        let (_, bobs) = create(&database.pool, bob.id, "Scripts", None)
            .await
            .unwrap();
        find(&database.pool, &token).await.unwrap();

        let tokens = list(&database.pool, ann.id).await.unwrap();

        let names: Vec<(&str, bool)> = tokens
            .iter()
            .map(|token| (token.name.as_str(), token.last_used_at.is_some()))
            .collect();
        assert_eq!(names, [("Backup", false), ("CI", true)]);
        assert!(!revoke(&database.pool, ann.id, bobs.id).await.unwrap());
        assert_eq!(list(&database.pool, bob.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_user_can_have_100_access_tokens_or_fewer() {
        let Some(database) = test_database().await else {
            return;
        };
        let ann = person(&database.pool, "ann@example.com").await;
        for index in 0..100 {
            create(&database.pool, ann.id, &format!("Token {index}"), None)
                .await
                .unwrap();
        }

        assert!(matches!(
            create(&database.pool, ann.id, "One more", None).await,
            Err(AccessTokenError::TooMany)
        ));
    }
}
