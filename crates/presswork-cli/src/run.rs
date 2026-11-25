use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use presswork_language::{Dictionaries, Dictionary, HouseStyle, RuleProblem, StyleError, WordList};
use presswork_project::{
    Colors, ExportProfile, HOUSE_STYLE, Manifest, Project, ProjectError, WORD_LIST,
};
use presswork_toolchain::{Engine, Packages, Version, choose, parse_pin};
use presswork_typst::{
    Analysis, Diagnostic, Fonts, Misspelling, Problem, ProblemKind, ProfileError, Severity,
    SourceRange, StyleProblem, World, analyze, today,
};

use crate::cli::{Cli, Command, Target};
use crate::i18n::{FluentArgs, Localizer};
use crate::location::line_column;

pub(crate) const SUCCESS: u8 = 0;
pub(crate) const FAILURE: u8 = 1;
const USER_AGENT: &str = concat!(
    "Presswork CLI/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/getpresswork/presswork)"
);

pub(crate) fn run(cli: &Cli, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let mut console = Console {
        localizer: Localizer::english(),
        out,
        err,
    };
    let (target, build) = match &cli.command {
        Command::Build(target) => (target, true),
        Command::Check(target) => (target, false),
    };
    match work(target, build, &mut console) {
        Ok(()) => SUCCESS,
        Err(Failed) => FAILURE,
    }
}

struct Failed;

fn work(target: &Target, build: bool, console: &mut Console<'_>) -> Result<(), Failed> {
    let project = Project::open(&target.folder).map_err(|error| console.project_error(&error))?;
    let manifest = project.manifest();
    accept_pin(manifest, console)?;
    let profiles = select(manifest.export_profiles(), &target.profiles).map_err(|name| {
        let mut args = FluentArgs::new();
        args.set("name", name);
        console.fail("profile-unknown", &args)
    })?;
    let text = fs::read_to_string(project.main()).map_err(|error| {
        let mut args = FluentArgs::new();
        args.set("file", manifest.main.display().to_string());
        args.set("error", error.to_string());
        console.fail("project-read", &args)
    })?;
    let world = World::open(
        Arc::new(Fonts::embedded()),
        today(),
        project.root().to_path_buf(),
        &manifest.main,
    )
    .with_downloader(Packages::downloader(USER_AGENT));
    let output = analyze(&Mutex::new(world), &text);
    let places = Places {
        folder: &target.folder,
        main: &manifest.main,
        text: &text,
    };
    for diagnostic in &output.diagnostics {
        console.diagnostic(&places, diagnostic);
    }
    if output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(Failed);
    }
    let problems = output.preflight_profiles(&profiles);
    for (profile, problem) in &problems {
        console.problem(&places, problem, &profile.name);
    }
    if !build {
        let layout = output
            .analysis
            .as_ref()
            .map_or_else(Vec::new, |analysis| analysis.quality(&manifest.typeset));
        for problem in &layout {
            console.problem(&places, problem, "");
        }
        let misspelled = output
            .analysis
            .as_ref()
            .is_some_and(|analysis| check_spelling(target, &project, analysis, &places, console));
        let styled = output
            .analysis
            .as_ref()
            .is_some_and(|analysis| check_style(&project, analysis, &places, console));
        return if problems.is_empty() && !misspelled && !styled {
            Ok(())
        } else {
            Err(Failed)
        };
    }
    let mut result = Ok(());
    for profile in &profiles {
        match output.export_profile(profile, project.root(), &manifest.main) {
            Ok(export) => {
                for file in export.files.iter().chain([&export.report]) {
                    console.written(&places, file);
                }
            }
            Err(error) => result = Err(console.export_error(&places, profile, &error)),
        }
    }
    result
}

fn check_spelling(
    target: &Target,
    project: &Project,
    analysis: &Analysis,
    places: &Places<'_>,
    console: &mut Console<'_>,
) -> bool {
    let folder = target.dictionaries.clone().or_else(default_dictionaries);
    let dictionaries = load_dictionaries(folder.as_deref());
    if dictionaries.is_empty() {
        return false;
    }
    let words =
        WordList::parse(&fs::read_to_string(project.root().join(WORD_LIST)).unwrap_or_default());
    let spelling = analysis.spelling(&dictionaries);
    for language in &spelling.missing {
        console.missing_dictionary(language);
    }
    let mut found = false;
    for misspelling in spelling
        .misspellings
        .iter()
        .filter(|misspelling| !words.contains(&misspelling.word))
    {
        console.misspelling(places, misspelling);
        found = true;
    }
    found
}

