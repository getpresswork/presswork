use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use typst::Library;
use typst::LibraryExt;
use typst::diag::{FileError, FileResult, PackageError};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;

use crate::fonts::Fonts;
use crate::package::{DownloadError, Downloader, PackageName, package_cache};

#[derive(Debug, Clone)]
pub struct World {
    library: LazyHash<Library>,
    fonts: Arc<Fonts>,
    root: Option<PathBuf>,
    main: FileId,
    overlays: HashMap<FileId, Source>,
    today: Option<Datetime>,
    packages: Vec<PathBuf>,
    downloader: Option<Arc<dyn Downloader>>,
}

impl World {
    #[must_use]
    pub fn new(fonts: Arc<Fonts>, today: Option<Datetime>) -> Self {
        Self::with_library(fonts, today, LazyHash::new(Library::default()))
    }

    pub(crate) fn with_library(
        fonts: Arc<Fonts>,
        today: Option<Datetime>,
        library: LazyHash<Library>,
    ) -> Self {
        Self {
            library,
            fonts,
            root: None,
            main: main_id(),
            overlays: HashMap::new(),
            today,
            packages: package_folders(),
            downloader: None,
        }
    }

    #[must_use]
    pub fn with_packages(self, packages: Vec<PathBuf>) -> Self {
        Self { packages, ..self }
    }

    #[must_use]
    pub fn with_downloader(mut self, downloader: Option<Arc<dyn Downloader>>) -> Self {
        self.set_downloader(downloader);
        self
    }

    pub fn set_downloader(&mut self, downloader: Option<Arc<dyn Downloader>>) {
        self.downloader = downloader;
    }

    #[must_use]
    pub fn open(fonts: Arc<Fonts>, today: Option<Datetime>, root: PathBuf, main: &Path) -> Self {
        Self {
            root: Some(root),
            main: file_id(main).unwrap_or_else(main_id),
            ..Self::new(fonts, today)
        }
    }

    pub fn set_main_text(&mut self, text: &str) {
        self.set_overlay(self.main, text);
    }

    pub fn set_text(&mut self, path: &Path, text: &str) {
        if let Some(id) = file_id(path) {
            self.set_overlay(id, text);
        }
    }

    pub fn remove_text(&mut self, path: &Path) {
        if let Some(id) = file_id(path) {
            self.overlays.remove(&id);
        }
    }

    fn read(&self, id: FileId) -> FileResult<Vec<u8>> {
        let rooted = id.get();
        let root = match rooted.root() {
            VirtualRoot::Package(spec) => self.package_root(spec)?,
            VirtualRoot::Project => self.root.clone().ok_or(FileError::AccessDenied)?,
        };
        let path = rooted
            .vpath()
            .realize(&root)
            .map_err(|_| FileError::AccessDenied)?;
        std::fs::read(&path).map_err(|error| FileError::from_io(error, &path))
    }

    fn package_root(&self, spec: &PackageSpec) -> FileResult<PathBuf> {
        let relative = Path::new(spec.namespace.as_str())
            .join(spec.name.as_str())
            .join(spec.version.to_string());
        if let Some(folder) = self
            .packages
            .iter()
            .map(|folder| folder.join(&relative))
            .find(|folder| folder.is_dir())
        {
            return Ok(folder);
        }
        let Some(downloader) = self
            .downloader
            .as_ref()
            .filter(|_| spec.namespace == "preview")
        else {
            return Err(FileError::Package(PackageError::NotFound(spec.clone())));
        };
        downloader
            .download(&PackageName::from(spec))
            .map_err(|error| {
                FileError::Package(match error {
                    DownloadError::NotFound => PackageError::NotFound(spec.clone()),
                    DownloadError::VersionNotFound(latest) => match latest.parse() {
                        Ok(latest) => PackageError::VersionNotFound(spec.clone(), latest),
                        Err(_) => PackageError::NotFound(spec.clone()),
                    },
                    DownloadError::Failed(message) => {
                        PackageError::NetworkFailed(Some(message.into()))
                    }
                })
            })
    }

    #[must_use]
    pub fn main_text(&self) -> &str {
        self.overlays.get(&self.main).map_or("", Source::text)
    }

