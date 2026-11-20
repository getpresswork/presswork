use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::ProjectError;

pub const MANIFEST: &str = "presswork.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub main: PathBuf,
    pub typst: Option<String>,
    pub exports: Vec<ExportProfile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    #[default]
    Pdf,
    Png,
    Svg,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExportProfile {
    pub name: String,
    #[serde(default)]
    pub format: ExportFormat,
    #[serde(default)]
    pub standards: Vec<String>,
    pub output: Option<PathBuf>,
    pub ppi: Option<u32>,
    pub image_ppi: Option<u32>,
}

impl ExportProfile {
    #[must_use]
    pub fn named(name: &str, format: ExportFormat) -> Self {
        Self {
            name: name.to_owned(),
            format,
            standards: Vec::new(),
            output: None,
            ppi: None,
            image_ppi: None,
        }
    }

    #[must_use]
    pub fn files(&self, main: &Path, pages: usize) -> Vec<PathBuf> {
        let extension = match self.format {
            ExportFormat::Pdf => "pdf",
            ExportFormat::Png => "png",
            ExportFormat::Svg => "svg",
        };
        let template = self.output.clone().unwrap_or_else(|| {
            let stem = main.file_stem().map_or_else(
                || "main".to_owned(),
                |stem| stem.to_string_lossy().into_owned(),
            );
            let name = match self.format {
                ExportFormat::Pdf => format!("{stem}.{extension}"),
                ExportFormat::Png | ExportFormat::Svg => format!("{stem}-{{p}}.{extension}"),
            };
            main.with_file_name(name)
        });
        if self.format == ExportFormat::Pdf {
            return vec![template];
        }
        let text = template.to_string_lossy().into_owned();
        let numbered = text.contains("{p}") || text.contains("{0p}");
        if !numbered && pages == 1 {
            return vec![template];
        }
        let text = if numbered { text } else { numbered_name(&text) };
        let width = pages.to_string().len();
        (1..=pages)
            .map(|page| {
                PathBuf::from(
                    text.replace("{0p}", &format!("{page:0width$}"))
                        .replace("{p}", &page.to_string()),
                )
            })
            .collect()
    }
}

fn numbered_name(path: &str) -> String {
    let name_start = path.rfind(['/', '\\']).map_or(0, |separator| separator + 1);
    match path.rfind('.').filter(|dot| *dot > name_start) {
        Some(dot) => format!("{}-{{p}}{}", &path[..dot], &path[dot..]),
        None => format!("{path}-{{p}}"),
    }
}

