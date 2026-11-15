use jiff::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Level {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl From<tracing::Level> for Level {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE => Self::Trace,
            tracing::Level::DEBUG => Self::Debug,
            tracing::Level::INFO => Self::Info,
            tracing::Level::WARN => Self::Warn,
            _ => Self::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogEntry {
    pub sequence: u64,
    pub time: Timestamp,
    pub level: Level,
    pub target: String,
    pub message: String,
    pub fields: Vec<(String, String)>,
    pub spans: Vec<String>,
}

impl LogEntry {
    #[must_use]
    pub fn area(&self) -> &str {
        area_of(&self.target)
    }
}

#[must_use]
pub fn area_of(target: &str) -> &str {
    let krate = target.split_once("::").map_or(target, |(krate, _)| krate);
    match krate.strip_prefix("presswork_") {
        Some(area) => area,
        None if krate == "presswork" => "app",
        None => krate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_order_by_severity() {
        let levels = [
            Level::Trace,
            Level::Debug,
            Level::Info,
            Level::Warn,
            Level::Error,
        ];

        assert!(levels.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn level_comes_from_a_tracing_level() {
        assert_eq!(Level::from(tracing::Level::WARN), Level::Warn);
        assert_eq!(Level::from(tracing::Level::TRACE), Level::Trace);
    }

    #[test]
    fn area_of_a_presswork_crate_is_its_name_without_the_prefix() {
        assert_eq!(area_of("presswork_typst::world"), "typst");
    }

    #[test]
    fn area_of_the_application_crate_is_app() {
        assert_eq!(area_of("presswork::app"), "app");
        assert_eq!(area_of("presswork"), "app");
    }

    #[test]
    fn area_of_an_external_crate_is_the_crate_name() {
        assert_eq!(area_of("wgpu_core::device"), "wgpu_core");
    }

    #[test]
    fn entry_gives_the_area_of_its_target() {
        let entry = LogEntry {
            target: "presswork_sync::client".to_owned(),
            ..LogEntry::default()
        };

        assert_eq!(entry.area(), "sync");
    }
}
