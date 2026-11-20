pub(crate) use fluent_bundle::FluentArgs;
use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::langid;

const ENGLISH: &str = include_str!("../locales/en-US/presswork-cli.ftl");

pub(crate) struct Localizer {
    bundle: FluentBundle<FluentResource>,
}

impl Localizer {
    pub(crate) fn english() -> Self {
        let resource =
            FluentResource::try_new(ENGLISH.to_owned()).unwrap_or_else(|(partial, _)| partial);
        let mut bundle = FluentBundle::new(vec![langid!("en-US")]);
        bundle.set_use_isolating(false);
        bundle.add_resource_overriding(resource);
        Self { bundle }
    }

    pub(crate) fn text(&self, id: &str) -> String {
        self.message(id, None)
    }

    pub(crate) fn format(&self, id: &str, args: &FluentArgs<'_>) -> String {
        self.message(id, Some(args))
    }

    fn message(&self, id: &str, args: Option<&FluentArgs<'_>>) -> String {
        let Some(pattern) = self
            .bundle
            .get_message(id)
            .and_then(|message| message.value())
        else {
            return id.to_owned();
        };
        let mut errors = Vec::new();
        self.bundle
            .format_pattern(pattern, args, &mut errors)
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_messages_parse_without_errors() {
        assert!(FluentResource::try_new(ENGLISH.to_owned()).is_ok());
    }

    #[test]
    fn each_message_has_english_text() {
        let localizer = Localizer::english();

        for id in [
            "severity-error",
            "severity-warning",
            "hint",
            "project-not-found",
            "project-invalid",
            "project-main-outside",
            "project-main-missing",
            "project-read",
            "pin-invalid",
            "pin-not-built-in",
            "profile-unknown",
            "problem-resolution",
            "problem-alt",
            "export-outside",
            "export-failed",
            "export-write-failed",
        ] {
            assert_ne!(localizer.text(id), id);
        }
    }
}
