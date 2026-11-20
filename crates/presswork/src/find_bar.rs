use std::ops::Range;

use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, text, text_input, tooltip};
use iced::{Element, Fill};
use presswork_editor::Editor;
use presswork_text::{Query, Search};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, size, space as gap};

use crate::chrome;
use crate::i18n::{FluentArgs, Localizer};

pub(crate) const FIND_INPUT: &str = "find-input";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Message {
    Query(String),
    Replacement(String),
    MatchCase,
    WholeWord,
    Pattern,
    ToggleReplace,
    Submit,
    Next,
    Previous,
    Replace,
    ReplaceAll,
    Close,
}

#[derive(Debug, Default)]
pub(crate) struct FindBar {
    open: bool,
    replacing: bool,
    query: Query,
    replacement: String,
    search: Option<Search>,
    invalid: bool,
    matches: Vec<Range<usize>>,
    searched: Option<(u64, u64)>,
}

impl FindBar {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn open(&mut self, replacing: bool, selected: Option<String>) {
        self.open = true;
        self.replacing = replacing;
        if let Some(text) = selected {
            self.query.text = text;
            self.compile();
        }
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.matches.clear();
        self.searched = None;
    }

    pub(crate) fn update(&mut self, message: Message) {
        match message {
            Message::Query(text) => {
                self.query.text = text;
                self.compile();
            }
            Message::Replacement(text) => self.replacement = text,
            Message::MatchCase => {
                self.query.match_case = !self.query.match_case;
                self.compile();
            }
            Message::WholeWord => {
                self.query.whole_word = !self.query.whole_word;
                self.compile();
            }
            Message::Pattern => {
                self.query.pattern = !self.query.pattern;
                self.compile();
            }
            Message::ToggleReplace => self.replacing = !self.replacing,
            _ => {}
        }
    }

    fn compile(&mut self) {
        (self.search, self.invalid) = match Search::new(&self.query) {
            Ok(search) => (search, false),
            Err(_) => (None, true),
        };
        self.searched = None;
    }

    pub(crate) fn refresh(&mut self, editor: &Editor) {
        if !self.open {
            return;
        }
        let key = (editor.id(), editor.revision());
        if self.searched == Some(key) {
            return;
        }
        self.searched = Some(key);
        self.matches = match &self.search {
            Some(search) => search.find(&editor.document().buffer().to_string()),
            None => Vec::new(),
        };
    }

    pub(crate) fn matches(&self) -> &[Range<usize>] {
        &self.matches
    }

    pub(crate) fn next(&self, from: usize) -> Option<Range<usize>> {
        self.matches
            .iter()
            .find(|found| found.start >= from)
            .or_else(|| self.matches.first())
            .cloned()
    }

    pub(crate) fn previous(&self, before: usize) -> Option<Range<usize>> {
        self.matches
            .iter()
            .rev()
            .find(|found| found.end <= before)
            .or_else(|| self.matches.last())
            .cloned()
    }

    pub(crate) fn index_of(&self, selected: &Range<usize>) -> Option<usize> {
        self.matches
            .binary_search_by(|found| found.start.cmp(&selected.start))
            .ok()
            .filter(|index| self.matches[*index] == *selected)
    }

    pub(crate) fn replacement_for(&self, text: &str, found: Range<usize>) -> String {
        match &self.search {
            Some(search) => search.replacement(text, found, &self.replacement),
            None => self.replacement.clone(),
        }
    }

    pub(crate) fn replacements(&self, text: &str) -> Vec<(Range<usize>, String)> {
        self.matches
            .iter()
            .map(|found| (found.clone(), self.replacement_for(text, found.clone())))
            .collect()
    }

