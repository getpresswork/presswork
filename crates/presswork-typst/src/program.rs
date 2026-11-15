use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use semver::Version;
use typst::syntax::Lines;

use crate::compile::{Diagnostic, Severity};
use crate::job::Output;
use crate::render::PageImage;

const PIXELS_PER_INCH: &str = "144";
const MAIN_FILE: &str = "<stdin>";

#[derive(Debug, thiserror::Error)]
pub enum ProgramError {
    #[error("the Typst program cannot start: {0}")]
    Start(#[source] io::Error),
    #[error("the Typst program cannot run: {0}")]
    Run(#[source] io::Error),
    #[error("the page image {path} cannot be read: {reason}")]
    Page { path: PathBuf, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    path: PathBuf,
    version: Version,
}

impl Program {
    #[must_use]
    pub fn new(path: PathBuf, version: Version) -> Self {
        Self { path, version }
    }

    #[must_use]
    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn run(&self, root: &Path, text: &str) -> Result<Output, ProgramError> {
        let start = Instant::now();
        let pages = tempfile::tempdir().map_err(ProgramError::Run)?;
        let mut command = Command::new(&self.path);
        command
            .args(self.arguments(root, pages.path()))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        hide_window(&mut command);
        let mut child = command.spawn().map_err(ProgramError::Start)?;
        if let Some(mut input) = child.stdin.take() {
            input
                .write_all(text.as_bytes())
                .map_err(ProgramError::Run)?;
        }
        let result = child.wait_with_output().map_err(ProgramError::Run)?;
        let diagnostics = parse_diagnostics(&String::from_utf8_lossy(&result.stderr), text);
        let pages = if result.status.success() {
            Some(read_pages(pages.path())?)
        } else {
            None
        };
        Ok(Output {
            pages,
            diagnostics,
            duration: start.elapsed(),
        })
    }

    fn arguments(&self, root: &Path, pages: &Path) -> Vec<OsString> {
        let modern = self.version >= Version::new(0, 12, 0);
        let template = if modern {
            "page-{0p}.png"
        } else {
            "page-{n}.png"
        };
        let mut arguments: Vec<OsString> = vec![
            "compile".into(),
            "-".into(),
            pages.join(template).into(),
            "--root".into(),
            root.into(),
            "--format".into(),
            "png".into(),
            "--ppi".into(),
            PIXELS_PER_INCH.into(),
            "--diagnostic-format".into(),
            "short".into(),
        ];
        if modern {
            arguments.push("--ignore-system-fonts".into());
        }
        arguments
    }
}

#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_window(_command: &mut Command) {}

fn parse_diagnostics(output: &str, text: &str) -> Vec<Diagnostic> {
    let lines = Lines::new(text);
    output
        .lines()
        .filter_map(|line| {
            let (location, severity, message) = split_diagnostic(line)?;
            let range = location
                .and_then(main_position)
                .and_then(|(line, column)| lines.line_column_to_byte(line, column))
                .map(|start| start..start + token_length(&text[start..]));
            Some(Diagnostic {
                severity,
                range,
                message: message.to_owned(),
                hints: Vec::new(),
            })
        })
        .collect()
}

fn split_diagnostic(line: &str) -> Option<(Option<&str>, Severity, &str)> {
    for (label, severity) in [
        ("error: ", Severity::Error),
        ("warning: ", Severity::Warning),
    ] {
        if let Some(message) = line.strip_prefix(label) {
            return Some((None, severity, message));
        }
        let marker = format!(": {label}");
        if let Some(index) = line.find(&marker) {
            return Some((
                Some(&line[..index]),
                severity,
                &line[index + marker.len()..],
            ));
        }
    }
    None
}

fn main_position(location: &str) -> Option<(usize, usize)> {
    let mut parts = location.rsplitn(3, ':');
    let column = parts.next()?.parse().ok()?;
    let line: usize = parts.next()?.parse().ok()?;
    let path = parts.next()?;
    path.ends_with(MAIN_FILE)
        .then(|| Some((line.checked_sub(1)?, column)))
        .flatten()
}

fn token_length(rest: &str) -> usize {
    let is_word = |character: char| character.is_alphanumeric() || matches!(character, '_' | '-');
    let mut characters = rest.chars();
    match characters.next() {
        None | Some('\n') => 0,
        Some('"') => rest[1..].find(['"', '\n']).map_or(rest.len(), |end| {
            if rest[1 + end..].starts_with('"') {
                end + 2
            } else {
                end + 1
            }
        }),
        Some('#') => {
            1 + rest[1..]
                .find(|character| !is_word(character))
                .unwrap_or(rest.len() - 1)
        }
        Some(first) if is_word(first) => rest
            .find(|character| !is_word(character))
            .unwrap_or(rest.len()),
        Some(first) => first.len_utf8(),
    }
}

fn read_pages(folder: &Path) -> Result<Vec<PageImage>, ProgramError> {
    let mut files: Vec<(u32, PathBuf)> = fs::read_dir(folder)
        .map_err(ProgramError::Run)?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let number = name
                .strip_prefix("page-")?
                .strip_suffix(".png")?
                .parse()
                .ok()?;
            Some((number, entry.path()))
        })
        .collect();
    files.sort_unstable_by_key(|(number, _)| *number);
    files.into_iter().map(|(_, path)| decode(&path)).collect()
}

fn decode(path: &Path) -> Result<PageImage, ProgramError> {
    let broken = |reason: String| ProgramError::Page {
        path: path.to_owned(),
        reason,
    };
    let file = File::open(path).map_err(|error| broken(error.to_string()))?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = decoder
        .read_info()
        .map_err(|error| broken(error.to_string()))?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| broken("the image is too large".to_owned()))?;
    let mut pixels = vec![0; size];
    let info = reader
        .next_frame(&mut pixels)
        .map_err(|error| broken(error.to_string()))?;
    pixels.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels,
        png::ColorType::GrayscaleAlpha => pixels
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[gray, alpha]| [gray, gray, gray, alpha])
            .collect(),
        other => return Err(broken(format!("the color type {other:?} is not supported"))),
    };
    Ok(PageImage {
        width: info.width,
        height: info.height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use std::io::BufWriter;

    use super::*;

    const MAIN: &str = r"\\?\C:\book\<stdin>";

    fn program(version: &str) -> Program {
        Program::new(PathBuf::from("typst"), Version::parse(version).unwrap())
    }

    fn texts(arguments: &[OsString]) -> Vec<String> {
        arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    fn write_png(path: &Path, width: u32, height: u32) {
        let mut encoder =
            png::Encoder::new(BufWriter::new(File::create(path).unwrap()), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let pixels = vec![255; (width * height * 4) as usize];
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }

    #[test]
    fn typst_0_12_and_later_get_the_new_page_template_and_no_system_fonts() {
        let arguments = texts(&program("0.14.2").arguments(Path::new("book"), Path::new("pages")));

        assert_eq!(arguments[..2], ["compile", "-"]);
        assert!(arguments[2].ends_with("page-{0p}.png"));
        assert!(arguments.contains(&"--ignore-system-fonts".to_owned()));
        assert!(arguments.windows(2).any(|pair| pair == ["--root", "book"]));
        assert!(arguments.windows(2).any(|pair| pair == ["--ppi", "144"]));
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--diagnostic-format", "short"])
        );
    }

    #[test]
    fn typst_0_11_gets_the_old_page_template_and_no_new_flags() {
        let arguments = texts(&program("0.11.1").arguments(Path::new("book"), Path::new("pages")));

        assert!(arguments[2].ends_with("page-{n}.png"));
        assert!(!arguments.contains(&"--ignore-system-fonts".to_owned()));
    }

    #[test]
    fn an_error_in_the_main_file_marks_the_word_at_its_position() {
        let text = "Line one.\n#unknown-name\nMore.";

        let diagnostics = parse_diagnostics(
            &format!("{MAIN}:2:1: error: unknown variable: unknown-name\n"),
            text,
        );

        assert_eq!(
            diagnostics,
            [Diagnostic {
                severity: Severity::Error,
                range: Some(11..23),
                message: "unknown variable: unknown-name".to_owned(),
                hints: Vec::new(),
            }]
        );
        assert_eq!(&text[11..23], "unknown-name");
    }

    #[test]
    fn a_warning_on_a_string_marks_the_full_string() {
        let text = "#set text(font: \"No Such Font\")";

        let diagnostics = parse_diagnostics(
            &format!("{MAIN}:1:16: warning: unknown font family: no such font"),
            text,
        );

        assert_eq!(diagnostics[0].severity, Severity::Warning);
        let range = diagnostics[0].range.clone().unwrap();
        assert_eq!(&text[range], "\"No Such Font\"");
    }

    #[test]
    fn a_diagnostic_in_another_file_or_without_a_place_has_no_range() {
        let diagnostics = parse_diagnostics(
            "\\\\?\\C:\\book\\chapters\\one.typ:3:4: error: unknown variable: x\nerror: input file not found\nhelp: error occurred in this call\n",
            "Text.",
        );

        let ranges: Vec<_> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.range.clone())
            .collect();
        assert_eq!(ranges, [None, None]);
        assert_eq!(diagnostics[1].message, "input file not found");
    }

    #[test]
    fn page_images_come_in_the_sequence_of_their_numbers() {
        let folder = tempfile::tempdir().unwrap();
        for (number, width) in [(2, 20), (10, 100), (1, 10)] {
            write_png(&folder.path().join(format!("page-{number}.png")), width, 4);
        }
        std::fs::write(folder.path().join("notes.txt"), "").unwrap();

        let pages = read_pages(folder.path()).unwrap();

        let widths: Vec<u32> = pages.iter().map(|page| page.width).collect();
        assert_eq!(widths, [10, 20, 100]);
        assert_eq!(pages[0].rgba.len(), 10 * 4 * 4);
    }

    #[test]
    fn a_gray_page_image_becomes_rgba() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("page-1.png");
        let mut encoder = png::Encoder::new(BufWriter::new(File::create(&path).unwrap()), 2, 1);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[10, 200])
            .unwrap();

        let pages = read_pages(folder.path()).unwrap();

        assert_eq!(pages[0].rgba, [10, 10, 10, 255, 200, 200, 200, 255]);
    }

    #[test]
    #[ignore = "runs the Typst program that PRESSWORK_TYPST names"]
    fn a_real_program_gives_pages_and_diagnostics() {
        let path = PathBuf::from(std::env::var_os("PRESSWORK_TYPST").unwrap());
        let version =
            std::env::var("PRESSWORK_TYPST_VERSION").unwrap_or_else(|_| "0.14.2".to_owned());
        let program = Program::new(path, Version::parse(&version).unwrap());
        let root = tempfile::tempdir().unwrap();
        let text = "One.\n#pagebreak()\n#set text(font: \"No Such Font\")\nTwo.";

        let output = program.run(root.path(), text).unwrap();

        assert_eq!(output.pages.map(|pages| pages.len()), Some(2));
        let range = output.diagnostics[0].range.clone().unwrap();
        assert_eq!(&text[range], "\"No Such Font\"");

        let failed = program.run(root.path(), "#unknown-name").unwrap();
        assert_eq!(failed.pages, None);
        assert_eq!(failed.diagnostics[0].severity, Severity::Error);
    }
}