fn check_style(
    project: &Project,
    analysis: &Analysis,
    places: &Places<'_>,
    console: &mut Console<'_>,
) -> bool {
    let Ok(text) = fs::read_to_string(project.root().join(HOUSE_STYLE)) else {
        return false;
    };
    let style = match HouseStyle::parse(&text) {
        Ok(style) => style,
        Err(error) => {
            console.style_error(&error);
            return true;
        }
    };
    let problems = analysis.style(&style);
    for problem in &problems {
        console.style_problem(places, &style, problem);
    }
    !problems.is_empty()
}

fn default_dictionaries() -> Option<PathBuf> {
    directories::ProjectDirs::from("io.github", "getpresswork", "Presswork")
        .map(|directories| directories.data_local_dir().join("dictionaries"))
}

fn load_dictionaries(folder: Option<&Path>) -> Dictionaries {
    let mut dictionaries = Dictionaries::default();
    let (Some(folder), Ok(catalog)) = (folder, presswork_toolchain::catalog()) else {
        return dictionaries;
    };
    let installed = presswork_toolchain::Dictionaries::new(folder, &catalog);
    for source in &catalog.sources {
        if let Some((aff, dic)) = installed.read(&source.tag)
            && let Ok(dictionary) = Dictionary::new(&aff, &dic)
        {
            dictionaries.insert(&source.tag, dictionary);
        }
    }
    dictionaries
}

fn accept_pin(manifest: &Manifest, console: &mut Console<'_>) -> Result<(), Failed> {
    let Some(pin) = &manifest.typst else {
        return Ok(());
    };
    let mut args = FluentArgs::new();
    args.set("pin", pin.clone());
    let Ok(requirement) = parse_pin(pin) else {
        return Err(console.fail("pin-invalid", &args));
    };
    let version = presswork_typst::version();
    let embedded = Version::parse(version).unwrap_or_else(|_| Version::new(0, 0, 0));
    match choose(Some(&requirement), &embedded, &[]) {
        Engine::Embedded => Ok(()),
        Engine::Managed(_) | Engine::Missing(_) => {
            args.set("version", version);
            Err(console.fail("pin-not-built-in", &args))
        }
    }
}

fn select(profiles: Vec<ExportProfile>, names: &[String]) -> Result<Vec<ExportProfile>, String> {
    if let Some(unknown) = names
        .iter()
        .find(|name| !profiles.iter().any(|profile| &profile.name == *name))
    {
        return Err(unknown.clone());
    }
    Ok(profiles
        .into_iter()
        .filter(|profile| names.is_empty() || names.contains(&profile.name))
        .collect())
}

fn colors_id(colors: Colors) -> &'static str {
    match colors {
        Colors::Cmyk => "colors-cmyk",
        Colors::Gray => "colors-gray",
    }
}

struct Places<'a> {
    folder: &'a Path,
    main: &'a Path,
    text: &'a str,
}

impl Places<'_> {
    fn shown(&self, path: &Path) -> String {
        if self.folder == Path::new(".") {
            return path.display().to_string();
        }
        self.folder.join(path).display().to_string()
    }

    fn location(&self, source: &SourceRange) -> String {
        let (path, place) = match source {
            SourceRange::Main(range) => {
                (self.main.to_path_buf(), line_column(self.text, range.start))
            }
            SourceRange::File(path, range) => (
                path.clone(),
                fs::read_to_string(self.folder.join(path))
                    .ok()
                    .and_then(|text| line_column(&text, range.start)),
            ),
        };
        match place {
            Some((line, column)) => format!("{}:{line}:{column}", self.shown(&path)),
            None => self.shown(&path),
        }
    }
}

struct Console<'a> {
    localizer: Localizer,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
}