    pub(crate) fn view(
        &self,
        localizer: &Localizer,
        selected: &Range<usize>,
    ) -> Element<'_, Message> {
        let find = text_input(&localizer.text("find-placeholder"), &self.query.text)
            .id(FIND_INPUT)
            .on_input(Message::Query)
            .on_submit(Message::Submit)
            .size(font_size::SMALL)
            .padding([gap::XS, gap::S])
            .style(style::field);
        let find_row = row![
            find,
            toggle(
                Icon::CaseSensitive,
                self.query.match_case,
                Message::MatchCase,
                localizer.text("find-match-case")
            ),
            toggle(
                Icon::WholeWord,
                self.query.whole_word,
                Message::WholeWord,
                localizer.text("find-whole-word")
            ),
            toggle(
                Icon::Regex,
                self.query.pattern,
                Message::Pattern,
                localizer.text("find-pattern")
            ),
            self.count(localizer, selected),
            action(
                Icon::ArrowUp,
                Message::Previous,
                localizer.text("find-previous")
            ),
            action(Icon::ArrowDown, Message::Next, localizer.text("find-next")),
            action(Icon::X, Message::Close, localizer.text("find-close")),
        ]
        .spacing(gap::XS)
        .align_y(Vertical::Center);
        let mut rows = column![find_row].spacing(gap::XS);
        if self.replacing {
            let found = !self.matches.is_empty();
            let replace = text_input(&localizer.text("replace-placeholder"), &self.replacement)
                .on_input(Message::Replacement)
                .on_submit(Message::Replace)
                .size(font_size::SMALL)
                .padding([gap::XS, gap::S])
                .style(style::field);
            rows = rows.push(
                row![
                    replace,
                    text_button(localizer.text("replace-one"), Message::Replace, found),
                    text_button(localizer.text("replace-all"), Message::ReplaceAll, found),
                ]
                .spacing(gap::XS)
                .align_y(Vertical::Center),
            );
        }
        let chevron = if self.replacing {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        };
        container(
            row![
                action(
                    chevron,
                    Message::ToggleReplace,
                    localizer.text("find-toggle-replace")
                ),
                rows
            ]
            .spacing(gap::XS),
        )
        .padding([gap::XS, gap::S])
        .width(Fill)
        .style(style::chrome)
        .into()
    }

    fn count(&self, localizer: &Localizer, selected: &Range<usize>) -> Element<'_, Message> {
        if self.invalid {
            return text(localizer.text("find-invalid"))
                .size(font_size::SMALL)
                .style(style::tone(Tone::Danger))
                .into();
        }
        let words = if self.query.text.is_empty() {
            String::new()
        } else {
            let mut args = FluentArgs::new();
            args.set("total", self.matches.len());
            match (self.index_of(selected), self.matches.len()) {
                (_, 0) => localizer.text("find-none"),
                (Some(index), _) => {
                    args.set("current", index + 1);
                    localizer.format("find-count", &args)
                }
                (None, _) => localizer.format("find-total", &args),
            }
        };
        text(words)
            .font(font::MONO)
            .size(font_size::SMALL)
            .style(style::muted)
            .into()
    }
}

fn toggle<'a>(symbol: Icon, on: bool, message: Message, words: String) -> Element<'a, Message> {
    chrome::tip(
        button(icon(symbol).size(size::ICON_SMALL))
            .padding(gap::XS)
            .style(style::tab(on))
            .on_press(message),
        words,
        tooltip::Position::Bottom,
    )
}

fn action<'a>(symbol: Icon, message: Message, words: String) -> Element<'a, Message> {
    chrome::tip(
        button(icon(symbol).size(size::ICON_SMALL))
            .padding(gap::XS)
            .style(style::ghost)
            .on_press(message),
        words,
        tooltip::Position::Bottom,
    )
}

