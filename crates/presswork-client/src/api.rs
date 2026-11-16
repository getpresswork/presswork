use std::time::Duration;

use presswork_api::{ApiError, Document, Membership, NewDocument, Project, SignIn, SignedIn};
use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::live::{ClientError, server_base};

const USER_AGENT: &str = concat!("Presswork/", env!("CARGO_PKG_VERSION"));
const CONNECT_TIME: Duration = Duration::from_secs(15);
const READ_TIME: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct Api {
    http: reqwest::Client,
    server: String,
}

impl Api {
    pub fn new(server: &str) -> Result<Self, ClientError> {
        let server = server_base(server)?.to_owned();
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(CONNECT_TIME)
            .read_timeout(READ_TIME)
            .build()?;
        Ok(Self { http, server })
    }

    #[must_use]
    pub fn server(&self) -> &str {
        &self.server
    }

    pub async fn sign_in(
        &self,
        email: &str,
        password: &str,
        device: &str,
    ) -> Result<SignedIn, ClientError> {
        let request = SignIn {
            email: email.to_owned(),
            password: password.to_owned(),
            device: device.to_owned(),
        };
        answer(self.http.post(self.url("/api/v1/sessions")).json(&request)).await
    }

    pub async fn organizations(&self, token: &str) -> Result<Vec<Membership>, ClientError> {
        answer(
            self.http
                .get(self.url("/api/v1/organizations"))
                .bearer_auth(token),
        )
        .await
    }

    pub async fn projects(
        &self,
        token: &str,
        organization: Uuid,
    ) -> Result<Vec<Project>, ClientError> {
        let path = format!("/api/v1/organizations/{organization}/projects");
        answer(self.http.get(self.url(&path)).bearer_auth(token)).await
    }

    pub async fn documents(
        &self,
        token: &str,
        project: Uuid,
    ) -> Result<Vec<Document>, ClientError> {
        let path = format!("/api/v1/projects/{project}/documents");
        answer(self.http.get(self.url(&path)).bearer_auth(token)).await
    }

    pub async fn create_document(
        &self,
        token: &str,
        project: Uuid,
        path: &str,
    ) -> Result<Document, ClientError> {
        let address = self.url(&format!("/api/v1/projects/{project}/documents"));
        let request = NewDocument {
            path: path.to_owned(),
        };
        answer(self.http.post(address).bearer_auth(token).json(&request)).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.server)
    }
}

async fn answer<T: DeserializeOwned>(request: RequestBuilder) -> Result<T, ClientError> {
    let response = request.send().await?;
    let status = response.status();
    if status.is_success() {
        return Ok(response.json().await?);
    }
    match response.json::<ApiError>().await {
        Ok(error) => Err(ClientError::Refused {
            code: error.code,
            message: error.message,
        }),
        Err(_) => Err(ClientError::Status(status.as_u16())),
    }
}

#[cfg(test)]
mod tests {
    use axum::extract::Path;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use jiff::Timestamp;
    use presswork_api::{
        ApiError, Document, Membership, NewDocument, Project, Role, Session, SignIn, SignedIn,
    };
    use tokio::net::TcpListener;
    use uuid::Uuid;

    use super::*;

