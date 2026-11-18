use std::fs::File;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::engine::{OcrEngine, OcrError, Recognized};
use crate::text_layer::text_layer;

const TIME_LIMIT: Duration = Duration::from_hours(1);
const POLL: Duration = Duration::from_millis(100);
const MAX_REASON_CHARACTERS: usize = 500;
const FILE_PROBLEMS: [i32; 4] = [2, 4, 6, 8];

#[derive(Debug, Clone)]
pub struct OcrMyPdf {
    program: PathBuf,
    version: String,
    languages: Vec<String>,
    jobs: usize,
    time_limit: Duration,
}

impl OcrMyPdf {
    pub fn detect(program: impl Into<PathBuf>) -> Result<Self, OcrError> {
        let program = program.into();
        let version = output_of(Command::new(&program).arg("--version"))?;
        let listing = output_of(Command::new("tesseract").arg("--list-langs"))?;
        Ok(Self {
            program,
            version: version.trim().to_owned(),
            languages: installed_languages(&listing),
            jobs: thread::available_parallelism().map_or(1, NonZeroUsize::get),
            time_limit: TIME_LIMIT,
        })
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn with_jobs(self, jobs: usize) -> Self {
        Self {
            jobs: jobs.max(1),
            ..self
        }
    }

    #[must_use]
    pub fn with_time_limit(self, time_limit: Duration) -> Self {
        Self { time_limit, ..self }
    }
}

impl OcrEngine for OcrMyPdf {
    fn languages(&self) -> &[String] {
        &self.languages
    }

    fn recognize(&self, pdf: &[u8], languages: &[String]) -> Result<Recognized, OcrError> {
        let folder = tempfile::tempdir()?;
        let input = folder.path().join("input.pdf");
        let output = folder.path().join("output.pdf");
        let log = folder.path().join("ocrmypdf.log");
        std::fs::write(&input, pdf)?;
        let child = start(
            Command::new(&self.program)
                .args(arguments(languages, self.jobs))
                .arg(&input)
                .arg(&output)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(File::create(&log)?),
        )
        .map_err(|error| OcrError::Engine(error.to_string()))?;
        let status = wait(child, self.time_limit)?;
        if !status.success() {
            let log = std::fs::read_to_string(&log).unwrap_or_default();
            return Err(failure(status.code(), &log));
        }
        let copy = std::fs::read(&output)?;
        let texts = text_layer(copy.clone())?;
        Ok(Recognized {
            pdf: Some(copy),
            texts,
        })
    }
}

fn output_of(command: &mut Command) -> Result<String, OcrError> {
    let output = command
        .stdin(Stdio::null())
        .output()
        .map_err(|error| OcrError::Engine(error.to_string()))?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(text)
    } else {
        Err(failure(None, &text))
    }
}

fn arguments(languages: &[String], jobs: usize) -> Vec<String> {
    let mut arguments: Vec<String> = [
        "--skip-text",
        "--output-type",
        "pdf",
        "--optimize",
        "0",
        "--quiet",
        "--jobs",
    ]
    .map(str::to_owned)
    .into();
    arguments.push(jobs.to_string());
    if !languages.is_empty() {
        arguments.push("--language".to_owned());
        arguments.push(languages.join("+"));
    }
    arguments
}

fn installed_languages(listing: &str) -> Vec<String> {
    listing
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("List of") && *line != "osd")
        .filter(|line| {
            line.chars()
                .all(|character| character.is_ascii_alphanumeric() || "_-/".contains(character))
        })
        .map(str::to_owned)
        .collect()
}

fn failure(code: Option<i32>, log: &str) -> OcrError {
    let last = log.lines().map(str::trim).rfind(|line| !line.is_empty());
    let reason = match (last, code) {
        (Some(line), Some(code)) => format!(
            "{} (exit code {code})",
            line.chars().take(MAX_REASON_CHARACTERS).collect::<String>()
        ),
        (Some(line), None) => line.chars().take(MAX_REASON_CHARACTERS).collect(),
        (None, Some(code)) => format!("OCRmyPDF stopped with the exit code {code}"),
        (None, None) => "OCRmyPDF stopped without an exit code".to_owned(),
    };
    match code {
        Some(code) if FILE_PROBLEMS.contains(&code) => OcrError::File(reason),
        _ => OcrError::Engine(reason),
    }
}

fn start(command: &mut Command) -> std::io::Result<Child> {
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(command, 0);
    command.spawn()
}