    fn set_overlay(&mut self, id: FileId, text: &str) {
        match self.overlays.get_mut(&id) {
            Some(source) => {
                if source.text() != text {
                    source.replace(text);
                }
            }
            None => {
                self.overlays.insert(id, Source::new(id, text.to_owned()));
            }
        }
    }
}

fn package_folders() -> Vec<PathBuf> {
    let data = directories::BaseDirs::new()
        .map(|folders| folders.data_dir().join("typst").join("packages"));
    data.into_iter().chain(package_cache()).collect()
}

pub(crate) fn file_id(path: &Path) -> Option<FileId> {
    let parts: Vec<&str> = path
        .components()
        .map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let vpath = VirtualPath::new(parts.join("/")).ok()?;
    Some(FileId::new(RootedPath::new(VirtualRoot::Project, vpath)))
}

#[expect(clippy::expect_used, reason = "the constant path is valid")]
fn main_id() -> FileId {
    file_id(Path::new("main.typ")).expect("main.typ is a valid path")
}

impl typst::World for World {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.fonts.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if let Some(source) = self.overlays.get(&id) {
            return Ok(source.clone());
        }
        let text = String::from_utf8(self.read(id)?).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(source) = self.overlays.get(&id) {
            return Ok(Bytes::new(source.text().as_bytes().to_vec()));
        }
        Ok(Bytes::new(self.read(id)?))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let _ = offset;
        self.today
    }
}

impl typst_ide::IdeWorld for World {
    fn upcast(&self) -> &dyn typst::World {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Mutex;

    use tempfile::TempDir;

    use super::*;
    use crate::compile::{Severity, compile};

    fn folder(files: &[(&str, &str)]) -> TempDir {
        let folder = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let path = folder.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        folder
    }

    fn project(root: &Path) -> World {
        World::open(
            Arc::new(Fonts::embedded()),
            None,
            root.to_path_buf(),
            Path::new("main.typ"),
        )
    }

    fn errors(world: &World) -> Vec<String> {
        compile(world)
            .diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .map(|diagnostic| diagnostic.message)
            .collect()
    }

    #[test]
    fn a_project_world_reads_imports_from_the_folder() {
        let folder = folder(&[
            ("main.typ", "#import \"lib/names.typ\": name\nHello #name."),
            ("lib/names.typ", "#let name = [Typst]"),
        ]);

        assert_eq!(errors(&project(folder.path())), Vec::<String>::new());
    }

    #[test]
    fn data_files_are_readable() {
        let folder = folder(&[("main.typ", "#read(\"data.txt\")"), ("data.txt", "Text.")]);

        assert_eq!(errors(&project(folder.path())), Vec::<String>::new());
    }

    #[test]
    fn an_overlay_replaces_the_text_on_disk() {
        let folder = folder(&[("main.typ", "#panic()")]);
        let mut world = project(folder.path());

        world.set_text(Path::new("main.typ"), "No problem.");

        assert_eq!(errors(&world), Vec::<String>::new());
    }

    #[test]
    fn an_overlay_of_an_included_file_is_used() {
        let folder = folder(&[("main.typ", "#include \"part.typ\""), ("part.typ", "Fine.")]);
        let mut world = project(folder.path());

        world.set_text(Path::new("part.typ"), "#panic()");

        assert_eq!(errors(&world).len(), 1);
    }

    #[test]
    fn a_removed_overlay_gives_the_file_on_disk_again() {
        let folder = folder(&[("main.typ", "#include \"part.typ\"")]);
        let mut world = project(folder.path());
        world.set_text(Path::new("part.typ"), "Unsaved.");
        assert_eq!(errors(&world), Vec::<String>::new());

        world.remove_text(Path::new("part.typ"));

        assert_eq!(errors(&world).len(), 1);
    }

    #[test]
    fn a_missing_file_gives_an_error() {
        let folder = folder(&[("main.typ", "#include \"missing.typ\"")]);

        assert_eq!(errors(&project(folder.path())).len(), 1);
    }

    #[test]
    fn a_package_comes_from_the_first_package_folder_that_has_it() {
        let packages = folder(&[
            (
                "local/greeting/0.1.0/typst.toml",
                "[package]\nname = \"greeting\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n",
            ),
            ("local/greeting/0.1.0/lib.typ", "#let greet = [Hello]"),
        ]);
        let empty = tempfile::tempdir().unwrap();
        let folder = folder(&[(
            "main.typ",
            "#import \"@local/greeting:0.1.0\": greet\n#greet",
        )]);

        let world = project(folder.path()).with_packages(vec![
            empty.path().to_path_buf(),
            packages.path().to_path_buf(),
        ]);

        assert_eq!(errors(&world), Vec::<String>::new());
    }

    #[test]
    fn a_package_that_is_not_installed_gives_an_error() {
        let packages = tempfile::tempdir().unwrap();
        let folder = folder(&[("main.typ", "#import \"@local/missing:0.1.0\": *")]);

        let world = project(folder.path()).with_packages(vec![packages.path().to_path_buf()]);

        let errors = errors(&world);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("missing"), "{errors:?}");
    }

