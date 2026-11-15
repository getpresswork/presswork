use std::path::PathBuf;

use serde::Deserialize;

use crate::error::ProjectError;

pub const MANIFEST: &str = "presswork.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub main: PathBuf,
    pub typst: Option<String>,
}

#[derive(Deserialize)]
struct File {
    project: Section,
}

#[derive(Deserialize)]
struct Section {
    name: String,
    #[serde(default = "default_main")]
    main: PathBuf,
    typst: Option<String>,
}

fn default_main() -> PathBuf {
    PathBuf::from("main.typ")
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, ProjectError> {
        let file: File = toml::from_str(text)
            .map_err(|error| ProjectError::InvalidManifest(error.message().to_owned()))?;
        Ok(Self {
            name: file.project.name,
            main: file.project.main,
            typst: file.project.typst,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_gives_the_name_the_main_file_and_the_pin() {
        let manifest = Manifest::parse(
            "[project]\nname = \"The Book\"\nmain = \"book.typ\"\ntypst = \"0.15\"\n",
        )
        .unwrap();

        assert_eq!(
            manifest,
            Manifest {
                name: "The Book".to_owned(),
                main: PathBuf::from("book.typ"),
                typst: Some("0.15".to_owned()),
            }
        );
    }

    #[test]
    fn the_main_file_is_main_typ_by_default() {
        let manifest = Manifest::parse("[project]\nname = \"Notes\"\n").unwrap();

        assert_eq!(manifest.main, PathBuf::from("main.typ"));
        assert_eq!(manifest.typst, None);
    }

    #[test]
    fn unknown_keys_are_permitted_for_later_versions() {
        let text = "[project]\nname = \"Notes\"\nfuture = 1\n\n[[export]]\nname = \"print\"\n";

        assert!(Manifest::parse(text).is_ok());
    }

    #[test]
    fn a_manifest_without_a_name_is_not_valid() {
        let error = Manifest::parse("[project]\nmain = \"a.typ\"\n").unwrap_err();

        assert!(
            matches!(error, ProjectError::InvalidManifest(message) if message.contains("name"))
        );
    }
}
