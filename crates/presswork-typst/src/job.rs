use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::compile::{Diagnostic, compile};
use crate::ide::Analysis;
use crate::render::{PageImage, render_page};
use crate::world::World;

#[derive(Debug, Clone, Default)]
pub struct Output {
    pub pages: Option<Vec<PageImage>>,
    pub diagnostics: Vec<Diagnostic>,
    pub duration: Duration,
    pub analysis: Option<Analysis>,
}

#[must_use]
pub fn run(world: &Mutex<World>, text: &str, pixel_per_pt: f32, bleed: bool) -> Output {
    let start = Instant::now();
    let mut world = world.lock().unwrap_or_else(PoisonError::into_inner);
    world.set_main_text(text);
    let compilation = compile(&world);
    let pages = compilation.document.as_ref().map(|document| {
        (0..document.pages().len())
            .filter_map(|index| render_page(document, index, pixel_per_pt, bleed))
            .collect()
    });
    Output {
        pages,
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

    fn world() -> Mutex<World> {
        let today = Datetime::from_ymd(2026, 11, 15).unwrap();
        Mutex::new(World::new(Arc::new(Fonts::embedded()), Some(today)))
    }

    #[test]
    fn a_job_gives_an_image_for_each_page() {
        let output = run(&world(), "One.\n#pagebreak()\nTwo.", 1.0, false);

        assert_eq!(output.pages.map(|pages| pages.len()), Some(2));
        assert_eq!(output.diagnostics, []);
    }

    #[test]
    fn a_job_can_render_the_bleed() {
        let output = run(
            &world(),
            "#set page(width: 100pt, height: 50pt, bleed: 5pt)\nText.",
            1.0,
            true,
        );

        let page = &output.pages.unwrap()[0];
        assert_eq!((page.width, page.height), (110, 60));
    }

    #[test]
    fn a_job_with_errors_gives_no_pages() {
        let output = run(&world(), "#unknown-name", 1.0, false);

        assert_eq!(output.pages, None);
        assert_eq!(output.diagnostics[0].severity, Severity::Error);
    }
}
