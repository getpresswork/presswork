use std::ops::Range;

use typst_syntax::{LinkedNode, Source, Tag};

use crate::editor::Editor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Comment,
    Keyword,
    Function,
    String,
    Number,
    Operator,
    Reference,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Style {
    pub role: Option<Role>,
    pub bold: bool,
    pub italic: bool,
}

impl Style {
    fn apply(&mut self, tag: Tag) {
        let role = match tag {
            Tag::Strong | Tag::Heading | Tag::ListTerm => {
                self.bold = true;
                None
            }
            Tag::Emph => {
                self.italic = true;
                None
            }
            Tag::Comment => {
                self.italic = true;
                Some(Role::Comment)
            }
            Tag::Keyword | Tag::ListMarker => Some(Role::Keyword),
            Tag::Function => Some(Role::Function),
            Tag::String | Tag::Raw => Some(Role::String),
            Tag::Number | Tag::MathDelimiter => Some(Role::Number),
            Tag::Punctuation | Tag::Operator | Tag::MathOperator | Tag::MathGroupingParens => {
                Some(Role::Operator)
            }
            Tag::Escape | Tag::Link | Tag::Label | Tag::Ref | Tag::Interpolated => {
                Some(Role::Reference)
            }
            Tag::Error => Some(Role::Error),
        };
        if role.is_some() {
            self.role = role;
        }
    }
}

#[derive(Debug)]
pub struct Highlighter {
    source: Source,
    version: Option<(u64, u64)>,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self {
            source: Source::detached(""),
            version: None,
        }
    }
}

impl Highlighter {
    pub fn sync(&mut self, editor: &Editor) {
        let version = Some((editor.id(), editor.revision()));
        if self.version != version {
            self.source.replace(&editor.document().buffer().to_string());
            self.version = version;
        }
    }

    pub(crate) fn source(&self) -> &Source {
        &self.source
    }

    #[must_use]
    pub fn spans(&self, range: Range<usize>) -> Vec<(Range<usize>, Style)> {
        let mut spans = Vec::new();
        collect(
            &LinkedNode::new(self.source.root()),
            &range,
            Style::default(),
            &mut spans,
        );
        spans
    }
}

fn collect(
    node: &LinkedNode<'_>,
    range: &Range<usize>,
    inherited: Style,
    spans: &mut Vec<(Range<usize>, Style)>,
) {
    let node_range = node.range();
    if node_range.end <= range.start || node_range.start >= range.end {
        return;
    }
    let mut style = inherited;
    if let Some(tag) = typst_syntax::highlight(node) {
        style.apply(tag);
    }
    let mut children = node.children().peekable();
    if children.peek().is_some() {
        for child in children {
            collect(&child, range, style, spans);
        }
        return;
    }
    let start = node_range.start.max(range.start);
    let end = node_range.end.min(range.end);
    match spans.last_mut() {
        Some((last, last_style)) if *last_style == style && last.end == start => last.end = end,
        _ => spans.push((start..end, style)),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::action::Action;

    fn highlighter(text: &str) -> Highlighter {
        let mut highlighter = Highlighter::default();
        highlighter.sync(&Editor::new(text));
        highlighter
    }

    fn style_at(text: &str, needle: &str) -> Style {
        let offset = text.find(needle).unwrap();
        highlighter(text)
            .spans(0..text.len())
            .into_iter()
            .find(|(range, _)| range.contains(&offset))
            .map(|(_, style)| style)
            .unwrap()
    }

    fn role_at(text: &str, needle: &str) -> Option<Role> {
        style_at(text, needle).role
    }

    #[test]
    fn code_gets_the_roles_of_its_parts() {
        let text = "#let size = text(1em, \"a\")";

        assert_eq!(role_at(text, "let"), Some(Role::Keyword));
        assert_eq!(role_at(text, "text"), Some(Role::Function));
        assert_eq!(role_at(text, "1em"), Some(Role::Number));
        assert_eq!(role_at(text, "\"a\""), Some(Role::String));
        assert_eq!(role_at(text, "("), Some(Role::Operator));
    }

    #[test]
    fn comments_are_quiet_and_italic() {
        let style = style_at("Text // note", "note");

        assert_eq!(style.role, Some(Role::Comment));
        assert!(style.italic);
    }

    #[test]
    fn markup_gets_bold_and_italic_styles() {
        assert!(style_at("*strong*", "strong").bold);
        assert!(style_at("_emphasis_", "emphasis").italic);
        assert!(style_at("= Title", "Title").bold);

        let both = style_at("*a _b_*", "b_");
        assert!(both.bold && both.italic);
    }

    #[test]
    fn labels_and_references_get_the_reference_role() {
        assert_eq!(role_at("See @intro.", "@intro"), Some(Role::Reference));
        assert_eq!(role_at("= Intro <intro>", "<intro>"), Some(Role::Reference));
    }

    #[test]
    fn plain_text_has_no_style() {
        assert_eq!(style_at("Plain words.", "words"), Style::default());
    }

    #[test]
    fn spans_cover_exactly_the_requested_range() {
        let text = "= A\n*b* c";
        let spans = highlighter(text).spans(4..9);

        assert_eq!(spans.first().map(|(range, _)| range.start), Some(4));
        assert_eq!(spans.last().map(|(range, _)| range.end), Some(9));
        for pair in spans.windows(2) {
            assert_eq!(pair[0].0.end, pair[1].0.start);
        }
    }

    #[test]
    fn the_highlighter_follows_the_editor() {
        let mut editor = Editor::new("#let x = ");
        let mut highlighter = Highlighter::default();
        highlighter.sync(&editor);

        editor
            .perform(
                Action::Move {
                    motion: crate::action::Motion::DocumentEnd,
                    extend: false,
                },
                Instant::now(),
            )
            .unwrap();
        editor
            .perform(Action::Insert("2".to_owned()), Instant::now())
            .unwrap();
        highlighter.sync(&editor);

        let spans = highlighter.spans(9..10);
        assert_eq!(
            spans.first().and_then(|(_, style)| style.role),
            Some(Role::Number)
        );
    }
}
