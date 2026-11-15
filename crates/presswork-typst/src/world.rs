use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use typst::Library;
use typst::LibraryExt;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;

use crate::fonts::Fonts;

#[derive(Debug)]
pub struct World {
    library: LazyHash<Library>,
    fonts: Arc<Fonts>,
    root: Option<PathBuf>,
    main: FileId,
    overlays: HashMap<FileId, Source>,
    today: Option<Datetime>,
}

impl World {
    #[must_use]
    pub fn new(fonts: Arc<Fonts>, today: Option<Datetime>) -> Self {
        Self {
            library: LazyHash::new(Library::default()),
            fonts,
            root: None,
            main: main_id(),
            overlays: HashMap::new(),
            today,
        }
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

    fn read(&self, id: FileId) -> FileResult<Vec<u8>> {
        let rooted = id.get();
        if let VirtualRoot::Package(_) = rooted.root() {
            return Err(FileError::Other(Some(
                "Packages are not available yet.".into(),
            )));
        }
        let root = self.root.as_ref().ok_or(FileError::AccessDenied)?;
        let path = rooted
            .vpath()
            .realize(root)
            .map_err(|_| FileError::AccessDenied)?;
        std::fs::read(&path).map_err(|error| FileError::from_io(error, &path))
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

fn file_id(path: &Path) -> Option<FileId> {
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

#[cfg(test)]
mod tests {
    use std::fs;

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
    fn a_missing_file_gives_an_error() {
        let folder = folder(&[("main.typ", "#include \"missing.typ\"")]);

        assert_eq!(errors(&project(folder.path())).len(), 1);
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
