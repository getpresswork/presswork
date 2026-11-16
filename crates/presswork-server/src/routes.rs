use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use presswork_api::{
    AcceptInvite, ApiError, CreatedInvite, Health, Joined, Membership, NewInvite, NewPassword,
    NewProject, NewTask, Project, ProjectChange, Role, Session, SignIn, SignedIn, Status, Task,
    TaskChange, TaskState,
};
use sqlx::PgPool;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use uuid::Uuid;

use presswork_domain::{Action, allowed};

use crate::http::{Caller, HttpError};
use crate::{accounts, database, links, sessions, work};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Presswork",
        description = "The API of the Presswork collaboration server."
    ),
    paths(
        live,
        ready,
        sign_in,
        list_sessions,
        end_session,
        accept_invite,
        reset_password,
        create_invite,
        list_organizations,
        list_projects,
        create_project,
        rename_project,
        list_tasks,
        create_task,
        update_task,
        delete_task
    ),
    components(schemas(
        Health,
        Status,
        ApiError,
        SignIn,
        SignedIn,
        Session,
        Role,
        AcceptInvite,
        Joined,
        NewPassword,
        NewInvite,
        CreatedInvite,
        Membership,
        Project,
        NewProject,
        ProjectChange,
        Task,
        TaskState,
        NewTask,
        TaskChange
    )),
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
        .route("/api/v1/invites/{token}/accept", post(accept_invite))
        .route("/api/v1/password-resets/{token}", post(reset_password))
        .route("/api/v1/organizations/{id}/invites", post(create_invite))
        .route("/api/v1/organizations", get(list_organizations))
        .route(
            "/api/v1/organizations/{id}/projects",
            get(list_projects).post(create_project),
        )
        .route("/api/v1/projects/{id}", patch(rename_project))
        .route(
            "/api/v1/projects/{id}/tasks",
            get(list_tasks).post(create_task),
        )
        .route("/api/v1/tasks/{id}", patch(update_task).delete(delete_task))
        .with_state(state)
}

async fn authorize(
    state: &AppState,
    organization_id: Uuid,
    user_id: Uuid,
    action: Action,
) -> Result<(), HttpError> {
    match links::role(&state.database, organization_id, user_id).await? {
        None => Err(HttpError::NotFound),
        Some(role) if allowed(role, action) => Ok(()),
        Some(_) => Err(HttpError::Forbidden),
    }
}

async fn project_organization(state: &AppState, project_id: Uuid) -> Result<Uuid, HttpError> {
    work::project_organization(&state.database, project_id)
        .await?
        .ok_or(HttpError::NotFound)
}