impl Console<'_> {
    fn fail(&mut self, id: &str, args: &FluentArgs<'_>) -> Failed {
        let message = self.localizer.format(id, args);
        self.report(None, Severity::Error, &message);
        Failed
    }

    fn report(&mut self, place: Option<String>, severity: Severity, message: &str) {
        let label = self.localizer.text(match severity {
            Severity::Error => "severity-error",
            Severity::Warning => "severity-warning",
        });
        let _ = match place {
            Some(place) => writeln!(self.err, "{place}: {label}: {message}"),
            None => writeln!(self.err, "{label}: {message}"),
        };
    }

    fn diagnostic(&mut self, places: &Places<'_>, diagnostic: &Diagnostic) {
        let place = diagnostic
            .source
            .as_ref()
            .map(|source| places.location(source));
        self.report(place, diagnostic.severity, &diagnostic.message);
        for hint in &diagnostic.hints {
            let mut args = FluentArgs::new();
            args.set("hint", hint.clone());
            let _ = writeln!(self.err, "  {}", self.localizer.format("hint", &args));
        }
    }

    fn problem(&mut self, places: &Places<'_>, problem: &Problem, profile: &str) {
        let mut args = FluentArgs::new();
        args.set("profile", profile.to_owned());
        let id = match &problem.kind {
            ProblemKind::LowResolution { ppi, minimum } => {
                args.set("ppi", *ppi);
                args.set("minimum", *minimum);
                "problem-resolution"
            }
            ProblemKind::MissingAltText => "problem-alt",
            ProblemKind::MissingFont { family } => {
                args.set("family", family.clone());
                "problem-font"
            }
            ProblemKind::MissingGlyphs { characters } => {
                args.set("characters", characters.clone());
                "problem-glyphs"
            }
            ProblemKind::PageSize {
                width,
                height,
                needed,
            } => {
                args.set("width", needed.unit.format(*width));
                args.set("height", needed.unit.format(*height));
                args.set("needed_width", needed.unit.format(needed.width));
                args.set("needed_height", needed.unit.format(needed.height));
                args.set("unit", needed.unit.name());
                "problem-page-size"
            }
            ProblemKind::Color { color, permitted } => {
                args.set("color", color.clone());
                args.set("permitted", self.localizer.text(colors_id(*permitted)));
                "problem-color"
            }
            ProblemKind::GradientSpace { space, permitted } => {
                args.set("space", space.clone());
                args.set("permitted", self.localizer.text(colors_id(*permitted)));
                "problem-gradient"
            }
            ProblemKind::Widow => "problem-widow",
            ProblemKind::Orphan => "problem-orphan",
            ProblemKind::StackedHyphens { lines } => {
                args.set("lines", *lines);
                "problem-hyphens"
            }
            ProblemKind::ShortLastLine { characters } => {
                args.set("characters", *characters);
                "problem-short-line"
            }
            ProblemKind::LooseLine { spacing } => {
                args.set("spacing", *spacing);
                "problem-loose-line"
            }
            ProblemKind::OffGrid { offset, unit } => {
                args.set("offset", unit.format(offset.unsigned_abs()));
                args.set("unit", unit.name());
                args.set("direction", if *offset < 0 { "above" } else { "below" });
                "problem-grid"
            }
        };
        let mut message = self.localizer.format(id, &args);
        if let Some(page) = problem.page {
            let mut args = FluentArgs::new();
            args.set("page", page + 1);
            args.set("problem", message);
            message = self.localizer.format("problem-on-page", &args);
        }
        let place = problem
            .source
            .as_ref()
            .map(|source| places.location(source));
        self.report(place, Severity::Warning, &message);
    }

    fn misspelling(&mut self, places: &Places<'_>, misspelling: &Misspelling) {
        let mut args = FluentArgs::new();
        args.set("word", misspelling.word.clone());
        args.set("language", misspelling.language.clone());
        let message = self.localizer.format("problem-spelling", &args);
        let place = places.location(&misspelling.source);
        self.report(Some(place), Severity::Warning, &message);
    }

    fn style_problem(&mut self, places: &Places<'_>, style: &HouseStyle, problem: &StyleProblem) {
        let mut args = FluentArgs::new();
        args.set("text", problem.text.clone());
        let mut message = style.message(problem.rule).map_or_else(
            || self.localizer.format("problem-style", &args),
            str::to_owned,
        );
        if let Some(replacement) = &problem.replacement {
            let mut args = FluentArgs::new();
            args.set("message", message);
            args.set("replacement", replacement.clone());
            message = self.localizer.format("problem-style-replacement", &args);
        }
        let place = places.location(&problem.source);
        self.report(Some(place), Severity::Warning, &message);
    }

    fn style_error(&mut self, error: &StyleError) {
        let mut args = FluentArgs::new();
        args.set("file", HOUSE_STYLE);
        let id = match error {
            StyleError::File(problem) => {
                args.set("problem", problem.clone());
                "style-error-file"
            }
            StyleError::Rule { rule, problem } => {
                args.set("rule", *rule);
                match problem {
                    RuleProblem::Missing => "style-error-missing",
                    RuleProblem::Both => "style-error-both",
                    RuleProblem::Empty => "style-error-empty",
                    RuleProblem::Pattern(problem) => {
                        args.set("problem", problem.clone());
                        "style-error-pattern"
                    }
                }
            }
        };
        self.fail(id, &args);
    }

    fn missing_dictionary(&mut self, language: &str) {
        let mut args = FluentArgs::new();
        args.set("language", language.to_owned());
        let message = self.localizer.format("spelling-missing", &args);
        self.report(None, Severity::Warning, &message);
    }

    fn written(&mut self, places: &Places<'_>, file: &Path) {
        let _ = writeln!(self.out, "{}", places.shown(file));
    }

    fn export_error(
        &mut self,
        places: &Places<'_>,
        profile: &ExportProfile,
        error: &ProfileError,
    ) -> Failed {
        let mut args = FluentArgs::new();
        args.set("profile", profile.name.clone());
        let id = match error {
            ProfileError::Outside(path) => {
                args.set("path", path.display().to_string());
                "export-outside"
            }
            ProfileError::Write { path, source } => {
                args.set("path", places.shown(path));
                args.set("error", source.to_string());
                "export-write-failed"
            }
            ProfileError::Export(error) => {
                args.set("error", error.to_string());
                "export-failed"
            }
        };
        self.fail(id, &args)
    }

    fn project_error(&mut self, error: &ProjectError) -> Failed {
        let mut args = FluentArgs::new();
        let id = match error {
            ProjectError::NoManifest(folder) => {
                args.set("folder", folder.display().to_string());
                "project-not-found"
            }
            ProjectError::InvalidManifest(problem) => {
                args.set("problem", problem.clone());
                "project-invalid"
            }
            ProjectError::MainOutside(file) => {
                args.set("file", file.display().to_string());
                "project-main-outside"
            }
            ProjectError::MainMissing(file) => {
                args.set("file", file.display().to_string());
                "project-main-missing"
            }
            ProjectError::Io { path, source } => {
                args.set("file", path.display().to_string());
                args.set("error", source.to_string());
                "project-read"
            }
        };
        self.fail(id, &args)
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::*;

    const MANIFEST: &str = "[project]\nname = \"Notes\"\n";
    const PROFILES: &str = "[project]\nname = \"Notes\"\n\n\
                            [[export]]\nname = \"print\"\noutput = \"out/print.pdf\"\n\n\
                            [[export]]\nname = \"pages\"\nformat = \"svg\"\noutput = \"out/page-{p}.svg\"\n";
    const DOT: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"><rect width=\"10\" height=\"10\"/></svg>";

    struct Ran {
        code: u8,
        out: String,
        err: String,
    }

    fn presswork(arguments: &[&str]) -> Ran {
        let empty = tempfile::tempdir().unwrap();
        let empty = empty.path().display().to_string();
        let mut all = vec!["presswork-cli", arguments[0]];
        if !arguments.contains(&"--dictionaries") {
            all.extend(["--dictionaries", empty.as_str()]);
        }
        all.extend(&arguments[1..]);
        let cli = Cli::try_parse_from(all).unwrap();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(&cli, &mut out, &mut err);
        Ran {
            code,
            out: String::from_utf8(out).unwrap(),
            err: String::from_utf8(err).unwrap(),
        }
    }

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let folder = tempfile::tempdir().unwrap();
        for (path, text) in files {
            fs::write(folder.path().join(path), text).unwrap();
        }
        folder
    }

    fn place(folder: &tempfile::TempDir, path: &str) -> String {
        folder.path().join(path).display().to_string()
    }

    fn argument(folder: &tempfile::TempDir) -> String {
        folder.path().display().to_string()
    }

    #[test]
    fn a_build_writes_the_pdf_file_of_a_project_without_profiles() {
        let folder = project(&[("presswork.toml", MANIFEST), ("main.typ", "Hello.")]);

        let ran = presswork(&["build", &argument(&folder)]);

        assert_eq!((ran.code, ran.err.as_str()), (SUCCESS, ""));
        assert_eq!(
            ran.out,
            format!(
                "{}\n{}\n",
                place(&folder, "main.pdf"),
                place(&folder, "PDF.report.json")
            )
        );
        assert!(folder.path().join("PDF.report.json").is_file());
        assert!(folder.path().join("main.pdf").is_file());
    }

    #[test]
    fn a_build_writes_only_the_selected_profiles() {
        let folder = project(&[("presswork.toml", PROFILES), ("main.typ", "Hello.")]);

        let ran = presswork(&["build", "--profile", "pages", &argument(&folder)]);

        assert_eq!(ran.code, SUCCESS);
        assert!(folder.path().join("out/page-1.svg").is_file());
        assert!(!folder.path().join("out/print.pdf").exists());
    }

    #[test]
    fn an_unknown_profile_stops_the_command() {
        let folder = project(&[("presswork.toml", PROFILES), ("main.typ", "Hello.")]);

        let ran = presswork(&["build", "--profile", "web", &argument(&folder)]);

        assert_eq!(
            (ran.code, ran.err.as_str()),
            (
                FAILURE,
                "error: The project has no export profile with the name web.\n"
            )
        );
        assert!(!folder.path().join("out").exists());
    }

    #[test]
    fn an_error_shows_its_file_line_and_column() {
        let folder = project(&[
            ("presswork.toml", MANIFEST),
            ("main.typ", "#include \"chapter.typ\""),
            ("chapter.typ", "Text.\n#unknown-name"),
        ]);

        let ran = presswork(&["build", &argument(&folder)]);

        assert_eq!(ran.code, FAILURE);
        assert!(
            ran.err.starts_with(&format!(
                "{}:2:2: error: unknown variable: unknown-name\n",
                place(&folder, "chapter.typ")
            )),
            "{}",
            ran.err
        );
        assert!(!folder.path().join("main.pdf").exists());
    }

    #[test]
    fn a_check_finds_preflight_problems_and_writes_no_files() {
        let folder = project(&[
            ("presswork.toml", MANIFEST),
            ("main.typ", "Text.\n#image(\"dot.svg\", width: 1in)"),
            ("dot.svg", DOT),
        ]);

        let ran = presswork(&["check", &argument(&folder)]);

        assert_eq!(
            (ran.code, ran.out.as_str(), ran.err),
            (
                FAILURE,
                "",
                format!(
                    "{}:2:2: warning: Page 1: The image has no alternative text.\n",
                    place(&folder, "main.typ")
                )
            )
        );
        assert!(!folder.path().join("main.pdf").exists());
    }

    #[test]
    fn a_check_without_problems_succeeds_without_output() {
        let folder = project(&[("presswork.toml", MANIFEST), ("main.typ", "Hello.")]);

        let ran = presswork(&["check", &argument(&folder)]);

        assert_eq!(
            (ran.code, ran.out.as_str(), ran.err.as_str()),
            (SUCCESS, "", "")
        );
    }

    #[test]
    fn a_folder_without_a_manifest_is_not_a_project() {
        let folder = project(&[("main.typ", "Hello.")]);

        let ran = presswork(&["check", &argument(&folder)]);

        assert_eq!(
            (ran.code, ran.err),
            (
                FAILURE,
                format!(
                    "error: The folder {} is not a Presswork project. A project folder contains a presswork.toml file.\n",
                    argument(&folder)
                )
            )
        );
    }

    #[test]
    fn a_pin_that_the_built_in_engine_does_not_accept_stops_the_command() {
        let folder = project(&[
            (
                "presswork.toml",
                "[project]\nname = \"Notes\"\ntypst = \"0.13\"\n",
            ),
            ("main.typ", "Hello."),
        ]);

        let ran = presswork(&["build", &argument(&folder)]);

        assert_eq!(ran.code, FAILURE);
        assert!(
            ran.err.starts_with("error: The project needs Typst 0.13."),
            "{}",
            ran.err
        );
        assert!(!folder.path().join("main.pdf").exists());
    }

    #[test]
    fn a_failed_profile_does_not_stop_the_other_profiles() {
        let folder = project(&[
            (
                "presswork.toml",
                "[project]\nname = \"Notes\"\n\n[[export]]\nname = \"outside\"\noutput = \"../outside.pdf\"\n\n[[export]]\nname = \"print\"\n",
            ),
            ("main.typ", "Hello."),
        ]);

        let ran = presswork(&["build", &argument(&folder)]);

        assert_eq!(ran.code, FAILURE);
        assert_eq!(
            ran.err,
            "error: The profile outside writes ../outside.pdf. This path is outside the project folder.\n"
        );
        assert!(folder.path().join("main.pdf").is_file());
    }

    const RULES: &str = "[project]\nname = \"Notes\"\n\n\
                         [[export]]\nname = \"print\"\npage-size = \"148mm x 210mm\"\ncolors = \"cmyk\"\n\n\
                         [[export]]\nname = \"screen\"\noutput = \"screen.pdf\"\n";

    #[test]
    fn a_check_finds_the_problems_of_each_profile() {
        let folder = project(&[
            ("presswork.toml", RULES),
            (
                "main.typ",
                "#set page(width: 100mm, height: 150mm)\n#text(fill: rgb(\"#ff0000\"))[Red]",
            ),
        ]);

        let ran = presswork(&["check", &argument(&folder)]);

        assert_eq!(
            (ran.code, ran.err),
            (
                FAILURE,
                format!(
                    "warning: Page 1: The page is 100 × 150 mm. The profile print needs 148 × 210 mm.\n\
                     {}:2:29: warning: Page 1: The document uses the color rgb(\"#ff0000\"). The profile print permits only CMYK, gray, and spot colors.\n",
                    place(&folder, "main.typ")
                )
            )
        );
    }

    #[test]
    fn a_problem_of_the_document_shows_one_time_for_all_profiles() {
        let folder = project(&[
            ("presswork.toml", RULES),
            ("main.typ", "#set text(font: \"No Such Font\")\nText."),
        ]);

        let ran = presswork(&["check", &argument(&folder)]);

        let problem = format!(
            "{}:1:17: warning: The font family no such font is not available. Typst uses a different font.\n",
            place(&folder, "main.typ")
        );
        assert_eq!(ran.code, FAILURE);
        assert_eq!(ran.err.matches(&problem).count(), 1, "{}", ran.err);
    }

    fn dictionary_folder(words: &str) -> tempfile::TempDir {
        let folder = tempfile::tempdir().unwrap();
        let english = folder.path().join("en-US");
        fs::create_dir_all(&english).unwrap();
        fs::write(english.join("dictionary.aff"), "SET UTF-8\n").unwrap();
        let count = words.split(' ').count();
        fs::write(
            english.join("dictionary.dic"),
            format!("{count}\n{}\n", words.replace(' ', "\n")),
        )
        .unwrap();
        folder
    }

    #[test]
    fn a_check_finds_misspelled_words() {
        let dictionaries = dictionary_folder("the cat sat on");
        let folder = project(&[
            ("presswork.toml", MANIFEST),
            ("main.typ", "The cat sat on the mta."),
        ]);

        let ran = presswork(&[
            "check",
            "--dictionaries",
            &argument(&dictionaries),
            &argument(&folder),
        ]);

        assert_eq!(
            (ran.code, ran.err),
            (
                FAILURE,
                format!(
                    "{}:1:20: warning: The word “mta” is not in the dictionary for en.\n",
                    place(&folder, "main.typ")
                )
            )
        );
    }

    #[test]
    fn the_word_list_of_the_project_permits_its_words() {
        let dictionaries = dictionary_folder("the cat sat on");
        let folder = project(&[
            ("presswork.toml", MANIFEST),
            ("main.typ", "The cat sat on the mta."),
            ("presswork-words.txt", "mta\n"),
        ]);

        let ran = presswork(&[
            "check",
            "--dictionaries",
            &argument(&dictionaries),
            &argument(&folder),
        ]);

        assert_eq!((ran.code, ran.err.as_str()), (SUCCESS, ""));
    }

    #[test]
    fn a_language_without_a_dictionary_is_named_once() {
        let dictionaries = dictionary_folder("the cat");
        let folder = project(&[
            ("presswork.toml", MANIFEST),
            ("main.typ", "#set text(lang: \"de\")\nDas Haus. Die Katze."),
        ]);

        let ran = presswork(&[
            "check",
            "--dictionaries",
            &argument(&dictionaries),
            &argument(&folder),
        ]);

        assert_eq!(
            (ran.code, ran.err.as_str()),
            (
                SUCCESS,
                "warning: Presswork has no dictionary for de, so it did not check the spelling of this text.\n"
            )
        );
    }

    const STYLE: &str = "[[rule]]\ntext = \"organise\"\nreplacement = \"organize\"\n\n\
                         [[rule]]\npattern = '(\\d+)-(\\d+)'\nreplacement = '$1–$2'\n\
                         message = \"Use an en dash between two numbers.\"\n\n\
                         [[rule]]\ntext = \"utilize\"\n";

    #[test]
    fn a_check_finds_house_style_problems() {
        let folder = project(&[
            ("presswork.toml", MANIFEST),
            ("main.typ", "We organise pages 12-19.\nWe utilize them."),
            ("presswork-style.toml", STYLE),
        ]);

        let ran = presswork(&["check", &argument(&folder)]);

        let main = place(&folder, "main.typ");
        assert_eq!(
            (ran.code, ran.err),
            (
                FAILURE,
                format!(
                    "{main}:1:4: warning: The house style does not permit “organise”. Use “organize”.\n\
                     {main}:1:19: warning: Use an en dash between two numbers. Use “12–19”.\n\
                     {main}:2:4: warning: The house style does not permit “utilize”.\n"
                )
            )
        );
    }

    #[test]
    fn a_house_style_file_that_is_not_valid_fails_the_check() {
        let folder = project(&[
            ("presswork.toml", MANIFEST),
            ("main.typ", "Hello."),
            ("presswork-style.toml", "[[rule]]\nmessage = \"No text.\"\n"),
        ]);

        let ran = presswork(&["check", &argument(&folder)]);

        assert_eq!(
            (ran.code, ran.err.as_str()),
            (
                FAILURE,
                "error: Rule 1 in presswork-style.toml has no text and no pattern.\n"
            )
        );
    }

    #[test]
    fn a_check_lists_layout_problems_but_does_not_fail_on_them() {
        let folder = project(&[
            ("presswork.toml", MANIFEST),
            ("main.typ", "A line of text \\ it."),
        ]);

        let ran = presswork(&["check", &argument(&folder)]);

        assert_eq!(
            (ran.code, ran.err),
            (
                SUCCESS,
                format!(
                    "{}:1:18: warning: Page 1: The last line of a paragraph has only 3 characters.\n",
                    place(&folder, "main.typ")
                )
            )
        );
    }

    #[test]
    fn a_check_uses_the_limits_of_the_qa_report_from_the_project() {
        let folder = project(&[
            (
                "presswork.toml",
                "[project]\nname = \"Notes\"\n\n[typeset]\nshort-last-line = 3\n",
            ),
            ("main.typ", "A line of text \\ it."),
        ]);

        let ran = presswork(&["check", &argument(&folder)]);

        assert_eq!((ran.code, ran.err), (SUCCESS, String::new()));
    }

    #[test]
    fn a_check_lists_body_lines_off_the_baseline_grid_of_the_project() {
        let folder = project(&[
            (
                "presswork.toml",
                "[project]\nname = \"Notes\"\n\n[typeset]\nbaseline-grid = \"12pt\"\nbaseline-start = \"20pt\"\n",
            ),
            (
                "main.typ",
                "#set page(width: 200pt, height: auto, margin: 10pt)\n\
                 #set text(size: 10pt, top-edge: 10pt, bottom-edge: 0pt)\n\
                 #set par(leading: 2pt, spacing: 2pt)\n\
                 The first one.\n\nThe second one.\n\n#v(6pt)\nThe third one.",
            ),
        ]);

        let ran = presswork(&["check", &argument(&folder)]);

        assert_eq!(ran.code, SUCCESS);
        assert!(
            ran.err.contains(
                "warning: Page 1: The baseline of this line is 6 pt below the baseline grid."
            ),
            "{}",
            ran.err
        );
    }

    #[test]
    fn a_build_writes_the_files_although_preflight_finds_problems() {
        let folder = project(&[
            ("presswork.toml", MANIFEST),
            ("main.typ", "Text.\n#image(\"dot.svg\", width: 1in)"),
            ("dot.svg", DOT),
        ]);

        let ran = presswork(&["build", &argument(&folder)]);

        assert_eq!(ran.code, SUCCESS);
        assert!(ran.err.contains("The image has no alternative text."));
        assert!(folder.path().join("main.pdf").is_file());
    }

    #[test]
    fn places_in_the_current_folder_have_no_folder_name() {
        let places = Places {
            folder: Path::new("."),
            main: Path::new("main.typ"),
            text: "",
        };

        assert_eq!(
            places.shown(Path::new("chapters/one.typ")),
            "chapters/one.typ"
        );
    }
}
