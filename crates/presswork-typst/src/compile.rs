use std::ops::Range;
use std::time::{Duration, Instant};

use typst::diag::{self, SourceDiagnostic, Warned};
use typst_layout::PagedDocument;

use crate::outline::{SourceRange, source_range};
use crate::world::World;

const CACHE_AGE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub source: Option<SourceRange>,
    pub message: String,
    pub hints: Vec<String>,
}

impl Diagnostic {
    #[must_use]
    pub fn main_range(&self) -> Option<&Range<usize>> {
        match &self.source {
            Some(SourceRange::Main(range)) => Some(range),
            Some(SourceRange::File(..)) | None => None,
        }
    }
}

#[derive(Debug)]
pub struct Compilation {
    pub document: Option<PagedDocument>,
    pub diagnostics: Vec<Diagnostic>,
    pub duration: Duration,
}

#[must_use]
pub fn compile(world: &World) -> Compilation {
    let start = Instant::now();
    let Warned { output, warnings } = typst::compile::<PagedDocument>(world);
    let (document, errors) = match output {
        Ok(document) => (Some(document), Vec::new()),
        Err(errors) => (None, errors.to_vec()),
    };
    let diagnostics = errors
        .iter()
        .chain(&warnings)
        .map(|diagnostic| convert(world, diagnostic))
        .collect();
    comemo::evict(CACHE_AGE);
    Compilation {
        document,
        diagnostics,
        duration: start.elapsed(),
    }
}

fn convert(world: &World, diagnostic: &SourceDiagnostic) -> Diagnostic {
    Diagnostic {
        severity: match diagnostic.severity {
            diag::Severity::Error => Severity::Error,
            diag::Severity::Warning => Severity::Warning,
        },
        source: source_range(world, diagnostic.span),
        message: diagnostic.message.to_string(),
        hints: diagnostic
            .hints
            .iter()
            .map(|hint| hint.v.to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use typst::foundations::Datetime;

    use super::*;
    use crate::fonts::Fonts;

    fn world(text: &str) -> World {
        let today = Datetime::from_ymd(2026, 11, 15).unwrap();
        let mut world = World::new(Arc::new(Fonts::embedded()), Some(today));
        world.set_main_text(text);
        world
    }

    fn errors(compilation: &Compilation) -> Vec<&Diagnostic> {
        compilation
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .collect()
    }

    #[test]
    fn a_document_compiles_to_pages() {
        let compilation = compile(&world("= Hello\n\nThe first page."));

        assert!(compilation.diagnostics.is_empty(), "{compilation:?}");
        assert_eq!(
            compilation.document.map(|document| document.pages().len()),
            Some(1)
        );
    }

    #[test]
    fn an_error_has_a_message_and_a_range_in_the_text() {
        let text = "Text before.\n#unknown-name";
        let compilation = compile(&world(text));

        let errors = errors(&compilation);
        assert!(compilation.document.is_none());
        assert_eq!(errors.len(), 1);
        assert_ne!(errors[0].message, "");
        assert_eq!(
            errors[0].main_range().map(|range| &text[range.clone()]),
            Some("unknown-name")
        );
    }

    #[test]
    fn an_error_in_another_file_names_the_file() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("chapter.typ"), "#unknown-name").unwrap();
        let today = Datetime::from_ymd(2026, 11, 20);
        let mut world = World::open(
            Arc::new(Fonts::embedded()),
            today,
            folder.path().to_path_buf(),
            std::path::Path::new("main.typ"),
        );
        world.set_main_text("#include \"chapter.typ\"");

        let compilation = compile(&world);

        assert_eq!(
            errors(&compilation)[0].source,
            Some(SourceRange::File(
                std::path::PathBuf::from("chapter.typ"),
                1..13
            ))
        );
    }

    #[test]
    fn a_warning_does_not_stop_the_document() {
        let compilation = compile(&world("#set text(font: \"No Such Font\")\nText."));

        assert!(compilation.document.is_some());
        assert!(
            compilation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Warning)
        );
    }

    #[test]
    fn the_world_gives_the_supplied_date() {
        let text = "#assert.eq(datetime.today(), datetime(year: 2026, month: 11, day: 15))";

        assert_eq!(compile(&world(text)).diagnostics, []);
    }

    #[test]
    fn a_world_without_a_date_gives_an_error_for_today() {
        let mut world = World::new(Arc::new(Fonts::embedded()), None);
        world.set_main_text("#datetime.today()");

        assert_eq!(errors(&compile(&world)).len(), 1);
    }

    #[test]
    fn new_main_text_replaces_the_old_text() {
        let mut world = world("#panic()");
        assert_eq!(errors(&compile(&world)).len(), 1);

        world.set_main_text("No problem.");

        assert_eq!(compile(&world).diagnostics, []);
        assert_eq!(world.main_text(), "No problem.");
    }
}
