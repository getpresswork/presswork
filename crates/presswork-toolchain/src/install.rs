use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};

use semver::Version;

use crate::client::{Client, Progress};
use crate::digest::Digest;
use crate::error::ToolchainError;
use crate::platform::Platform;
use crate::release::Release;
use crate::unpack::unpack;

#[derive(Debug, Clone)]
pub struct Toolchains {
    root: PathBuf,
    platform: Platform,
}

impl Toolchains {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            platform: Platform::current(),
        }
    }

    #[must_use]
    pub fn executable(&self, version: &Version) -> PathBuf {
        self.folder(version).join(self.platform.executable())
    }

    pub fn installed(&self) -> Result<Vec<Version>, ToolchainError> {
        let typst = self.typst();
        let entries = match fs::read_dir(&typst) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(io_error(&typst, source)),
        };
        let mut versions: Vec<Version> = entries
            .flatten()
            .filter_map(|entry| Version::parse(&entry.file_name().to_string_lossy()).ok())
            .filter(|version| self.executable(version).is_file())
            .collect();
        versions.sort_unstable_by(|left, right| right.cmp(left));
        Ok(versions)
    }

    pub fn remove(&self, version: &Version) -> Result<(), ToolchainError> {
        let _lock = self.lock(version)?;
        let folder = self.folder(version);
        fs::remove_dir_all(&folder).map_err(|source| io_error(&folder, source))
    }

    pub async fn install(
        &self,
        client: &Client,
        release: &Release,
        progress: impl FnMut(Progress),
    ) -> Result<(), ToolchainError> {
        let version = release.version.clone();
        let (Some(name), Some(file)) = (self.platform.release_file(), release.file(&self.platform))
        else {
            return Err(ToolchainError::NoFile(version));
        };
        let published = file
            .digest
            .ok_or_else(|| ToolchainError::NoDigest(version.clone()))?;
        let lock = self.lock(&version)?;
        let typst = self.typst();
        let work = tempfile::Builder::new()
            .prefix(".download-")
            .tempdir_in(&typst)
            .map_err(|source| io_error(&typst, source))?;
        let archive = work.path().join(name);
        client.download(&file.url, &archive, progress).await?;
        let toolchains = self.clone();
        let installed = tokio::task::spawn_blocking(move || {
            toolchains.install_unlocked(&version, &archive, &published)
        })
        .await
        .map_err(|error| ToolchainError::Interrupted(error.to_string()));
        drop(lock);
        installed?
    }

    pub fn install_archive(
        &self,
        version: &Version,
        archive: &Path,
        published: &Digest,
    ) -> Result<(), ToolchainError> {
        let _lock = self.lock(version)?;
        self.install_unlocked(version, archive, published)
    }

    fn install_unlocked(
        &self,
        version: &Version,
        archive: &Path,
        published: &Digest,
    ) -> Result<(), ToolchainError> {
        let actual = Digest::of_file(archive).map_err(|source| io_error(archive, source))?;
        if actual != *published {
            return Err(ToolchainError::DigestMismatch {
                file: archive.display().to_string(),
                published: published.to_hex(),
                actual: actual.to_hex(),
            });
        }
        let typst = self.typst();
        let work = tempfile::Builder::new()
            .prefix(".unpack-")
            .tempdir_in(&typst)
            .map_err(|source| io_error(&typst, source))?;
        let files = work.path().join("files");
        unpack(archive, &files)?;
        let source = self
            .program_folder(&files)
            .ok_or_else(|| ToolchainError::NoProgram(version.clone()))?;
        let target = self.folder(version);
        if target.exists() {
            fs::remove_dir_all(&target).map_err(|source| io_error(&target, source))?;
        }
        fs::rename(&source, &target).map_err(|error| io_error(&target, error))
    }

    pub(crate) fn lock(&self, version: &Version) -> Result<File, ToolchainError> {
        let typst = self.typst();
        fs::create_dir_all(&typst).map_err(|source| io_error(&typst, source))?;
        let path = typst.join(format!(".{version}.lock"));
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| io_error(&path, source))?;
        match file.try_lock() {
            Ok(()) => Ok(file),
            Err(TryLockError::WouldBlock) => Err(ToolchainError::Busy(version.clone())),
            Err(TryLockError::Error(source)) => Err(io_error(&path, source)),
        }
    }

    fn typst(&self) -> PathBuf {
        self.root.join("typst")
    }

    fn folder(&self, version: &Version) -> PathBuf {
        self.typst().join(version.to_string())
    }

    fn program_folder(&self, files: &Path) -> Option<PathBuf> {
        let program = self.platform.executable();
        if files.join(program).is_file() {
            return Some(files.to_owned());
        }
        fs::read_dir(files)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|folder| folder.join(program).is_file())
    }
}

