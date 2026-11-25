use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use presswork_editor::Editor;
use presswork_project::{Project, ProjectError};

use crate::i18n::{FluentArgs, Localizer};
use crate::sidecar;

const RESERVED: [char; 7] = ['<', '>', ':', '"', '|', '?', '*'];

pub(crate) struct OpenFile {
    pub(crate) editor: Editor,
    pub(crate) saved: u64,
}

pub(crate) enum ReadError {
    NotText,
    Io,
}

pub(crate) fn read_text(path: &Path) -> Result<String, ReadError> {
    let bytes = fs::read(path).map_err(|_| ReadError::Io)?;
    String::from_utf8(bytes).map_err(|_| ReadError::NotText)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileError {
    Invalid,
    Exists,
    MakeFailed,
    RenameFailed,
    DeleteFailed,
}

pub(crate) fn project_path(name: &str) -> Result<PathBuf, FileError> {
    let mut path = PathBuf::new();
    for part in name.trim().split(['/', '\\']) {
        let valid = !part.is_empty()
            && !part.starts_with('.')
            && !part.ends_with(['.', ' '])
            && !part
                .chars()
                .any(|character| character.is_control() || RESERVED.contains(&character));
        if !valid {
            return Err(FileError::Invalid);
        }
        path.push(part);
    }
    Ok(path)
}

pub(crate) fn make_file(root: &Path, path: &Path) -> Result<(), FileError> {
    let full = root.join(path);
    if let Some(folder) = full.parent() {
        fs::create_dir_all(folder).map_err(|_| FileError::MakeFailed)?;
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&full)
        .map(drop)
        .map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => FileError::Exists,
            _ => FileError::MakeFailed,
        })
}

pub(crate) fn rename_file(root: &Path, old: &Path, new: &Path) -> Result<(), FileError> {
    let from = root.join(old);
    let to = root.join(new);
    if to.exists() && !same_file(&from, &to) {
        return Err(FileError::Exists);
    }
    if let Some(folder) = to.parent() {
        fs::create_dir_all(folder).map_err(|_| FileError::RenameFailed)?;
    }
    fs::rename(&from, &to).map_err(|_| FileError::RenameFailed)?;
    if let Err(error) = sidecar::rename(root, old, new) {
        tracing::warn!(%error, "The suggestions did not move with the file");
    }
    Ok(())
}

pub(crate) fn delete_file(root: &Path, path: &Path) -> Result<(), FileError> {
    fs::remove_file(root.join(path)).map_err(|_| FileError::DeleteFailed)?;
    if let Err(error) = sidecar::remove(root, path) {
        tracing::warn!(%error, "The suggestions of the deleted file stay on the disk");
    }
    Ok(())
}

#[cfg(unix)]
fn same_file(one: &Path, other: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (fs::metadata(one), fs::metadata(other)) {
        (Ok(one), Ok(other)) => one.dev() == other.dev() && one.ino() == other.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_file(one: &Path, other: &Path) -> bool {
    matches!(
        (fs::canonicalize(one), fs::canonicalize(other)),
        (Ok(one), Ok(other)) if one == other
    )
}

pub(crate) fn file_error_text(localizer: &Localizer, error: FileError, file: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("file", file.to_owned());
    let id = match error {
        FileError::Invalid => "files-path-invalid",
        FileError::Exists => "files-exists",
        FileError::MakeFailed => "files-make-error",
        FileError::RenameFailed => "files-rename-error",
        FileError::DeleteFailed => "files-delete-error",
    };
    localizer.format(id, &args)
}

pub(crate) fn display_path(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

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

    fn relative(path: &str) -> PathBuf {
        path.split('/').collect()
    }

    fn suggestions_of(root: &Path, file: &Path) -> PathBuf {
        let path = crate::sidecar::path(root, file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{}").unwrap();
        path
    }

    #[test]
    fn a_file_name_becomes_a_path_in_the_project() {
        for (name, path) in [
            ("chapters/two.typ", "chapters/two.typ"),
            ("  notes.typ ", "notes.typ"),
            ("chapters\\three.typ", "chapters/three.typ"),
        ] {
            assert_eq!(project_path(name), Ok(relative(path)), "{name}");
        }
    }

    #[test]
    fn a_file_name_outside_the_project_or_with_bad_characters_is_invalid() {
        for name in [
            "",
            "   ",
            "../two.typ",
            "/two.typ",
            "chapters//two.typ",
            "chapters/",
            ".hidden.typ",
            "chapters/.draft.typ",
            "C:two.typ",
            "two?.typ",
            "two.",
            "a\tb.typ",
        ] {
            assert_eq!(project_path(name), Err(FileError::Invalid), "{name:?}");
        }
    }

    #[test]
    fn make_file_makes_an_empty_file_and_its_folders() {
        let folder = tempfile::tempdir().unwrap();
        let path = relative("chapters/two.typ");

        make_file(folder.path(), &path).unwrap();

        assert_eq!(fs::read_to_string(folder.path().join(&path)).unwrap(), "");
        assert_eq!(make_file(folder.path(), &path), Err(FileError::Exists));
    }

    #[test]
    fn rename_file_moves_the_file_and_its_suggestions() {
        let folder = tempfile::tempdir().unwrap();
        let old = relative("one.typ");
        let new = relative("chapters/first.typ");
        fs::write(folder.path().join(&old), "Text.").unwrap();
        let suggestions = suggestions_of(folder.path(), &old);

        rename_file(folder.path(), &old, &new).unwrap();

        assert!(!folder.path().join(&old).exists());
        assert_eq!(
            fs::read_to_string(folder.path().join(&new)).unwrap(),
            "Text."
        );
        assert!(!suggestions.exists());
        assert!(crate::sidecar::path(folder.path(), &new).is_file());
    }

    #[test]
    fn rename_file_does_not_replace_another_file() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("one.typ"), "One.").unwrap();
        fs::write(folder.path().join("two.typ"), "Two.").unwrap();

        let result = rename_file(folder.path(), Path::new("one.typ"), Path::new("two.typ"));

        assert_eq!(result, Err(FileError::Exists));
        assert_eq!(
            fs::read_to_string(folder.path().join("two.typ")).unwrap(),
            "Two."
        );
    }

    #[test]
    fn rename_file_can_change_only_the_case_of_a_name() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("one.typ"), "One.").unwrap();

        rename_file(folder.path(), Path::new("one.typ"), Path::new("One.typ")).unwrap();

        let names: Vec<String> = fs::read_dir(folder.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["One.typ"]);
    }

    #[test]
    fn delete_file_removes_the_file_and_its_suggestions() {
        let folder = tempfile::tempdir().unwrap();
        let file = Path::new("one.typ");
        fs::write(folder.path().join(file), "One.").unwrap();
        let suggestions = suggestions_of(folder.path(), file);

        delete_file(folder.path(), file).unwrap();

        assert!(!folder.path().join(file).exists());
        assert!(!suggestions.exists());
    }

    #[test]
    fn each_file_error_has_english_text_with_the_file_name() {
        let localizer = Localizer::english();

        for error in [
            FileError::Invalid,
            FileError::Exists,
            FileError::MakeFailed,
            FileError::RenameFailed,
            FileError::DeleteFailed,
        ] {
            let text = file_error_text(&localizer, error, "two.typ");
            assert!(text.contains("two.typ"), "{error:?}: {text}");
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
