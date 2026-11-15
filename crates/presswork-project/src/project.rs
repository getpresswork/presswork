use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::error::ProjectError;
use crate::manifest::{MANIFEST, Manifest};

#[derive(Debug, Clone)]
pub struct Project {
    root: PathBuf,
    manifest: Manifest,
    files: Vec<PathBuf>,
}

impl Project {
    pub fn open(root: &Path) -> Result<Self, ProjectError> {
        let manifest_path = root.join(MANIFEST);
        let text = match fs::read_to_string(&manifest_path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(ProjectError::NoManifest(root.to_path_buf()));
            }
            Err(source) => {
                return Err(ProjectError::Io {
                    path: manifest_path,
                    source,
                });
            }
        };
        let manifest = Manifest::parse(&text)?;
        if !is_inside(&manifest.main) {
            return Err(ProjectError::MainOutside(manifest.main));
        }
        if !root.join(&manifest.main).is_file() {
            return Err(ProjectError::MainMissing(manifest.main));
        }
        let mut files = Vec::new();
        list(root, Path::new(""), &mut files)?;
        files.sort();
        Ok(Self {
            root: root.to_path_buf(),
            manifest,
            files,
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    #[must_use]
    pub fn main(&self) -> PathBuf {
        self.root.join(&self.manifest.main)
    }

    #[must_use]
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }
}

fn is_inside(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn list(root: &Path, relative: &Path, files: &mut Vec<PathBuf>) -> Result<(), ProjectError> {
    let directory = root.join(relative);
    let io_error = |source| ProjectError::Io {
        path: directory.clone(),
        source,
    };
    for entry in fs::read_dir(&directory).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = relative.join(&name);
        let file_type = entry.file_type().map_err(io_error)?;
        if file_type.is_dir() {
            list(root, &path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::manifest::MANIFEST;

    fn folder(files: &[(&str, &str)]) -> TempDir {
        let folder = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let path = folder.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        folder
    }

    #[test]
    fn a_folder_with_a_manifest_opens_as_a_project() {
        let folder = folder(&[
            (MANIFEST, "[project]\nname = \"The Book\"\n"),
            ("main.typ", "= Title"),
        ]);

        let project = Project::open(folder.path()).unwrap();

        assert_eq!(project.manifest().name, "The Book");
        assert_eq!(project.main(), folder.path().join("main.typ"));
    }

    #[test]
    fn a_folder_without_a_manifest_is_not_a_project() {
        let folder = folder(&[("main.typ", "")]);

        let error = Project::open(folder.path()).unwrap_err();

        assert!(matches!(error, ProjectError::NoManifest(_)));
    }

    #[test]
    fn the_main_file_must_be_in_the_project_folder() {
        let folder = folder(&[(MANIFEST, "[project]\nname = \"A\"\nmain = \"../a.typ\"\n")]);

        let error = Project::open(folder.path()).unwrap_err();

        assert!(matches!(error, ProjectError::MainOutside(_)));
    }

    #[test]
    fn the_main_file_must_exist() {
        let folder = folder(&[(MANIFEST, "[project]\nname = \"A\"\n")]);

        let error = Project::open(folder.path()).unwrap_err();

        assert!(matches!(error, ProjectError::MainMissing(_)));
    }

    #[test]
    fn the_file_list_has_the_project_files_without_hidden_files() {
        let folder = folder(&[
            (MANIFEST, "[project]\nname = \"A\"\n"),
            ("main.typ", ""),
            ("chapters/two.typ", ""),
            ("chapters/one.typ", ""),
            (".git/config", ""),
            (".hidden.typ", ""),
        ]);

        let project = Project::open(folder.path()).unwrap();

        assert_eq!(
            project.files(),
            [
                PathBuf::from("chapters").join("one.typ"),
                PathBuf::from("chapters").join("two.typ"),
                PathBuf::from("main.typ"),
                PathBuf::from(MANIFEST),
            ]
        );
    }
}
