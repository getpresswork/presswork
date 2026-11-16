use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use presswork_api::{ApiError, Health, Session, SignIn, SignedIn, Status};
use sqlx::PgPool;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use uuid::Uuid;

use crate::http::{Caller, HttpError};
use crate::{accounts, database, sessions};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Presswork",
        description = "The API of the Presswork collaboration server."
    ),
    paths(live, ready, sign_in, list_sessions, end_session),
    components(schemas(Health, Status, ApiError, SignIn, SignedIn, Session)),
    modifiers(&Bearer)
)]
struct Description;

struct Bearer;

impl Modify for Bearer {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
        );
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub database: PgPool,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health/live", get(live))
        .route("/api/v1/health/ready", get(ready))
        .route("/api/v1/openapi.json", get(openapi))
        .route("/api/v1/sessions", post(sign_in).get(list_sessions))
        .route("/api/v1/sessions/{id}", delete(end_session))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/api/v1/sessions",
    tag = "sessions",
    request_body = SignIn,
    responses(
        (status = 201, description = "The sign-in made a session for this device.", body = SignedIn),
        (status = 400, description = "The request is not valid.", body = ApiError),
        (status = 401, description = "The email address or the password is not correct.", body = ApiError)
    )
)]
async fn sign_in(
    State(state): State<AppState>,
    Json(request): Json<SignIn>,
) -> Result<(StatusCode, Json<SignedIn>), HttpError> {
    let user = accounts::authenticate(&state.database, &request.email, &request.password).await?;
    let (token, session) = sessions::start(&state.database, user.id, &request.device).await?;
    tracing::info!(user = %user.id, session = %session.id, "Signed in");
    Ok((StatusCode::CREATED, Json(SignedIn { token, session })))
}

#[utoipa::path(
    get,
    path = "/api/v1/sessions",
    tag = "sessions",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "The sessions of the user.", body = [Session]),
        (status = 401, description = "The request has no valid session.", body = ApiError)
    )
)]
async fn list_sessions(
    State(state): State<AppState>,
    Caller(signed): Caller,
) -> Result<Json<Vec<Session>>, HttpError> {
    let sessions = sessions::list(&state.database, signed.user_id, signed.session_id).await?;
    Ok(Json(sessions))
}

