use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use presswork_typst::{DownloadError, Downloader, PackageName};
use serde_json::Value;

use crate::client::Client;
use crate::error::ToolchainError;
use crate::unpack::unpack;

pub const PACKAGE_SERVER: &str = "https://packages.typst.org";
const RETRY_AFTER: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub disciplines: Vec<String>,
    pub compiler: Option<String>,
    pub updated: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Packages {
    client: Client,
    server: String,
    root: PathBuf,
    lock: Arc<Mutex<()>>,
    failures: Arc<Failures>,
}

#[derive(Debug, Default)]
struct Failures(Mutex<HashMap<String, (Instant, DownloadError)>>);

impl Packages {
    #[must_use]
    pub fn new(client: Client, root: PathBuf) -> Self {
        Self {
            client,
            server: PACKAGE_SERVER.to_owned(),
            root,
            lock: Arc::default(),
            failures: Arc::default(),
        }
    }

    #[must_use]
    pub fn downloader(user_agent: &str) -> Option<Arc<dyn Downloader>> {
        let root = presswork_typst::package_cache()?;
        let client = Client::new(user_agent).ok()?;
        Some(Arc::new(Self::new(client, root)))
    }

    #[must_use]
    pub fn with_server(self, server: &str) -> Self {
        Self {
            server: server.trim_end_matches('/').to_owned(),
            ..self
        }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn install(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
    ) -> Result<PathBuf, ToolchainError> {
        let package = format!("@{namespace}/{name}:{version}");
        if !valid_name(namespace) || !valid_name(name) || !valid_version(version) {
            return Err(ToolchainError::InvalidPackage(package));
        }
        let parent = self.root.join(namespace).join(name);
        let folder = parent.join(version);
        if folder.is_dir() {
            return Ok(folder);
        }
        fs::create_dir_all(&self.root).map_err(io_error(&self.root))?;
        let temp = tempfile::Builder::new()
            .prefix(".download-")
            .tempdir_in(&self.root)
            .map_err(io_error(&self.root))?;
        let archive = temp.path().join("package.tar.gz");
        let url = format!("{}/{namespace}/{name}-{version}.tar.gz", self.server);
        match self.client.download(&url, &archive, |_| {}).await {
            Err(ToolchainError::Http { status: 404 }) => {
                return Err(self.missing(namespace, name, package).await);
            }
            result => result?,
        }
        let files = temp.path().join("files");
        unpack(&archive, &files)?;
        fs::create_dir_all(&parent).map_err(io_error(&parent))?;
        match fs::rename(&files, &folder) {
            Ok(()) => Ok(folder),
            Err(_) if folder.is_dir() => Ok(folder),
            Err(source) => Err(ToolchainError::Io {
                path: folder,
                source,
            }),
        }
    }

    pub async fn index(&self, namespace: &str) -> Result<Vec<PackageInfo>, ToolchainError> {
        if !valid_name(namespace) {
            return Err(ToolchainError::InvalidPackage(namespace.to_owned()));
        }
        let url = format!("{}/{namespace}/index.json", self.server);
        parse_index(&self.client.text(&url).await?)
    }

    async fn missing(&self, namespace: &str, name: &str, package: String) -> ToolchainError {
        let latest = self
            .index(namespace)
            .await
            .ok()
            .and_then(|index| latest_version(&index, name));
        match latest {
            Some(latest) => ToolchainError::PackageVersionNotFound { package, latest },
            None => ToolchainError::PackageNotFound(package),
        }
    }
}

pub fn parse_index(text: &str) -> Result<Vec<PackageInfo>, ToolchainError> {
    let values: Vec<Value> = serde_json::from_str(text)
        .map_err(|error| ToolchainError::InvalidPackageIndex(error.to_string()))?;
    Ok(values.iter().filter_map(package_info).collect())
}

fn package_info(value: &Value) -> Option<PackageInfo> {
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    let list = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
    };
    Some(PackageInfo {
        name: text("name")?,
        version: text("version")?,
        description: text("description"),
        authors: list("authors"),
        license: text("license"),
        repository: text("repository"),
        homepage: text("homepage"),
        keywords: list("keywords"),
        categories: list("categories"),
        disciplines: list("disciplines"),
        compiler: text("compiler"),
        updated: value.get("updatedAt").and_then(Value::as_i64),
    })
}

fn latest_version(index: &[PackageInfo], name: &str) -> Option<String> {
    index
        .iter()
        .filter(|info| info.name == name)
        .filter_map(|info| Some((semver::Version::parse(&info.version).ok()?, &info.version)))
        .max_by(|one, other| one.0.cmp(&other.0))
        .map(|(_, version)| version.clone())
}