    #[derive(Debug)]
    struct FakeDownloader {
        folder: PathBuf,
        failure: Option<DownloadError>,
        calls: Mutex<Vec<PackageName>>,
    }

    impl FakeDownloader {
        fn new(folder: &Path, failure: Option<DownloadError>) -> Arc<Self> {
            Arc::new(Self {
                folder: folder.to_path_buf(),
                failure,
                calls: Mutex::new(Vec::new()),
            })
        }

        fn calls(&self) -> Vec<String> {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .map(ToString::to_string)
                .collect()
        }
    }

    impl Downloader for FakeDownloader {
        fn download(&self, package: &PackageName) -> Result<PathBuf, DownloadError> {
            self.calls.lock().unwrap().push(package.clone());
            if let Some(failure) = &self.failure {
                return Err(failure.clone());
            }
            let folder = self
                .folder
                .join(&package.namespace)
                .join(&package.name)
                .join(&package.version);
            fs::create_dir_all(&folder).unwrap();
            fs::write(
                folder.join("typst.toml"),
                "[package]\nname = \"greeting\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n",
            )
            .unwrap();
            fs::write(folder.join("lib.typ"), "#let greet = [Hello]").unwrap();
            Ok(folder)
        }
    }

    fn downloading(
        main: &str,
        failure: Option<DownloadError>,
    ) -> (World, Arc<FakeDownloader>, TempDir, TempDir) {
        let packages = tempfile::tempdir().unwrap();
        let downloader = FakeDownloader::new(packages.path(), failure);
        let folder = folder(&[("main.typ", main)]);
        let world = project(folder.path())
            .with_packages(vec![packages.path().to_path_buf()])
            .with_downloader(Some(Arc::clone(&downloader) as Arc<dyn Downloader>));
        (world, downloader, packages, folder)
    }

    const PREVIEW_IMPORT: &str = "#import \"@preview/greeting:0.1.0\": greet\n#greet";

    #[test]
    fn a_missing_preview_package_comes_from_the_downloader() {
        let (world, downloader, _packages, _folder) = downloading(PREVIEW_IMPORT, None);

        assert_eq!(errors(&world), Vec::<String>::new());
        assert_eq!(downloader.calls(), ["@preview/greeting:0.1.0"]);
    }

    #[test]
    fn a_package_of_another_namespace_is_not_downloaded() {
        let (world, downloader, _packages, _folder) =
            downloading("#import \"@local/greeting:0.1.0\": greet", None);

        assert_eq!(errors(&world).len(), 1);
        assert_eq!(downloader.calls(), Vec::<String>::new());
    }

    #[test]
    fn a_failed_download_names_the_problem() {
        let failure = DownloadError::Failed("no connection".to_owned());
        let (world, _downloader, _packages, _folder) = downloading(PREVIEW_IMPORT, Some(failure));

        let errors = errors(&world);

        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("no connection"), "{errors:?}");
    }

    #[test]
    fn a_missing_version_names_the_latest_version() {
        let failure = DownloadError::VersionNotFound("0.2.0".to_owned());
        let (world, _downloader, _packages, _folder) = downloading(PREVIEW_IMPORT, Some(failure));

        let errors = errors(&world);

        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("0.2.0"), "{errors:?}");
    }

    #[test]
    fn files_outside_the_folder_are_not_readable() {
        let folder = folder(&[
            ("book/main.typ", "#read(\"../secret.txt\")"),
            ("secret.txt", "secret"),
        ]);

        assert_eq!(errors(&project(&folder.path().join("book"))).len(), 1);
    }
}
