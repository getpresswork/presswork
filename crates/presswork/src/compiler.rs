use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::Duration;

use futures::channel::oneshot;
use presswork_project::ExportProfile;
pub(crate) use presswork_typst::Export;
use presswork_typst::{
    ExportError, Fonts, Output, ProfileError, Program, Scheduler, Stop, World, today,
};

pub(crate) const SCALE: f32 = 2.0;
const PROGRAM_TIME_LIMIT: Duration = Duration::from_secs(120);

static FONTS: LazyLock<Arc<Fonts>> = LazyLock::new(|| Arc::new(Fonts::embedded()));

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExportFailure {
    NeedsEngine,
    NoPages,
    Stopped,
    Outside(PathBuf),
    Export(String),
    Write(PathBuf, String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Sources {
    pub(crate) main: String,
    pub(crate) files: Vec<(PathBuf, String)>,
}

impl Sources {
    pub(crate) fn main(text: &str) -> Self {
        Self {
            main: text.to_owned(),
            files: Vec::new(),
        }
    }
}

impl From<&str> for Sources {
    fn from(text: &str) -> Self {
        Self::main(text)
    }
}

impl From<String> for Sources {
    fn from(main: String) -> Self {
        Self {
            main,
            files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Backend {
    Embedded,
    Program(Program),
    Unavailable,
}

pub(crate) struct Compiler {
    engine: Arc<Mutex<World>>,
    root: Option<PathBuf>,
    main: PathBuf,
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
            main: PathBuf::from("main.typ"),
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
        self.main = main.to_path_buf();
        self.backend = backend;
        self.restart();
    }

    pub(crate) fn scratch(&mut self) {
        self.engine = Arc::new(Mutex::new(World::new(Arc::clone(&FONTS), today())));
        self.root = None;
        self.backend = Backend::Embedded;
        self.restart();
    }

    pub(crate) fn backend(&self) -> &Backend {
        &self.backend
    }

    pub(crate) fn set_file(&mut self, path: &Path, text: &str) {
        self.engine
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .set_text(path, text);
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
        sources: impl Into<Sources>,
    ) -> impl Future<Output = Result<Output, String>> + Send + 'static {
        let sources = sources.into();
        let compile = self.compile_fn();
        let (sender, receiver) = oneshot::channel();
        let spawned = std::thread::Builder::new()
            .name("typst".to_owned())
            .spawn(move || {
                let _ = sender.send(compile(&sources));
            });
        if let Err(error) = spawned {
            tracing::error!(%error, "The compile thread did not start");
        }
        async move { receiver.await.unwrap_or_else(|_| Ok(Output::default())) }
    }

    #[cfg(test)]
    pub(crate) fn run_now(&self, sources: impl Into<Sources>) -> Result<Output, String> {
        self.compile_fn()(&sources.into())
    }

    #[cfg(test)]
    pub(crate) fn export_now(
        &self,
        sources: impl Into<Sources>,
        profile: &ExportProfile,
    ) -> Result<Export, ExportFailure> {
        self.export_fn(profile.clone())(&sources.into())
    }

    pub(crate) fn export_job(
        &self,
        sources: impl Into<Sources>,
        profile: ExportProfile,
    ) -> impl Future<Output = Result<Export, ExportFailure>> + Send + 'static {
        let sources = sources.into();
        let export = self.export_fn(profile);
        async move {
            let (sender, receiver) = oneshot::channel();
            let spawned = std::thread::Builder::new()
                .name("typst-export".to_owned())
                .spawn(move || {
                    let _ = sender.send(export(&sources));
                });
            if let Err(error) = spawned {
                tracing::error!(%error, "The export thread did not start");
            }
            receiver.await.unwrap_or(Err(ExportFailure::Stopped))
        }
    }

    fn export_fn(
        &self,
        profile: ExportProfile,
    ) -> impl FnOnce(&Sources) -> Result<Export, ExportFailure> + Send + 'static {
        let engine = Arc::clone(&self.engine);
        let root = self.root.clone().unwrap_or_default();
        let main = self.main.clone();
        let embedded = self.backend == Backend::Embedded;
        move |sources| {
            if !embedded {
                return Err(ExportFailure::NeedsEngine);
            }
            if let Some(path) = profile.outside(&main) {
                return Err(ExportFailure::Outside(path));
            }
            set_files(&engine, &sources.files);
            Ok(presswork_typst::analyze(&engine, &sources.main)
                .export_profile(&profile, &root, &main)?)
        }
    }

    fn compile_fn(&self) -> impl FnOnce(&Sources) -> Result<Output, String> + Send + 'static {
        let engine = Arc::clone(&self.engine);
        let root = self.root.clone().unwrap_or_default();
        let backend = self.backend.clone();
        let stop = self.stop.clone();
        move |sources| match backend {
            Backend::Embedded => {
                set_files(&engine, &sources.files);
                Ok(presswork_typst::analyze(&engine, &sources.main))
            }
            Backend::Program(program) => program
                .run(&root, &sources.main, &stop)
                .map_err(|error| error.to_string()),
            Backend::Unavailable => Ok(Output::default()),
        }
    }
}

fn set_files(engine: &Mutex<World>, files: &[(PathBuf, String)]) {
    let mut world = engine.lock().unwrap_or_else(PoisonError::into_inner);
    for (path, text) in files {
        world.set_text(path, text);
    }
}

