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
    main: Source,
    today: Option<Datetime>,
}

impl World {
    #[must_use]
    pub fn new(fonts: Arc<Fonts>, today: Option<Datetime>) -> Self {
        Self {
            library: LazyHash::new(Library::default()),
            fonts,
            main: Source::new(main_id(), String::new()),
            today,
        }
    }

    pub fn set_main_text(&mut self, text: &str) {
        if self.main.text() != text {
            self.main.replace(text);
        }
    }

    #[must_use]
    pub fn main_text(&self) -> &str {
        self.main.text()
    }
}

#[expect(clippy::expect_used, reason = "the constant path is valid")]
fn main_id() -> FileId {
    let path = VirtualPath::new("main.typ").expect("main.typ is a valid path");
    FileId::new(RootedPath::new(VirtualRoot::Project, path))
}

impl typst::World for World {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.fonts.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let _ = id;
        Err(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let _ = offset;
        self.today
    }
}
