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

    fn render(document: &typst_layout::PagedDocument, pages: &[usize]) -> Duration {
        let start = Instant::now();
        for &index in pages {
            let _ = render_page(document, index, 2.0, false);
        }
        start.elapsed()
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
        let start = Instant::now();
        let sizes = crate::render::page_sizes(&document, false).len();
        let first_view = start.elapsed() + render(&document, &[0, 1, 2]);

        let start = Instant::now();
        let analysis = Analysis::new(world.clone(), Some(document.clone()));
        let headings = analysis.outline().len();
        let outline_time = start.elapsed();
        let start = Instant::now();
        let problems = analysis
            .quality(&presswork_project::TypesetSettings::default())
            .len();
        let quality_time = start.elapsed();
        let start = Instant::now();
        let _ = analysis.baseline_grid(&presswork_project::TypesetSettings::default());
        let grid_time = start.elapsed();
        let start = Instant::now();
        let output = Output {
            analysis: Some(analysis.clone()),
            ..Output::default()
        };
        let profiles = [presswork_project::ExportProfile::named(
            "PDF",
            presswork_project::ExportFormat::Pdf,
        )];
        let preflight_problems = output.preflight_profiles(&profiles).len();
        let preflight_time = start.elapsed();
        let start = Instant::now();
        let points = analysis.jump_from_cursor(None, text.len() / 2).len();
        let cursor_time = start.elapsed();

        let middle = text[..text.len() / 2].rfind(' ').unwrap_or(0);
        let mut changed = text.clone();
        changed.insert_str(middle, " word");
        world.set_main_text(&changed);
        let start = Instant::now();
        let hot = compile(&world);
        let hot_time = start.elapsed();
        let hot_document = hot.document.unwrap();
        let start = Instant::now();
        let key = crate::render::page_key;
        let changed: Vec<usize> = (0..pages)
            .filter(|&index| key(&document, index, false) != key(&hot_document, index, false))
            .collect();
        let compare_time = start.elapsed();
        let redraw = render(&hot_document, &changed);

        eprintln!("pages: {pages} ({sizes} sizes), headings: {headings}, QA problems: {problems}");
        eprintln!("fonts: {fonts_time:?}");
        eprintln!("cold compile: {cold_time:?}");
        eprintln!("page sizes and the first three pages: {first_view:?}");
        eprintln!("outline: {outline_time:?}, QA report: {quality_time:?}");
        eprintln!(
            "baseline grid: {grid_time:?}, preflight of the PDF profile: {preflight_time:?} ({preflight_problems} problems)"
        );
        eprintln!("page point of the cursor in the middle: {cursor_time:?} ({points} points)");
        eprintln!("hot compile after one word: {hot_time:?}");
        eprintln!(
            "pages changed by the word: {} (keys of all pages compared in {compare_time:?}), drawn again in {redraw:?}",
            changed.len()
        );
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
