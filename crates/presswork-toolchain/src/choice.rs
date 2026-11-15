use semver::{Version, VersionReq};

use crate::error::ToolchainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Engine {
    Embedded,
    Managed(Version),
    Missing(VersionReq),
}

pub fn parse_pin(pin: &str) -> Result<VersionReq, ToolchainError> {
    VersionReq::parse(pin).map_err(|error| ToolchainError::InvalidPin {
        pin: pin.to_owned(),
        reason: error.to_string(),
    })
}

#[must_use]
pub fn choose(pin: Option<&VersionReq>, embedded: &Version, installed: &[Version]) -> Engine {
    let Some(pin) = pin else {
        return Engine::Embedded;
    };
    if pin.matches(embedded) {
        return Engine::Embedded;
    }
    installed
        .iter()
        .filter(|version| pin.matches(version))
        .max()
        .map_or_else(
            || Engine::Missing(pin.clone()),
            |version| Engine::Managed(version.clone()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn installed() -> Vec<Version> {
        ["0.13.1", "0.14.0", "0.14.2"].map(version).to_vec()
    }

    #[test]
    fn a_pin_with_two_numbers_accepts_each_patch_of_that_minor_version() {
        let pin = parse_pin("0.14").unwrap();

        assert!(pin.matches(&version("0.14.0")));
        assert!(pin.matches(&version("0.14.2")));
        assert!(!pin.matches(&version("0.15.0")));
        assert!(!pin.matches(&version("0.13.1")));
    }

    #[test]
    fn a_pin_with_an_equal_sign_accepts_one_version() {
        let pin = parse_pin("=0.14.1").unwrap();

        assert!(pin.matches(&version("0.14.1")));
        assert!(!pin.matches(&version("0.14.2")));
    }

    #[test]
    fn a_pin_that_is_not_a_requirement_gives_an_error() {
        let error = parse_pin("latest").unwrap_err();

        assert!(error.to_string().contains("latest"));
    }

    #[test]
    fn a_project_without_a_pin_uses_the_embedded_engine() {
        assert_eq!(
            choose(None, &version("0.15.1"), &installed()),
            Engine::Embedded
        );
    }

    #[test]
    fn a_pin_that_accepts_the_embedded_version_uses_the_embedded_engine() {
        let pin = parse_pin("0.15").unwrap();

        assert_eq!(
            choose(Some(&pin), &version("0.15.1"), &installed()),
            Engine::Embedded
        );
    }

    #[test]
    fn a_different_pin_uses_the_highest_installed_version_that_agrees() {
        let pin = parse_pin("0.14").unwrap();

        assert_eq!(
            choose(Some(&pin), &version("0.15.1"), &installed()),
            Engine::Managed(version("0.14.2"))
        );
    }

    #[test]
    fn a_pin_without_an_installed_version_is_missing() {
        let pin = parse_pin("=0.14.1").unwrap();

        assert_eq!(
            choose(Some(&pin), &version("0.15.1"), &installed()),
            Engine::Missing(pin)
        );
    }
}
