use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use presswork_api::{
    AcceptInvite, ApiError, CreatedInvite, Document, Health, Joined, LibraryFile, Membership,
    NewDocument, NewInvite, NewPassword, NewProject, NewTask, NewWork, Project, ProjectChange,
    Role, SearchHit, Session, SignIn, SignedIn, Status, Task, TaskChange, TaskState, Work,
};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use uuid::Uuid;

use presswork_domain::{Action, allowed};

use crate::http::{Caller, HttpError};
use crate::hub::Hub;
use crate::library::{self, Library};
use crate::{accounts, database, documents, links, sessions, work};

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
        delete_task,
        list_documents,
        create_document,
        list_works,
        create_work,
        upload_file,
        search_library
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
        TaskChange,
        Document,
        NewDocument,
        Work,
        NewWork,
        LibraryFile,
        SearchHit
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
    pub(crate) hub: Arc<Hub>,
    pub(crate) library: Library,
}

impl AppState {
    #[must_use]
    pub fn new(database: PgPool, library: Library) -> Self {
        Self {
            database,
            hub: Arc::default(),
            library,
        }
    }
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
        .route(
            "/api/v1/projects/{id}/documents",
            get(list_documents).post(create_document),
        )
        .route(
            "/api/v1/organizations/{id}/works",
            get(list_works).post(create_work),
        )
        .route(
            "/api/v1/works/{id}/file",
            put(upload_file).layer(DefaultBodyLimit::max(library::MAX_FILE_BYTES)),
        )
        .route(
            "/api/v1/organizations/{id}/library/search",
            get(search_library),
        )
        .route("/api/v1/live", get(crate::live::upgrade))
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
    path = "/api/v1/projects/{id}/documents",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the project.")),
    responses(
        (status = 200, description = "The documents of the project, in the sequence of their paths.", body = [Document]),
        (status = 404, description = "The project does not exist for this user.", body = ApiError)
    )
)]
async fn list_documents(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Document>>, HttpError> {
    let organization = project_organization(&state, id).await?;
    authorize(&state, organization, signed.user_id, Action::ReadProjects).await?;
    Ok(Json(documents::documents(&state.database, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/v1/projects/{id}/documents",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the project.")),
    request_body = NewDocument,
    responses(
        (status = 201, description = "The new document. It is empty.", body = Document),
        (status = 400, description = "The path is not valid.", body = ApiError),
        (status = 404, description = "The project does not exist for this user.", body = ApiError),
        (status = 409, description = "The project has a document with this path.", body = ApiError)
    )
)]
async fn create_document(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
    Json(request): Json<NewDocument>,
) -> Result<(StatusCode, Json<Document>), HttpError> {
    let organization = project_organization(&state, id).await?;
    authorize(&state, organization, signed.user_id, Action::EditDocuments).await?;
    let document =
        documents::create_document(&state.database, id, signed.user_id, &request.path).await?;
    Ok((StatusCode::CREATED, Json(document)))
}

#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/works",
    tag = "library",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the organization.")),
    responses(
        (status = 200, description = "The works of the library of the organization, in the sequence of their titles.", body = [Work]),
        (status = 401, description = "The request has no valid session.", body = ApiError),
        (status = 404, description = "The user is not a member of the organization.", body = ApiError)
    )
)]
async fn list_works(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Work>>, HttpError> {
    authorize(&state, id, signed.user_id, Action::ReadLibrary).await?;
    Ok(Json(library::works(&state.database, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/v1/organizations/{id}/works",
    tag = "library",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the organization.")),
    request_body = NewWork,
    responses(
        (status = 201, description = "The new work. It has no file.", body = Work),
        (status = 400, description = "A value is not valid.", body = ApiError),
        (status = 404, description = "The user is not a member of the organization.", body = ApiError),
        (status = 409, description = "The organization has a work with this citation key.", body = ApiError)
    )
)]
async fn create_work(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
    Json(request): Json<NewWork>,
) -> Result<(StatusCode, Json<Work>), HttpError> {
    authorize(&state, id, signed.user_id, Action::AddToLibrary).await?;
    let work = library::create_work(&state.database, id, signed.user_id, &request).await?;
    Ok((StatusCode::CREATED, Json(work)))
}

#[utoipa::path(
    put,
    path = "/api/v1/works/{id}/file",
    tag = "library",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The ID of the work.")),
    request_body(
        description = "The PDF file of the work. The limit is 64 MiB. A new file takes the place of the old file.",
        content(("application/pdf"))
    ),
    responses(
        (status = 200, description = "The work with its file. The server has read the text of all pages.", body = Work),
        (status = 400, description = "The file is not a PDF file that the server can read, or it is encrypted.", body = ApiError),
        (status = 404, description = "The work does not exist for this user.", body = ApiError),
        (status = 413, description = "The file is larger than 64 MiB.")
    )
)]
async fn upload_file(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
    file: Bytes,
) -> Result<Json<Work>, HttpError> {
    let organization = library::work_organization(&state.database, id)
        .await?
        .ok_or(HttpError::NotFound)?;
    authorize(&state, organization, signed.user_id, Action::AddToLibrary).await?;
    let work = state
        .library
        .add_file(&state.database, organization, id, signed.user_id, file)
        .await?;
    Ok(Json(work))
}

#[derive(Debug, Deserialize)]
struct LibraryQuery {
    q: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/library/search",
    tag = "library",
    security(("bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "The ID of the organization."),
        ("q" = String, Query, description = "The words to find. Each page in the result has all the words.")
    ),
    responses(
        (status = 200, description = "The pages with the words. The best matches come first.", body = [SearchHit]),
        (status = 404, description = "The user is not a member of the organization.", body = ApiError)
    )
)]
async fn search_library(
    State(state): State<AppState>,
    Caller(signed): Caller,
    Path(id): Path<Uuid>,
    Query(query): Query<LibraryQuery>,
) -> Result<Json<Vec<SearchHit>>, HttpError> {
    authorize(&state, id, signed.user_id, Action::ReadLibrary).await?;
    Ok(Json(
        state.library.search(&state.database, id, &query.q).await?,
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
        get_from(router(state(unreachable())), path).await
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

        let (status, body) = get_from(router(state(database.pool)), "/api/v1/health/ready").await;

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

    fn state(pool: PgPool) -> AppState {
        AppState::new(pool, Library::in_memory().unwrap())
    }

    fn app(pool: &PgPool) -> Router {
        router(state(pool.clone()))
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
            router(state(unreachable())),
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

    async fn document(
        pool: &PgPool,
        project: &str,
        token: &str,
        path: &str,
    ) -> (StatusCode, Value) {
        send(
            app(pool),
            "POST",
            &format!("/api/v1/projects/{project}/documents"),
            Some(token),
            Some(json!({ "path": path })),
        )
        .await
    }

    #[tokio::test]
    async fn members_make_and_list_the_documents_of_a_project() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let team = team(pool).await;
        let project = project(pool, &team).await;

        let (status, main) = document(pool, &project, &team.member, "main.typ").await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(main["path"], "main.typ");
        assert_eq!(main["project_id"], project);
        let (status, _) = document(pool, &project, &team.administrator, "chapters/one.typ").await;
        assert_eq!(status, StatusCode::CREATED);

        let (status, documents) = send(
            app(pool),
            "GET",
            &format!("/api/v1/projects/{project}/documents"),
            Some(&team.member),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let paths: Vec<&str> = documents
            .as_array()
            .unwrap()
            .iter()
            .map(|document| document["path"].as_str().unwrap())
            .collect();
        assert_eq!(paths, ["chapters/one.typ", "main.typ"]);
    }

    #[tokio::test]
    async fn a_document_needs_a_member_and_a_valid_unused_path() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let team = team(pool).await;
        let project = project(pool, &team).await;
        document(pool, &project, &team.member, "main.typ").await;

        let (status, body) = document(pool, &project, &team.member, "main.typ").await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], "conflict");
        let (status, body) = document(pool, &project, &team.member, "../main.typ").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "invalid_request");
        let (status, _) = document(pool, &project, &team.outsider, "notes.typ").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn the_openapi_description_contains_the_document_routes() {
        let (_, body) = get("/api/v1/openapi.json").await;

        for method in ["get", "post"] {
            assert!(
                body["paths"]["/api/v1/projects/{id}/documents"][method].is_object(),
                "{method}"
            );
        }
    }

    const PDF: &[u8] = include_bytes!("../../presswork-ocr/fixtures/typst.pdf");

    async fn upload(router: Router, work: &str, token: &str, file: &'static [u8]) -> StatusCode {
        let request = Request::put(format!("/api/v1/works/{work}/file"))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/pdf")
            .body(Body::from(file))
            .unwrap();
        router.oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn members_add_works_and_find_the_pages_of_their_files() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let team = team(pool).await;
        let library = Library::in_memory().unwrap();
        let app = || router(AppState::new(pool.clone(), library.clone()));
        let works = format!("/api/v1/organizations/{}/works", team.organization);

        let (status, work) = send(
            app(),
            "POST",
            &works,
            Some(&team.member),
            Some(json!({
                "key": "field-notes",
                "title": "Field Notes",
                "creators": ["Gutenberg, Johannes"],
                "year": 1455
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(work["file"], Value::Null);
        let id = work["id"].as_str().unwrap();
        assert_eq!(upload(app(), id, &team.member, PDF).await, StatusCode::OK);

        let (status, listed) = send(app(), "GET", &works, Some(&team.administrator), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(listed[0]["key"], "field-notes");
        assert_eq!(listed[0]["file"]["pages"], 2);
        let (status, hits) = send(
            app(),
            "GET",
            &format!(
                "/api/v1/organizations/{}/library/search?q=fine%20grain",
                team.organization
            ),
            Some(&team.member),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(hits[0]["key"], "field-notes");
        assert_eq!(hits[0]["page"], 1);
    }

    #[tokio::test]
    async fn the_library_refuses_wrong_works_files_and_persons() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let team = team(pool).await;
        let works = format!("/api/v1/organizations/{}/works", team.organization);
        let new = json!({ "key": "field-notes", "title": "Field Notes" });

        let (status, _) = send(
            app(pool),
            "POST",
            &works,
            Some(&team.member),
            Some(json!({ "key": "Field Notes", "title": "Field Notes" })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, work) = send(
            app(pool),
            "POST",
            &works,
            Some(&team.member),
            Some(new.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let (status, _) = send(app(pool), "POST", &works, Some(&team.member), Some(new)).await;
        assert_eq!(status, StatusCode::CONFLICT);
        let id = work["id"].as_str().unwrap();
        assert_eq!(
            upload(app(pool), id, &team.member, b"Good paper").await,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            upload(app(pool), &Uuid::nil().to_string(), &team.member, PDF).await,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            upload(app(pool), id, &team.outsider, PDF).await,
            StatusCode::NOT_FOUND
        );
        let (status, _) = send(app(pool), "GET", &works, Some(&team.outsider), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn the_openapi_description_contains_the_library_routes() {
        let (_, body) = get("/api/v1/openapi.json").await;

        for (path, method) in [
            ("/api/v1/organizations/{id}/works", "get"),
            ("/api/v1/organizations/{id}/works", "post"),
            ("/api/v1/works/{id}/file", "put"),
            ("/api/v1/organizations/{id}/library/search", "get"),
        ] {
            assert!(body["paths"][path][method].is_object(), "{path} {method}");
        }
    }

    #[tokio::test]
    async fn an_unknown_path_is_not_found() {
        let (status, _) = get("/api/v1/nothing").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
