use std::collections::VecDeque;

use iced::alignment::{Horizontal, Vertical};
use iced::futures::StreamExt;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Element, Fill, Subscription};
use jiff::tz::TimeZone;
use presswork_log::{Level, LogEntry, LogFilter, LogStore};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, size, space as gap};

use crate::i18n::Localizer;

const LEVELS: [Level; 5] = [
    Level::Error,
    Level::Warn,
    Level::Info,
    Level::Debug,
    Level::Trace,
];

const SEARCH_WIDTH: f32 = 220.0;
const TIME_WIDTH: f32 = 96.0;
const LEVEL_WIDTH: f32 = 52.0;
const AREA_WIDTH: f32 = 96.0;

#[derive(Debug)]
pub(crate) struct LogPanel {
    store: LogStore,
    entries: VecDeque<LogEntry>,
    last_sequence: u64,
    filter: LogFilter,
    open: bool,
    time_zone: TimeZone,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Toggle,
    Close,
    Changed,
    LevelSelected(Level),
    SearchChanged(String),
}

impl LogPanel {
    pub(crate) fn new(store: LogStore) -> Self {
        let mut panel = Self {
            store,
            entries: VecDeque::new(),
            last_sequence: 0,
            filter: LogFilter::default(),
            open: false,
            time_zone: TimeZone::system(),
        };
        panel.load();
        panel
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.open = !self.open,
            Message::Close => self.open = false,
            Message::Changed => self.load(),
            Message::LevelSelected(level) => self.filter.minimum = level,
            Message::SearchChanged(text) => self.filter.text = text,
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::run_with(self.store.clone(), |store| {
            store.watch().map(|()| Message::Changed)
        })
    }

    pub(crate) fn toggle_button(&self, localizer: &Localizer) -> Element<'_, Message> {
        let content = row![
            icon(Icon::ScrollText).size(size::ICON_SMALL),
            text(localizer.text("logs-toggle")).size(font_size::SMALL)
        ]
        .spacing(gap::XS)
        .align_y(Vertical::Center);
        button(content)
            .padding([gap::XXS, gap::S])
            .style(style::tab(self.open))
            .on_press(Message::Toggle)
            .into()
    }

    pub(crate) fn view(&self, localizer: &Localizer) -> Element<'_, Message> {
        let rows: Vec<Element<'_, Message>> = self
            .visible()
            .map(|entry| entry_row(entry, &self.time_zone))
            .collect();
        let body: Element<'_, Message> = if rows.is_empty() {
            container(
                text(localizer.text("logs-empty"))
                    .size(font_size::SMALL)
                    .style(style::muted),
            )
            .center(Fill)
            .into()
        } else {
            scrollable(column(rows).spacing(gap::XS))
                .anchor_bottom()
                .height(Fill)
                .into()
        };

        container(column![self.header(localizer), columns(localizer), body].spacing(gap::S))
            .padding([gap::S, gap::L])
            .width(Fill)
            .height(size::PANEL)
            .style(style::chrome)
            .into()
    }

    fn header(&self, localizer: &Localizer) -> Element<'_, Message> {
        let levels = row(LEVELS
            .into_iter()
            .map(|level| level_tab(level, self.filter.minimum, localizer)))
        .spacing(gap::XXS);
        let search = text_input(&localizer.text("logs-find"), &self.filter.text)
            .on_input(Message::SearchChanged)
            .icon(text_input::Icon {
                font: font::ICONS,
                code_point: Icon::Search.unicode(),
                size: Some(size::ICON_SMALL.into()),
                spacing: gap::XS,
                side: text_input::Side::Left,
            })
            .size(font_size::SMALL)
            .padding([gap::XS, gap::S])
            .width(SEARCH_WIDTH)
            .style(style::field);
        let close = button(icon(Icon::X))
            .padding(gap::XS)
            .style(style::ghost)
            .on_press(Message::Close);
        row![
            text(localizer.text("logs-title")).font(font::UI_SEMIBOLD),
            space().width(gap::M),
            levels,
            space().width(Fill),
            search,
            close
        ]
        .spacing(gap::S)
        .align_y(Vertical::Center)
        .into()
    }

    fn visible(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries
            .iter()
            .filter(|entry| self.filter.matches(entry))
    }

    fn load(&mut self) {
        let capacity = self.store.capacity();
        for entry in self.store.after(self.last_sequence) {
            self.last_sequence = entry.sequence;
            if self.entries.len() == capacity {
                self.entries.pop_front();
            }
            self.entries.push_back(entry);
        }
    }
}

fn level_tab<'a>(level: Level, minimum: Level, localizer: &Localizer) -> Element<'a, Message> {
    button(text(localizer.text(level_text_id(level))).size(font_size::SMALL))
        .padding([gap::XXS, gap::S])
        .style(style::tab(level == minimum))
        .on_press(Message::LevelSelected(level))
        .into()
}

fn columns<'a>(localizer: &Localizer) -> Element<'a, Message> {
    let heading = |id: &str| {
        text(localizer.text(id))
            .size(font_size::CAPTION)
            .font(font::UI_MEDIUM)
            .style(style::muted)
    };
    row![
        heading("logs-column-time").width(TIME_WIDTH),
        heading("logs-column-level").width(LEVEL_WIDTH),
        heading("logs-column-area").width(AREA_WIDTH),
        heading("logs-column-message"),
    ]
    .spacing(gap::S)
    .into()
}

