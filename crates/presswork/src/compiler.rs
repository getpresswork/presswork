use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};

use futures::channel::oneshot;
use presswork_typst::{Datetime, Fonts, Output, Scheduler, World};

pub(crate) const SCALE: f32 = 2.0;

static FONTS: LazyLock<Arc<Fonts>> = LazyLock::new(|| Arc::new(Fonts::embedded()));

pub(crate) struct Compiler {
    engine: Arc<Mutex<World>>,
    scheduler: Scheduler,
    generation: u64,
}

impl Compiler {
    pub(crate) fn new() -> Self {
        Self {
            engine: Arc::new(Mutex::new(World::new(Arc::clone(&FONTS), today()))),
            scheduler: Scheduler::default(),
            generation: 0,
        }
    }

    pub(crate) fn open(&mut self, root: &Path, main: &Path) {
        let world = World::open(Arc::clone(&FONTS), today(), root.to_path_buf(), main);
        self.engine = Arc::new(Mutex::new(world));
        self.scheduler = Scheduler::default();
        self.generation += 1;
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn request(&mut self) -> bool {
        self.scheduler.request()
    }

    pub(crate) fn finish(&mut self) -> bool {
        self.scheduler.finish()
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.scheduler.is_running()
    }

    pub(crate) fn job(&self, text: String) -> impl Future<Output = Output> + Send + 'static {
        let engine = Arc::clone(&self.engine);
        let (sender, receiver) = oneshot::channel();
        let spawned = std::thread::Builder::new()
            .name("typst".to_owned())
            .spawn(move || {
                let _ = sender.send(presswork_typst::run(&engine, &text, SCALE));
            });
        if let Err(error) = spawned {
            tracing::error!(%error, "The compile thread did not start");
        }
        async move { receiver.await.unwrap_or_default() }
    }

    #[cfg(test)]
    pub(crate) fn run_now(&self, text: &str) -> Output {
        presswork_typst::run(&self.engine, text, SCALE)
    }
}

fn today() -> Option<Datetime> {
    let date = jiff::Zoned::now().date();
    Datetime::from_ymd(
        i32::from(date.year()),
        u8::try_from(date.month()).ok()?,
        u8::try_from(date.day()).ok()?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_compiles_the_text_on_another_thread() {
        let compiler = Compiler::new();

        let output = futures::executor::block_on(compiler.job("Hello.".to_owned()));

        assert_eq!(output.pages.map(|pages| pages.len()), Some(1));
    }

    #[test]
    fn the_world_knows_the_date_of_today() {
        assert!(today().is_some());
    }
}
