use jiff_sqlx::Timestamp as SqlTimestamp;
use presswork_api::Session;
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounts::{self, AccountError, User};
use crate::sessions::{self, SessionError};

pub const INVITE_PREFIX: &str = "pwi_";
pub const RESET_PREFIX: &str = "pwr_";
const INVITE_DAYS: i32 = 7;
const RESET_HOURS: i32 = 24;
const MAX_ORGANIZATION_NAME_CHARACTERS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Administrator,
    Member,
}

impl Role {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Administrator => "administrator",
            Self::Member => "member",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "administrator" => Some(Self::Administrator),
            "member" => Some(Self::Member),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("the link is not valid: it was used, it expired, or it does not exist")]
    NotValid,
    #[error("no user has this email address")]
    UnknownUser,
    #[error("an organization name must have 1 to 200 characters")]
    InvalidName,
    #[error(transparent)]
    Account(#[from] AccountError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error("the database failed: {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invite {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub role: Role,
    pub expires_at: jiff::Timestamp,
}

#[derive(Debug, Clone)]
pub struct Accepted {
    pub user: User,
    pub organization_id: Uuid,
    pub role: Role,
    pub session_token: String,
    pub session: Session,
}

pub async fn create_organization(pool: &PgPool, name: &str) -> Result<Organization, LinkError> {
    let name = name.trim();
    if !(1..=MAX_ORGANIZATION_NAME_CHARACTERS).contains(&name.chars().count()) {
        return Err(LinkError::InvalidName);
    }
    let organization = Organization {
        id: Uuid::now_v7(),
        name: name.to_owned(),
    };
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(organization.id)
        .bind(&organization.name)
        .execute(pool)
        .await?;
    Ok(organization)
}

pub async fn invite(
    pool: &PgPool,
    organization_id: Uuid,
    email: &str,
    role: Role,
) -> Result<(String, Invite), LinkError> {
    let email = accounts::parse_email(email)?;
    let token = sessions::random_token(INVITE_PREFIX)?;
    let id = Uuid::now_v7();
    let expires_at: SqlTimestamp = sqlx::query_scalar(
        "INSERT INTO invites (id, organization_id, email, role, token_hash, expires_at) \
         VALUES ($1, $2, $3, $4, $5, now() + make_interval(days => $6)) RETURNING expires_at",
    )
    .bind(id)
    .bind(organization_id)
    .bind(email)
    .bind(role.as_str())
    .bind(sessions::token_hash(&token))
    .bind(INVITE_DAYS)
    .fetch_one(pool)
    .await?;
    let invite = Invite {
        id,
        organization_id,
        email: email.to_owned(),
        role,
        expires_at: expires_at.to_jiff(),
    };
    Ok((token, invite))
}

