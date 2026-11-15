use axum::routing::get;
use axum::{Json, Router};
use presswork_api::{Health, Status};
use utoipa::OpenApi;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Presswork",
        description = "The API of the Presswork collaboration server."
    ),
    paths(live),
    components(schemas(Health, Status))
)]
struct Description;

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/health/live", get(live))
        .route("/api/v1/openapi.json", get(openapi))
}

#[utoipa::path(
    get,
    path = "/api/v1/health/live",
    tag = "health",
    responses((status = 200, description = "The server process runs.", body = Health))
)]
async fn live() -> Json<Health> {
    Json(Health {
        status: Status::Ok,
        version: VERSION.to_owned(),
    })
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

    async fn get(path: &str) -> (StatusCode, Value) {
        let response = router()
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
    async fn an_unknown_path_is_not_found() {
        let (status, _) = get("/api/v1/nothing").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
