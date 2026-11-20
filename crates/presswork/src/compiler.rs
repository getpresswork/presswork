use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::Duration;

use futures::channel::oneshot;
use presswork_project::ExportProfile;
use presswork_typst::{
    ExportError, Fonts, Output, Problem, ProfileError, Program, Rules, Scheduler, Stop, World,
    today,
};

pub(crate) const SCALE: f32 = 2.0;
const PROGRAM_TIME_LIMIT: Duration = Duration::from_secs(120);

static FONTS: LazyLock<Arc<Fonts>> = LazyLock::new(|| Arc::new(Fonts::embedded()));

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExportReport {
    pub(crate) files: Vec<PathBuf>,
    pub(crate) problems: Vec<Problem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExportFailure {
    NeedsEngine,
    NoPages,
    Stopped,
    Outside(PathBuf),
    Export(String),
    Write(PathBuf, String),
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
    bleed: bool,
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
            bleed: false,
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

    pub(crate) fn set_bleed(&mut self, bleed: bool) -> bool {
        let changed = self.bleed != bleed;
        self.bleed = bleed;
        changed
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

    #[cfg(test)]
    pub(crate) fn export_now(
        &self,
        text: &str,
        profile: &ExportProfile,
    ) -> Result<ExportReport, ExportFailure> {
        self.export_fn(profile.clone())(text)
    }

    pub(crate) fn export_job(
        &self,
        text: String,
        profile: ExportProfile,
    ) -> impl Future<Output = Result<ExportReport, ExportFailure>> + Send + 'static {
        let export = self.export_fn(profile);
        async move {
            let (sender, receiver) = oneshot::channel();
            let spawned = std::thread::Builder::new()
                .name("typst-export".to_owned())
                .spawn(move || {
                    let _ = sender.send(export(&text));
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
    ) -> impl FnOnce(&str) -> Result<ExportReport, ExportFailure> + Send + 'static {
        let engine = Arc::clone(&self.engine);
        let root = self.root.clone().unwrap_or_default();
        let main = self.main.clone();
        let embedded = self.backend == Backend::Embedded;
        move |text| {
            if !embedded {
                return Err(ExportFailure::NeedsEngine);
            }
            if let Some(path) = profile.outside(&main) {
                return Err(ExportFailure::Outside(path));
            }
            let output = presswork_typst::analyze(&engine, text);
            let problems = output.preflight(&Rules::of(&profile));
            let analysis = output.analysis.ok_or(ExportFailure::NoPages)?;
            let files = analysis.export_profile(&profile, &root, &main, presswork_typst::now())?;
            Ok(ExportReport { files, problems })
        }
    }

    fn compile_fn(&self) -> impl FnOnce(&str) -> Result<Output, String> + Send + 'static {
        let engine = Arc::clone(&self.engine);
        let root = self.root.clone().unwrap_or_default();
        let backend = self.backend.clone();
        let stop = self.stop.clone();
        let bleed = self.bleed;
        move |text| match backend {
            Backend::Embedded => Ok(presswork_typst::run(&engine, text, SCALE, bleed)),
            Backend::Program(program) => program
                .run(&root, text, &stop)
                .map_err(|error| error.to_string()),
            Backend::Unavailable => Ok(Output::default()),
        }
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

        let output = futures::executor::block_on(compiler.job("Hello.".to_owned())).unwrap();

        assert_eq!(output.pages.map(|pages| pages.len()), Some(1));
    }

    fn project_compiler() -> (Compiler, tempfile::TempDir) {
        let folder = tempfile::tempdir().unwrap();
        let mut compiler = Compiler::new();
        compiler.open(folder.path(), Path::new("main.typ"), Backend::Embedded);
        (compiler, folder)
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
    fn the_bleed_setting_renders_the_bleed_of_the_pages() {
        let mut compiler = Compiler::new();
        let text = "#set page(width: 100pt, height: 50pt, bleed: 5pt)\nText.";

        assert!(compiler.set_bleed(true));
        assert!(!compiler.set_bleed(true));
        let output = compiler.run_now(text).unwrap();

        assert_eq!(output.pages.unwrap()[0].width, 220);
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
        assert_eq!(output.pages.map(|pages| pages.len()), Some(1));
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
