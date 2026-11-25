use std::collections::HashMap;
use std::fs;
use std::io::Read;
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
const README: &str = "README.md";
const MAX_ARCHIVE: u64 = 64 * 1024 * 1024;
const MAX_README: u64 = 1024 * 1024;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    pub info: PackageInfo,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageIndex {
    entries: Vec<PackageEntry>,
}

impl PackageIndex {
    #[must_use]
    pub fn new(infos: Vec<PackageInfo>) -> Self {
        let mut groups: HashMap<String, Vec<PackageInfo>> = HashMap::new();
        for info in infos {
            groups.entry(info.name.clone()).or_default().push(info);
        }
        let mut entries: Vec<PackageEntry> = groups
            .into_values()
            .filter_map(|mut versions| {
                versions.sort_by_key(|info| std::cmp::Reverse(version_key(&info.version)));
                let names = versions.iter().map(|info| info.version.clone()).collect();
                Some(PackageEntry {
                    info: versions.into_iter().next()?,
                    versions: names,
                })
            })
            .collect();
        entries.sort_by(|one, other| one.info.name.cmp(&other.info.name));
        Self { entries }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn entry(&self, name: &str) -> Option<&PackageEntry> {
        self.entries.iter().find(|entry| entry.info.name == name)
    }

    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&PackageEntry> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let mut found: Vec<(usize, &PackageEntry)> = self
            .entries
            .iter()
            .filter_map(|entry| Some((score(entry, &words)?, entry)))
            .collect();
        found.sort_by(|one, other| {
            one.0
                .cmp(&other.0)
                .then_with(|| one.1.info.name.cmp(&other.1.info.name))
        });
        found.into_iter().map(|(_, entry)| entry).collect()
    }
}

fn score(entry: &PackageEntry, words: &[String]) -> Option<usize> {
    let info = &entry.info;
    let name = info.name.to_lowercase();
    let tags: Vec<String> = info
        .keywords
        .iter()
        .chain(&info.categories)
        .chain(&info.disciplines)
        .map(|tag| tag.to_lowercase())
        .collect();
    let description = info
        .description
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    words
        .iter()
        .map(|word| {
            let word = word.as_str();
            if name == word {
                Some(0)
            } else if name.starts_with(word) {
                Some(1)
            } else if name.contains(word) {
                Some(2)
            } else if tags.iter().any(|tag| tag == word) {
                Some(3)
            } else if tags.iter().any(|tag| tag.contains(word)) {
                Some(4)
            } else if description.contains(word) {
                Some(5)
            } else {
                None
            }
        })
        .sum()
}

