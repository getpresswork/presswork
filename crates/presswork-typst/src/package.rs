use std::fmt;
use std::path::PathBuf;

use typst::syntax::package::PackageSpec;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageName {
    pub namespace: String,
    pub name: String,
    pub version: String,
}

impl fmt::Display for PackageName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "@{}/{}:{}",
            self.namespace, self.name, self.version
        )
    }
}

impl From<&PackageSpec> for PackageName {
    fn from(spec: &PackageSpec) -> Self {
        Self {
            namespace: spec.namespace.to_string(),
            name: spec.name.to_string(),
            version: spec.version.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadError {
    NotFound,
    VersionNotFound(String),
    Failed(String),
}

pub trait Downloader: Send + Sync + fmt::Debug {
    fn download(&self, package: &PackageName) -> Result<PathBuf, DownloadError>;
}

#[must_use]
pub fn package_cache() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|folders| folders.cache_dir().join("typst").join("packages"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_package_name_has_the_form_of_an_import() {
        let package = PackageName {
            namespace: "preview".to_owned(),
            name: "cetz".to_owned(),
            version: "0.3.4".to_owned(),
        };

        assert_eq!(package.to_string(), "@preview/cetz:0.3.4");
    }
}
