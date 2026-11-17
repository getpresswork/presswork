use std::fmt::Write as _;

use jiff_sqlx::Timestamp as SqlTimestamp;
use presswork_api::Session;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::access_tokens;

pub const TOKEN_PREFIX: &str = "pws_";
const TOKEN_BYTES: usize = 32;
const IDLE_DAYS: i32 = 30;
const MAX_DEVICE_CHARACTERS: usize = 200;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("the system cannot make a random token: {0}")]
    Random(String),
    #[error("a device name must have 1 to 200 characters")]
    InvalidDevice,
    #[error("the database failed: {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signed {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    pub user_id: Uuid,
    pub session_id: Option<Uuid>,
}

pub fn new_token() -> Result<String, SessionError> {
    random_token(TOKEN_PREFIX)
}

pub(crate) fn random_token(prefix: &str) -> Result<String, SessionError> {
    let mut bytes = [0; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|error| SessionError::Random(error.to_string()))?;
    Ok(bytes.iter().fold(String::from(prefix), |mut token, byte| {
        let _ = write!(token, "{byte:02x}");
        token
    }))
}

pub(crate) fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn parse_device(device: &str) -> Result<&str, SessionError> {
    let device = device.trim();
    if (1..=MAX_DEVICE_CHARACTERS).contains(&device.chars().count()) {
        Ok(device)
    } else {
        Err(SessionError::InvalidDevice)
    }
}

pub async fn start(
    pool: &PgPool,
    user_id: Uuid,
    device: &str,
) -> Result<(String, Session), SessionError> {
    let device = parse_device(device)?;
    let token = new_token()?;
    let id = Uuid::now_v7();
    let (created_at, last_used_at): (SqlTimestamp, SqlTimestamp) = sqlx::query_as(
        "INSERT INTO sessions (id, user_id, token_hash, device) VALUES ($1, $2, $3, $4) \
         RETURNING created_at, last_used_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(token_hash(&token))
    .bind(device)
    .fetch_one(pool)
    .await?;
    let session = Session {
        id,
        device: device.to_owned(),
        created_at: created_at.to_jiff(),
        last_used_at: last_used_at.to_jiff(),
        current: true,
    };
    Ok((token, session))
}

pub async fn find(pool: &PgPool, token: &str) -> Result<Option<Signed>, SessionError> {
    let row: Option<(Uuid, Uuid, bool)> = sqlx::query_as(
        "SELECT id, user_id, last_used_at < now() - interval '1 minute' FROM sessions \
         WHERE token_hash = $1 AND last_used_at > now() - make_interval(days => $2)",
    )
    .bind(token_hash(token))
    .bind(IDLE_DAYS)
    .fetch_optional(pool)
    .await?;
    let Some((session_id, user_id, stale)) = row else {
        return Ok(None);
    };
    if stale {
        sqlx::query("UPDATE sessions SET last_used_at = now() WHERE id = $1")
            .bind(session_id)
            .execute(pool)
            .await?;
    }
    Ok(Some(Signed {
        user_id,
        session_id,
    }))
}

pub async fn identify(pool: &PgPool, token: &str) -> Result<Option<Identity>, SessionError> {
    if token.starts_with(access_tokens::TOKEN_PREFIX) {
        let user_id = access_tokens::find(pool, token).await?;
        return Ok(user_id.map(|user_id| Identity {
            user_id,
            session_id: None,
        }));
    }
    Ok(find(pool, token).await?.map(|signed| Identity {
        user_id: signed.user_id,
        session_id: Some(signed.session_id),
    }))
}

pub async fn list(
    pool: &PgPool,
    user_id: Uuid,
    current: Option<Uuid>,
) -> Result<Vec<Session>, SessionError> {
    let rows: Vec<(Uuid, String, SqlTimestamp, SqlTimestamp)> = sqlx::query_as(
        "SELECT id, device, created_at, last_used_at FROM sessions \
         WHERE user_id = $1 AND last_used_at > now() - make_interval(days => $2) \
         ORDER BY last_used_at DESC",
    )
    .bind(user_id)
    .bind(IDLE_DAYS)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, device, created_at, last_used_at)| Session {
            id,
            device,
            created_at: created_at.to_jiff(),
            last_used_at: last_used_at.to_jiff(),
            current: Some(id) == current,
        })
        .collect())
}

