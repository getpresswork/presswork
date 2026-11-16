use std::sync::LazyLock;

use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::{Algorithm, Argon2, Params, Version};
use sqlx::PgPool;
use uuid::Uuid;

const MIN_PASSWORD_CHARACTERS: usize = 12;
const MAX_NAME_CHARACTERS: usize = 200;
const MEMORY_KIB: u32 = 19 * 1024;
const PASSES: u32 = 2;
const LANES: u32 = 1;
const UNIQUE_VIOLATION: &str = "23505";

static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password("a password that no user has").unwrap_or_default());

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("a password must have 12 characters or more")]
    ShortPassword,
    #[error("the email address is not valid")]
    InvalidEmail,
    #[error("a name must have 1 to 200 characters")]
    InvalidName,
    #[error("another user has this email address")]
    EmailTaken,
    #[error("the email address or the password is not correct")]
    WrongCredentials,
    #[error("the password cannot be hashed: {0}")]
    Hash(String),
    #[error("the database failed: {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}

fn argon2() -> Argon2<'static> {
    let params = Params::new(MEMORY_KIB, PASSES, LANES, None).unwrap_or(Params::DEFAULT);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

pub fn hash_password(password: &str) -> Result<String, AccountError> {
    argon2()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|error| AccountError::Hash(error.to_string()))
}

#[must_use]
pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|parsed| {
        argon2()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

pub fn check_password(password: &str) -> Result<(), AccountError> {
    if password.chars().count() < MIN_PASSWORD_CHARACTERS {
        return Err(AccountError::ShortPassword);
    }
    Ok(())
}

pub fn parse_email(email: &str) -> Result<&str, AccountError> {
    let email = email.trim();
    let valid = !email.chars().any(char::is_whitespace)
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && !domain.is_empty() && !domain.contains('@')
        });
    if valid {
        Ok(email)
    } else {
        Err(AccountError::InvalidEmail)
    }
}

fn parse_name(name: &str) -> Result<&str, AccountError> {
    let name = name.trim();
    if (1..=MAX_NAME_CHARACTERS).contains(&name.chars().count()) {
        Ok(name)
    } else {
        Err(AccountError::InvalidName)
    }
}

async fn hash_in_background(password: &str) -> Result<String, AccountError> {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|error| AccountError::Hash(error.to_string()))?
}

async fn verify_in_background(password: &str, hash: String) -> bool {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .unwrap_or(false)
}

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    name: &str,
    password: &str,
) -> Result<User, AccountError> {
    let email = parse_email(email)?;
    let name = parse_name(name)?;
    check_password(password)?;
    let hash = hash_in_background(password).await?;
    let user = User {
        id: Uuid::now_v7(),
        email: email.to_owned(),
        name: name.to_owned(),
    };
    sqlx::query("INSERT INTO users (id, email, name, password_hash) VALUES ($1, $2, $3, $4)")
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.name)
        .bind(&hash)
        .execute(pool)
        .await
        .map_err(|error| match &error {
            sqlx::Error::Database(database)
                if database.code().as_deref() == Some(UNIQUE_VIOLATION) =>
            {
                AccountError::EmailTaken
            }
            _ => AccountError::Database(error),
        })?;
    Ok(user)
}

pub async fn find_user(pool: &PgPool, email: &str) -> Result<Option<User>, AccountError> {
    Ok(
        sqlx::query_as("SELECT id, email, name FROM users WHERE lower(email) = lower($1)")
            .bind(email.trim())
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn authenticate(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<User, AccountError> {
    let row: Option<(Uuid, String, String, String)> = sqlx::query_as(
        "SELECT id, email, name, password_hash FROM users WHERE lower(email) = lower($1)",
    )
    .bind(email.trim())
    .fetch_optional(pool)
    .await?;
    let Some((id, email, name, hash)) = row else {
        let _ = verify_in_background(password, DUMMY_HASH.clone()).await;
        return Err(AccountError::WrongCredentials);
    };
    if verify_in_background(password, hash).await {
        Ok(User { id, email, name })
    } else {
        Err(AccountError::WrongCredentials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::test_database;

    const PASSWORD: &str = "correct horse battery";

    #[test]
    fn a_password_hash_uses_argon2id_with_the_owasp_parameters() {
        let hash = hash_password(PASSWORD).unwrap();

        assert!(
            hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
            "{hash}"
        );
        assert!(verify_password(PASSWORD, &hash));
        assert!(!verify_password("correct horse battery!", &hash));
    }

    #[test]
    fn two_hashes_of_one_password_have_different_salts() {
        assert_ne!(
            hash_password(PASSWORD).unwrap(),
            hash_password(PASSWORD).unwrap()
        );
    }

    #[test]
    fn a_password_needs_twelve_characters() {
        assert!(matches!(
            check_password("short"),
            Err(AccountError::ShortPassword)
        ));
        assert!(check_password("twelve chars").is_ok());
        assert!(matches!(
            check_password("ééééééééééé"),
            Err(AccountError::ShortPassword)
        ));
    }

    #[test]
    fn an_email_address_loses_its_spaces_and_needs_an_at_sign() {
        assert_eq!(
            parse_email("  Ann@Example.com ").unwrap(),
            "Ann@Example.com"
        );
        for wrong in ["ann", "@example.com", "ann@", "ann @example.com", ""] {
            assert!(
                matches!(parse_email(wrong), Err(AccountError::InvalidEmail)),
                "{wrong}"
            );
        }
    }

    #[tokio::test]
    async fn a_new_user_is_found_by_email_in_any_case() {
        let Some(database) = test_database().await else {
            return;
        };

        let user = create_user(&database.pool, "Ann@Example.com", "Ann", PASSWORD)
            .await
            .unwrap();

        let found = find_user(&database.pool, "ann@example.COM")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found, user);
        assert_eq!(found.email, "Ann@Example.com");
        assert_eq!(found.name, "Ann");
    }

    #[tokio::test]
    async fn an_email_address_belongs_to_one_user() {
        let Some(database) = test_database().await else {
            return;
        };
        create_user(&database.pool, "ann@example.com", "Ann", PASSWORD)
            .await
            .unwrap();

        let second = create_user(&database.pool, "ANN@example.com", "Anne", PASSWORD).await;

        assert!(matches!(second, Err(AccountError::EmailTaken)));
    }

    #[tokio::test]
    async fn sign_in_checks_the_password() {
        let Some(database) = test_database().await else {
            return;
        };
        let user = create_user(&database.pool, "ann@example.com", "Ann", PASSWORD)
            .await
            .unwrap();

        let signed_in = authenticate(&database.pool, "Ann@Example.com", PASSWORD)
            .await
            .unwrap();

        assert_eq!(signed_in, user);
        for (email, password) in [
            ("ann@example.com", "a wrong password"),
            ("nobody@example.com", PASSWORD),
        ] {
            assert!(matches!(
                authenticate(&database.pool, email, password).await,
                Err(AccountError::WrongCredentials)
            ));
        }
    }
}
