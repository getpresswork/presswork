use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Fill, padding};
use presswork_typst::{Heading, SourceRange};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::{font, font_size, size, space as gap, style};

use crate::i18n::Localizer;

const MAX_INDENT_LEVEL: u16 = 6;
pub(crate) const TOGGLE: &str = "outline-toggle";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Message {
    Toggle,
    Select(usize),
}

#[derive(Debug)]
pub(crate) struct OutlinePanel {
    open: bool,
    headings: Vec<Heading>,
}

impl Default for OutlinePanel {
    fn default() -> Self {
        Self {
            open: true,
            headings: Vec::new(),
        }
    }
}

impl OutlinePanel {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub(crate) fn set_headings(&mut self, headings: Vec<Heading>) {
        self.headings = headings;
    }

    pub(crate) fn heading(&self, index: usize) -> Option<&Heading> {
        self.headings.get(index)
    }

    pub(crate) fn current(&self, cursor: usize) -> Option<usize> {
        self.headings
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, heading)| match &heading.source {
                Some(SourceRange::Main(range)) if range.start <= cursor => Some(index),
                _ => None,
            })
    }

    pub(crate) fn toggle_button(&self, localizer: &Localizer) -> Element<'_, Message> {
        let content = row![
            icon(Icon::TableOfContents).size(size::ICON_SMALL),
            text(localizer.text("outline-toggle")).size(font_size::SMALL)
        ]
        .spacing(gap::XS)
        .align_y(Vertical::Center);
        container(
            button(content)
                .padding([gap::XXS, gap::S])
                .style(style::tab(self.open))
                .on_press(Message::Toggle),
        )
        .id(TOGGLE)
        .into()
    }

    pub(crate) fn view(
        &self,
        localizer: &Localizer,
        cursor: Option<usize>,
        note: Option<String>,
    ) -> Element<'_, Message> {
        let note = note.or_else(|| {
            self.headings
                .is_empty()
                .then(|| localizer.text("outline-empty"))
        });
        let body: Element<'_, Message> = if let Some(note) = note {
            text(note).size(font_size::SMALL).style(style::muted).into()
        } else {
            let current = cursor.and_then(|cursor| self.current(cursor));
            let rows = self
                .headings
                .iter()
                .enumerate()
                .map(|(index, heading)| heading_row(index, heading, current == Some(index)));
            scrollable(column(rows).spacing(gap::XXS))
                .height(Fill)
                .into()
        };
        container(
            column![
                text(localizer.text("outline-title")).font(font::UI_SEMIBOLD),
                body
            ]
            .spacing(gap::S),
        )
        .padding([gap::S, gap::S])
        .width(size::PANEL)
        .height(Fill)
        .style(style::chrome)
        .into()
    }
}

fn heading_row(index: usize, heading: &Heading, current: bool) -> Element<'_, Message> {
    let title = match &heading.number {
        Some(number) => format!("{number} {}", heading.title),
        None => heading.title.clone(),
    };
    let depth = u16::try_from(heading.level.saturating_sub(1))
        .unwrap_or(MAX_INDENT_LEVEL)
        .min(MAX_INDENT_LEVEL);
    let content = row![
        container(
            text(title)
                .size(font_size::BODY)
                .wrapping(text::Wrapping::None)
        )
        .width(Fill)
        .clip(true),
        text(heading.page.clone().unwrap_or_default())
            .font(font::MONO)
            .size(font_size::SMALL)
            .style(style::muted)
    ]
    .spacing(gap::S)
    .align_y(Vertical::Center);
    button(content)
        .padding(
            padding::left(gap::S + f32::from(depth) * gap::M)
                .right(gap::S)
                .top(gap::XS)
                .bottom(gap::XS),
        )
        .width(Fill)
        .style(style::list_item(current))
        .on_press(Message::Select(index))
        .into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use iced_test::simulator;

    fn heading(title: &str, source: Option<SourceRange>) -> Heading {
        Heading {
            level: 1,
            number: None,
            title: title.to_owned(),
            source,
            point: None,
            page: None,
        }
    }

    fn panel() -> OutlinePanel {
        let mut panel = OutlinePanel::default();
        panel.set_headings(vec![
            heading("One", Some(SourceRange::Main(0..5))),
            heading(
                "Part",
                Some(SourceRange::File(PathBuf::from("part.typ"), 0..6)),
            ),
            heading("Two", Some(SourceRange::Main(20..25))),
        ]);
        panel
    }

    #[test]
    fn starts_open() {
        assert!(OutlinePanel::default().is_open());
    }

    #[test]
    fn toggle_closes_and_opens_the_panel() {
        let mut panel = OutlinePanel::default();

        panel.toggle();
        assert!(!panel.is_open());

        panel.toggle();
        assert!(panel.is_open());
    }

    #[test]
    fn the_current_heading_is_the_last_heading_before_the_cursor() {
        let panel = panel();

        assert_eq!(panel.current(3), Some(0));
        assert_eq!(panel.current(19), Some(0));
        assert_eq!(panel.current(20), Some(2));
    }

    #[test]
    fn text_before_the_first_heading_has_no_current_heading() {
        let mut panel = OutlinePanel::default();
        panel.set_headings(vec![heading("One", Some(SourceRange::Main(10..15)))]);

        assert_eq!(panel.current(4), None);
    }

    #[test]
    fn view_shows_the_number_the_title_and_the_page_of_each_heading() {
        let mut panel = OutlinePanel::default();
        panel.set_headings(vec![Heading {
            number: Some("1.2".to_owned()),
            page: Some("iv".to_owned()),
            ..heading("Methods", Some(SourceRange::Main(0..10)))
        }]);
        let localizer = Localizer::english();

        let mut ui = simulator(panel.view(&localizer, Some(0), None));

        assert!(ui.find("1.2 Methods").is_ok());
        assert!(ui.find("iv").is_ok());
    }

    #[test]
    fn a_click_on_a_heading_selects_it() {
        let panel = panel();
        let localizer = Localizer::english();
        let mut ui = simulator(panel.view(&localizer, Some(0), None));

        ui.click("Two").unwrap();

        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Message::Select(2)]);
    }

    #[test]
    fn view_tells_when_the_document_has_no_headings() {
        let panel = OutlinePanel::default();
        let localizer = Localizer::english();

        let mut ui = simulator(panel.view(&localizer, Some(0), None));

        assert!(ui.find("The document has no headings.").is_ok());
    }

    #[test]
    fn a_note_replaces_the_headings() {
        let panel = panel();
        let localizer = Localizer::english();

        let mut ui = simulator(panel.view(&localizer, Some(0), Some("A note.".to_owned())));

        assert!(ui.find("A note.").is_ok());
        assert!(ui.find("One").is_err());
    }

    #[test]
    fn each_text_of_the_panel_has_english_text() {
        let localizer = Localizer::english();

        for id in [
            "outline-title",
            "outline-toggle",
            "outline-empty",
            "outline-needs-engine",
        ] {
            assert_ne!(localizer.text(id), id);
        }
    }
}