pub async fn end(pool: &PgPool, user_id: Uuid, session_id: Uuid) -> Result<bool, SessionError> {
    let result = sqlx::query("DELETE FROM sessions WHERE id = $1 AND user_id = $2")
        .bind(session_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{User, create_user};
    use crate::testing::test_database;

    async fn ann(pool: &PgPool) -> User {
        create_user(pool, "ann@example.com", "Ann", "correct horse battery")
            .await
            .unwrap()
    }

    async fn age(pool: &PgPool, session: Uuid, days: i32) {
        sqlx::query(
            "UPDATE sessions SET last_used_at = now() - make_interval(days => $2) WHERE id = $1",
        )
        .bind(session)
        .bind(days)
        .execute(pool)
        .await
        .unwrap();
    }

    #[test]
    fn a_token_has_a_prefix_and_256_random_bits() {
        let token = new_token().unwrap();

        assert!(token.starts_with(TOKEN_PREFIX));
        let hex = &token[TOKEN_PREFIX.len()..];
        assert_eq!(hex.len(), 64);
        assert!(hex.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(token, new_token().unwrap());
    }

    #[tokio::test]
    async fn a_session_token_finds_its_user() {
        let Some(database) = test_database().await else {
            return;
        };
        let user = ann(&database.pool).await;

        let (token, session) = start(&database.pool, user.id, "Laptop").await.unwrap();

        let found = find(&database.pool, &token).await.unwrap().unwrap();
        assert_eq!(
            found,
            Signed {
                user_id: user.id,
                session_id: session.id
            }
        );
        assert_eq!(session.device, "Laptop");
        assert!(session.current);
        assert_eq!(find(&database.pool, "pws_other").await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_session_without_use_for_30_days_is_not_valid() {
        let Some(database) = test_database().await else {
            return;
        };
        let user = ann(&database.pool).await;
        let (token, session) = start(&database.pool, user.id, "Laptop").await.unwrap();

        age(&database.pool, session.id, 31).await;

        assert_eq!(find(&database.pool, &token).await.unwrap(), None);
    }

    #[tokio::test]
    async fn the_use_of_a_session_extends_it() {
        let Some(database) = test_database().await else {
            return;
        };
        let user = ann(&database.pool).await;
        let (token, session) = start(&database.pool, user.id, "Laptop").await.unwrap();
        age(&database.pool, session.id, 29).await;

        assert!(find(&database.pool, &token).await.unwrap().is_some());

        let idle_days: f64 = sqlx::query_scalar(
            "SELECT (extract(epoch FROM now() - last_used_at) / 86400)::float8 FROM sessions WHERE id = $1",
        )
        .bind(session.id)
        .fetch_one(&database.pool)
        .await
        .unwrap();
        assert!(idle_days < 0.01, "{idle_days}");
    }

    #[tokio::test]
    async fn a_user_lists_and_ends_only_their_own_sessions() {
        let Some(database) = test_database().await else {
            return;
        };
        let user = ann(&database.pool).await;
        let other = create_user(
            &database.pool,
            "bob@example.com",
            "Bob",
            "correct horse battery",
        )
        .await
        .unwrap();
        let (_, laptop) = start(&database.pool, user.id, "Laptop").await.unwrap();
        let (_, phone) = start(&database.pool, user.id, "Phone").await.unwrap();
        let (_, bobs) = start(&database.pool, other.id, "Desktop").await.unwrap();

        let sessions = list(&database.pool, user.id, Some(laptop.id))
            .await
            .unwrap();
        let devices: Vec<(&str, bool)> = sessions
            .iter()
            .map(|session| (session.device.as_str(), session.current))
            .collect();
        assert_eq!(devices.len(), 2);
        assert!(devices.contains(&("Laptop", true)));
        assert!(devices.contains(&("Phone", false)));

        assert!(end(&database.pool, user.id, phone.id).await.unwrap());
        assert!(!end(&database.pool, user.id, bobs.id).await.unwrap());
        assert_eq!(
            list(&database.pool, user.id, Some(laptop.id))
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn a_session_or_an_access_token_identifies_the_user() {
        let Some(database) = test_database().await else {
            return;
        };
        let user = ann(&database.pool).await;
        let (session_token, session) = start(&database.pool, user.id, "Laptop").await.unwrap();
        let (access_token, _) = access_tokens::create(&database.pool, user.id, "CI", None)
            .await
            .unwrap();

        assert_eq!(
            identify(&database.pool, &session_token).await.unwrap(),
            Some(Identity {
                user_id: user.id,
                session_id: Some(session.id)
            })
        );
        assert_eq!(
            identify(&database.pool, &access_token).await.unwrap(),
            Some(Identity {
                user_id: user.id,
                session_id: None
            })
        );
        assert_eq!(identify(&database.pool, "pwt_other").await.unwrap(), None);
        assert_eq!(identify(&database.pool, "pws_other").await.unwrap(), None);
    }
}