fn io_error(path: &Path, source: io::Error) -> ToolchainError {
    ToolchainError::Io {
        path: path.to_owned(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::ReleaseFile;
    use crate::testing::{Route, platform_archive, run, serve, zip_file};

    struct Served {
        release: Release,
        base: String,
        size: u64,
    }

    fn served(digest: impl FnOnce(Digest) -> Option<Digest>) -> Served {
        let downloads = tempfile::tempdir().unwrap();
        let archive = platform_archive(downloads.path());
        let body = std::fs::read(&archive).unwrap();
        let size = body.len() as u64;
        let digest = digest(Digest::of_file(&archive).unwrap());
        let base = serve(move |_| vec![Route::new("/typst", body)]);
        let release = Release {
            version: version("0.14.2"),
            files: vec![ReleaseFile {
                name: Platform::current().release_file().unwrap(),
                url: format!("{base}/typst"),
                size,
                digest,
            }],
        };
        Served {
            release,
            base,
            size,
        }
    }

    fn client(base: &str) -> Client {
        Client::with_releases("Presswork tests", &format!("{base}/releases")).unwrap()
    }

    #[test]
    #[ignore = "downloads a real Typst release from GitHub"]
    fn a_real_release_installs_and_runs() {
        let temporary = tempfile::tempdir().unwrap();
        let root = std::env::var_os("PRESSWORK_TOOLCHAINS")
            .map_or_else(|| temporary.path().to_path_buf(), PathBuf::from);
        let toolchains = Toolchains::new(root);
        let client = Client::new("Presswork tests").unwrap();
        let wanted = version("0.14.2");
        let releases = run(client.releases()).unwrap();
        let release = releases
            .iter()
            .find(|release| release.version == wanted)
            .unwrap();

        run(toolchains.install(&client, release, |_| {})).unwrap();

        let output = std::process::Command::new(toolchains.executable(&wanted))
            .arg("--version")
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&output.stdout).contains("0.14.2"));
    }

    #[test]
    fn install_downloads_checks_and_installs_a_release() {
        let served = served(Some);
        let root = tempfile::tempdir().unwrap();
        let toolchains = Toolchains::new(root.path());
        let mut received = 0;

        run(
            toolchains.install(&client(&served.base), &served.release, |progress| {
                received = progress.received;
            }),
        )
        .unwrap();

        assert_eq!(toolchains.installed().unwrap(), [version("0.14.2")]);
        assert_eq!(received, served.size);
        assert_eq!(leftovers(root.path()), ["0.14.2"]);
    }

    #[test]
    fn a_download_with_another_digest_installs_nothing() {
        let served = served(|_| Digest::from_hex(&"0".repeat(64)));
        let root = tempfile::tempdir().unwrap();
        let toolchains = Toolchains::new(root.path());

        let result = run(toolchains.install(&client(&served.base), &served.release, |_| {}));

        assert!(matches!(result, Err(ToolchainError::DigestMismatch { .. })));
        assert_eq!(leftovers(root.path()), Vec::<String>::new());
    }

    #[test]
    fn a_release_without_a_digest_installs_nothing() {
        let served = served(|_| None);
        let root = tempfile::tempdir().unwrap();
        let toolchains = Toolchains::new(root.path());

        let result = run(toolchains.install(&client(&served.base), &served.release, |_| {}));

        assert!(
            matches!(result, Err(ToolchainError::NoDigest(missing)) if missing == version("0.14.2"))
        );
        assert_eq!(toolchains.installed().unwrap(), []);
    }

    #[test]
    fn a_release_without_a_file_for_the_platform_installs_nothing() {
        let mut served = served(Some);
        served.release.files.clear();
        let root = tempfile::tempdir().unwrap();
        let toolchains = Toolchains::new(root.path());

        let result = run(toolchains.install(&client(&served.base), &served.release, |_| {}));

        assert!(
            matches!(result, Err(ToolchainError::NoFile(missing)) if missing == version("0.14.2"))
        );
    }

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn executable() -> &'static str {
        Platform::current().executable()
    }

    fn release_archive(folder: &Path, entries: &[(&str, &[u8])]) -> (PathBuf, Digest) {
        let path = zip_file(folder, entries);
        let digest = Digest::of_file(&path).unwrap();
        (path, digest)
    }

    fn typst_archive(folder: &Path) -> (PathBuf, Digest) {
        let program = format!("typst-x/{}", executable());
        release_archive(
            folder,
            &[
                (program.as_str(), b"binary"),
                ("typst-x/LICENSE", b"Apache"),
            ],
        )
    }

    fn leftovers(root: &Path) -> Vec<String> {
        std::fs::read_dir(root.join("typst"))
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| Path::new(name).extension().is_none_or(|end| end != "lock"))
            .collect()
    }

    #[test]
    fn an_archive_with_the_published_digest_installs_its_version() {
        let downloads = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let toolchains = Toolchains::new(root.path());
        let (archive, digest) = typst_archive(downloads.path());

        toolchains
            .install_archive(&version("0.14.2"), &archive, &digest)
            .unwrap();

        assert_eq!(
            std::fs::read(toolchains.executable(&version("0.14.2"))).unwrap(),
            b"binary"
        );
        assert_eq!(toolchains.installed().unwrap(), [version("0.14.2")]);
        assert_eq!(leftovers(root.path()), ["0.14.2"]);
    }

    #[test]
    fn an_archive_with_another_digest_installs_nothing() {
        let downloads = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let toolchains = Toolchains::new(root.path());
        let (archive, _) = typst_archive(downloads.path());
        let other = Digest::from_hex(&"0".repeat(64)).unwrap();

        let result = toolchains.install_archive(&version("0.14.2"), &archive, &other);

        assert!(matches!(result, Err(ToolchainError::DigestMismatch { .. })));
        assert_eq!(toolchains.installed().unwrap(), []);
        assert_eq!(leftovers(root.path()), Vec::<String>::new());
    }

    #[test]
    fn an_archive_without_the_program_installs_nothing() {
        let downloads = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let toolchains = Toolchains::new(root.path());
        let (archive, digest) = release_archive(downloads.path(), &[("typst-x/README.md", b"")]);

        let result = toolchains.install_archive(&version("0.14.2"), &archive, &digest);

        assert!(
            matches!(result, Err(ToolchainError::NoProgram(found)) if found == version("0.14.2"))
        );
        assert_eq!(leftovers(root.path()), Vec::<String>::new());
    }

    #[test]
    fn installed_versions_start_with_the_highest() {
        let root = tempfile::tempdir().unwrap();
        let toolchains = Toolchains::new(root.path());
        for name in ["0.13.1", "0.14.2", "0.14.0"] {
            let folder = root.path().join("typst").join(name);
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join(executable()), "binary").unwrap();
        }
        std::fs::create_dir_all(root.path().join("typst/notes")).unwrap();
        std::fs::create_dir_all(root.path().join("typst/0.12.0")).unwrap();

        assert_eq!(
            toolchains.installed().unwrap(),
            [version("0.14.2"), version("0.14.0"), version("0.13.1")]
        );
    }

    #[test]
    fn a_missing_folder_has_no_installed_versions() {
        let root = tempfile::tempdir().unwrap();

        assert_eq!(
            Toolchains::new(root.path().join("missing"))
                .installed()
                .unwrap(),
            []
        );
    }

    #[test]
    fn remove_deletes_a_version() {
        let downloads = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let toolchains = Toolchains::new(root.path());
        let (archive, digest) = typst_archive(downloads.path());
        toolchains
            .install_archive(&version("0.14.2"), &archive, &digest)
            .unwrap();

        toolchains.remove(&version("0.14.2")).unwrap();

        assert_eq!(toolchains.installed().unwrap(), []);
    }

    #[test]
    fn an_installation_in_progress_blocks_a_second_one() {
        let downloads = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let toolchains = Toolchains::new(root.path());
        let (archive, digest) = typst_archive(downloads.path());
        let _first = toolchains.lock(&version("0.14.2")).unwrap();

        let result = toolchains.install_archive(&version("0.14.2"), &archive, &digest);

        assert!(matches!(result, Err(ToolchainError::Busy(busy)) if busy == version("0.14.2")));
    }
}
