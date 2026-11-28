use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const FILE: &str = "recent-projects.json";
const KEPT: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RecentProject {
    pub(crate) name: String,
    pub(crate) folder: PathBuf,
}

#[derive(Debug, Default)]
pub(crate) struct RecentProjects {
    file: Option<PathBuf>,
    projects: Vec<RecentProject>,
}

impl RecentProjects {
    pub(crate) fn load(file: Option<PathBuf>) -> Self {
        let projects = file
            .as_deref()
            .and_then(|file| fs::read_to_string(file).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { file, projects }
    }

    pub(crate) fn projects(&self) -> &[RecentProject] {
        &self.projects
    }

    pub(crate) fn add(&mut self, name: &str, folder: &Path) {
        self.projects.retain(|project| project.folder != folder);
        self.projects.insert(
            0,
            RecentProject {
                name: name.to_owned(),
                folder: folder.to_path_buf(),
            },
        );
        self.projects.truncate(KEPT);
        self.save();
    }

    pub(crate) fn remove(&mut self, folder: &Path) {
        self.projects.retain(|project| project.folder != folder);
        self.save();
    }

    fn save(&self) {
        let Some(file) = &self.file else {
            return;
        };
        let written = file
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| {
                let text =
                    serde_json::to_string_pretty(&self.projects).map_err(std::io::Error::other)?;
                fs::write(file, text)
            });
        if let Err(error) = written {
            tracing::warn!(%error, path = %file.display(), "The recent projects were not saved");
        }
    }
}

pub(crate) fn recent_projects_file() -> Option<PathBuf> {
    directories::ProjectDirs::from("io.github", "getpresswork", "Presswork")
        .map(|folders| folders.data_local_dir().join(FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(recent: &RecentProjects) -> Vec<String> {
        recent
            .projects()
            .iter()
            .map(|project| project.name.clone())
            .collect()
    }

    #[test]
    fn a_project_opened_again_moves_to_the_top_and_the_list_keeps_ten() {
        let mut recent = RecentProjects::default();

        for n in 0..12 {
            recent.add(&format!("Book {n}"), Path::new(&format!("books/{n}")));
        }
        recent.add("Book 5", Path::new("books/5"));

        let names = names(&recent);
        assert_eq!(names.len(), KEPT);
        assert_eq!(names[0], "Book 5");
        assert_eq!(names[1], "Book 11");
        assert!(!names.contains(&"Book 0".to_owned()));
        assert_eq!(names.iter().filter(|name| *name == "Book 5").count(), 1);
    }

    #[test]
    fn the_list_comes_back_from_its_file() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("data").join(FILE);
        let mut recent = RecentProjects::load(Some(file.clone()));
        recent.add("Field Notes", Path::new("books/notes"));
        recent.add("The Press", Path::new("books/press"));

        let loaded = RecentProjects::load(Some(file));

        assert_eq!(names(&loaded), ["The Press", "Field Notes"]);
    }

    #[test]
    fn a_removed_project_leaves_the_list_and_its_file() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join(FILE);
        let mut recent = RecentProjects::load(Some(file.clone()));
        recent.add("Field Notes", Path::new("books/notes"));
        recent.add("The Press", Path::new("books/press"));

        recent.remove(Path::new("books/notes"));

        assert_eq!(names(&recent), ["The Press"]);
        assert_eq!(names(&RecentProjects::load(Some(file))), ["The Press"]);
    }

    #[test]
    fn a_broken_file_gives_an_empty_list() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join(FILE);
        fs::write(&file, "not json").unwrap();

        assert_eq!(
            names(&RecentProjects::load(Some(file))),
            Vec::<String>::new()
        );
    }
}
