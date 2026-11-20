use std::fs;
use std::path::{Path, PathBuf};

use presswork_project::{ExportFormat, ExportProfile};
use typst::foundations::Datetime;

use crate::export::{ExportError, ExportSettings, Format};
use crate::ide::Analysis;

const POINTS_PER_INCH: f32 = 72.0;

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("the output path {} is outside the project folder", .0.display())]
    Outside(PathBuf),
    #[error(transparent)]
    Export(#[from] ExportError),
    #[error("cannot write {}: {source}", .path.display())]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl Analysis {
    pub fn export_profile(
        &self,
        profile: &ExportProfile,
        root: &Path,
        main: &Path,
        date: Option<Datetime>,
    ) -> Result<Vec<PathBuf>, ProfileError> {
        if let Some(path) = profile.outside(main) {
            return Err(ProfileError::Outside(path));
        }
        let files = self.export(&settings(profile, date))?;
        let paths = profile.files(main, files.len());
        for (path, bytes) in paths.iter().zip(&files) {
            let write_error = |source| ProfileError::Write {
                path: path.clone(),
                source,
            };
            let full = root.join(path);
            if let Some(folder) = full.parent() {
                fs::create_dir_all(folder).map_err(write_error)?;
            }
            fs::write(&full, bytes).map_err(write_error)?;
        }
        Ok(paths)
    }
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

    use super::*;
    use crate::fonts::Fonts;
    use crate::job::analyze;
    use crate::world::World;

    fn project(folder: &Path, text: &str) -> Analysis {
        fs::create_dir_all(folder).unwrap();
        fs::write(folder.join("main.typ"), text).unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 20),
            folder.to_path_buf(),
            Path::new("main.typ"),
        ));
        analyze(&world, text).analysis.unwrap()
    }

    fn profile(format: ExportFormat, output: &str) -> ExportProfile {
        ExportProfile {
            output: Some(PathBuf::from(output)),
            ..ExportProfile::named("test", format)
        }
    }

    #[test]
    fn a_profile_writes_its_files_into_the_project_folder() {
        let folder = tempfile::tempdir().unwrap();
        let analysis = project(folder.path(), "One.\n#pagebreak()\nTwo.");

        let files = analysis
            .export_profile(
                &profile(ExportFormat::Svg, "out/page-{p}.svg"),
                folder.path(),
                Path::new("main.typ"),
                None,
            )
            .unwrap();

        assert_eq!(
            files,
            [
                PathBuf::from("out/page-1.svg"),
                PathBuf::from("out/page-2.svg")
            ]
        );
        assert!(folder.path().join("out/page-2.svg").is_file());
    }

    #[test]
    fn page_images_have_the_resolution_of_the_profile() {
        let folder = tempfile::tempdir().unwrap();
        let analysis = project(folder.path(), "#set page(width: 100pt, height: 50pt)\nA.");
        let mut pages = profile(ExportFormat::Png, "page.png");
        pages.ppi = Some(144);

        analysis
            .export_profile(&pages, folder.path(), Path::new("main.typ"), None)
            .unwrap();

        let bytes = fs::read(folder.path().join("page.png")).unwrap();
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        assert_eq!(width, 200);
    }

    #[test]
    fn a_profile_does_not_write_outside_the_project_folder() {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().join("book");
        let analysis = project(&root, "Text.");

        let result = analysis.export_profile(
            &profile(ExportFormat::Pdf, "../book.pdf"),
            &root,
            Path::new("main.typ"),
            None,
        );

        assert!(
            matches!(result, Err(ProfileError::Outside(path)) if path == Path::new("../book.pdf"))
        );
        assert!(!folder.path().join("book.pdf").exists());
    }
}
