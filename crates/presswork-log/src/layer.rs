use std::fmt;
use std::io::Write;

use jiff::Timestamp;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_appender::non_blocking::NonBlocking;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

use crate::{Level, LogEntry, LogStore, to_json_line};

pub type Clock = fn() -> Timestamp;

const REMOVED: &str = "[removed]";

const SECRET_NAMES: [&str; 12] = [
    "authorization",
    "body",
    "content",
    "cookie",
    "credential",
    "credentials",
    "key",
    "passphrase",
    "password",
    "secret",
    "text",
    "token",
];

#[derive(Debug, Clone)]
pub struct LogLayer {
    store: LogStore,
    clock: Clock,
    file: Option<NonBlocking>,
}

impl LogLayer {
    #[must_use]
    pub fn new(store: LogStore) -> Self {
        Self::with_clock(store, Timestamp::now)
    }

    #[must_use]
    pub fn with_clock(store: LogStore, clock: Clock) -> Self {
        Self {
            store,
            clock,
            file: None,
        }
    }

    #[must_use]
    pub fn with_file(self, file: NonBlocking) -> Self {
        Self {
            file: Some(file),
            ..self
        }
    }
}

impl<S> Layer<S> for LogLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &Event<'_>, context: Context<'_, S>) {
        let mut collector = FieldCollector::default();
        event.record(&mut collector);
        let spans = context.event_scope(event).map_or_default(|scope| {
            scope
                .from_root()
                .map(|span| span.name().to_owned())
                .collect()
        });
        let metadata = event.metadata();
        let entry = LogEntry {
            sequence: 0,
            time: (self.clock)(),
            level: Level::from(*metadata.level()),
            target: metadata.target().to_owned(),
            message: collector.message,
            fields: collector.fields,
            spans,
        };
        if let Some(file) = &self.file {
            write_line(file.clone(), &entry);
        }
        self.store.push(entry);
    }
}

fn write_line(mut file: NonBlocking, entry: &LogEntry) {
    if let Ok(mut line) = to_json_line(entry) {
        line.push('\n');
        let _ = file.write_all(line.as_bytes());
    }
}

#[derive(Default)]
struct FieldCollector {
    message: String,
    fields: Vec<(String, String)>,
}

impl FieldCollector {
    fn record(&mut self, field: &Field, value: String) {
        let name = field.name();
        if name == "message" {
            self.message = value;
        } else if is_secret(name) {
            self.fields.push((name.to_owned(), REMOVED.to_owned()));
        } else {
            self.fields.push((name.to_owned(), value));
        }
    }
}

impl Visit for FieldCollector {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record(field, value.to_owned());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record(field, format!("{value:?}"));
    }
}

fn is_secret(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let last_word = name.rsplit('_').next().unwrap_or(&name);
    SECRET_NAMES.contains(&last_word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    fn fixed_time() -> Timestamp {
        Timestamp::UNIX_EPOCH
    }

    fn capture(log: impl FnOnce()) -> Vec<LogEntry> {
        let store = LogStore::with_capacity(100);
        let subscriber =
            tracing_subscriber::registry().with(LogLayer::with_clock(store.clone(), fixed_time));
        tracing::subscriber::with_default(subscriber, log);
        store.after(0)
    }

    fn pair(name: &str, value: &str) -> (String, String) {
        (name.to_owned(), value.to_owned())
    }

    #[test]
    fn captures_the_level_the_target_the_message_and_the_fields() {
        let entries = capture(|| tracing::warn!(project = "book", pages = 12, "compiled slowly"));

        let entry = &entries[0];
        assert_eq!(entry.level, Level::Warn);
        assert_eq!(entry.target, module_path!());
        assert_eq!(entry.message, "compiled slowly");
        assert_eq!(entry.fields, [pair("project", "book"), pair("pages", "12")]);
        assert_eq!(entry.time, fixed_time());
    }

    #[test]
    fn captures_the_names_of_the_open_spans() {
        let entries = capture(|| {
            let _export = tracing::info_span!("export").entered();
            let _compile = tracing::info_span!("compile").entered();
            tracing::info!("done");
        });

        assert_eq!(entries[0].spans, ["export", "compile"]);
    }

    #[test]
    fn removes_the_values_of_secret_fields() {
        let entries = capture(|| {
            tracing::info!(
                access_token = "abc",
                api_key = "def",
                document_text = "ghi",
                monkey = "visible",
                "signed in"
            );
        });

        let values: Vec<&str> = entries[0]
            .fields
            .iter()
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(values, ["[removed]", "[removed]", "[removed]", "visible"]);
    }
}
