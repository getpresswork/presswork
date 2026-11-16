use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use futures::channel::oneshot;
use presswork_typst::{Datetime, Fonts, Output, Program, Scheduler, Stop, World};

pub(crate) const SCALE: f32 = 2.0;
const PROGRAM_TIME_LIMIT: Duration = Duration::from_secs(120);

static FONTS: LazyLock<Arc<Fonts>> = LazyLock::new(|| Arc::new(Fonts::embedded()));

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Backend {
    Embedded,
    Program(Program),
    Unavailable,
}

pub(crate) struct Compiler {
    engine: Arc<Mutex<World>>,
    root: Option<PathBuf>,
    backend: Backend,
    scheduler: Scheduler,
    generation: u64,
    stop: Stop,
}

impl Compiler {
    pub(crate) fn new() -> Self {
        Self {
            engine: Arc::new(Mutex::new(World::new(Arc::clone(&FONTS), today()))),
            root: None,
            backend: Backend::Embedded,
            scheduler: Scheduler::default(),
            generation: 0,
            stop: Stop::new(PROGRAM_TIME_LIMIT),
        }
    }

    pub(crate) fn open(&mut self, root: &Path, main: &Path, backend: Backend) {
        let world = World::open(Arc::clone(&FONTS), today(), root.to_path_buf(), main);
        self.engine = Arc::new(Mutex::new(world));
        self.root = Some(root.to_path_buf());
        self.backend = backend;
        self.restart();
    }

    pub(crate) fn backend(&self) -> &Backend {
        &self.backend
    }

    pub(crate) fn set_backend(&mut self, backend: Backend) -> bool {
        if self.backend == backend {
            return false;
        }
        self.backend = backend;
        self.restart();
        true
    }

    fn restart(&mut self) {
        self.stop.cancel();
        self.stop = Stop::new(PROGRAM_TIME_LIMIT);
        self.scheduler = Scheduler::default();
        self.generation += 1;
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn request(&mut self) -> bool {
        self.backend != Backend::Unavailable && self.scheduler.request()
    }

    pub(crate) fn finish(&mut self) -> bool {
        self.scheduler.finish()
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.scheduler.is_running()
    }

    pub(crate) fn job(
        &self,
        text: String,
    ) -> impl Future<Output = Result<Output, String>> + Send + 'static {
        let compile = self.compile_fn();
        let (sender, receiver) = oneshot::channel();
        let spawned = std::thread::Builder::new()
            .name("typst".to_owned())
            .spawn(move || {
                let _ = sender.send(compile(&text));
            });
        if let Err(error) = spawned {
            tracing::error!(%error, "The compile thread did not start");
        }
        async move { receiver.await.unwrap_or_else(|_| Ok(Output::default())) }
    }

    #[cfg(test)]
    pub(crate) fn run_now(&self, text: &str) -> Result<Output, String> {
        self.compile_fn()(text)
    }

    fn compile_fn(&self) -> impl FnOnce(&str) -> Result<Output, String> + Send + 'static {
        let engine = Arc::clone(&self.engine);
        let root = self.root.clone().unwrap_or_default();
        let backend = self.backend.clone();
        let stop = self.stop.clone();
        move |text| match backend {
            Backend::Embedded => Ok(presswork_typst::run(&engine, text, SCALE)),
            Backend::Program(program) => program
                .run(&root, text, &stop)
                .map_err(|error| error.to_string()),
            Backend::Unavailable => Ok(Output::default()),
        }
    }
}

fn today() -> Option<Datetime> {
    let date = jiff::ZonedDateTime::now().date();
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

        let output = futures::executor::block_on(compiler.job("Hello.".to_owned())).unwrap();

        assert_eq!(output.pages.map(|pages| pages.len()), Some(1));
    }

    #[test]
    fn an_unavailable_backend_starts_no_compilation() {
        let mut compiler = Compiler::new();
        compiler.set_backend(Backend::Unavailable);

        assert!(!compiler.request());
    }

    #[test]
    fn a_new_backend_starts_a_new_generation() {
        let mut compiler = Compiler::new();
        let generation = compiler.generation();

        assert!(compiler.set_backend(Backend::Unavailable));
        assert!(!compiler.set_backend(Backend::Unavailable));

        assert_eq!(compiler.generation(), generation + 1);
    }

    #[test]
    fn a_new_backend_or_project_stops_the_running_program() {
        let mut compiler = Compiler::new();
        let first = compiler.stop.clone();

        compiler.set_backend(Backend::Unavailable);
        assert!(first.is_cancelled());
        assert!(!compiler.stop.is_cancelled());

        let second = compiler.stop.clone();
        let folder = std::env::temp_dir();
        compiler.open(&folder, Path::new("main.typ"), Backend::Embedded);
        assert!(second.is_cancelled());
    }

    #[test]
    fn a_program_that_does_not_start_gives_an_error() {
        let mut compiler = Compiler::new();
        compiler.set_backend(Backend::Program(Program::new(
            PathBuf::from("no-such-typst-program"),
            semver_version(),
        )));

        assert!(compiler.run_now("Hello.").is_err());
    }

    fn semver_version() -> presswork_toolchain::Version {
        presswork_toolchain::Version::new(0, 14, 2)
    }

    #[test]
    fn the_world_knows_the_date_of_today() {
        assert!(today().is_some());
    }
}
