use std::fs;
use std::path::{Path, PathBuf};

use presswork_project::{Project, ProjectError};

use crate::i18n::{FluentArgs, Localizer};

pub(crate) async fn pick_folder(title: String) -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title(title)
        .pick_folder()
        .await
        .map(|folder| folder.path().to_path_buf())
}

pub(crate) struct Opened {
    pub(crate) project: Project,
    pub(crate) text: String,
}

pub(crate) fn open(root: &Path) -> Result<Opened, ProjectError> {
    let project = Project::open(root)?;
    let main = project.main();
    let text =
        fs::read_to_string(&main).map_err(|source| ProjectError::Io { path: main, source })?;
    Ok(Opened { project, text })
}

pub(crate) fn error_text(localizer: &Localizer, error: &ProjectError) -> String {
    let mut args = FluentArgs::new();
    let id = match error {
        ProjectError::NoManifest(_) => "project-error-no-manifest",
        ProjectError::InvalidManifest(problem) => {
            args.set("problem", problem.clone());
            "project-error-invalid-manifest"
        }
        ProjectError::MainOutside(_) => "project-error-main-outside",
        ProjectError::MainMissing(file) => {
            args.set("file", file.display().to_string());
            "project-error-main-missing"
        }
        ProjectError::Io { path, .. } => {
            args.set("file", path.display().to_string());
            "project-error-read"
        }
    };
    localizer.format(id, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_project_error_has_english_text() {
        let localizer = Localizer::english();
        let errors = [
            ProjectError::NoManifest(PathBuf::from("a")),
            ProjectError::InvalidManifest("missing field `name`".to_owned()),
            ProjectError::MainOutside(PathBuf::from("../a.typ")),
            ProjectError::MainMissing(PathBuf::from("main.typ")),
            ProjectError::Io {
                path: PathBuf::from("main.typ"),
                source: std::io::Error::other("denied"),
            },
        ];

        for error in &errors {
            let text = error_text(&localizer, error);
            assert!(!text.starts_with("project-error"), "{text}");
        }
    }

    #[test]
    fn open_gives_the_project_and_the_text_of_its_main_file() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(
            folder.path().join("presswork.toml"),
            "[project]\nname = \"A\"\n",
        )
        .unwrap();
        fs::write(folder.path().join("main.typ"), "= A").unwrap();

        let opened = open(folder.path()).unwrap();

        assert_eq!(opened.project.manifest().name, "A");
        assert_eq!(opened.text, "= A");
    }
}
