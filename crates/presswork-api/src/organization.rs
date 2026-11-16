use std::fmt;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::Session;

const REMOVED: &str = "[removed]";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
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

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AcceptInvite {
    pub name: String,
    pub password: String,
    pub device: String,
}

impl fmt::Debug for AcceptInvite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AcceptInvite")
            .field("name", &self.name)
            .field("password", &REMOVED)
            .field("device", &self.device)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Joined {
    pub organization_id: Uuid,
    pub role: Role,
    pub token: String,
    pub session: Session,
}

impl fmt::Debug for Joined {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Joined")
            .field("organization_id", &self.organization_id)
            .field("role", &self.role)
            .field("token", &REMOVED)
            .field("session", &self.session)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NewPassword {
    pub password: String,
}

impl fmt::Debug for NewPassword {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NewPassword")
            .field("password", &REMOVED)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NewInvite {
    pub email: String,
    pub role: Role,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreatedInvite {
    pub token: String,
    pub email: String,
    pub role: Role,
    #[schema(value_type = String, format = DateTime)]
    pub expires_at: Timestamp,
}

impl fmt::Debug for CreatedInvite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreatedInvite")
            .field("token", &REMOVED)
            .field("email", &self.email)
            .field("role", &self.role)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_have_lowercase_names() {
        assert_eq!(
            serde_json::to_value(Role::Administrator).unwrap(),
            "administrator"
        );
        assert_eq!(Role::Member.as_str(), "member");
        assert_eq!(Role::parse("administrator"), Some(Role::Administrator));
        assert_eq!(Role::parse("owner"), None);
    }

    #[test]
    fn the_debug_texts_hide_passwords_and_tokens() {
        let accept = AcceptInvite {
            name: "Bob".to_owned(),
            password: "correct horse battery".to_owned(),
            device: "Laptop".to_owned(),
        };
        let reset = NewPassword {
            password: "correct horse battery".to_owned(),
        };
        let invite = CreatedInvite {
            token: "pwi_secret".to_owned(),
            email: "bob@example.com".to_owned(),
            role: Role::Member,
            expires_at: Timestamp::UNIX_EPOCH,
        };

        for text in [
            format!("{accept:?}"),
            format!("{reset:?}"),
            format!("{invite:?}"),
        ] {
            assert!(!text.contains("correct horse battery"), "{text}");
            assert!(!text.contains("pwi_secret"), "{text}");
        }
    }
}
