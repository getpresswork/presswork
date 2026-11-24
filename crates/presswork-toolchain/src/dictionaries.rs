use std::fs;
use std::path::{Path, PathBuf};

use crate::client::{Client, Progress};
use crate::digest::Digest;
use crate::error::ToolchainError;

const LIST: &str = include_str!("../dictionaries.txt");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub path: String,
    pub size: u64,
    pub digest: Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionarySource {
    pub tag: String,
    pub license: String,
    pub aff: SourceFile,
    pub dic: SourceFile,
    pub notice: SourceFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    pub commit: String,
    pub sources: Vec<DictionarySource>,
}

pub fn catalog() -> Result<Catalog, ToolchainError> {
    parse_catalog(LIST)
}

fn parse_catalog(text: &str) -> Result<Catalog, ToolchainError> {
    let mut commit = None;
    let mut sources = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let invalid = || ToolchainError::InvalidDictionaryList(index + 1);
        if let Some(value) = line.strip_prefix("commit ") {
            commit = Some(value.trim().to_owned());
            continue;
        }
        let parts: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [tag, license, aff, dic, notice] = parts.as_slice() else {
            return Err(invalid());
        };
        sources.push(DictionarySource {
            tag: (*tag).to_owned(),
            license: (*license).to_owned(),
            aff: source_file(aff).ok_or_else(invalid)?,
            dic: source_file(dic).ok_or_else(invalid)?,
            notice: source_file(notice).ok_or_else(invalid)?,
        });
    }
    let commit = commit.ok_or(ToolchainError::InvalidDictionaryList(0))?;
    Ok(Catalog { commit, sources })
}

fn source_file(text: &str) -> Option<SourceFile> {
    let mut parts = text.split_whitespace();
    let (Some(path), Some(size), Some(digest), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    Some(SourceFile {
        path: path.to_owned(),
        size: size.parse().ok()?,
        digest: Digest::from_hex(digest)?,
    })
}

const RAW: &str = "https://raw.githubusercontent.com/LibreOffice/dictionaries";
const AFF: &str = "dictionary.aff";
const DIC: &str = "dictionary.dic";
const NOTICE: &str = "license.txt";

#[derive(Debug, Clone)]
pub struct Dictionaries {
    root: PathBuf,
    base: String,
}

impl Dictionaries {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, catalog: &Catalog) -> Self {
        Self::with_base(root, &format!("{RAW}/{}", catalog.commit))
    }

    #[must_use]
    pub fn with_base(root: impl Into<PathBuf>, base: &str) -> Self {
        Self {
            root: root.into(),
            base: base.to_owned(),
        }
    }

    #[must_use]
    pub fn installed(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut tags: Vec<String> = entries
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|tag| self.files(tag).is_some())
            .collect();
        tags.sort();
        tags
    }

    #[must_use]
    pub fn read(&self, tag: &str) -> Option<(Vec<u8>, Vec<u8>)> {
        let (aff, dic) = self.files(tag)?;
        Some((fs::read(aff).ok()?, fs::read(dic).ok()?))
    }

    #[must_use]
    pub fn files(&self, tag: &str) -> Option<(PathBuf, PathBuf)> {
        let folder = self.folder(tag)?;
        let (aff, dic) = (folder.join(AFF), folder.join(DIC));
        (aff.is_file() && dic.is_file()).then_some((aff, dic))
    }

    pub async fn install(
        &self,
        client: &Client,
        source: &DictionarySource,
        mut progress: impl FnMut(Progress),
    ) -> Result<(), ToolchainError> {
        let target = self
            .folder(&source.tag)
            .ok_or(ToolchainError::InvalidDictionaryList(0))?;
        fs::create_dir_all(&self.root).map_err(|error| io_error(&self.root, error))?;
        let work = tempfile::Builder::new()
            .prefix(".download-")
            .tempdir_in(&self.root)
            .map_err(|error| io_error(&self.root, error))?;
        let total = source.aff.size + source.dic.size + source.notice.size;
        let mut done = 0;
        for (file, name) in [
            (&source.aff, AFF),
            (&source.dic, DIC),
            (&source.notice, NOTICE),
        ] {
            let path = work.path().join(name);
            let url = format!("{}/{}", self.base, file.path);
            client
                .download(&url, &path, |step| {
                    progress(Progress {
                        received: done + step.received,
                        total: Some(total),
                    });
                })
                .await?;
            let actual = Digest::of_file(&path).map_err(|error| io_error(&path, error))?;
            if actual != file.digest {
                return Err(ToolchainError::DigestMismatch {
                    file: file.path.clone(),
                    published: file.digest.to_hex(),
                    actual: actual.to_hex(),
                });
            }
            done += file.size;
        }
        if target.exists() {
            fs::remove_dir_all(&target).map_err(|error| io_error(&target, error))?;
        }
        let kept = work.keep();
        fs::rename(&kept, &target).map_err(|error| {
            let _ = fs::remove_dir_all(&kept);
            io_error(&target, error)
        })
    }

    pub fn remove(&self, tag: &str) -> Result<(), ToolchainError> {
        let Some(folder) = self.folder(tag).filter(|folder| folder.exists()) else {
            return Ok(());
        };
        fs::remove_dir_all(&folder).map_err(|error| io_error(&folder, error))
    }

    fn folder(&self, tag: &str) -> Option<PathBuf> {
        let valid = !tag.is_empty()
            && tag
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-');
        valid.then(|| self.root.join(tag))
    }
}

