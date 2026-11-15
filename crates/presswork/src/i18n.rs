pub(crate) use fluent_bundle::FluentArgs;
use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::langid;

const ENGLISH: &str = include_str!("../locales/en-US/presswork.ftl");

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
    fn text_gives_the_english_message() {
        assert_eq!(Localizer::english().text("app-title"), "Presswork");
    }

    #[test]
    fn format_puts_the_arguments_into_the_message() {
        let mut args = FluentArgs::new();
        args.set("line", 3);
        args.set("column", 14);

        assert_eq!(
            Localizer::english().format("status-position", &args),
            "Line 3, column 14"
        );
    }

    #[test]
    fn text_gives_the_id_of_an_unknown_message() {
        assert_eq!(
            Localizer::english().text("no-such-message"),
            "no-such-message"
        );
    }
}
