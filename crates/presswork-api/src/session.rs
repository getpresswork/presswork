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
    #[serde(default)]
    pub user_name: String,
}

impl fmt::Debug for SignedIn {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignedIn")
            .field("token", &"[removed]")
            .field("session", &self.session)
            .field("user_name", &self.user_name)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccessToken {
    pub id: Uuid,
    pub name: String,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: Timestamp,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub expires_at: Option<Timestamp>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_used_at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NewAccessToken {
    pub name: String,
    #[serde(default)]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub expires_at: Option<Timestamp>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreatedAccessToken {
    pub token: String,
    pub access_token: AccessToken,
}

impl fmt::Debug for CreatedAccessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreatedAccessToken")
            .field("token", &"[removed]")
            .field("access_token", &self.access_token)
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
            user_name: "Ann".to_owned(),
        };

        assert!(!format!("{signed_in:?}").contains("pws_secret"));
    }

    #[test]
    fn a_new_session_gives_the_name_of_the_user() {
        let session = r#"{ "id": "00000000-0000-0000-0000-000000000000", "device": "Laptop",
            "created_at": "2026-11-15T10:00:00Z", "last_used_at": "2026-11-15T10:00:00Z",
            "current": true }"#;
        let named =
            format!(r#"{{ "token": "pws_secret", "user_name": "Ann", "session": {session} }}"#);
        let unnamed = format!(r#"{{ "token": "pws_secret", "session": {session} }}"#);

        let named: SignedIn = serde_json::from_str(&named).unwrap();
        let unnamed: SignedIn = serde_json::from_str(&unnamed).unwrap();

        assert_eq!(named.user_name, "Ann");
        assert_eq!(unnamed.user_name, "");
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

    fn ci_token() -> AccessToken {
        AccessToken {
            id: Uuid::nil(),
            name: "CI".to_owned(),
            created_at: "2026-11-17T08:00:00Z".parse().unwrap(),
            expires_at: None,
            last_used_at: None,
        }
    }

    #[test]
    fn the_debug_text_of_a_new_access_token_hides_the_token() {
        let created = CreatedAccessToken {
            token: "pwt_secret".to_owned(),
            access_token: ci_token(),
        };

        let text = format!("{created:?}");

        assert!(text.contains("CI"));
        assert!(!text.contains("pwt_secret"));
    }

    #[test]
    fn an_access_token_without_an_expiry_has_null_times() {
        let json = serde_json::to_value(ci_token()).unwrap();
        assert_eq!(json["created_at"], "2026-11-17T08:00:00Z");
        assert_eq!(json["expires_at"], serde_json::Value::Null);
        assert_eq!(json["last_used_at"], serde_json::Value::Null);

        let new: NewAccessToken = serde_json::from_str(r#"{ "name": "CI" }"#).unwrap();
        assert_eq!(new.expires_at, None);
        let new: NewAccessToken =
            serde_json::from_str(r#"{ "name": "CI", "expires_at": "2027-01-01T00:00:00Z" }"#)
                .unwrap();
        assert_eq!(
            new.expires_at,
            Some("2027-01-01T00:00:00Z".parse().unwrap())
        );
    }
}