#[utoipa::path(
    get,
    path = "/api/v1/organizations",
    tag = "organizations",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "The organizations of the user, with the role in each.", body = [Membership]),
        (status = 401, description = "The request has no valid session.", body = ApiError)
    )
)]
async fn list_organizations(
    State(state): State<AppState>,
    Caller(signed): Caller,
) -> Result<Json<Vec<Membership>>, HttpError> {
    Ok(Json(
        work::memberships(&state.database, signed.user_id).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/projects",
    tag = "projects",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the organization.")),
    responses(
        (status = 200, description = "The projects of the organization.", body = [Project]),
        (status = 401, description = "The request has no valid session.", body = ApiError),
        (status = 404, description = "The user is not a member of the organization.", body = ApiError)
    )
)]
async fn list_projects(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Project>>, HttpError> {
    authorize(&state, id, signed.user_id, Action::ReadProjects).await?;
    Ok(Json(work::projects(&state.database, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/v1/organizations/{id}/projects",
    tag = "projects",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the organization.")),
    request_body = NewProject,
    responses(
        (status = 201, description = "The new project.", body = Project),
        (status = 400, description = "The request is not valid.", body = ApiError),
        (status = 403, description = "Only administrators make projects.", body = ApiError),
        (status = 404, description = "The user is not a member of the organization.", body = ApiError)
    )
)]
async fn create_project(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
    Json(request): Json<NewProject>,
) -> Result<(StatusCode, Json<Project>), HttpError> {
    authorize(&state, id, signed.user_id, Action::ManageProjects).await?;
    let project = work::create_project(&state.database, id, &request.name).await?;
    Ok((StatusCode::CREATED, Json(project)))
}

#[utoipa::path(
    patch,
    path = "/api/v1/projects/{id}",
    tag = "projects",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the project.")),
    request_body = ProjectChange,
    responses(
        (status = 200, description = "The renamed project.", body = Project),
        (status = 400, description = "The request is not valid.", body = ApiError),
        (status = 403, description = "Only administrators rename projects.", body = ApiError),
        (status = 404, description = "The project does not exist for this user.", body = ApiError)
    )
)]
async fn rename_project(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
    Json(request): Json<ProjectChange>,
) -> Result<Json<Project>, HttpError> {
    let organization = project_organization(&state, id).await?;
    authorize(&state, organization, signed.user_id, Action::ManageProjects).await?;
    Ok(Json(
        work::rename_project(&state.database, id, &request.name).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{id}/tasks",
    tag = "tasks",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the project.")),
    responses(
        (status = 200, description = "The tasks of the project.", body = [Task]),
        (status = 404, description = "The project does not exist for this user.", body = ApiError)
    )
)]
async fn list_tasks(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Task>>, HttpError> {
    let organization = project_organization(&state, id).await?;
    authorize(&state, organization, signed.user_id, Action::ReadProjects).await?;
    Ok(Json(work::tasks(&state.database, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/v1/projects/{id}/tasks",
    tag = "tasks",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the project.")),
    request_body = NewTask,
    responses(
        (status = 201, description = "The new task. Its state is to_do.", body = Task),
        (status = 400, description = "The request is not valid.", body = ApiError),
        (status = 404, description = "The project does not exist for this user.", body = ApiError)
    )
)]
async fn create_task(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
    Json(request): Json<NewTask>,
) -> Result<(StatusCode, Json<Task>), HttpError> {
    let organization = project_organization(&state, id).await?;
    authorize(&state, organization, signed.user_id, Action::WriteTasks).await?;
    let task = work::create_task(&state.database, id, signed.user_id, &request).await?;
    Ok((StatusCode::CREATED, Json(task)))
}

#[utoipa::path(
    patch,
    path = "/api/v1/tasks/{id}",
    tag = "tasks",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the task.")),
    request_body = TaskChange,
    responses(
        (status = 200, description = "The changed task.", body = Task),
        (status = 400, description = "The request is not valid.", body = ApiError),
        (status = 404, description = "The task does not exist for this user.", body = ApiError)
    )
)]
async fn update_task(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
    Json(request): Json<TaskChange>,
) -> Result<Json<Task>, HttpError> {
    let (organization, _) = work::task_owner(&state.database, id)
        .await?
        .ok_or(HttpError::NotFound)?;
    authorize(&state, organization, signed.user_id, Action::WriteTasks).await?;
    Ok(Json(
        work::update_task(&state.database, id, &request).await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/tasks/{id}",
    tag = "tasks",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the task.")),
    responses(
        (status = 204, description = "The task is deleted."),
        (status = 403, description = "A member can delete only the tasks that the member made.", body = ApiError),
        (status = 404, description = "The task does not exist for this user.", body = ApiError)
    )
)]
async fn delete_task(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, HttpError> {
    let (organization, creator) = work::task_owner(&state.database, id)
        .await?
        .ok_or(HttpError::NotFound)?;
    let own = creator == Some(signed.user_id);
    authorize(
        &state,
        organization,
        signed.user_id,
        Action::DeleteTask { own },
    )
    .await?;
    work::delete_task(&state.database, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v1/invites/{token}/accept",
    tag = "organizations",
    params(("token" = String, Path, description = "The token of the invite link.")),
    request_body = AcceptInvite,
    responses(
        (status = 201, description = "The person is a member and has a session for this device.", body = Joined),
        (status = 400, description = "The request is not valid.", body = ApiError),
        (status = 401, description = "The password of the existing account is not correct.", body = ApiError),
        (status = 410, description = "The link was used, expired, or does not exist.", body = ApiError)
    )
)]
async fn accept_invite(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(request): Json<AcceptInvite>,
) -> Result<(StatusCode, Json<Joined>), HttpError> {
    let accepted = links::accept_invite(
        &state.database,
        &token,
        &request.name,
        &request.password,
        &request.device,
    )
    .await?;
    tracing::info!(
        user = %accepted.user.id,
        organization = %accepted.organization_id,
        "Invite accepted"
    );
    let joined = Joined {
        organization_id: accepted.organization_id,
        role: accepted.role,
        token: accepted.session_token,
        session: accepted.session,
    };
    Ok((StatusCode::CREATED, Json(joined)))
}

#[utoipa::path(
    post,
    path = "/api/v1/password-resets/{token}",
    tag = "sessions",
    params(("token" = String, Path, description = "The token of the password reset link.")),
    request_body = NewPassword,
    responses(
        (status = 204, description = "The password changed, and all sessions of the user ended."),
        (status = 400, description = "The password is not valid.", body = ApiError),
        (status = 410, description = "The link was used, expired, or does not exist.", body = ApiError)
    )
)]
async fn reset_password(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(request): Json<NewPassword>,
) -> Result<StatusCode, HttpError> {
    links::reset_password(&state.database, &token, &request.password).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v1/organizations/{id}/invites",
    tag = "organizations",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the organization.")),
    request_body = NewInvite,
    responses(
        (status = 201, description = "The invite link. Send it to the person.", body = CreatedInvite),
        (status = 401, description = "The request has no valid session.", body = ApiError),
        (status = 403, description = "Only administrators of the organization make invite links.", body = ApiError)
    )
)]
async fn create_invite(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
    Json(request): Json<NewInvite>,
) -> Result<(StatusCode, Json<CreatedInvite>), HttpError> {
    authorize(&state, id, signed.user_id, Action::InvitePeople).await?;
    let (token, invite) = links::invite(&state.database, id, &request.email, request.role).await?;
    tracing::info!(organization = %id, invite = %invite.id, "Invite made");
    let created = CreatedInvite {
        token,
        email: invite.email,
        role: invite.role,
        expires_at: invite.expires_at,
    };
    Ok((StatusCode::CREATED, Json(created)))
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

    async fn press_with_invite(pool: &PgPool, role: Role) -> (Uuid, String) {
        let organization = crate::links::create_organization(pool, "Field Press")
            .await
            .unwrap();
        let (token, _) = crate::links::invite(pool, organization.id, "bob@example.com", role)
            .await
            .unwrap();
        (organization.id, token)
    }

    async fn accept(pool: &PgPool, token: &str) -> (StatusCode, Value) {
        send(
            app(pool),
            "POST",
            &format!("/api/v1/invites/{token}/accept"),
            None,
            Some(json!({ "name": "Bob", "password": PASSWORD, "device": "Laptop" })),
        )
        .await
    }

    #[tokio::test]
    async fn an_invite_link_signs_in_the_new_member() {
        let Some(database) = test_database().await else {
            return;
        };
        let (organization, token) = press_with_invite(&database.pool, Role::Member).await;

        let (status, body) = accept(&database.pool, &token).await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["organization_id"], organization.to_string());
        assert_eq!(body["role"], "member");
        assert!(body["token"].as_str().unwrap().starts_with("pws_"));
    }

    #[tokio::test]
    async fn a_used_invite_link_is_gone() {
        let Some(database) = test_database().await else {
            return;
        };
        let (_, token) = press_with_invite(&database.pool, Role::Member).await;
        accept(&database.pool, &token).await;

        let (status, body) = accept(&database.pool, &token).await;

        assert_eq!(status, StatusCode::GONE);
        assert_eq!(body["code"], "link_not_valid");
    }

    #[tokio::test]
    async fn administrators_make_invite_links_and_members_cannot() {
        let Some(database) = test_database().await else {
            return;
        };
        let (organization, token) = press_with_invite(&database.pool, Role::Administrator).await;
        let (_, administrator) = accept(&database.pool, &token).await;
        let (member_token, _) =
            crate::links::invite(&database.pool, organization, "cy@example.com", Role::Member)
                .await
                .unwrap();
        let (_, member) = send(
            app(&database.pool),
            "POST",
            &format!("/api/v1/invites/{member_token}/accept"),
            None,
            Some(json!({ "name": "Cy", "password": PASSWORD, "device": "Phone" })),
        )
        .await;
        let path = format!("/api/v1/organizations/{organization}/invites");
        let request = json!({ "email": "dee@example.com", "role": "member" });

        let (status, invite) = send(
            app(&database.pool),
            "POST",
            &path,
            administrator["token"].as_str(),
            Some(request.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(invite["token"].as_str().unwrap().starts_with("pwi_"));
        assert_eq!(invite["email"], "dee@example.com");

        let (status, body) = send(
            app(&database.pool),
            "POST",
            &path,
            member["token"].as_str(),
            Some(request),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "forbidden");
    }

    #[tokio::test]
    async fn a_reset_link_sets_a_new_password() {
        let Some(database) = test_database().await else {
            return;
        };
        sign_in_ann(&database.pool).await;
        let token = crate::links::reset_link(&database.pool, "ann@example.com")
            .await
            .unwrap();
        let new_password = "a brand new password";

        let (status, _) = send(
            app(&database.pool),
            "POST",
            &format!("/api/v1/password-resets/{token}"),
            None,
            Some(json!({ "password": new_password })),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);

        let (status, _) = send(
            app(&database.pool),
            "POST",
            "/api/v1/sessions",
            None,
            Some(
                json!({ "email": "ann@example.com", "password": new_password, "device": "Laptop" }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn the_openapi_description_contains_the_link_routes() {
        let (_, body) = get("/api/v1/openapi.json").await;

        for (path, method) in [
            ("/api/v1/invites/{token}/accept", "post"),
            ("/api/v1/password-resets/{token}", "post"),
            ("/api/v1/organizations/{id}/invites", "post"),
        ] {
            assert!(body["paths"][path][method].is_object(), "{path}");
        }
    }

    struct Team {
        organization: Uuid,
        administrator: String,
        member: String,
        outsider: String,
    }

    async fn team(pool: &PgPool) -> Team {
        let organization = crate::links::create_organization(pool, "Field Press")
            .await
            .unwrap()
            .id;
        let mut tokens = Vec::new();
        for (email, role) in [
            ("ann@example.com", Some(Role::Administrator)),
            ("bob@example.com", Some(Role::Member)),
            ("eve@example.com", None),
        ] {
            let user = crate::accounts::create_user(pool, email, "Someone", PASSWORD)
                .await
                .unwrap();
            if let Some(role) = role {
                sqlx::query(
                    "INSERT INTO memberships (organization_id, user_id, role) VALUES ($1, $2, $3)",
                )
                .bind(organization)
                .bind(user.id)
                .bind(role.as_str())
                .execute(pool)
                .await
                .unwrap();
            }
            let (token, _) = sessions::start(pool, user.id, "Laptop").await.unwrap();
            tokens.push(token);
        }
        let [administrator, member, outsider] = <[String; 3]>::try_from(tokens).unwrap();
        Team {
            organization,
            administrator,
            member,
            outsider,
        }
    }

    async fn project(pool: &PgPool, team: &Team) -> String {
        let (status, body) = send(
            app(pool),
            "POST",
            &format!("/api/v1/organizations/{}/projects", team.organization),
            Some(&team.administrator),
            Some(json!({ "name": "Field Notes" })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        body["id"].as_str().unwrap().to_owned()
    }

    async fn task(pool: &PgPool, project: &str, token: &str) -> String {
        let (status, body) = send(
            app(pool),
            "POST",
            &format!("/api/v1/projects/{project}/tasks"),
            Some(token),
            Some(json!({ "title": "Proof the cover", "due_date": "2026-11-23" })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["state"], "to_do");
        body["id"].as_str().unwrap().to_owned()
    }

    #[tokio::test]
    async fn members_see_their_organizations_and_projects() {
        let Some(database) = test_database().await else {
            return;
        };
        let team = team(&database.pool).await;
        let project = project(&database.pool, &team).await;

        let (status, memberships) = send(
            app(&database.pool),
            "GET",
            "/api/v1/organizations",
            Some(&team.member),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(memberships[0]["name"], "Field Press");
        assert_eq!(memberships[0]["role"], "member");

        let path = format!("/api/v1/organizations/{}/projects", team.organization);
        let (status, projects) =
            send(app(&database.pool), "GET", &path, Some(&team.member), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(projects[0]["id"], project);

        let (status, _) = send(
            app(&database.pool),
            "GET",
            &path,
            Some(&team.outsider),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn only_administrators_make_and_rename_projects() {
        let Some(database) = test_database().await else {
            return;
        };
        let team = team(&database.pool).await;
        let project = project(&database.pool, &team).await;

        let (status, body) = send(
            app(&database.pool),
            "POST",
            &format!("/api/v1/organizations/{}/projects", team.organization),
            Some(&team.member),
            Some(json!({ "name": "Other" })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "forbidden");

        let path = format!("/api/v1/projects/{project}");
        let rename = json!({ "name": "Field Notes, 2nd edition" });
        let (status, _) = send(
            app(&database.pool),
            "PATCH",
            &path,
            Some(&team.member),
            Some(rename.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, renamed) = send(
            app(&database.pool),
            "PATCH",
            &path,
            Some(&team.administrator),
            Some(rename),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(renamed["name"], "Field Notes, 2nd edition");
    }

    #[tokio::test]
    async fn members_make_change_and_complete_tasks() {
        let Some(database) = test_database().await else {
            return;
        };
        let team = team(&database.pool).await;
        let project = project(&database.pool, &team).await;
        let task = task(&database.pool, &project, &team.member).await;

        let (status, changed) = send(
            app(&database.pool),
            "PATCH",
            &format!("/api/v1/tasks/{task}"),
            Some(&team.member),
            Some(json!({ "state": "done", "due_date": null })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(changed["state"], "done");
        assert_eq!(changed["due_date"], Value::Null);
        assert_eq!(changed["title"], "Proof the cover");

        let (status, tasks) = send(
            app(&database.pool),
            "GET",
            &format!("/api/v1/projects/{project}/tasks"),
            Some(&team.administrator),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(tasks[0]["state"], "done");
    }

    #[tokio::test]
    async fn a_member_deletes_only_their_own_tasks() {
        let Some(database) = test_database().await else {
            return;
        };
        let team = team(&database.pool).await;
        let project = project(&database.pool, &team).await;
        let administrators_task = task(&database.pool, &project, &team.administrator).await;
        let members_task = task(&database.pool, &project, &team.member).await;
        let pool = &database.pool;

        assert_eq!(
            delete_task(pool, &administrators_task, &team.member).await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            delete_task(pool, &members_task, &team.outsider).await,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            delete_task(pool, &members_task, &team.member).await,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            delete_task(pool, &administrators_task, &team.administrator).await,
            StatusCode::NO_CONTENT
        );
    }

    async fn delete_task(pool: &PgPool, task: &str, token: &str) -> StatusCode {
        send(
            app(pool),
            "DELETE",
            &format!("/api/v1/tasks/{task}"),
            Some(token),
            None,
        )
        .await
        .0
    }

    #[tokio::test]
    async fn the_openapi_description_contains_the_project_and_task_routes() {
        let (_, body) = get("/api/v1/openapi.json").await;

        for (path, method) in [
            ("/api/v1/organizations", "get"),
            ("/api/v1/organizations/{id}/projects", "get"),
            ("/api/v1/organizations/{id}/projects", "post"),
            ("/api/v1/projects/{id}", "patch"),
            ("/api/v1/projects/{id}/tasks", "get"),
            ("/api/v1/projects/{id}/tasks", "post"),
            ("/api/v1/tasks/{id}", "patch"),
            ("/api/v1/tasks/{id}", "delete"),
        ] {
            assert!(body["paths"][path][method].is_object(), "{method} {path}");
        }
    }

    #[tokio::test]
    async fn an_unknown_path_is_not_found() {
        let (status, _) = get("/api/v1/nothing").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