fn text_button<'a>(words: String, message: Message, enabled: bool) -> Element<'a, Message> {
    button(text(words).size(font_size::SMALL))
        .padding([gap::XXS, gap::S])
        .style(style::secondary)
        .on_press_maybe(enabled.then_some(message))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    const TEXT: &str = "a cat and a cat and a cat";

    fn bar(query: &str) -> FindBar {
        let mut bar = FindBar::default();
        bar.open(false, Some(query.to_owned()));
        bar.refresh(&Editor::new(TEXT));
        bar
    }

    fn spans(bar: &FindBar) -> Vec<(usize, usize)> {
        bar.matches()
            .iter()
            .map(|found| (found.start, found.end))
            .collect()
    }

    #[test]
    fn opening_with_a_selection_finds_it() {
        assert_eq!(spans(&bar("cat")), [(2, 5), (12, 15), (22, 25)]);
    }

    #[test]
    fn next_goes_to_the_first_match_after_the_place_and_wraps() {
        let bar = bar("cat");

        assert_eq!(bar.next(0), Some(2..5));
        assert_eq!(bar.next(5), Some(12..15));
        assert_eq!(bar.next(25), Some(2..5));
    }

    #[test]
    fn previous_goes_to_the_last_match_before_the_place_and_wraps() {
        let bar = bar("cat");

        assert_eq!(bar.previous(25), Some(22..25));
        assert_eq!(bar.previous(12), Some(2..5));
        assert_eq!(bar.previous(2), Some(22..25));
    }

    #[test]
    fn only_an_exact_match_has_an_index() {
        let bar = bar("cat");

        assert_eq!(bar.index_of(&(12..15)), Some(1));
        assert_eq!(bar.index_of(&(12..14)), None);
    }

    #[test]
    fn an_invalid_pattern_finds_nothing() {
        let mut bar = bar("cat");
        bar.update(Message::Pattern);
        bar.update(Message::Query("(".to_owned()));
        bar.refresh(&Editor::new(TEXT));

        assert_eq!(spans(&bar), Vec::<(usize, usize)>::new());
        assert!(bar.invalid);
    }

    #[test]
    fn the_search_runs_again_after_a_change_of_the_text() {
        let mut bar = bar("cat");

        bar.refresh(&Editor::new("one cat"));

        assert_eq!(spans(&bar), [(4, 7)]);
    }

    #[test]
    fn replacements_expand_the_groups_of_a_pattern() {
        let mut bar = FindBar::default();
        bar.open(true, None);
        bar.update(Message::Pattern);
        bar.update(Message::Query(r"(\w+) cat".to_owned()));
        bar.update(Message::Replacement("$1 dog".to_owned()));
        bar.refresh(&Editor::new(TEXT));

        assert_eq!(
            bar.replacements(TEXT),
            [
                (0..5, "a dog".to_owned()),
                (10..15, "a dog".to_owned()),
                (20..25, "a dog".to_owned()),
            ]
        );
    }

    #[test]
    fn closing_forgets_the_matches() {
        let mut bar = bar("cat");

        bar.close();

        assert!(!bar.is_open());
        assert_eq!(spans(&bar), Vec::<(usize, usize)>::new());
    }

    #[test]
    fn view_shows_the_place_of_the_selected_match() {
        let bar = bar("cat");
        let localizer = Localizer::english();

        let mut ui = simulator(bar.view(&localizer, &(12..15)));

        assert!(ui.find("2 of 3").is_ok());
    }

    #[test]
    fn view_shows_the_total_without_a_selected_match() {
        let bar = bar("cat");
        let localizer = Localizer::english();

        let mut ui = simulator(bar.view(&localizer, &(0..0)));

        assert!(ui.find("3 results").is_ok());
    }

    #[test]
    fn the_replace_row_shows_only_when_asked() {
        let localizer = Localizer::english();
        let mut bar = bar("cat");
        assert!(
            simulator(bar.view(&localizer, &(0..0)))
                .find("Replace all")
                .is_err()
        );

        bar.update(Message::ToggleReplace);

        assert!(
            simulator(bar.view(&localizer, &(0..0)))
                .find("Replace all")
                .is_ok()
        );
    }

    #[test]
    fn each_text_of_the_bar_has_english_text() {
        let localizer = Localizer::english();

        for id in [
            "find-placeholder",
            "replace-placeholder",
            "find-none",
            "find-invalid",
            "find-match-case",
            "find-whole-word",
            "find-pattern",
            "find-previous",
            "find-next",
            "find-close",
            "find-toggle-replace",
            "replace-one",
            "replace-all",
        ] {
            assert_ne!(localizer.text(id), id);
        }
    }
}
