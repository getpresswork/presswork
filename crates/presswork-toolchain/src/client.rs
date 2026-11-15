use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use reqwest::header::{ACCEPT, LINK};
use reqwest::{Response, Url};

use crate::error::ToolchainError;
use crate::release::{Release, parse_releases};

const GITHUB_RELEASES: &str = "https://api.github.com/repos/typst/typst/releases?per_page=100";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PAGES: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub received: u64,
    pub total: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    releases: Url,
}

impl Client {
    pub fn new(user_agent: &str) -> Result<Self, ToolchainError> {
        Self::with_releases(user_agent, GITHUB_RELEASES)
    }

    pub fn with_releases(user_agent: &str, releases: &str) -> Result<Self, ToolchainError> {
        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .build()
            .map_err(|error| network(&error))?;
        let releases = Url::parse(releases).map_err(|error| network(&error))?;
        Ok(Self { http, releases })
    }

    pub async fn releases(&self) -> Result<Vec<Release>, ToolchainError> {
        let mut releases = Vec::new();
        let mut next = Some(self.releases.clone());
        for _ in 0..MAX_PAGES {
            let Some(url) = next.take() else { break };
            let response = self
                .http
                .get(url)
                .header(ACCEPT, "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await
                .map_err(|error| network(&error))?;
            let response = success(response)?;
            next = response
                .headers()
                .get(LINK)
                .and_then(|value| value.to_str().ok())
                .and_then(next_link)
                .and_then(|link| Url::parse(&link).ok())
                .filter(|link| link.origin() == self.releases.origin());
            let text = response.text().await.map_err(|error| network(&error))?;
            releases.extend(parse_releases(&text)?);
        }
        Ok(releases)
    }

    pub async fn download(
        &self,
        url: &str,
        target: &Path,
        mut progress: impl FnMut(Progress),
    ) -> Result<(), ToolchainError> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|error| network(&error))?;
        let mut response = success(response)?;
        let total = response.content_length();
        let io_error = |source| ToolchainError::Io {
            path: target.to_owned(),
            source,
        };
        let mut file = File::create(target).map_err(io_error)?;
        let mut received = 0;
        progress(Progress { received, total });
        while let Some(chunk) = response.chunk().await.map_err(|error| network(&error))? {
            file.write_all(&chunk).map_err(io_error)?;
            received += chunk.len() as u64;
            progress(Progress { received, total });
        }
        file.sync_all().map_err(io_error)
    }
}

fn success(response: Response) -> Result<Response, ToolchainError> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(ToolchainError::Http {
            status: status.as_u16(),
        })
    }
}

fn next_link(header: &str) -> Option<String> {
    let part = header
        .split(',')
        .find(|part| part.contains(r#"rel="next""#))?;
    let start = part.find('<')? + 1;
    let end = part.find('>')?;
    part.get(start..end).map(str::to_owned)
}

fn network(error: &dyn std::fmt::Display) -> ToolchainError {
    ToolchainError::Network(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Route, run, serve};

    const AGENT: &str = "Presswork tests";

    fn release(tag: &str) -> String {
        format!(r#"{{ "tag_name": "{tag}", "draft": false, "assets": [] }}"#)
    }

    fn versions(releases: &[Release]) -> Vec<String> {
        releases
            .iter()
            .map(|release| release.version.to_string())
            .collect()
    }

    #[test]
    fn the_releases_come_from_the_api() {
        let base = serve(|_| vec![Route::new("/releases", format!("[{}]", release("v0.15.1")))]);
        let client = Client::with_releases(AGENT, &format!("{base}/releases")).unwrap();

        let releases = run(client.releases()).unwrap();

        assert_eq!(versions(&releases), ["0.15.1"]);
    }

    #[test]
    fn the_releases_continue_on_the_next_page() {
        let base = serve(|base| {
            vec![
                Route::new("/releases", format!("[{}]", release("v0.15.1"))).header(
                    "Link",
                    &format!(
                        r#"<{base}/releases?page=2>; rel="next", <{base}/releases?page=2>; rel="last""#
                    ),
                ),
                Route::new("/releases?page=2", format!("[{}]", release("v0.14.2"))),
            ]
        });
        let client = Client::with_releases(AGENT, &format!("{base}/releases")).unwrap();

        let releases = run(client.releases()).unwrap();

        assert_eq!(versions(&releases), ["0.15.1", "0.14.2"]);
    }

    #[test]
    fn a_next_page_on_another_server_is_ignored() {
        let base = serve(|_| {
            vec![
                Route::new("/releases", format!("[{}]", release("v0.15.1"))).header(
                    "Link",
                    r#"<https://example.com/releases?page=2>; rel="next""#,
                ),
            ]
        });
        let client = Client::with_releases(AGENT, &format!("{base}/releases")).unwrap();

        let releases = run(client.releases()).unwrap();

        assert_eq!(versions(&releases), ["0.15.1"]);
    }

    #[test]
    fn an_error_status_gives_an_error_with_the_status() {
        let base = serve(|_| vec![Route::new("/releases", "rate limit").status(403)]);
        let client = Client::with_releases(AGENT, &format!("{base}/releases")).unwrap();

        let result = run(client.releases());

        assert!(matches!(result, Err(ToolchainError::Http { status: 403 })));
    }

    #[test]
    fn a_download_writes_the_file_and_tells_the_progress() {
        let data: Vec<u8> = (0..200_000_u32)
            .map(|value| value.to_le_bytes()[0])
            .collect();
        let body = data.clone();
        let base = serve(move |_| vec![Route::new("/typst.zip", body)]);
        let client = Client::with_releases(AGENT, &format!("{base}/releases")).unwrap();
        let folder = tempfile::tempdir().unwrap();
        let target = folder.path().join("typst.zip");
        let mut steps = Vec::new();

        run(
            client.download(&format!("{base}/typst.zip"), &target, |progress| {
                steps.push(progress);
            }),
        )
        .unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), data);
        assert_eq!(
            steps.last(),
            Some(&Progress {
                received: 200_000,
                total: Some(200_000)
            })
        );
        assert!(
            steps
                .windows(2)
                .all(|pair| pair[0].received <= pair[1].received)
        );
    }
}
