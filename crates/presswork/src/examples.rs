use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::Task;
use iced::futures::channel::mpsc;
use iced::widget::image;
use presswork_typst::{DocExample, PageImage, render_example};

const PIXEL_PER_PT: f32 = 2.0;
const SHOWN_PER_PT: f32 = 96.0 / 72.0;
const LIMIT: usize = 48;

#[derive(Debug, Clone)]
pub(crate) struct Rendered {
    id: u128,
    pages: Option<Vec<ExamplePage>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ExamplePage {
    pub(crate) handle: image::Handle,
    pub(crate) width: f32,
}

#[derive(Debug, Default)]
pub(crate) struct Examples {
    shown: HashMap<u128, Option<Vec<ExamplePage>>>,
    order: VecDeque<u128>,
    pending: HashSet<u128>,
    generation: Arc<AtomicU64>,
}

impl Examples {
    pub(crate) fn pages(&self, id: u128) -> Option<&[ExamplePage]> {
        self.shown.get(&id)?.as_deref()
    }

    pub(crate) fn render(&mut self, examples: &[DocExample]) -> Task<Rendered> {
        let wanted = self.wanted(examples);
        if wanted.is_empty() {
            return Task::none();
        }
        let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        self.pending = wanted.iter().map(|example| example.id).collect();
        let current = Arc::clone(&self.generation);
        let (sender, receiver) = mpsc::unbounded();
        let spawned = std::thread::Builder::new()
            .name("doc-examples".to_owned())
            .spawn(move || {
                for example in wanted {
                    if current.load(Ordering::Relaxed) != generation
                        || sender.unbounded_send(draw(&example)).is_err()
                    {
                        break;
                    }
                }
            });
        if spawned.is_err() {
            self.pending.clear();
            return Task::none();
        }
        Task::stream(receiver)
    }

    pub(crate) fn rendered(&mut self, rendered: Rendered, keep: &[DocExample]) {
        self.pending.remove(&rendered.id);
        if self.shown.insert(rendered.id, rendered.pages).is_none() {
            self.order.push_back(rendered.id);
        }
        self.evict(keep);
    }

    #[cfg(test)]
    pub(crate) fn render_now(&mut self, examples: &[DocExample]) -> usize {
        let wanted: Vec<DocExample> = examples
            .iter()
            .filter(|example| !self.shown.contains_key(&example.id))
            .cloned()
            .collect();
        for example in &wanted {
            self.rendered(draw(example), examples);
        }
        wanted.len()
    }

    fn wanted(&self, examples: &[DocExample]) -> Vec<DocExample> {
        examples
            .iter()
            .filter(|example| {
                !self.shown.contains_key(&example.id) && !self.pending.contains(&example.id)
            })
            .cloned()
            .collect()
    }

    fn evict(&mut self, keep: &[DocExample]) {
        let mut excess = self.shown.len().saturating_sub(LIMIT);
        let shown = &mut self.shown;
        self.order.retain(|id| {
            if excess == 0 || keep.iter().any(|example| example.id == *id) {
                return true;
            }
            shown.remove(id);
            excess -= 1;
            false
        });
    }
}

fn draw(example: &DocExample) -> Rendered {
    Rendered {
        id: example.id,
        pages: render_example(&example.source, PIXEL_PER_PT)
            .map(|pages| pages.into_iter().map(page).collect()),
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "example images are far below 2^24 pixels wide"
)]
fn page(image: PageImage) -> ExamplePage {
    ExamplePage {
        width: image.width as f32 * SHOWN_PER_PT / PIXEL_PER_PT,
        handle: image::Handle::from_rgba(image.width, image.height, image.rgba),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(source: &str) -> DocExample {
        use std::hash::{Hash as _, Hasher as _};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        DocExample {
            id: u128::from(hasher.finish()),
            source: source.to_owned(),
        }
    }

    #[test]
    fn an_example_renders_at_the_size_of_the_preview() {
        let mut examples = Examples::default();
        let wanted = [example("#set page(width: 120pt, height: 60pt)\nHello")];

        assert_eq!(examples.render_now(&wanted), 1);

        let pages = examples.pages(wanted[0].id).unwrap();
        assert_eq!(pages.len(), 1);
        assert!((pages[0].width - 160.0).abs() < 0.01, "{}", pages[0].width);
        assert_eq!(examples.render_now(&wanted), 0);
    }

    #[test]
    fn an_example_that_does_not_compile_is_tried_one_time() {
        let mut examples = Examples::default();
        let wanted = [example("#image(\"tiger.jpg\")")];

        assert_eq!(examples.render_now(&wanted), 1);

        assert!(examples.pages(wanted[0].id).is_none());
        assert_eq!(examples.render_now(&wanted), 0);
    }

    #[test]
    fn the_oldest_examples_go_first_but_the_examples_of_the_page_stay() {
        let mut examples = Examples::default();
        let page: Vec<DocExample> = (0..3).map(|n| example(&format!("Page {n}"))).collect();
        examples.render_now(&page);
        let others: Vec<DocExample> = (0..LIMIT).map(|n| example(&format!("= {n}"))).collect();

        for other in &others {
            examples.rendered(
                Rendered {
                    id: other.id,
                    pages: None,
                },
                &page,
            );
        }

        assert_eq!(examples.shown.len(), LIMIT);
        assert!(page.iter().all(|kept| examples.pages(kept.id).is_some()));
        assert!(!examples.shown.contains_key(&others[0].id));
        assert!(examples.shown.contains_key(&others[LIMIT - 1].id));
    }

    #[test]
    fn a_render_skips_the_examples_that_are_shown_or_on_the_way() {
        let mut examples = Examples::default();
        let first = [example("One")];
        examples.render_now(&first);
        let both = [example("One"), example("Two")];

        let task = examples.render(&both);

        assert!(task.units() > 0);
        assert!(examples.pending.contains(&both[1].id));
        assert!(!examples.pending.contains(&both[0].id));
        assert_eq!(examples.render(&both).units(), 0);
    }
}