fn valid_name(part: &str) -> bool {
    !part.is_empty()
        && part
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn valid_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty() && part.chars().all(|character| character.is_ascii_digit())
        })
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> ToolchainError {
    let path = path.to_path_buf();
    move |source| ToolchainError::Io { path, source }
}

impl Failures {
    fn recent(&self, package: &str, now: Instant) -> Option<DownloadError> {
        let failures = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        failures
            .get(package)
            .filter(|(at, _)| now.saturating_duration_since(*at) < RETRY_AFTER)
            .map(|(_, failure)| failure.clone())
    }

    fn remember(&self, package: String, now: Instant, result: &Result<PathBuf, DownloadError>) {
        let mut failures = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match result {
            Ok(_) => {
                failures.remove(&package);
            }
            Err(failure) => {
                failures.insert(package, (now, failure.clone()));
            }
        }
    }
}

impl Downloader for Packages {
    fn download(&self, package: &PackageName) -> Result<PathBuf, DownloadError> {
        let key = package.to_string();
        if let Some(failure) = self.failures.recent(&key, Instant::now()) {
            return Err(failure);
        }
        let _one_at_a_time = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| DownloadError::Failed(error.to_string()))
            .and_then(|runtime| {
                runtime
                    .block_on(self.install(&package.namespace, &package.name, &package.version))
                    .map_err(download_error)
            });
        match &result {
            Ok(_) => tracing::info!(%package, "Package downloaded"),
            Err(error) => tracing::warn!(%package, ?error, "The package was not downloaded"),
        }
        self.failures.remember(key, Instant::now(), &result);
        result
    }
}

