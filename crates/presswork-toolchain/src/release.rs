use semver::Version;
use serde::Deserialize;

use crate::digest::Digest;
use crate::error::ToolchainError;
use crate::platform::Platform;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub files: Vec<ReleaseFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseFile {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub digest: Option<Digest>,
}

impl Release {
    #[must_use]
    pub fn is_stable(&self) -> bool {
        self.version.pre.is_empty()
    }

    #[must_use]
    pub fn file(&self, platform: &Platform) -> Option<&ReleaseFile> {
        let name = platform.release_file()?;
        self.files.iter().find(|file| file.name == name)
    }
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    size: u64,
    digest: Option<String>,
    browser_download_url: String,
}

pub fn parse_releases(json: &str) -> Result<Vec<Release>, ToolchainError> {
    let releases: Vec<GithubRelease> = serde_json::from_str(json)
        .map_err(|error| ToolchainError::InvalidReleaseList(error.to_string()))?;
    Ok(releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = Version::parse(release.tag_name.strip_prefix('v')?).ok()?;
            let files = release
                .assets
                .into_iter()
                .map(|asset| ReleaseFile {
                    name: asset.name,
                    url: asset.browser_download_url,
                    size: asset.size,
                    digest: asset.digest.as_deref().and_then(Digest::from_github),
                })
                .collect();
            Some(Release { version, files })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "19ce3551153c2fe7ee9fa2f95208310c8f4d3209fedb699e0333faf8913f6736";

    fn list() -> String {
        format!(
            r#"[
  {{
    "tag_name": "v0.15.1", "prerelease": false, "draft": false,
    "assets": [
      {{
        "name": "typst-x86_64-pc-windows-msvc.zip", "size": 22463684,
        "digest": "sha256:{DIGEST}",
        "browser_download_url": "https://github.com/typst/typst/releases/download/v0.15.1/typst-x86_64-pc-windows-msvc.zip"
      }},
      {{
        "name": "typst-x86_64-apple-darwin.tar.xz", "size": 15635600, "digest": null,
        "browser_download_url": "https://github.com/typst/typst/releases/download/v0.15.1/typst-x86_64-apple-darwin.tar.xz"
      }}
    ]
  }},
  {{ "tag_name": "v0.15.0-rc.1", "prerelease": true, "draft": false, "assets": [] }},
  {{ "tag_name": "v0.16.0", "prerelease": false, "draft": true, "assets": [] }},
  {{ "tag_name": "v23-03-28", "prerelease": true, "draft": false, "assets": [] }}
]"#
        )
    }

    #[test]
    fn a_release_gives_its_version_and_its_files() {
        let releases = parse_releases(&list()).unwrap();

        let release = &releases[0];
        assert_eq!(release.version, Version::new(0, 15, 1));
        assert!(release.is_stable());
        assert_eq!(
            release.files[0],
            ReleaseFile {
                name: "typst-x86_64-pc-windows-msvc.zip".to_owned(),
                url: "https://github.com/typst/typst/releases/download/v0.15.1/typst-x86_64-pc-windows-msvc.zip".to_owned(),
                size: 22_463_684,
                digest: Digest::from_hex(DIGEST),
            }
        );
        assert_eq!(release.files[1].digest, None);
    }

    #[test]
    fn a_release_candidate_is_not_stable() {
        let releases = parse_releases(&list()).unwrap();

        assert_eq!(releases[1].version, Version::parse("0.15.0-rc.1").unwrap());
        assert!(!releases[1].is_stable());
    }

    #[test]
    fn drafts_and_tags_without_a_version_are_ignored() {
        let versions: Vec<String> = parse_releases(&list())
            .unwrap()
            .iter()
            .map(|release| release.version.to_string())
            .collect();

        assert_eq!(versions, ["0.15.1", "0.15.0-rc.1"]);
    }

    #[test]
    fn a_release_finds_the_file_of_a_platform() {
        let releases = parse_releases(&list()).unwrap();

        let file = releases[0].file(&Platform::new("macos", "x86_64")).unwrap();
        assert_eq!(file.name, "typst-x86_64-apple-darwin.tar.xz");
        assert!(
            releases[0]
                .file(&Platform::new("linux", "x86_64"))
                .is_none()
        );
    }

    #[test]
    fn a_list_that_is_not_json_gives_an_error() {
        assert!(matches!(
            parse_releases("<html>rate limit</html>"),
            Err(ToolchainError::InvalidReleaseList(_))
        ));
    }
}
