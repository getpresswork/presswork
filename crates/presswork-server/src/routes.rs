use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use presswork_api::{Health, Status};
use sqlx::PgPool;
use utoipa::OpenApi;

use crate::database;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Presswork",
        description = "The API of the Presswork collaboration server."
    ),
    paths(live, ready),
    components(schemas(Health, Status))
)]
struct Description;

#[derive(Debug, Clone)]
pub struct AppState {
    pub database: PgPool,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health/live", get(live))
        .route("/api/v1/health/ready", get(ready))
        .route("/api/v1/openapi.json", get(openapi))
        .with_state(state)
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

    #[tokio::test]
    async fn an_unknown_path_is_not_found() {
        let (status, _) = get("/api/v1/nothing").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