fn download_error(error: ToolchainError) -> DownloadError {
    match error {
        ToolchainError::PackageNotFound(_) => DownloadError::NotFound,
        ToolchainError::PackageVersionNotFound { latest, .. } => {
            DownloadError::VersionNotFound(latest)
        }
        error => DownloadError::Failed(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::testing::{Route, run, serve, tar_gz_bytes};

    const AGENT: &str = "Presswork tests";

    const INDEX: &str = r#"[
        {
            "name": "greeting",
            "version": "0.1.5",
            "description": "Says hello.",
            "authors": ["Ann <ann@example.com>"],
            "license": "MIT",
            "repository": "https://example.com/greeting",
            "keywords": ["hello"],
            "categories": ["utility"],
            "compiler": "0.13.0",
            "updatedAt": 1763700000
        },
        { "name": "greeting", "version": "0.10.0", "description": null },
        { "version": "1.0.0" },
        { "name": "other", "version": "2.0.0", "keywords": "not a list" }
    ]"#;

    fn packages(base: &str, root: &Path) -> Packages {
        Packages::new(Client::new(AGENT).unwrap(), root.to_path_buf()).with_server(base)
    }

    fn greeting() -> Vec<u8> {
        tar_gz_bytes(&[
            (
                "typst.toml",
                tar::EntryType::Regular,
                b"[package]\nname = \"greeting\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n",
            ),
            ("lib.typ", tar::EntryType::Regular, b"#let greet = [Hello]"),
        ])
    }

    fn package(name: &str, version: &str) -> PackageName {
        PackageName {
            namespace: "preview".to_owned(),
            name: name.to_owned(),
            version: version.to_owned(),
        }
    }

    #[test]
    fn a_package_that_is_not_found_leaves_no_folder() {
        let base = serve(|_| Vec::new());
        let root = tempfile::tempdir().unwrap();

        let result = run(packages(&base, root.path()).install("preview", "greeting", "0.1.0"));

        assert!(result.is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn a_package_installs_from_the_package_server() {
        let archive = greeting();
        let base = serve(move |_| vec![Route::new("/preview/greeting-0.1.0.tar.gz", archive)]);
        let root = tempfile::tempdir().unwrap();

        let folder =
            run(packages(&base, root.path()).install("preview", "greeting", "0.1.0")).unwrap();

        assert_eq!(folder, root.path().join("preview/greeting/0.1.0"));
        assert_eq!(
            fs::read_to_string(folder.join("lib.typ")).unwrap(),
            "#let greet = [Hello]"
        );
        let names: Vec<String> = fs::read_dir(root.path().join("preview/greeting"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["0.1.0"]);
    }

    #[test]
    fn an_installed_package_is_not_downloaded_again() {
        let base = serve(|_| Vec::new());
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("preview/greeting/0.1.0");
        fs::create_dir_all(&folder).unwrap();

        let installed = run(packages(&base, root.path()).install("preview", "greeting", "0.1.0"));

        assert_eq!(installed.unwrap(), folder);
    }

    #[test]
    fn a_missing_version_names_the_latest_version() {
        let base = serve(|_| vec![Route::new("/preview/index.json", INDEX)]);
        let root = tempfile::tempdir().unwrap();

        let result = run(packages(&base, root.path()).install("preview", "greeting", "0.3.0"));

        assert!(
            matches!(&result, Err(ToolchainError::PackageVersionNotFound { latest, .. }) if latest == "0.10.0"),
            "{result:?}"
        );
        assert!(!root.path().join("preview/greeting/0.3.0").exists());
    }

    #[test]
    fn a_package_that_is_not_in_the_index_is_not_found() {
        let base = serve(|_| vec![Route::new("/preview/index.json", INDEX)]);
        let root = tempfile::tempdir().unwrap();

        let result = run(packages(&base, root.path()).install("preview", "missing", "0.1.0"));

        assert!(
            matches!(&result, Err(ToolchainError::PackageNotFound(package)) if package == "@preview/missing:0.1.0"),
            "{result:?}"
        );
    }

    #[test]
    fn an_invalid_package_name_is_refused() {
        let base = serve(|_| Vec::new());
        let root = tempfile::tempdir().unwrap();
        let packages = packages(&base, root.path());

        for (namespace, name, version) in [
            ("preview", "../escape", "0.1.0"),
            ("pre/view", "greeting", "0.1.0"),
            ("preview", "", "0.1.0"),
            ("preview", "greeting", "0.1"),
            ("preview", "greeting", "../0.1.0"),
        ] {
            let result = run(packages.install(namespace, name, version));

            assert!(
                matches!(result, Err(ToolchainError::InvalidPackage(_))),
                "{namespace}/{name}:{version}: {result:?}"
            );
        }
    }

    #[test]
    fn the_index_gives_the_details_of_each_version() {
        let index = parse_index(INDEX).unwrap();

        assert_eq!(
            index
                .iter()
                .map(|info| format!("{}:{}", info.name, info.version))
                .collect::<Vec<_>>(),
            ["greeting:0.1.5", "greeting:0.10.0", "other:2.0.0"]
        );
        assert_eq!(
            index[0],
            PackageInfo {
                name: "greeting".to_owned(),
                version: "0.1.5".to_owned(),
                description: Some("Says hello.".to_owned()),
                authors: vec!["Ann <ann@example.com>".to_owned()],
                license: Some("MIT".to_owned()),
                repository: Some("https://example.com/greeting".to_owned()),
                homepage: None,
                keywords: vec!["hello".to_owned()],
                categories: vec!["utility".to_owned()],
                disciplines: Vec::new(),
                compiler: Some("0.13.0".to_owned()),
                updated: Some(1_763_700_000),
            }
        );
        assert_eq!(index[1].description, None);
        assert_eq!(index[2].keywords, Vec::<String>::new());
    }

    #[test]
    fn an_index_that_is_not_a_list_gives_an_error() {
        assert!(matches!(
            parse_index("{}"),
            Err(ToolchainError::InvalidPackageIndex(_))
        ));
    }

    #[test]
    fn the_index_comes_from_the_package_server() {
        let base = serve(|_| vec![Route::new("/preview/index.json", INDEX)]);
        let root = tempfile::tempdir().unwrap();

        let index = run(packages(&base, root.path()).index("preview")).unwrap();

        assert_eq!(index.len(), 3);
    }

    #[test]
    fn the_downloader_gives_the_folder_of_the_package() {
        let archive = greeting();
        let base = serve(move |_| vec![Route::new("/preview/greeting-0.1.0.tar.gz", archive)]);
        let root = tempfile::tempdir().unwrap();

        let folder =
            Downloader::download(&packages(&base, root.path()), &package("greeting", "0.1.0"))
                .unwrap();

        assert!(folder.join("lib.typ").is_file());
    }

    #[test]
    fn the_downloader_tells_a_missing_version() {
        let base = serve(|_| vec![Route::new("/preview/index.json", INDEX)]);
        let root = tempfile::tempdir().unwrap();

        let result =
            Downloader::download(&packages(&base, root.path()), &package("greeting", "0.3.0"));

        assert_eq!(
            result,
            Err(DownloadError::VersionNotFound("0.10.0".to_owned()))
        );
    }

    #[test]
    fn a_failure_is_remembered_for_a_short_time() {
        let failures = Failures::default();
        let now = Instant::now();
        let offline = DownloadError::Failed("offline".to_owned());

        failures.remember("@preview/a:0.1.0".to_owned(), now, &Err(offline.clone()));

        let later = now + Duration::from_secs(10);
        assert_eq!(failures.recent("@preview/a:0.1.0", later), Some(offline));
        assert_eq!(failures.recent("@preview/b:0.1.0", later), None);
        let much_later = now + RETRY_AFTER + Duration::from_secs(1);
        assert_eq!(failures.recent("@preview/a:0.1.0", much_later), None);
        failures.remember("@preview/a:0.1.0".to_owned(), now, &Ok(PathBuf::new()));
        assert_eq!(failures.recent("@preview/a:0.1.0", later), None);
    }
}
