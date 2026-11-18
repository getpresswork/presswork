use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use serde::Deserialize;

const LISTEN_VARIABLE: &str = "PRESSWORK_LISTEN";
const DATABASE_VARIABLE: &str = "PRESSWORK_DATABASE_URL";
const DATA_VARIABLE: &str = "PRESSWORK_DATA";
const TRUST_FORWARDED_VARIABLE: &str = "PRESSWORK_TRUST_FORWARDED";
const OCRMYPDF_VARIABLE: &str = "PRESSWORK_OCRMYPDF";
const OCR_LANGUAGES_VARIABLE: &str = "PRESSWORK_OCR_LANGUAGES";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_DATA: &str = "data";
const DEFAULT_OCRMYPDF: &str = "ocrmypdf";
const DEFAULT_OCR_LANGUAGES: &str = "eng";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("the configuration file is not valid: {0}")]
    File(String),
    #[error("the environment variable {name} does not contain a valid address: {value}")]
    Address { name: &'static str, value: String },
    #[error("the environment variable {name} must be true or false, not {value}")]
    Flag { name: &'static str, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub listen: SocketAddr,
    pub database_url: Option<String>,
    pub data: PathBuf,
    pub trust_forwarded: bool,
    pub ocrmypdf: String,
    pub ocr_languages: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from((Ipv4Addr::LOCALHOST, DEFAULT_PORT)),
            database_url: None,
            data: PathBuf::from(DEFAULT_DATA),
            trust_forwarded: false,
            ocrmypdf: DEFAULT_OCRMYPDF.to_owned(),
            ocr_languages: DEFAULT_OCR_LANGUAGES.to_owned(),
        }
    }
}

impl Config {
    pub fn parse(
        text: &str,
        variable: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        let mut config: Self =
            toml::from_str(text).map_err(|error| ConfigError::File(error.to_string()))?;
        if let Some(value) = variable(LISTEN_VARIABLE) {
            config.listen = value.parse().map_err(|_| ConfigError::Address {
                name: LISTEN_VARIABLE,
                value,
            })?;
        }
        if let Some(url) = variable(DATABASE_VARIABLE) {
            config.database_url = Some(url);
        }
        if let Some(data) = variable(DATA_VARIABLE) {
            config.data = PathBuf::from(data);
        }
        if let Some(value) = variable(TRUST_FORWARDED_VARIABLE) {
            config.trust_forwarded = value.parse().map_err(|_| ConfigError::Flag {
                name: TRUST_FORWARDED_VARIABLE,
                value,
            })?;
        }
        if let Some(program) = variable(OCRMYPDF_VARIABLE) {
            config.ocrmypdf = program;
        }
        if let Some(languages) = variable(OCR_LANGUAGES_VARIABLE) {
            config.ocr_languages = languages;
        }
        Ok(config)
    }

    #[must_use]
    pub fn ocr_program(&self) -> Option<&str> {
        Some(self.ocrmypdf.trim()).filter(|program| !program.is_empty())
    }