pub async fn accept_invite(
    pool: &PgPool,
    token: &str,
    name: &str,
    password: &str,
    device: &str,
) -> Result<Accepted, LinkError> {
    let mut transaction = pool.begin().await?;
    let row: Option<(Uuid, Uuid, String, String)> = sqlx::query_as(
        "SELECT id, organization_id, email, role FROM invites \
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now() FOR UPDATE",
    )
    .bind(sessions::token_hash(token))
    .fetch_optional(&mut *transaction)
    .await?;
    let Some((invite_id, organization_id, email, role)) = row else {
        return Err(LinkError::NotValid);
    };
    let role = Role::parse(&role).ok_or(LinkError::NotValid)?;
    let user = match accounts::find_user(pool, &email).await? {
        Some(_) => accounts::authenticate(pool, &email, password).await?,
        None => accounts::create_user(&mut *transaction, &email, name, password).await?,
    };
    sqlx::query(
        "INSERT INTO memberships (organization_id, user_id, role) VALUES ($1, $2, $3) \
         ON CONFLICT (organization_id, user_id) DO UPDATE SET role = EXCLUDED.role",
    )
    .bind(organization_id)
    .bind(user.id)
    .bind(role.as_str())
    .execute(&mut *transaction)
    .await?;
    sqlx::query("UPDATE invites SET used_at = now() WHERE id = $1")
        .bind(invite_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    let (session_token, session) = sessions::start(pool, user.id, device).await?;
    Ok(Accepted {
        user,
        organization_id,
        role,
        session_token,
        session,
    })
}

pub async fn role(
    pool: &PgPool,
    organization_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Role>, LinkError> {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM memberships WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(organization_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(role.as_deref().and_then(Role::parse))
}

pub async fn reset_link(pool: &PgPool, email: &str) -> Result<String, LinkError> {
    let user = accounts::find_user(pool, email)
        .await?
        .ok_or(LinkError::UnknownUser)?;
    let token = sessions::random_token(RESET_PREFIX)?;
    sqlx::query(
        "INSERT INTO password_resets (id, user_id, token_hash, expires_at) \
         VALUES ($1, $2, $3, now() + make_interval(hours => $4))",
    )
    .bind(Uuid::now_v7())
    .bind(user.id)
    .bind(sessions::token_hash(&token))
    .bind(RESET_HOURS)
    .execute(pool)
    .await?;
    Ok(token)
}

pub async fn reset_password(pool: &PgPool, token: &str, password: &str) -> Result<(), LinkError> {
    accounts::check_password(password)?;
    let mut transaction = pool.begin().await?;
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, user_id FROM password_resets \
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now() FOR UPDATE",
    )
    .bind(sessions::token_hash(token))
    .fetch_optional(&mut *transaction)
    .await?;
    let Some((reset_id, user_id)) = row else {
        return Err(LinkError::NotValid);
    };
    let hash = accounts::hash_in_background(password).await?;
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&hash)
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("UPDATE password_resets SET used_at = now() WHERE id = $1")
        .bind(reset_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::AssertSqlSafe;

    use super::*;
    use crate::accounts::{authenticate, create_user};
    use crate::testing::test_database;

    const PASSWORD: &str = "correct horse battery";
    const OTHER_PASSWORD: &str = "another good password";

    async fn expire(pool: &PgPool, table: &str) {
        sqlx::query(AssertSqlSafe(format!(
            "UPDATE {table} SET expires_at = now() - interval '1 second'"
        )))
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn an_invite_makes_a_signed_in_member() {
        let Some(database) = test_database().await else {
            return;
        };
        let organization = create_organization(&database.pool, "Field Press")
            .await
            .unwrap();
        let (token, invite) = invite(
            &database.pool,
            organization.id,
            "bob@example.com",
            Role::Member,
        )
        .await
        .unwrap();

        let accepted = accept_invite(&database.pool, &token, "Bob", PASSWORD, "Laptop")
            .await
            .unwrap();

        assert!(token.starts_with(INVITE_PREFIX));
        assert_eq!(invite.email, "bob@example.com");
        assert_eq!(accepted.organization_id, organization.id);
        assert_eq!(accepted.role, Role::Member);
        assert_eq!(
            role(&database.pool, organization.id, accepted.user.id)
                .await
                .unwrap(),
            Some(Role::Member)
        );
        let signed = sessions::find(&database.pool, &accepted.session_token)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(signed.user_id, accepted.user.id);
    }

    #[tokio::test]
    async fn an_invite_works_one_time() {
        let Some(database) = test_database().await else {
            return;
        };
        let organization = create_organization(&database.pool, "Field Press")
            .await
            .unwrap();
        let (token, _) = invite(
            &database.pool,
            organization.id,
            "bob@example.com",
            Role::Member,
        )
        .await
        .unwrap();
        accept_invite(&database.pool, &token, "Bob", PASSWORD, "Laptop")
            .await
            .unwrap();

        let second = accept_invite(&database.pool, &token, "Bob", PASSWORD, "Phone").await;

        assert!(matches!(second, Err(LinkError::NotValid)));
    }

    #[tokio::test]
    async fn an_expired_invite_does_not_work() {
        let Some(database) = test_database().await else {
            return;
        };
        let organization = create_organization(&database.pool, "Field Press")
            .await
            .unwrap();
        let (token, _) = invite(
            &database.pool,
            organization.id,
            "bob@example.com",
            Role::Member,
        )
        .await
        .unwrap();
        expire(&database.pool, "invites").await;

        let result = accept_invite(&database.pool, &token, "Bob", PASSWORD, "Laptop").await;

        assert!(matches!(result, Err(LinkError::NotValid)));
    }

    #[tokio::test]
    async fn an_invite_for_a_known_user_needs_their_password() {
        let Some(database) = test_database().await else {
            return;
        };
        let bob = create_user(&database.pool, "bob@example.com", "Bob", PASSWORD)
            .await
            .unwrap();
        let organization = create_organization(&database.pool, "Field Press")
            .await
            .unwrap();
        let (token, _) = invite(
            &database.pool,
            organization.id,
            "Bob@Example.com",
            Role::Administrator,
        )
        .await
        .unwrap();

        let wrong = accept_invite(&database.pool, &token, "Bob", OTHER_PASSWORD, "Laptop").await;
        assert!(matches!(
            wrong,
            Err(LinkError::Account(AccountError::WrongCredentials))
        ));

        let accepted = accept_invite(&database.pool, &token, "Bob", PASSWORD, "Laptop")
            .await
            .unwrap();
        assert_eq!(accepted.user.id, bob.id);
        assert_eq!(
            role(&database.pool, organization.id, bob.id).await.unwrap(),
            Some(Role::Administrator)
        );
    }

    #[tokio::test]
    async fn a_password_reset_sets_a_new_password_and_ends_the_sessions() {
        let Some(database) = test_database().await else {
            return;
        };
        let ann = create_user(&database.pool, "ann@example.com", "Ann", PASSWORD)
            .await
            .unwrap();
        let (session_token, _) = sessions::start(&database.pool, ann.id, "Laptop")
            .await
            .unwrap();
        let token = reset_link(&database.pool, "ANN@example.com").await.unwrap();

        reset_password(&database.pool, &token, OTHER_PASSWORD)
            .await
            .unwrap();

        assert!(token.starts_with(RESET_PREFIX));
        assert!(
            authenticate(&database.pool, "ann@example.com", OTHER_PASSWORD)
                .await
                .is_ok()
        );
        assert!(
            authenticate(&database.pool, "ann@example.com", PASSWORD)
                .await
                .is_err()
        );
        assert_eq!(
            sessions::find(&database.pool, &session_token)
                .await
                .unwrap(),
            None
        );
        assert!(matches!(
            reset_password(&database.pool, &token, PASSWORD).await,
            Err(LinkError::NotValid)
        ));
    }

    #[tokio::test]
    async fn a_reset_link_needs_a_known_user() {
        let Some(database) = test_database().await else {
            return;
        };

        assert!(matches!(
            reset_link(&database.pool, "nobody@example.com").await,
            Err(LinkError::UnknownUser)
        ));
    }

    #[test]
    fn roles_have_lowercase_names() {
        assert_eq!(Role::Administrator.as_str(), "administrator");
        assert_eq!(Role::parse("member"), Some(Role::Member));
        assert_eq!(Role::parse("owner"), None);
    }
}
