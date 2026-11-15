use std::collections::HashMap;
use std::fmt::{self, Write as _};
use std::path::Path;

use semver::Version;
use sha2::{Digest as _, Sha256};

use crate::error::ToolchainError;

pub(crate) const KNOWN_DIGESTS: &str = include_str!("../digests.txt");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    pub fn of_file(path: &Path) -> std::io::Result<Self> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        Ok(Self(hasher.finalize().into()))
    }

    #[must_use]
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let mut bytes = [0; 32];
        for (byte, pair) in bytes.iter_mut().zip(hex.as_bytes().chunks(2)) {
            *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
        }
        Some(Self(bytes))
    }

    #[must_use]
    pub fn from_github(text: &str) -> Option<Self> {
        text.strip_prefix("sha256:").and_then(Self::from_hex)
    }

    #[must_use]
    pub fn to_hex(&self) -> String {
        self.0
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            })
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Digest({})", self.to_hex())
    }
}

#[derive(Debug, Default)]
pub(crate) struct KnownDigests(HashMap<(Version, String), Digest>);

impl KnownDigests {
    pub(crate) fn parse(text: &str) -> Result<Self, ToolchainError> {
        let mut digests = HashMap::new();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let invalid = || ToolchainError::InvalidDigestList(index + 1);
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [version, file, hex] = fields[..] else {
                return Err(invalid());
            };
            let version = Version::parse(version).map_err(|_| invalid())?;
            let digest = Digest::from_hex(hex).ok_or_else(invalid)?;
            digests.insert((version, file.to_owned()), digest);
        }
        Ok(Self(digests))
    }

    pub(crate) fn included() -> Self {
        Self::parse(KNOWN_DIGESTS).unwrap_or_default()
    }

    pub(crate) fn get(&self, version: &Version, file: &str) -> Option<Digest> {
        self.0.get(&(version.clone(), file.to_owned())).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "19ce3551153c2fe7ee9fa2f95208310c8f4d3209fedb699e0333faf8913f6736";

    #[test]
    fn a_digest_reads_64_hex_digits() {
        let digest = Digest::from_hex(HEX).unwrap();

        assert_eq!(digest.to_hex(), HEX);
        assert_eq!(Digest::from_hex(&HEX.to_uppercase()), Some(digest));
    }

    #[test]
    fn a_digest_with_a_wrong_length_or_letter_is_not_valid() {
        assert_eq!(Digest::from_hex(&HEX[1..]), None);
        assert_eq!(Digest::from_hex(&format!("{}g", &HEX[1..])), None);
    }

    #[test]
    fn the_digest_of_a_file_is_its_sha256() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("abc.txt");
        std::fs::write(&path, "abc").unwrap();

        assert_eq!(
            Digest::of_file(&path).unwrap().to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn a_list_of_known_digests_gives_the_digest_of_a_version_and_a_file() {
        let known = KnownDigests::parse(&format!(
            "# version file sha256\n\n0.13.1 typst-x86_64-pc-windows-msvc.zip {HEX}\n"
        ))
        .unwrap();

        assert_eq!(
            known.get(&Version::new(0, 13, 1), "typst-x86_64-pc-windows-msvc.zip"),
            Digest::from_hex(HEX)
        );
        assert_eq!(
            known.get(&Version::new(0, 13, 0), "typst-x86_64-pc-windows-msvc.zip"),
            None
        );
    }

    #[test]
    fn a_wrong_line_in_a_list_of_known_digests_gives_its_number() {
        let error = KnownDigests::parse("0.13.1 typst.zip\n").unwrap_err();

        assert!(error.to_string().contains("line 1"));
    }

    #[test]
    fn the_included_list_of_known_digests_is_valid() {
        assert!(KnownDigests::parse(KNOWN_DIGESTS).is_ok());
    }

    #[test]
    fn a_github_digest_must_use_sha256() {
        assert_eq!(
            Digest::from_github(&format!("sha256:{HEX}")),
            Digest::from_hex(HEX)
        );
        assert_eq!(Digest::from_github(&format!("sha512:{HEX}")), None);
        assert_eq!(Digest::from_github(HEX), None);
    }
}
