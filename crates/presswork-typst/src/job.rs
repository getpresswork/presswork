use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::compile::{Diagnostic, compile};
use crate::ide::Analysis;
use crate::render::PageImage;
use crate::world::World;

#[derive(Debug, Clone, Default)]
pub struct Output {
    pub pages: Option<Vec<PageImage>>,
    pub diagnostics: Vec<Diagnostic>,
    pub duration: Duration,
    pub analysis: Option<Analysis>,
}

impl Output {
    #[must_use]
    pub fn has_pages(&self) -> bool {
        self.pages.is_some()
            || self
                .analysis
                .as_ref()
                .is_some_and(|analysis| analysis.shared_document().is_some())
    }
}

#[must_use]
pub fn analyze(world: &Mutex<World>, text: &str) -> Output {
    let start = Instant::now();
    let mut world = world.lock().unwrap_or_else(PoisonError::into_inner);
    world.set_main_text(text);
    let compilation = compile(&world);
    Output {
        pages: None,
        diagnostics: compilation.diagnostics,
        duration: start.elapsed(),
        analysis: Some(Analysis::new(world.clone(), compilation.document)),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use typst::foundations::Datetime;

    use super::*;
    use crate::compile::Severity;
    use crate::fonts::Fonts;
    use crate::render::render_page;

    fn world() -> Mutex<World> {
        let today = Datetime::from_ymd(2026, 11, 15).unwrap();
        Mutex::new(World::new(Arc::new(Fonts::embedded()), Some(today)))
    }

    fn render_all(document: &typst_layout::PagedDocument) -> (Duration, usize) {
        let start = Instant::now();
        let bytes = (0..document.pages().len())
            .filter_map(|index| render_page(document, index, 2.0, false))
            .map(|image| image.rgba.len())
            .sum();
        (start.elapsed(), bytes)
    }

    #[test]
    #[ignore = "times a large book; set PRESSWORK_BENCH_PROJECT to its folder"]
    fn time_a_large_book() {
        let Some(root) = std::env::var_os("PRESSWORK_BENCH_PROJECT").map(std::path::PathBuf::from)
        else {
            eprintln!("Set PRESSWORK_BENCH_PROJECT to a project folder with main.typ.");
            return;
        };
        let text = std::fs::read_to_string(root.join("main.typ")).unwrap();
        let start = Instant::now();
        let fonts = Arc::new(Fonts::embedded());
        let fonts_time = start.elapsed();
        let mut world = World::open(
            fonts,
            crate::today(),
            root.clone(),
            std::path::Path::new("main.typ"),
        );
        world.set_main_text(&text);

        let start = Instant::now();
        let cold = compile(&world);
        let cold_time = start.elapsed();
        let errors: Vec<&str> = cold
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        assert_eq!(errors, Vec::<&str>::new());
        let document = cold.document.unwrap();
        let pages = document.pages().len();
        let (cold_render, bytes) = render_all(&document);

        let start = Instant::now();
        let analysis = Analysis::new(world.clone(), Some(document.clone()));
        let headings = analysis.outline().len();
        let outline_time = start.elapsed();
        let start = Instant::now();
        let problems = analysis
            .quality(&presswork_project::TypesetSettings::default())
            .len();
        let quality_time = start.elapsed();

        let middle = text[..text.len() / 2].rfind(' ').unwrap_or(0);
        let mut changed = text.clone();
        changed.insert_str(middle, " word");
        world.set_main_text(&changed);
        let start = Instant::now();
        let hot = compile(&world);
        let hot_time = start.elapsed();
        let (hot_render, _) = render_all(&hot.document.unwrap());

        eprintln!("pages: {pages}, headings: {headings}, QA problems: {problems}");
        eprintln!("fonts: {fonts_time:?}");
        eprintln!("cold compile: {cold_time:?}");
        eprintln!(
            "cold render of all pages: {cold_render:?} ({:?} per page, {} MB of images)",
            cold_render / u32::try_from(pages.max(1)).unwrap(),
            bytes / 1_000_000
        );
        eprintln!("outline: {outline_time:?}, QA report: {quality_time:?}");
        eprintln!("hot compile after one word: {hot_time:?}");
        eprintln!("hot render of all pages: {hot_render:?}");
    }

    #[test]
    fn an_analysis_keeps_the_document_for_page_images() {
        let output = analyze(&world(), "One.\n#pagebreak()\nTwo.");

        let pages = output
            .analysis
            .as_ref()
            .and_then(Analysis::shared_document)
            .map(|document| document.pages().len());
        assert_eq!(pages, Some(2));
        assert!(output.has_pages());
    }

    #[test]
    fn an_analysis_compiles_without_page_images() {
        let output = analyze(&world(), "One.\n#pagebreak()\nTwo.");

        assert_eq!(output.pages, None);
        assert!(
            output
                .analysis
                .unwrap()
                .export(&crate::ExportSettings {
                    format: crate::Format::Svg,
                    standards: Vec::new(),
                    pixel_per_pt: 1.0,
                    date: None,
                })
                .is_ok_and(|files| files.len() == 2)
        );
    }

    #[test]
    fn an_analysis_with_errors_gives_no_pages() {
        let output = analyze(&world(), "#unknown-name");

        assert!(!output.has_pages());
        assert_eq!(output.diagnostics[0].severity, Severity::Error);
    }
}