fn io_error(path: &Path, source: std::io::Error) -> ToolchainError {
    ToolchainError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use sha2::{Digest as _, Sha256};

    use super::*;
    use crate::testing::{Route, run, serve};

    const FILES: [(&str, &[u8]); 3] = [
        ("xx/xx.aff", b"SET UTF-8\n"),
        ("xx/xx.dic", b"1\nword\n"),
        ("xx/README.txt", b"The license."),
    ];

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
    }

    fn file(path: &str, bytes: &[u8]) -> SourceFile {
        SourceFile {
            path: path.to_owned(),
            size: bytes.len() as u64,
            digest: Digest::from_hex(&hex(&Sha256::digest(bytes))).unwrap(),
        }
    }

    fn source() -> DictionarySource {
        DictionarySource {
            tag: "xx-XX".to_owned(),
            license: "MIT".to_owned(),
            aff: file(FILES[0].0, FILES[0].1),
            dic: file(FILES[1].0, FILES[1].1),
            notice: file(FILES[2].0, FILES[2].1),
        }
    }

    fn server() -> String {
        serve(|_| {
            FILES
                .iter()
                .map(|(path, bytes)| Route::new(&format!("/{path}"), *bytes))
                .collect()
        })
    }

    fn client() -> Client {
        Client::new("presswork-test").unwrap()
    }

    #[test]
    fn the_included_list_has_the_libreoffice_dictionaries() {
        let catalog = catalog().unwrap();

        assert_eq!(catalog.commit.len(), 40);
        let tags: Vec<&str> = catalog
            .sources
            .iter()
            .map(|source| source.tag.as_str())
            .collect();
        assert_eq!(tags.len(), 14);
        assert_eq!(tags[0], "en-US");
        let extension = |file: &SourceFile| {
            Path::new(&file.path)
                .extension()
                .map(|extension| extension.to_string_lossy().into_owned())
        };
        assert!(catalog.sources.iter().all(|source| {
            extension(&source.aff).as_deref() == Some("aff")
                && extension(&source.dic).as_deref() == Some("dic")
        }));
    }

    #[test]
    fn a_broken_line_in_the_list_is_an_error() {
        let result =
            parse_catalog("commit 0123\nen-US | MIT | a.aff 1 nothex | a.dic 1 00 | r 1 00\n");

        assert!(matches!(
            result,
            Err(ToolchainError::InvalidDictionaryList(2))
        ));
    }

    #[test]
    fn a_dictionary_installs_its_three_files() {
        let root = tempfile::tempdir().unwrap();
        let dictionaries = Dictionaries::with_base(root.path(), &server());
        let mut steps = Vec::new();

        run(dictionaries.install(&client(), &source(), |step| steps.push(step))).unwrap();

        assert_eq!(dictionaries.installed(), ["xx-XX"]);
        let (aff, dic) = dictionaries.read("xx-XX").unwrap();
        assert_eq!(aff, FILES[0].1);
        assert_eq!(dic, FILES[1].1);
        assert!(root.path().join("xx-XX").join("license.txt").is_file());
        let total = source().aff.size + source().dic.size + source().notice.size;
        assert_eq!(steps.last().map(|step| step.received), Some(total));
    }

    #[test]
    fn a_file_with_another_digest_stops_the_installation() {
        let root = tempfile::tempdir().unwrap();
        let dictionaries = Dictionaries::with_base(root.path(), &server());
        let mut changed = source();
        changed.dic.digest = Digest::from_hex(&"0".repeat(64)).unwrap();

        let result = run(dictionaries.install(&client(), &changed, |_| {}));

        assert!(matches!(result, Err(ToolchainError::DigestMismatch { .. })));
        assert_eq!(dictionaries.installed(), Vec::<String>::new());
    }

    #[test]
    fn a_removed_dictionary_is_not_installed() {
        let root = tempfile::tempdir().unwrap();
        let dictionaries = Dictionaries::with_base(root.path(), &server());
        run(dictionaries.install(&client(), &source(), |_| {})).unwrap();

        dictionaries.remove("xx-XX").unwrap();

        assert_eq!(dictionaries.installed(), Vec::<String>::new());
        assert_eq!(dictionaries.files("../xx-XX"), None);
    }
}