fn stop(child: &mut Child) {
    #[cfg(unix)]
    let _ = Command::new("sh")
        .arg("-c")
        .arg("kill -9 -\"$0\"")
        .arg(child.id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
}

fn wait(mut child: Child, limit: Duration) -> Result<ExitStatus, OcrError> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if started.elapsed() >= limit {
            stop(&mut child);
            child.wait()?;
            return Err(OcrError::TimeLimit(limit));
        }
        thread::sleep(POLL);
    }
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Stdio};
    use std::time::Instant;

    use super::*;
    use crate::testing::{TestPage, lines, pdf_of};
    use crate::text_layer::text_layer;

    fn owned(values: &[&str]) -> Vec<String> {
        values.iter().map(|&value| value.to_owned()).collect()
    }

    #[test]
    fn ocrmypdf_runs_only_on_pages_without_text_and_keeps_the_file() {
        assert_eq!(
            arguments(&owned(&["deu", "eng"]), 3),
            [
                "--skip-text",
                "--output-type",
                "pdf",
                "--optimize",
                "0",
                "--quiet",
                "--jobs",
                "3",
                "--language",
                "deu+eng",
            ]
        );
        assert!(!arguments(&[], 1).contains(&"--language".to_owned()));
    }

    #[test]
    fn tesseract_lists_its_languages_without_the_orientation_data() {
        let listing = "List of available languages in \"C:/tesseract/tessdata/\" (4):\n\
                       deu\r\neng\nosd\nscript/Latin\n";

        assert_eq!(installed_languages(listing), ["deu", "eng", "script/Latin"]);
        assert_eq!(installed_languages(""), Vec::<String>::new());
    }

    #[test]
    fn the_exit_code_tells_a_problem_of_the_file_from_a_problem_of_the_engine() {
        let log = "   1 page processed\nERROR - InputFileError: the file is not a PDF\n\n";

        let error = failure(Some(2), log);
        assert!(error.is_permanent());
        assert_eq!(
            error.to_string(),
            "the OCR engine cannot use the file: \
             ERROR - InputFileError: the file is not a PDF (exit code 2)"
        );
        for code in [4, 6, 8] {
            assert!(failure(Some(code), "").is_permanent(), "{code}");
        }
        for code in [Some(1), Some(3), Some(7), Some(15), None] {
            assert!(!failure(code, "").is_permanent(), "{code:?}");
        }
        assert_eq!(
            failure(None, "").to_string(),
            "the OCR engine failed: OCRmyPDF stopped without an exit code"
        );
    }

    fn sleeper() -> Command {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("ping");
            command.args(["-n", "30", "127.0.0.1"]);
            command
        } else {
            let mut command = Command::new("sleep");
            command.arg("30");
            command
        };
        command.stdout(Stdio::null()).stderr(Stdio::null());
        command
    }

    #[test]
    fn a_program_that_runs_too_long_stops_at_the_time_limit() {
        let started = Instant::now();

        let result = wait(sleeper().spawn().unwrap(), Duration::from_millis(300));

        assert!(matches!(result, Err(OcrError::TimeLimit(_))), "{result:?}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[cfg(unix)]
    fn within_five_seconds(mut condition: impl FnMut() -> bool) -> bool {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(5) {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    #[cfg(unix)]
    #[test]
    fn the_time_limit_also_stops_the_child_processes() {
        let folder = tempfile::tempdir().unwrap();
        let pid_file = folder.path().join("child.pid");
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("sleep 30 & echo $! > \"$0\"; wait")
            .arg(&pid_file);
        let child = start(&mut command).unwrap();
        let mut pid = String::new();
        assert!(within_five_seconds(|| {
            pid = std::fs::read_to_string(&pid_file).unwrap_or_default();
            pid.ends_with('\n')
        }));

        let result = wait(child, Duration::from_millis(300));

        assert!(matches!(result, Err(OcrError::TimeLimit(_))), "{result:?}");
        let stopped = within_five_seconds(|| {
            !Command::new("sh")
                .arg("-c")
                .arg("kill -0 \"$0\" 2> /dev/null")
                .arg(pid.trim())
                .status()
                .unwrap()
                .success()
        });
        assert!(stopped, "the child process {} still runs", pid.trim());
    }

    #[test]
    fn a_missing_program_is_a_problem_of_the_engine() {
        let error = OcrMyPdf::detect("presswork-no-such-program").unwrap_err();

        assert!(matches!(error, OcrError::Engine(_)), "{error:?}");
    }

    fn words(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn ocrmypdf_gives_the_text_of_the_scanned_pages() {
        let Some(program) = std::env::var_os("PRESSWORK_TEST_OCRMYPDF") else {
            eprintln!("PRESSWORK_TEST_OCRMYPDF is not set. This test does not run.");
            return;
        };
        let engine = OcrMyPdf::detect(program).unwrap();
        assert!(engine.languages().contains(&"eng".to_owned()));
        let scan = pdf_of(
            &[
                crate::render::scan(include_bytes!("../fixtures/typst.pdf"), 0),
                TestPage::Text(lines(&["The ink must dry."])),
            ],
            "",
        );
        assert_eq!(text_layer(scan.clone()).unwrap(), ["", "The ink must dry."]);

        let recognized = engine.recognize(&scan, &owned(&["eng"])).unwrap();

        let copy = recognized.pdf.unwrap();
        assert_eq!(text_layer(copy).unwrap(), recognized.texts);
        assert!(
            words(&recognized.texts[0]).contains("Good books start with good paper."),
            "{:?}",
            recognized.texts[0]
        );
        assert_eq!(recognized.texts[1], "The ink must dry.");
    }
}
