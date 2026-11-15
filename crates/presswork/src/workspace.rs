#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Workspace {
    Plan,
    Research,
    #[default]
    Write,
    Translate,
    Review,
    Typeset,
    Publish,
}

impl Workspace {
    pub(crate) const ALL: [Self; 7] = [
        Self::Plan,
        Self::Research,
        Self::Write,
        Self::Translate,
        Self::Review,
        Self::Typeset,
        Self::Publish,
    ];

    pub(crate) fn text_id(self) -> &'static str {
        match self {
            Self::Plan => "workspace-plan",
            Self::Research => "workspace-research",
            Self::Write => "workspace-write",
            Self::Translate => "workspace-translate",
            Self::Review => "workspace-review",
            Self::Typeset => "workspace-typeset",
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
    fn workspaces_follow_the_sequence_of_a_publication() {
        assert_eq!(
            Workspace::ALL,
            [
                Workspace::Plan,
                Workspace::Research,
                Workspace::Write,
                Workspace::Translate,
                Workspace::Review,
                Workspace::Typeset,
                Workspace::Publish,
            ]
        );
    }

    #[test]
    fn each_workspace_has_english_text() {
        let localizer = Localizer::english();

        for workspace in Workspace::ALL {
            assert_ne!(localizer.text(workspace.text_id()), workspace.text_id());
        }
    }
}
