use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use jiff::Timestamp;
use presswork_project::ExportProfile;
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use typst::layout::{Frame, FrameItem};
use typst::text::FontStyle;
use typst_layout::PagedDocument;

use crate::compile::{Diagnostic, Severity};
use crate::outline::SourceRange;
use crate::preflight::{Problem, ProblemKind};

const BUILT_IN: &str = "built-in";

pub(crate) struct Inputs<'a> {
    pub(crate) profile: &'a ExportProfile,
    pub(crate) main: &'a Path,
    pub(crate) created: Timestamp,
    pub(crate) document: Option<&'a PagedDocument>,
    pub(crate) compile: Duration,
    pub(crate) export: Duration,
    pub(crate) diagnostics: &'a [Diagnostic],
    pub(crate) problems: &'a [Problem],
    pub(crate) files: &'a [(PathBuf, Vec<u8>)],
}

#[derive(Serialize)]
struct Report<'a> {
    presswork: &'static str,
    typst: &'static str,
    engine: &'static str,
    profile: &'a ExportProfile,
    created: String,
    pages: usize,
    times: Times,
    fonts: Vec<Font>,
    warnings: Vec<Warning<'a>>,
    problems: Vec<ProblemRecord<'a>>,
    files: Vec<File<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Times {
    compile_ms: u64,
    export_ms: u64,
}

#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct Font {
    family: String,
    style: &'static str,
    weight: u16,
    source: &'static str,
}

#[derive(Serialize)]
struct Warning<'a> {
    message: &'a str,
    #[serde(flatten)]
    place: Option<Place<'a>>,
}

#[derive(Serialize)]
struct ProblemRecord<'a> {
    page: Option<usize>,
    #[serde(flatten)]
    kind: &'a ProblemKind,
    #[serde(flatten)]
    place: Option<Place<'a>>,
}

#[derive(Serialize)]
struct Place<'a> {
    file: &'a Path,
    start: usize,
    end: usize,
}

#[derive(Serialize)]
struct File<'a> {
    path: &'a Path,
    bytes: usize,
    sha256: String,
}

fn place<'a>(main: &'a Path, source: Option<&'a SourceRange>) -> Option<Place<'a>> {
    source.map(|source| match source {
        SourceRange::Main(range) => Place {
            file: main,
            start: range.start,
            end: range.end,
        },
        SourceRange::File(path, range) => Place {
            file: path,
            start: range.start,
            end: range.end,
        },
    })
}

pub(crate) fn report(inputs: &Inputs<'_>) -> serde_json::Result<Vec<u8>> {
    let report = Report {
        presswork: env!("CARGO_PKG_VERSION"),
        typst: crate::version(),
        engine: BUILT_IN,
        profile: inputs.profile,
        created: inputs.created.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
        pages: inputs.document.map_or(0, |document| document.pages().len()),
        times: Times {
            compile_ms: milliseconds(inputs.compile),
            export_ms: milliseconds(inputs.export),
        },
        fonts: inputs.document.map_or_else(Vec::new, fonts),
        warnings: inputs
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Warning)
            .map(|diagnostic| Warning {
                message: &diagnostic.message,
                place: place(inputs.main, diagnostic.source.as_ref()),
            })
            .collect(),
        problems: inputs
            .problems
            .iter()
            .map(|problem| ProblemRecord {
                page: problem.page.map(|page| page + 1),
                kind: &problem.kind,
                place: place(inputs.main, problem.source.as_ref()),
            })
            .collect(),
        files: inputs
            .files
            .iter()
            .map(|(path, bytes)| File {
                path,
                bytes: bytes.len(),
                sha256: format!("{:x}", Sha256::digest(bytes)),
            })
            .collect(),
    };
    serde_json::to_vec_pretty(&report)
}

pub(crate) fn report_path(profile: &ExportProfile, main: &Path) -> PathBuf {
    let name: String = profile
        .name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect();
    let template = profile.template(main).to_string_lossy().into_owned();
    let folder = template
        .rfind(['/', '\\'])
        .map_or(0, |separator| separator + 1);
    PathBuf::from(format!("{}{name}.report.json", &template[..folder]))
}

fn fonts(document: &PagedDocument) -> Vec<Font> {
    let mut found = BTreeSet::new();
    for page in document.pages() {
        collect_fonts(&page.frame, &mut found);
    }
    found.into_iter().collect()
}

fn collect_fonts(frame: &Frame, found: &mut BTreeSet<Font>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => collect_fonts(&group.frame, found),
            FrameItem::Text(text) => {
                let info = text.font.font().info();
                found.insert(Font {
                    family: info.family.clone(),
                    style: match info.variant.style {
                        FontStyle::Normal => "normal",
                        FontStyle::Italic => "italic",
                        FontStyle::Oblique => "oblique",
                    },
                    weight: info.variant.weight.to_number(),
                    source: BUILT_IN,
                });
            }
            FrameItem::Shape(..)
            | FrameItem::Image(..)
            | FrameItem::Link(..)
            | FrameItem::Tag(_) => {}
        }
    }
}

fn milliseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use presswork_project::ExportFormat;

    use super::*;

    #[test]
    fn the_build_report_has_the_name_of_the_profile() {
        let mut profile = ExportProfile {
            output: Some(PathBuf::from("out/pages/page-{0p}.png")),
            ..ExportProfile::named("Print run/2", ExportFormat::Png)
        };
        assert_eq!(
            report_path(&profile, Path::new("main.typ")).to_string_lossy(),
            "out/pages/Print-run-2.report.json"
        );

        profile.output = None;
        assert_eq!(
            report_path(&profile, Path::new("book/main.typ")),
            Path::new("book/Print-run-2.report.json")
        );
    }
}