    #[must_use]
    pub fn fallback_languages(&self) -> Vec<String> {
        self.ocr_languages
            .split('+')
            .map(str::trim)
            .filter(|language| !language.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn an_empty_file_gives_the_defaults() {
        let config = Config::parse("", none).unwrap();

        assert_eq!(config.listen, "127.0.0.1:8080".parse().unwrap());
        assert_eq!(config.database_url, None);
        assert_eq!(config.data, PathBuf::from("data"));
    }

    #[test]
    fn the_file_and_the_environment_set_the_data_folder() {
        let config = Config::parse("data = \"/srv/presswork\"\n", none).unwrap();
        assert_eq!(config.data, PathBuf::from("/srv/presswork"));

        let config = Config::parse("data = \"/srv/presswork\"\n", |name| {
            (name == "PRESSWORK_DATA").then(|| "/var/lib/presswork".to_owned())
        })
        .unwrap();
        assert_eq!(config.data, PathBuf::from("/var/lib/presswork"));
    }

    #[test]
    fn the_file_sets_the_listen_address_and_the_database() {
        let config = Config::parse(
            "listen = \"0.0.0.0:9000\"\ndatabase_url = \"postgres://db/presswork\"\n",
            none,
        )
        .unwrap();

        assert_eq!(config.listen, "0.0.0.0:9000".parse().unwrap());
        assert_eq!(
            config.database_url.as_deref(),
            Some("postgres://db/presswork")
        );
    }

    #[test]
    fn environment_variables_replace_the_file() {
        let variables = |name: &str| match name {
            "PRESSWORK_LISTEN" => Some("127.0.0.1:7000".to_owned()),
            "PRESSWORK_DATABASE_URL" => Some("postgres://other/presswork".to_owned()),
            _ => None,
        };

        let config = Config::parse("listen = \"0.0.0.0:9000\"\n", variables).unwrap();

        assert_eq!(config.listen, "127.0.0.1:7000".parse().unwrap());
        assert_eq!(
            config.database_url.as_deref(),
            Some("postgres://other/presswork")
        );
    }

    #[test]
    fn the_file_and_the_environment_set_trust_forwarded() {
        let trust = |value: &'static str| {
            move |name: &str| (name == "PRESSWORK_TRUST_FORWARDED").then(|| value.to_owned())
        };

        assert!(!Config::parse("", none).unwrap().trust_forwarded);
        assert!(
            Config::parse("trust_forwarded = true\n", none)
                .unwrap()
                .trust_forwarded
        );
        assert!(Config::parse("", trust("true")).unwrap().trust_forwarded);
        assert!(
            !Config::parse("trust_forwarded = true\n", trust("false"))
                .unwrap()
                .trust_forwarded
        );
        let error = Config::parse("", trust("yes")).unwrap_err();
        assert!(error.to_string().contains("PRESSWORK_TRUST_FORWARDED"));
    }

    #[test]
    fn the_file_and_the_environment_set_the_ocr_engine() {
        let defaults = Config::parse("", none).unwrap();
        assert_eq!(defaults.ocr_program(), Some("ocrmypdf"));
        assert_eq!(defaults.fallback_languages(), ["eng"]);

        let file = "ocrmypdf = \"/opt/ocrmypdf/bin/ocrmypdf\"\nocr_languages = \"eng + deu\"\n";
        let config = Config::parse(file, none).unwrap();
        assert_eq!(config.ocr_program(), Some("/opt/ocrmypdf/bin/ocrmypdf"));
        assert_eq!(config.fallback_languages(), ["eng", "deu"]);

        let config = Config::parse(file, |name| match name {
            "PRESSWORK_OCRMYPDF" => Some(" ".to_owned()),
            "PRESSWORK_OCR_LANGUAGES" => Some("lat".to_owned()),
            _ => None,
        })
        .unwrap();
        assert_eq!(config.ocr_program(), None);
        assert_eq!(config.fallback_languages(), ["lat"]);
    }

    #[test]
    fn the_example_settings_file_is_valid() {
        let example = include_str!("../../../packaging/server/server.toml");

        let config = Config::parse(example, none).unwrap();

        assert_eq!(config.data, PathBuf::from("/var/lib/presswork"));
        assert!(config.database_url.is_some());
        assert_eq!(config.ocr_program(), Some("ocrmypdf"));
    }

    #[test]
    fn an_unknown_key_is_an_error() {
        let error = Config::parse("databse_url = \"postgres://db\"\n", none).unwrap_err();

        assert!(error.to_string().contains("databse_url"));
    }

    #[test]
    fn a_wrong_address_in_the_environment_names_the_variable() {
        let error = Config::parse("", |name| {
            (name == "PRESSWORK_LISTEN").then(|| "nowhere".to_owned())
        })
        .unwrap_err();

        assert!(error.to_string().contains("PRESSWORK_LISTEN"));
    }
}
