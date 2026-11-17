use std::convert::Infallible;
use std::future::ready;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use axum::Json;
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::StatusCode;
use axum::http::header::{AUTHORIZATION, RETRY_AFTER};
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use presswork_api::ApiError;

use crate::access_tokens::AccessTokenError;
use crate::accounts::AccountError;
use crate::documents::DocumentError;
use crate::library::LibraryError;
use crate::limits;
use crate::links::LinkError;
use crate::routes::AppState;
use crate::sessions::{self, Identity, SessionError};
use crate::work::WorkError;

const BEARER: &str = "Bearer ";

#[derive(Debug)]
pub(crate) enum HttpError {
    Unauthenticated,
    WrongCredentials,
    TooManyAttempts(Duration),
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
            Self::TooManyAttempts(wait) => return too_many_attempts(wait),
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

fn too_many_attempts(wait: Duration) -> Response {
    let seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    let minutes = seconds.div_ceil(60).max(1);
    let unit = if minutes == 1 { "minute" } else { "minutes" };
    let body = ApiError {
        code: "too_many_attempts".to_owned(),
        message: format!("Too many sign-ins failed. Try again in {minutes} {unit}."),
    };
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(RETRY_AFTER, seconds.to_string())],
        Json(body),
    )
        .into_response()
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

impl From<AccessTokenError> for HttpError {
    fn from(error: AccessTokenError) -> Self {
        match error {
            AccessTokenError::InvalidName | AccessTokenError::PastExpiry => {
                Self::Invalid(sentence(&error.to_string()))
            }
            AccessTokenError::TooMany => Self::Conflict(sentence(&error.to_string())),
            AccessTokenError::Random(_) | AccessTokenError::Database(_) => {
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

impl From<DocumentError> for HttpError {
    fn from(error: DocumentError) -> Self {
        match error {
            DocumentError::InvalidPath => Self::Invalid(sentence(&error.to_string())),
            DocumentError::PathTaken => Self::Conflict(sentence(&error.to_string())),
            DocumentError::Database(_) => Self::Internal(error.to_string()),
        }
    }
}

impl From<WorkError> for HttpError {
    fn from(error: WorkError) -> Self {
        match error {
            WorkError::InvalidName
            | WorkError::InvalidTitle
            | WorkError::LongDescription
            | WorkError::AssigneeNotMember => Self::Invalid(sentence(&error.to_string())),
            WorkError::NotFound => Self::NotFound,
            WorkError::Database(_) => Self::Internal(error.to_string()),
        }
    }
}

impl From<LibraryError> for HttpError {
    fn from(error: LibraryError) -> Self {
        match error {
            LibraryError::InvalidKey(_)
            | LibraryError::InvalidTitle
            | LibraryError::InvalidCreators
            | LibraryError::InvalidTranslators
            | LibraryError::InvalidPublisher
            | LibraryError::InvalidLanguage
            | LibraryError::InvalidYear
            | LibraryError::NoFile
            | LibraryError::InvalidPageMap
            | LibraryError::NotPdf
            | LibraryError::Encrypted => Self::Invalid(sentence(&error.to_string())),
            LibraryError::KeyTaken => Self::Conflict(sentence(&error.to_string())),
            LibraryError::NotFound => Self::NotFound,
            LibraryError::Blobs(_)
            | LibraryError::Search(_)
            | LibraryError::Folder(_)
            | LibraryError::Job(_)
            | LibraryError::Database(_) => Self::Internal(error.to_string()),
        }
    }
}

pub(crate) struct Caller(pub(crate) Identity);

impl FromRequestParts<AppState> for Caller {
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, HttpError> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix(BEARER))
            .ok_or(HttpError::Unauthenticated)?;
        sessions::identify(&state.database, token)
            .await?
            .map(Caller)
            .ok_or(HttpError::Unauthenticated)
    }
}

pub(crate) struct ClientAddress(pub(crate) Option<IpAddr>);

impl FromRequestParts<AppState> for ClientAddress {
    type Rejection = Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Infallible>> + Send {
        let connection = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.0);
        ready(Ok(Self(limits::client_address(
            &parts.headers,
            connection,
            state.trust_forwarded,
        ))))
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
