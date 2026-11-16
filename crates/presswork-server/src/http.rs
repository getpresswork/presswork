use axum::Json;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use presswork_api::ApiError;

use crate::accounts::AccountError;
use crate::links::LinkError;
use crate::routes::AppState;
use crate::sessions::{self, SessionError, Signed};

const BEARER: &str = "Bearer ";

#[derive(Debug)]
pub(crate) enum HttpError {
    Unauthenticated,
    WrongCredentials,
    Invalid(String),
    Conflict(String),
    Forbidden,
    NotFound,
    LinkNotValid,
    Internal(String),
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Sign in to use this function.".to_owned(),
            ),
            Self::WrongCredentials => (
                StatusCode::UNAUTHORIZED,
                "wrong_credentials",
                "The email address or the password is not correct.".to_owned(),
            ),
            Self::Invalid(message) => (StatusCode::BAD_REQUEST, "invalid_request", message),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", message),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "You do not have the permission for this function.".to_owned(),
            ),
            Self::LinkNotValid => (
                StatusCode::GONE,
                "link_not_valid",
                "The link was used, expired, or does not exist.".to_owned(),
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "The item does not exist.".to_owned(),
            ),
            Self::Internal(detail) => {
                tracing::error!(%detail, "A request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "The server cannot complete the request.".to_owned(),
                )
            }
        };
        let body = ApiError {
            code: code.to_owned(),
            message,
        };
        (status, Json(body)).into_response()
    }
}

impl From<AccountError> for HttpError {
    fn from(error: AccountError) -> Self {
        match error {
            AccountError::WrongCredentials => Self::WrongCredentials,
            AccountError::ShortPassword
            | AccountError::InvalidEmail
            | AccountError::InvalidName => Self::Invalid(sentence(&error.to_string())),
            AccountError::EmailTaken => Self::Conflict(sentence(&error.to_string())),
            AccountError::Hash(_) | AccountError::Database(_) => Self::Internal(error.to_string()),
        }
    }
}

impl From<SessionError> for HttpError {
    fn from(error: SessionError) -> Self {
        match error {
            SessionError::InvalidDevice => Self::Invalid(sentence(&error.to_string())),
            SessionError::Random(_) | SessionError::Database(_) => {
                Self::Internal(error.to_string())
            }
        }
    }
}

impl From<LinkError> for HttpError {
    fn from(error: LinkError) -> Self {
        match error {
            LinkError::NotValid => Self::LinkNotValid,
            LinkError::UnknownUser => Self::NotFound,
            LinkError::InvalidName => Self::Invalid(sentence(&error.to_string())),
            LinkError::Account(error) => error.into(),
            LinkError::Session(error) => error.into(),
            LinkError::Database(_) => Self::Internal(error.to_string()),
        }
    }
}

pub(crate) struct Caller(pub(crate) Signed);

impl FromRequestParts<AppState> for Caller {
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, HttpError> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix(BEARER))
            .ok_or(HttpError::Unauthenticated)?;
        sessions::find(&state.database, token)
            .await?
            .map(Caller)
            .ok_or(HttpError::Unauthenticated)
    }
}

fn sentence(text: &str) -> String {
    let mut characters = text.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    let mut sentence: String = first.to_uppercase().chain(characters).collect();
    if !sentence.ends_with('.') {
        sentence.push('.');
    }
    sentence
}
