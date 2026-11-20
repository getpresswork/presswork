use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use jiff::Timestamp;
use presswork_project::{ExportFormat, ExportProfile};
use typst::foundations::Datetime;

use crate::clock::datetime;
use crate::export::{ExportError, ExportSettings, Format};
use crate::job::Output;
use crate::preflight::{Problem, Rules};
use crate::report::{Inputs, report, report_path};

const POINTS_PER_INCH: f32 = 72.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    pub files: Vec<PathBuf>,
    pub problems: Vec<Problem>,
    pub report: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("the output path {} is outside the project folder", .0.display())]
    Outside(PathBuf),
    #[error(transparent)]
    Export(#[from] ExportError),
    #[error("cannot write {}: {source}", .path.display())]
    Write { path: PathBuf, source: io::Error },
}

impl Output {
    pub fn export_profile(
        &self,
        profile: &ExportProfile,
        root: &Path,
        main: &Path,
    ) -> Result<Export, ProfileError> {
        if let Some(path) = profile.outside(main) {
            return Err(ProfileError::Outside(path));
        }
        let analysis = self
            .analysis
            .as_ref()
            .ok_or(ProfileError::Export(ExportError::NoPages))?;
        let problems = self.preflight(&Rules::of(profile));
        let created = Timestamp::now();
        let start = Instant::now();
        let contents = analysis.export(&settings(profile, datetime(created)))?;
        let files: Vec<(PathBuf, Vec<u8>)> = profile
            .files(main, contents.len())
            .into_iter()
            .zip(contents)
            .collect();
        for (path, bytes) in &files {
            write(root, path, bytes)?;
        }
        let path = report_path(profile, main);
        let json = report(&Inputs {
            profile,
            main,
            created,
            document: analysis.document(),
            compile: self.duration,
            export: start.elapsed(),
            diagnostics: &self.diagnostics,
            problems: &problems,
            files: &files,
        })
        .map_err(|error| ProfileError::Write {
            path: path.clone(),
            source: io::Error::other(error),
        })?;
        write(root, &path, &json)?;
        Ok(Export {
            files: files.into_iter().map(|(path, _)| path).collect(),
            problems,
            report: path,
        })
    }
}

fn write(root: &Path, path: &Path, bytes: &[u8]) -> Result<(), ProfileError> {
    let write_error = |source| ProfileError::Write {
        path: path.to_path_buf(),
        source,
    };
    let full = root.join(path);
    if let Some(folder) = full.parent() {
        fs::create_dir_all(folder).map_err(write_error)?;
    }
    fs::write(&full, bytes).map_err(write_error)
}

fn settings(profile: &ExportProfile, date: Option<Datetime>) -> ExportSettings {
    ExportSettings {
        format: match profile.format {
            ExportFormat::Pdf => Format::Pdf,
            ExportFormat::Png => Format::Png,
            ExportFormat::Svg => Format::Svg,
        },
        standards: profile.standards.clone(),
        pixel_per_pt: f32::from(u16::try_from(profile.resolution()).unwrap_or(u16::MAX))
            / POINTS_PER_INCH,
        date,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use serde_json::Value;
    use sha2::{Digest as _, Sha256};

    use super::*;
    use crate::fonts::Fonts;
    use crate::job::analyze;
    use crate::world::World;

    const DOT: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"><rect width=\"10\" height=\"10\"/></svg>";

    fn output(folder: &Path, text: &str) -> Output {
        fs::create_dir_all(folder).unwrap();
        fs::write(folder.join("main.typ"), text).unwrap();
        fs::write(folder.join("dot.svg"), DOT).unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 20),
            folder.to_path_buf(),
            Path::new("main.typ"),
        ));
        analyze(&world, text)
    }

    fn profile(format: ExportFormat, output: &str) -> ExportProfile {
        ExportProfile {
            output: Some(PathBuf::from(output)),
            ..ExportProfile::named("test", format)
        }
    }

    fn json(folder: &Path, path: &Path) -> Value {
        serde_json::from_slice(&fs::read(folder.join(path)).unwrap()).unwrap()
    }

    #[test]
    fn a_profile_writes_its_files_and_a_build_report() {
        let folder = tempfile::tempdir().unwrap();
        let output = output(folder.path(), "One.\n#pagebreak()\nTwo.");

        let export = output
            .export_profile(
                &profile(ExportFormat::Svg, "out/page-{p}.svg"),
                folder.path(),
                Path::new("main.typ"),
            )
            .unwrap();

        assert_eq!(
            export.files,
            [
                PathBuf::from("out/page-1.svg"),
                PathBuf::from("out/page-2.svg")
            ]
        );
        assert_eq!(export.report, Path::new("out/test.report.json"));
        let report = json(folder.path(), &export.report);
        assert_eq!(report["typst"], crate::version());
        assert_eq!(report["engine"], "built-in");
        assert_eq!(report["profile"]["name"], "test");
        assert_eq!(report["pages"], 2);
        assert!(report["created"].as_str().unwrap().ends_with('Z'));
        assert_eq!(report["fonts"][0]["family"], "Libertinus Serif");
        let second = fs::read(folder.path().join("out/page-2.svg")).unwrap();
        assert_eq!(report["files"][1]["path"], "out/page-2.svg");
        assert_eq!(report["files"][1]["bytes"], second.len());
        assert_eq!(
            report["files"][1]["sha256"],
            format!("{:x}", Sha256::digest(&second))
        );
    }

    #[test]
    fn the_build_report_lists_the_warnings_and_the_problems() {
        let folder = tempfile::tempdir().unwrap();
        let output = output(
            folder.path(),
            "#set text(font: \"No Such Font\")\n#image(\"dot.svg\", width: 1in)",
        );

        let export = output
            .export_profile(
                &profile(ExportFormat::Pdf, "book.pdf"),
                folder.path(),
                Path::new("main.typ"),
            )
            .unwrap();

        let report = json(folder.path(), &export.report);
        assert_eq!(
            report["warnings"][0]["message"],
            "unknown font family: no such font"
        );
        assert_eq!(report["warnings"][0]["file"], "main.typ");
        let kinds: Vec<&str> = report["problems"]
            .as_array()
            .unwrap()
            .iter()
            .map(|problem| problem["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, ["missing-font", "missing-alt-text"]);
        assert_eq!(report["problems"][1]["page"], 1);
    }

    #[test]
    fn page_images_have_the_resolution_of_the_profile() {
        let folder = tempfile::tempdir().unwrap();
        let output = output(folder.path(), "#set page(width: 100pt, height: 50pt)\nA.");
        let mut pages = profile(ExportFormat::Png, "page.png");
        pages.ppi = Some(144);

        output
            .export_profile(&pages, folder.path(), Path::new("main.typ"))
            .unwrap();

        let bytes = fs::read(folder.path().join("page.png")).unwrap();
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        assert_eq!(width, 200);
    }

    #[test]
    fn a_profile_does_not_write_outside_the_project_folder() {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().join("book");
        let output = output(&root, "Text.");

        let result = output.export_profile(
            &profile(ExportFormat::Pdf, "../book.pdf"),
            &root,
            Path::new("main.typ"),
        );

        assert!(
            matches!(result, Err(ProfileError::Outside(path)) if path == Path::new("../book.pdf"))
        );
        assert!(!folder.path().join("book.pdf").exists());
    }
}