    const PRESS: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0001);
    const NOTES: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0002);
    const MAIN: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0003);

    async fn serve(router: Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        format!("http://{address}/")
    }

    fn signed_in(headers: &HeaderMap) -> bool {
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            == Some("Bearer pws_ann")
    }

    fn refused() -> (StatusCode, Json<ApiError>) {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                code: "unauthenticated".to_owned(),
                message: "Sign in to use this function.".to_owned(),
            }),
        )
    }

    fn main_document() -> Document {
        Document {
            id: MAIN,
            project_id: NOTES,
            path: "main.typ".to_owned(),
        }
    }

    fn press() -> Router {
        Router::new()
            .route(
                "/api/v1/sessions",
                post(|Json(request): Json<SignIn>| async move {
                    if request.password != "correct horse battery" {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(ApiError {
                                code: "wrong_credentials".to_owned(),
                                message: "The email address or the password is not correct."
                                    .to_owned(),
                            }),
                        ));
                    }
                    Ok((
                        StatusCode::CREATED,
                        Json(SignedIn {
                            token: "pws_ann".to_owned(),
                            session: Session {
                                id: Uuid::nil(),
                                device: request.device,
                                created_at: Timestamp::UNIX_EPOCH,
                                last_used_at: Timestamp::UNIX_EPOCH,
                                current: true,
                            },
                        }),
                    ))
                }),
            )
            .route(
                "/api/v1/organizations",
                get(|headers: HeaderMap| async move {
                    if !signed_in(&headers) {
                        return Err(refused());
                    }
                    Ok(Json(vec![Membership {
                        organization_id: PRESS,
                        name: "Field Press".to_owned(),
                        role: Role::Member,
                    }]))
                }),
            )
            .route(
                "/api/v1/organizations/{id}/projects",
                get(|headers: HeaderMap, Path(id): Path<Uuid>| async move {
                    if !signed_in(&headers) {
                        return Err(refused());
                    }
                    Ok(Json(vec![Project {
                        id: NOTES,
                        organization_id: id,
                        name: "Field Notes".to_owned(),
                    }]))
                }),
            )
            .route(
                "/api/v1/projects/{id}/documents",
                get(|headers: HeaderMap, Path(_): Path<Uuid>| async move {
                    if !signed_in(&headers) {
                        return Err(refused());
                    }
                    Ok(Json(vec![main_document()]))
                })
                .post(
                    |headers: HeaderMap,
                     Path(project): Path<Uuid>,
                     Json(request): Json<NewDocument>| async move {
                        if !signed_in(&headers) {
                            return Err(refused());
                        }
                        Ok((
                            StatusCode::CREATED,
                            Json(Document {
                                id: MAIN,
                                project_id: project,
                                path: request.path,
                            }),
                        ))
                    },
                ),
            )
    }

    #[test]
    fn a_server_address_needs_http_or_https() {
        assert!(matches!(
            Api::new("press.example.com"),
            Err(ClientError::Address)
        ));
        assert_eq!(
            Api::new("https://press.example.com/").unwrap().server(),
            "https://press.example.com"
        );
    }

    #[tokio::test]
    async fn sign_in_gives_the_token_of_the_new_session() {
        let api = Api::new(&serve(press()).await).unwrap();

        let signed = api
            .sign_in("ann@example.com", "correct horse battery", "Laptop")
            .await
            .unwrap();

        assert_eq!(signed.token, "pws_ann");
        assert_eq!(signed.session.device, "Laptop");
    }

    #[tokio::test]
    async fn a_refusal_gives_the_code_of_the_server() {
        let api = Api::new(&serve(press()).await).unwrap();

        let refused = api
            .sign_in("ann@example.com", "wrong password", "Laptop")
            .await;
        let unauthenticated = api.organizations("pws_old").await;

        assert!(
            matches!(&refused, Err(ClientError::Refused { code, .. }) if code == "wrong_credentials"),
            "{refused:?}"
        );
        assert!(
            matches!(&unauthenticated, Err(ClientError::Refused { code, .. }) if code == "unauthenticated"),
            "{unauthenticated:?}"
        );
    }

    #[tokio::test]
    async fn lists_come_from_the_server_with_the_session() {
        let api = Api::new(&serve(press()).await).unwrap();

        let organizations = api.organizations("pws_ann").await.unwrap();
        let projects = api.projects("pws_ann", PRESS).await.unwrap();
        let documents = api.documents("pws_ann", NOTES).await.unwrap();

        assert_eq!(organizations[0].name, "Field Press");
        assert_eq!(projects[0].organization_id, PRESS);
        assert_eq!(documents, [main_document()]);
    }

    #[tokio::test]
    async fn a_new_document_gets_its_path() {
        let api = Api::new(&serve(press()).await).unwrap();

        let document = api
            .create_document("pws_ann", NOTES, "chapters/one.typ")
            .await
            .unwrap();

        assert_eq!(document.project_id, NOTES);
        assert_eq!(document.path, "chapters/one.typ");
    }
}
