use serde::{Serialize, Serializer};

use crate::LogEntry;

#[derive(Serialize)]
struct Line<'entry> {
    time: String,
    level: &'static str,
    target: &'entry str,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    spans: &'entry [String],
    message: &'entry str,
    #[serde(
        serialize_with = "fields_as_map",
        skip_serializing_if = "<[(String, String)]>::is_empty"
    )]
    fields: &'entry [(String, String)],
}

fn fields_as_map<S: Serializer>(
    fields: &&[(String, String)],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_map(fields.iter().map(|(name, value)| (name, value)))
}

pub fn to_json_line(entry: &LogEntry) -> Result<String, serde_json::Error> {
    serde_json::to_string(&Line {
        time: entry.time.to_string(),
        level: entry.level.as_str(),
        target: &entry.target,
        spans: &entry.spans,
        message: &entry.message,
        fields: &entry.fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Level;
    use jiff::Timestamp;

    fn entry() -> LogEntry {
        LogEntry {
            sequence: 7,
            time: Timestamp::UNIX_EPOCH,
            level: Level::Warn,
            target: "presswork_typst::world".to_owned(),
            message: "compiled slowly".to_owned(),
            fields: vec![
                ("project".to_owned(), "book".to_owned()),
                ("pages".to_owned(), "12".to_owned()),
            ],
            spans: vec!["export".to_owned()],
        }
    }

    #[test]
    fn writes_one_json_object_with_the_fields_in_their_sequence() {
        let line = to_json_line(&entry()).unwrap();

        assert_eq!(
            line,
            r#"{"time":"1970-01-01T00:00:00Z","level":"WARN","target":"presswork_typst::world","spans":["export"],"message":"compiled slowly","fields":{"project":"book","pages":"12"}}"#
        );
    }

    #[test]
    fn leaves_out_empty_spans_and_fields() {
        let entry = LogEntry {
            spans: Vec::new(),
            fields: Vec::new(),
            ..entry()
        };

        let line = to_json_line(&entry).unwrap();

        assert!(!line.contains("spans"));
        assert!(!line.contains("fields"));
    }
}
