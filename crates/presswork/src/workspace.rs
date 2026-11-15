#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Workspace {
    #[default]
    Write,
    Typeset,
    Review,
    Plan,
    Publish,
}

impl Workspace {
    pub(crate) const ALL: [Self; 5] = [
        Self::Write,
        Self::Typeset,
        Self::Review,
        Self::Plan,
        Self::Publish,
    ];

    pub(crate) fn text_id(self) -> &'static str {
        match self {
            Self::Write => "workspace-write",
            Self::Typeset => "workspace-typeset",
            Self::Review => "workspace-review",
            Self::Plan => "workspace-plan",
            Self::Publish => "workspace-publish",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Localizer;

    #[test]
    fn write_is_the_default_workspace() {
        assert_eq!(Workspace::default(), Workspace::Write);
    }

    #[test]
    fn each_workspace_has_english_text() {
        let localizer = Localizer::english();

        for workspace in Workspace::ALL {
            assert_ne!(localizer.text(workspace.text_id()), workspace.text_id());
        }
    }
}