#[derive(Deserialize)]
struct File {
    project: Section,
    #[serde(default, rename = "export")]
    exports: Vec<ExportProfile>,
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

pub(crate) fn with_pin(text: &str, pin: &str) -> Result<String, ProjectError> {
    let mut document: toml_edit::DocumentMut =
        text.parse().map_err(|error: toml_edit::TomlError| {
            ProjectError::InvalidManifest(error.message().to_owned())
        })?;
    let project = document
        .get_mut("project")
        .and_then(toml_edit::Item::as_table_like_mut)
        .ok_or_else(|| ProjectError::InvalidManifest("missing [project] table".to_owned()))?;
    match project
        .get_mut("typst")
        .and_then(toml_edit::Item::as_value_mut)
    {
        Some(value) => {
            let decor = value.decor().clone();
            *value = pin.into();
            *value.decor_mut() = decor;
        }
        None => {
            project.insert("typst", toml_edit::value(pin));
        }
    }
    Ok(document.to_string())
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, ProjectError> {
        let file: File = toml::from_str(text)
            .map_err(|error| ProjectError::InvalidManifest(error.message().to_owned()))?;
        Ok(Self {
            name: file.project.name,
            main: file.project.main,
            typst: file.project.typst,
            exports: file.exports,
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
                exports: Vec::new(),
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
    fn a_pin_goes_into_the_project_table() {
        let text = with_pin("[project]\nname = \"Notes\"\n", "0.14").unwrap();

        assert_eq!(
            Manifest::parse(&text).unwrap().typst.as_deref(),
            Some("0.14")
        );
    }

    #[test]
    fn a_new_pin_keeps_the_comments_and_the_layout() {
        let text = "# The field notes\n[project]\nname = \"Notes\"  # short name\ntypst = \"0.13\"  # for the printer\n\n[[export]]\nname = \"print\"\n";

        assert_eq!(
            with_pin(text, "0.14").unwrap(),
            "# The field notes\n[project]\nname = \"Notes\"  # short name\ntypst = \"0.14\"  # for the printer\n\n[[export]]\nname = \"print\"\n"
        );
    }

    #[test]
    fn a_pin_needs_a_project_table() {
        let error = with_pin("name = \"Notes\"\n", "0.14").unwrap_err();

        assert!(matches!(error, ProjectError::InvalidManifest(_)));
    }

    #[test]
    fn export_profiles_come_from_the_export_tables() {
        let manifest = Manifest::parse(
            "[project]\nname = \"Book\"\n\n\
             [[export]]\nname = \"print\"\nstandards = [\"a-2b\"]\noutput = \"out/book.pdf\"\nimage-ppi = 300\n\n\
             [[export]]\nname = \"screen\"\nformat = \"png\"\nppi = 144\n",
        )
        .unwrap();

        assert_eq!(
            manifest.exports,
            [
                ExportProfile {
                    name: "print".to_owned(),
                    format: ExportFormat::Pdf,
                    standards: vec!["a-2b".to_owned()],
                    output: Some(PathBuf::from("out/book.pdf")),
                    ppi: None,
                    image_ppi: Some(300),
                },
                ExportProfile {
                    name: "screen".to_owned(),
                    format: ExportFormat::Png,
                    standards: Vec::new(),
                    output: None,
                    ppi: Some(144),
                    image_ppi: None,
                },
            ]
        );
    }

    #[test]
    fn an_unknown_export_format_is_not_valid() {
        let error = Manifest::parse(
            "[project]\nname = \"Book\"\n\n[[export]]\nname = \"x\"\nformat = \"doc\"\n",
        )
        .unwrap_err();

        assert!(matches!(error, ProjectError::InvalidManifest(_)));
    }

    #[test]
    fn the_output_follows_the_main_file_by_default() {
        let pdf = ExportProfile::named("PDF", ExportFormat::Pdf);
        let png = ExportProfile::named("Images", ExportFormat::Png);
        let main = Path::new("chapters/book.typ");

        assert_eq!(pdf.files(main, 1), [PathBuf::from("chapters/book.pdf")]);
        assert_eq!(
            png.files(main, 2),
            [
                PathBuf::from("chapters/book-1.png"),
                PathBuf::from("chapters/book-2.png")
            ]
        );
    }

    #[test]
    fn page_files_take_the_page_number() {
        let mut profile = ExportProfile::named("Pages", ExportFormat::Svg);
        let main = Path::new("main.typ");

        profile.output = Some(PathBuf::from("out/page-{0p}.svg"));
        assert_eq!(
            profile.files(main, 10)[..2],
            [
                PathBuf::from("out/page-01.svg"),
                PathBuf::from("out/page-02.svg")
            ]
        );

        profile.output = Some(PathBuf::from("out/page.svg"));
        assert_eq!(profile.files(main, 1), [PathBuf::from("out/page.svg")]);
        assert_eq!(
            profile.files(main, 2),
            [
                PathBuf::from("out/page-1.svg"),
                PathBuf::from("out/page-2.svg")
            ]
        );
    }

    #[test]
    fn a_manifest_without_a_name_is_not_valid() {
        let error = Manifest::parse("[project]\nmain = \"a.typ\"\n").unwrap_err();

        assert!(
            matches!(error, ProjectError::InvalidManifest(message) if message.contains("name"))
        );
    }
}