impl From<ProfileError> for ExportFailure {
    fn from(error: ProfileError) -> Self {
        match error {
            ProfileError::Outside(path) => Self::Outside(path),
            ProfileError::Export(ExportError::NoPages) => Self::NoPages,
            ProfileError::Export(error) => Self::Export(error.to_string()),
            ProfileError::Write { path, source } => Self::Write(path, source.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use presswork_project::ExportFormat;

    use super::*;

    #[test]
    fn a_job_compiles_the_text_on_another_thread() {
        let compiler = Compiler::new();

        let output = futures::executor::block_on(compiler.job(Sources::main("Hello."))).unwrap();

        assert!(output.has_pages());
        assert_eq!(output.pages, None);
    }

    fn project_compiler() -> (Compiler, tempfile::TempDir) {
        let folder = tempfile::tempdir().unwrap();
        let mut compiler = Compiler::new();
        compiler.open(folder.path(), Path::new("main.typ"), Backend::Embedded);
        (compiler, folder)
    }

    #[test]
    fn a_job_uses_the_unsaved_text_of_other_files() {
        let (compiler, folder) = project_compiler();
        std::fs::write(folder.path().join("chapter.typ"), "= Saved").unwrap();
        let sources = Sources {
            main: "#include \"chapter.typ\"".to_owned(),
            files: vec![(PathBuf::from("chapter.typ"), "= Unsaved".to_owned())],
        };

        let output = futures::executor::block_on(compiler.job(sources)).unwrap();

        let titles: Vec<String> = output
            .analysis
            .unwrap()
            .outline()
            .into_iter()
            .map(|heading| heading.title)
            .collect();
        assert_eq!(titles, ["Unsaved"]);
    }

    fn profile(format: ExportFormat, output: Option<&str>) -> ExportProfile {
        ExportProfile {
            output: output.map(PathBuf::from),
            ..ExportProfile::named("test", format)
        }
    }

    #[test]
    fn an_export_job_writes_the_files_when_it_runs() {
        let (compiler, folder) = project_compiler();
        let job = compiler.export_job("Hello.".to_owned(), profile(ExportFormat::Pdf, None));

        let report = futures::executor::block_on(job).unwrap();

        assert_eq!(report.files, [PathBuf::from("main.pdf")]);
        assert!(folder.path().join("main.pdf").is_file());
    }

    #[test]
    fn an_export_writes_a_pdf_file_into_the_project() {
        let (compiler, folder) = project_compiler();

        let report = compiler
            .export_now("Hello.", &profile(ExportFormat::Pdf, None))
            .unwrap();

        assert_eq!(report.files, [PathBuf::from("main.pdf")]);
        let bytes = std::fs::read(folder.path().join("main.pdf")).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn page_images_go_into_numbered_files() {
        let (compiler, folder) = project_compiler();

        let report = compiler
            .export_now(
                "One.\n#pagebreak()\nTwo.",
                &profile(ExportFormat::Png, Some("out/page-{p}.png")),
            )
            .unwrap();

        assert_eq!(report.files.len(), 2);
        assert!(folder.path().join("out/page-2.png").is_file());
    }

    #[test]
    fn an_export_outside_the_project_folder_is_refused() {
        let (compiler, folder) = project_compiler();

        let result = compiler.export_now("Hello.", &profile(ExportFormat::Pdf, Some("../out.pdf")));

        assert!(
            matches!(result, Err(ExportFailure::Outside(_))),
            "{result:?}"
        );
        assert!(!folder.path().join("../out.pdf").exists());
    }

    #[test]
    fn a_document_with_errors_does_not_export() {
        let (compiler, _folder) = project_compiler();

        let result = compiler.export_now("#let", &profile(ExportFormat::Pdf, None));

        assert_eq!(result, Err(ExportFailure::NoPages));
    }

    #[test]
    fn only_the_built_in_engine_exports() {
        let (mut compiler, _folder) = project_compiler();
        compiler.set_backend(Backend::Program(Program::new(
            PathBuf::from("typst"),
            semver_version(),
        )));

        let result = compiler.export_now("Hello.", &profile(ExportFormat::Pdf, None));

        assert_eq!(result, Err(ExportFailure::NeedsEngine));
    }

    #[test]
    fn the_export_reports_the_preflight_problems() {
        let (compiler, folder) = project_compiler();
        std::fs::write(
            folder.path().join("a.svg"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"/>",
        )
        .unwrap();

        let report = compiler
            .export_now("#image(\"a.svg\")", &profile(ExportFormat::Pdf, None))
            .unwrap();

        assert_eq!(report.problems.len(), 1);
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
    fn scratch_forgets_the_project_and_uses_the_built_in_engine() {
        let mut compiler = Compiler::new();
        let folder = tempfile::tempdir().unwrap();
        compiler.open(folder.path(), Path::new("main.typ"), Backend::Unavailable);
        let generation = compiler.generation();

        compiler.scratch();

        assert_eq!(compiler.backend(), &Backend::Embedded);
        assert_eq!(compiler.root, None);
        assert!(compiler.generation() > generation);
        let output = compiler.run_now("Hello.").unwrap();
        assert!(output.has_pages());
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