#[utoipa::path(
    delete,
    path = "/api/v1/sessions/{id}",
    tag = "sessions",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the session.")),
    responses(
        (status = 204, description = "The session ended."),
        (status = 401, description = "The request has no valid session.", body = ApiError),
        (status = 404, description = "The user has no session with this ID.", body = ApiError)
    )
)]
async fn end_session(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, HttpError> {
    if sessions::end(&state.database, signed.user_id, id).await? {
        tracing::info!(user = %signed.user_id, session = %id, "Session ended");
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(HttpError::NotFound)
    }
}

fn health(status: Status) -> Json<Health> {
    Json(Health {
        status,
        version: VERSION.to_owned(),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/health/ready",
    tag = "health",
    responses(
        (status = 200, description = "The server can use its database.", body = Health),
        (status = 503, description = "The server cannot use its database.", body = Health)
    )
)]
async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Health>) {
    if database::is_ready(&state.database).await {
        (StatusCode::OK, health(Status::Ok))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, health(Status::Unavailable))
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/health/live",
    tag = "health",
    responses((status = 200, description = "The server process runs.", body = Health))
)]
async fn live() -> Json<Health> {
    health(Status::Ok)
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(Description::openapi())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::*;
    use crate::database::unreachable;
    use crate::testing::test_database;

    async fn get(path: &str) -> (StatusCode, Value) {
        get_from(
            router(AppState {
                database: unreachable(),
            }),
            path,
        )
        .await
    }

    async fn get_from(router: Router, path: &str) -> (StatusCode, Value) {
        let response = router
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    #[tokio::test]
    async fn the_live_check_answers_with_the_version() {
        let (status, body) = get("/api/v1/health/live").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body,
            json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") })
        );
    }

    #[tokio::test]
    async fn the_openapi_description_contains_each_route() {
        let (status, body) = get("/api/v1/openapi.json").await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["openapi"].as_str().unwrap().starts_with("3."));
        assert!(body["paths"]["/api/v1/health/live"]["get"].is_object());
    }

    #[tokio::test]
    async fn the_ready_check_fails_while_the_database_is_unreachable() {
        let (status, body) = get("/api/v1/health/ready").await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["status"], "unavailable");
    }

    #[tokio::test]
    async fn the_ready_check_passes_with_a_database() {
        let Some(database) = test_database().await else {
            return;
        };

        let (status, body) = get_from(
            router(AppState {
                database: database.pool,
            }),
            "/api/v1/health/ready",
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    async fn the_openapi_description_contains_the_ready_check() {
        let (_, body) = get("/api/v1/openapi.json").await;

        assert!(body["paths"]["/api/v1/health/ready"]["get"].is_object());
    }

    const PASSWORD: &str = "correct horse battery";

    async fn send(
        router: Router,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let request = match body {
            Some(body) => request
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
            None => request.body(Body::empty()).unwrap(),
        };
        let response = router.oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    fn app(pool: &PgPool) -> Router {
        router(AppState {
            database: pool.clone(),
        })
    }

    async fn sign_in_ann(pool: &PgPool) -> Value {
        crate::accounts::create_user(pool, "ann@example.com", "Ann", PASSWORD)
            .await
            .unwrap();
        let (status, body) = send(
            app(pool),
            "POST",
            "/api/v1/sessions",
            None,
            Some(json!({ "email": "ann@example.com", "password": PASSWORD, "device": "Laptop" })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        body
    }

    #[tokio::test]
    async fn a_request_without_a_token_is_unauthenticated() {
        let (status, body) = send(
            router(AppState {
                database: unreachable(),
            }),
            "GET",
            "/api/v1/sessions",
            None,
            None,
        )
        .await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn sign_in_gives_a_token_for_the_right_password() {
        let Some(database) = test_database().await else {
            return;
        };

        let body = sign_in_ann(&database.pool).await;

        assert!(body["token"].as_str().unwrap().starts_with("pws_"));
        assert_eq!(body["session"]["device"], "Laptop");
        assert_eq!(body["session"]["current"], true);
    }

    #[tokio::test]
    async fn sign_in_with_a_wrong_password_is_refused() {
        let Some(database) = test_database().await else {
            return;
        };
        sign_in_ann(&database.pool).await;

        let (status, body) = send(
            app(&database.pool),
            "POST",
            "/api/v1/sessions",
            None,
            Some(json!({ "email": "ann@example.com", "password": "a wrong password", "device": "Phone" })),
        )
        .await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "wrong_credentials");
    }

    #[tokio::test]
    async fn the_sessions_show_the_current_device_and_sign_out_ends_one() {
        let Some(database) = test_database().await else {
            return;
        };
        let signed_in = sign_in_ann(&database.pool).await;
        let token = signed_in["token"].as_str().unwrap();
        let id = signed_in["session"]["id"].as_str().unwrap();

        let (status, sessions) = send(
            app(&database.pool),
            "GET",
            "/api/v1/sessions",
            Some(token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(sessions[0]["device"], "Laptop");
        assert_eq!(sessions[0]["current"], true);

        let (status, _) = send(
            app(&database.pool),
            "DELETE",
            &format!("/api/v1/sessions/{id}"),
            Some(token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);

        let (status, _) = send(
            app(&database.pool),
            "GET",
            "/api/v1/sessions",
            Some(token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn the_openapi_description_contains_the_sessions_and_the_bearer_scheme() {
        let (_, body) = get("/api/v1/openapi.json").await;

        assert!(body["paths"]["/api/v1/sessions"]["post"].is_object());
        assert!(body["paths"]["/api/v1/sessions"]["get"].is_object());
        assert!(body["paths"]["/api/v1/sessions/{id}"]["delete"].is_object());
        assert_eq!(
            body["components"]["securitySchemes"]["bearer"]["scheme"],
            "bearer"
        );
    }

    #[tokio::test]
    async fn an_unknown_path_is_not_found() {
        let (status, _) = get("/api/v1/nothing").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