fn version_key(version: &str) -> Option<semver::Version> {
    semver::Version::parse(version).ok()
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
    pub fn in_typst_cache(user_agent: &str) -> Option<Self> {
        let root = presswork_typst::package_cache()?;
        let client = Client::new(user_agent).ok()?;
        Some(Self::new(client, root))
    }

    #[must_use]
    pub fn downloader(user_agent: &str) -> Option<Arc<dyn Downloader>> {
        Self::in_typst_cache(user_agent).map(|packages| Arc::new(packages) as Arc<dyn Downloader>)
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

    pub async fn readme(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
    ) -> Result<Option<String>, ToolchainError> {
        if !valid_name(namespace) || !valid_name(name) || !valid_version(version) {
            return Err(ToolchainError::InvalidPackage(format!(
                "@{namespace}/{name}:{version}"
            )));
        }
        let folder = self.root.join(namespace).join(name).join(version);
        if folder.is_dir() {
            return Ok(readme_in_folder(&folder));
        }
        let url = format!("{}/{namespace}/{name}-{version}.tar.gz", self.server);
        readme_in_archive(&self.client.bytes(&url, MAX_ARCHIVE).await?)
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

fn is_readme(name: &str) -> bool {
    name.eq_ignore_ascii_case(README)
}

fn readme_in_folder(folder: &Path) -> Option<String> {
    let entry = fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .find(|entry| is_readme(&entry.file_name().to_string_lossy()))?;
    let bytes = fs::read(entry.path()).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn readme_in_archive(bytes: &[u8]) -> Result<Option<String>, ToolchainError> {
    let broken = |error: std::io::Error| ToolchainError::BrokenArchive(error.to_string());
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    for entry in archive.entries().map_err(broken)? {
        let entry = entry.map_err(broken)?;
        let path = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        if !entry.header().entry_type().is_file() || !is_readme(path.trim_start_matches("./")) {
            continue;
        }
        let mut text = Vec::new();
        entry
            .take(MAX_README)
            .read_to_end(&mut text)
            .map_err(broken)?;
        return Ok(Some(String::from_utf8_lossy(&text).into_owned()));
    }
    Ok(None)
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
    use std::collections::BTreeSet;
    use std::fs;

    use presswork_typst::{Fonts, World, compile};

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

    type Check = fn(&Value) -> bool;

    fn is_list(value: &Value) -> bool {
        value
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_string))
    }

    #[test]
    #[ignore = "downloads the real package index and a package from Typst Universe"]
    fn the_real_package_server_gives_the_index_and_the_packages() {
        let root = tempfile::tempdir().unwrap();
        let packages = Packages::new(Client::new(AGENT).unwrap(), root.path().to_path_buf());
        let url = format!("{PACKAGE_SERVER}/preview/index.json");
        let text = run(packages.client.text(&url)).unwrap();
        let values: Vec<Value> = serde_json::from_str(&text).unwrap();
        let keys: BTreeSet<&str> = values
            .iter()
            .filter_map(Value::as_object)
            .flat_map(|entry| entry.keys().map(String::as_str))
            .collect();
        let fields: [(&str, Check); 12] = [
            ("name", Value::is_string),
            ("version", Value::is_string),
            ("description", Value::is_string),
            ("authors", is_list),
            ("license", Value::is_string),
            ("repository", Value::is_string),
            ("homepage", Value::is_string),
            ("keywords", is_list),
            ("categories", is_list),
            ("disciplines", is_list),
            ("compiler", Value::is_string),
            ("updatedAt", Value::is_i64),
        ];
        for (key, valid) in fields {
            let found: Vec<&Value> = values.iter().filter_map(|value| value.get(key)).collect();
            assert!(
                !found.is_empty(),
                "No entry has {key}. The keys are {keys:?}."
            );
            assert!(found.into_iter().all(valid), "{key} has a different type.");
        }
        let index = parse_index(&text).unwrap();
        assert_eq!(index.len(), values.len());
        assert!(PackageIndex::new(index).entry("example").is_some());

        let readme = run(packages.readme("preview", "example", "0.1.0")).unwrap();
        assert!(readme.is_some_and(|text| !text.is_empty()));

        let mut world = World::new(Arc::new(Fonts::embedded()), None)
            .with_packages(vec![root.path().to_path_buf()])
            .with_downloader(Some(Arc::new(packages)));
        world.set_main_text("#import \"@preview/example:0.1.0\": add\n#add(2, 7)");
        let compilation = compile(&world);

        assert!(
            compilation.document.is_some(),
            "{:?}",
            compilation.diagnostics
        );
        assert!(
            root.path()
                .join("preview/example/0.1.0/typst.toml")
                .is_file()
        );
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

    fn info(name: &str, version: &str, description: &str, keywords: &[&str]) -> PackageInfo {
        PackageInfo {
            name: name.to_owned(),
            version: version.to_owned(),
            description: Some(description.to_owned()),
            keywords: keywords.iter().map(|word| (*word).to_owned()).collect(),
            ..PackageInfo::default()
        }
    }

    fn names(entries: &[&PackageEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| entry.info.name.clone())
            .collect()
    }

    #[test]
    fn the_index_keeps_one_entry_for_each_package_with_its_newest_version() {
        let index = PackageIndex::new(vec![
            info("cetz", "0.3.4", "Old.", &[]),
            info("cetz", "0.10.0", "New.", &[]),
            info("algo", "1.0.0", "Algorithms.", &[]),
        ]);

        assert_eq!(index.len(), 2);
        assert_eq!(names(&index.search("")), ["algo", "cetz"]);
        let cetz = index.entry("cetz").unwrap();
        assert_eq!(cetz.info.description.as_deref(), Some("New."));
        assert_eq!(cetz.versions, ["0.10.0", "0.3.4"]);
    }

    #[test]
    fn a_search_puts_name_matches_before_other_matches() {
        let index = PackageIndex::new(vec![
            info(
                "fletcher",
                "0.5.0",
                "Diagrams with arrows, after cetz.",
                &["diagram"],
            ),
            info("cetz", "0.3.4", "Drawing with Typst.", &["draw", "canvas"]),
            info("cetz-plot", "0.1.0", "Plots.", &["plot"]),
            info("tablex", "0.0.9", "More tables.", &["table"]),
        ]);

        assert_eq!(
            names(&index.search("cetz")),
            ["cetz", "cetz-plot", "fletcher"]
        );
        assert_eq!(names(&index.search("CANVAS")), ["cetz"]);
        assert_eq!(names(&index.search("plot cetz")), ["cetz-plot"]);
        assert_eq!(names(&index.search("tables")), ["tablex"]);
        assert_eq!(names(&index.search("nothing")), Vec::<String>::new());
    }

    fn with_readme(name: &str, text: &[u8]) -> Vec<u8> {
        tar_gz_bytes(&[
            ("typst.toml", tar::EntryType::Regular, b"[package]"),
            (name, tar::EntryType::Regular, text),
        ])
    }

    #[test]
    fn the_readme_comes_from_the_archive_without_an_installation() {
        let archive = with_readme("./README.md", b"# Greeting\nSays hello.");
        let base = serve(move |_| vec![Route::new("/preview/greeting-0.1.0.tar.gz", archive)]);
        let root = tempfile::tempdir().unwrap();

        let readme = run(packages(&base, root.path()).readme("preview", "greeting", "0.1.0"));

        assert_eq!(readme.unwrap().as_deref(), Some("# Greeting\nSays hello."));
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn the_readme_of_an_installed_package_comes_from_its_folder() {
        let base = serve(|_| Vec::new());
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("preview/greeting/0.1.0");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("readme.md"), "Local.").unwrap();

        let readme = run(packages(&base, root.path()).readme("preview", "greeting", "0.1.0"));

        assert_eq!(readme.unwrap().as_deref(), Some("Local."));
    }

    #[test]
    fn a_package_without_a_readme_gives_no_text() {
        let archive = with_readme("docs/README.md", b"Not at the top.");
        let base = serve(move |_| vec![Route::new("/preview/greeting-0.1.0.tar.gz", archive)]);
        let root = tempfile::tempdir().unwrap();

        let readme = run(packages(&base, root.path()).readme("preview", "greeting", "0.1.0"));

        assert_eq!(readme.unwrap(), None);
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
