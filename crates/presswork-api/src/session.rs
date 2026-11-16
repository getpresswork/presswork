use std::fmt;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SignIn {
    pub email: String,
    pub password: String,
    pub device: String,
}

impl fmt::Debug for SignIn {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignIn")
            .field("email", &self.email)
            .field("password", &"[removed]")
            .field("device", &self.device)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Session {
    pub id: Uuid,
    pub device: String,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: Timestamp,
    #[schema(value_type = String, format = DateTime)]
    pub last_used_at: Timestamp,
    pub current: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SignedIn {
    pub token: String,
    pub session: Session,
}

impl fmt::Debug for SignedIn {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignedIn")
            .field("token", &"[removed]")
            .field("session", &self.session)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_debug_text_of_a_sign_in_hides_the_password() {
        let request = SignIn {
            email: "ann@example.com".to_owned(),
            password: "correct horse battery".to_owned(),
            device: "Laptop".to_owned(),
        };

        let text = format!("{request:?}");

        assert!(text.contains("ann@example.com"));
        assert!(!text.contains("correct horse battery"));
    }

    #[test]
    fn the_debug_text_of_a_new_session_hides_the_token() {
        let signed_in = SignedIn {
            token: "pws_secret".to_owned(),
            session: Session {
                id: Uuid::nil(),
                device: "Laptop".to_owned(),
                created_at: Timestamp::UNIX_EPOCH,
                last_used_at: Timestamp::UNIX_EPOCH,
                current: true,
            },
        };

        assert!(!format!("{signed_in:?}").contains("pws_secret"));
    }

    #[test]
    fn a_session_gives_its_times_in_rfc_3339() {
        let session = Session {
            id: Uuid::nil(),
            device: "Laptop".to_owned(),
            created_at: "2026-11-15T10:00:00Z".parse().unwrap(),
            last_used_at: "2026-11-15T11:30:00Z".parse().unwrap(),
            current: true,
        };

        let json = serde_json::to_value(&session).unwrap();

        assert_eq!(json["created_at"], "2026-11-15T10:00:00Z");
        assert_eq!(json["last_used_at"], "2026-11-15T11:30:00Z");
        assert_eq!(json["current"], true);
    }
}