fn level_text_id(level: Level) -> &'static str {
    match level {
        Level::Error => "logs-level-error",
        Level::Warn => "logs-level-warn",
        Level::Info => "logs-level-info",
        Level::Debug => "logs-level-debug",
        Level::Trace => "logs-level-trace",
    }
}

fn level_tone(level: Level) -> Tone {
    match level {
        Level::Error => Tone::Danger,
        Level::Warn => Tone::Warning,
        Level::Info => Tone::Info,
        Level::Debug | Level::Trace => Tone::Neutral,
    }
}

fn entry_row<'a>(entry: &'a LogEntry, time_zone: &TimeZone) -> Element<'a, Message> {
    let time = entry
        .time
        .to_zoned(time_zone.clone())
        .strftime("%H:%M:%S%.3f")
        .to_string();
    let fields = entry
        .fields
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(" ");
    let badge = container(
        text(entry.level.as_str())
            .font(font::MONO)
            .size(font_size::CAPTION),
    )
    .width(LEVEL_WIDTH)
    .align_x(Horizontal::Center)
    .style(style::badge(level_tone(entry.level)));
    row![
        text(time)
            .font(font::MONO)
            .size(font_size::SMALL)
            .style(style::muted)
            .width(TIME_WIDTH),
        badge,
        text(entry.area())
            .font(font::MONO)
            .size(font_size::SMALL)
            .style(style::muted)
            .width(AREA_WIDTH),
        text(entry.message.as_str()).size(font_size::SMALL),
        text(fields)
            .font(font::MONO)
            .size(font_size::SMALL)
            .style(style::muted),
    ]
    .spacing(gap::S)
    .align_y(Vertical::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    fn entry(level: Level, message: &str) -> LogEntry {
        LogEntry {
            level,
            message: message.to_owned(),
            target: "presswork::app".to_owned(),
            ..LogEntry::default()
        }
    }

    fn messages(panel: &LogPanel) -> Vec<String> {
        panel.visible().map(|entry| entry.message.clone()).collect()
    }

    #[test]
    fn starts_closed() {
        assert!(!LogPanel::new(LogStore::with_capacity(10)).is_open());
    }

    #[test]
    fn toggle_opens_and_closes_the_panel() {
        let mut panel = LogPanel::new(LogStore::with_capacity(10));

        panel.update(Message::Toggle);
        assert!(panel.is_open());

        panel.update(Message::Toggle);
        assert!(!panel.is_open());
    }

    #[test]
    fn loads_the_entries_that_exist_at_the_start() {
        let store = LogStore::with_capacity(10);
        store.push(entry(Level::Info, "Presswork started"));

        let panel = LogPanel::new(store);

        assert_eq!(messages(&panel), ["Presswork started"]);
    }

    #[test]
    fn changed_adds_the_new_entries() {
        let store = LogStore::with_capacity(10);
        let mut panel = LogPanel::new(store.clone());

        store.push(entry(Level::Info, "Compiled"));
        panel.update(Message::Changed);

        assert_eq!(messages(&panel), ["Compiled"]);
    }

    #[test]
    fn level_selection_hides_less_severe_entries() {
        let store = LogStore::with_capacity(10);
        store.push(entry(Level::Info, "Compiled"));
        store.push(entry(Level::Warn, "Font not found"));
        let mut panel = LogPanel::new(store);

        panel.update(Message::LevelSelected(Level::Warn));

        assert_eq!(messages(&panel), ["Font not found"]);
    }

    #[test]
    fn search_hides_entries_without_the_text() {
        let store = LogStore::with_capacity(10);
        store.push(entry(Level::Info, "Compiled"));
        store.push(entry(Level::Info, "Exported"));
        let mut panel = LogPanel::new(store);

        panel.update(Message::SearchChanged("export".to_owned()));

        assert_eq!(messages(&panel), ["Exported"]);
    }

    #[test]
    fn keeps_at_most_the_capacity_of_the_store() {
        let store = LogStore::with_capacity(2);
        let mut panel = LogPanel::new(store.clone());

        for message in ["a", "b", "c"] {
            store.push(entry(Level::Info, message));
            panel.update(Message::Changed);
        }

        assert_eq!(messages(&panel), ["b", "c"]);
    }

    #[test]
    fn view_shows_the_messages_of_the_entries() {
        let store = LogStore::with_capacity(10);
        store.push(entry(Level::Info, "Compiled"));
        let panel = LogPanel::new(store);
        let localizer = Localizer::english();

        let mut ui = simulator(panel.view(&localizer));

        assert!(ui.find("Compiled").is_ok());
    }

    #[test]
    fn view_tells_when_no_entry_agrees_with_the_filter() {
        let panel = LogPanel::new(LogStore::with_capacity(10));
        let localizer = Localizer::english();

        let mut ui = simulator(panel.view(&localizer));

        assert!(ui.find("No log entries agree with the filter.").is_ok());
    }

    #[test]
    fn each_text_of_the_panel_has_english_text() {
        let localizer = Localizer::english();
        let ids = [
            "logs-toggle",
            "logs-title",
            "logs-find",
            "logs-empty",
            "logs-column-time",
            "logs-column-level",
            "logs-column-area",
            "logs-column-message",
        ]
        .into_iter()
        .chain(LEVELS.into_iter().map(level_text_id));

        for id in ids {
            assert_ne!(localizer.text(id), id);
        }
    }
}
