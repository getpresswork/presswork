use iced::alignment::Vertical;
use iced::widget::{
    button, column, container, markdown, row, rule, scrollable, text, text_input, tooltip,
};
use iced::{Element, Fill, Theme};
use presswork_typst::{DOCS_LINK, DocKind, DocPage, DocParam, ParamFlag, reference};
use presswork_ui::icon::{Icon, icon};
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, size, space as gap};

use crate::chrome;
use crate::i18n::{FluentArgs, Localizer};

pub(crate) const SEARCH_INPUT: &str = "docs-search";
const SHOWN: usize = 60;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Query(String),
    Open(String),
    Back,
    Forward,
    Contents,
    Link(markdown::Uri),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Request {
    Open(String),
}

#[derive(Debug, Default)]
pub(crate) struct DocsView {
    query: String,
    shown: Option<Shown>,
    back: Vec<String>,
    forward: Vec<String>,
}

#[derive(Debug)]
struct Shown {
    path: String,
    docs: markdown::Content,
    params: Vec<markdown::Content>,
}

impl DocsView {
    pub(crate) fn current(&self) -> Option<&str> {
        self.shown.as_ref().map(|shown| shown.path.as_str())
    }

    pub(crate) fn open(&mut self, target: &str) -> bool {
        let Some(page) = reference().resolve(target) else {
            return false;
        };
        self.query.clear();
        if self.current() == Some(page.path.as_str()) {
            return true;
        }
        if let Some(current) = self.shown.take() {
            self.back.push(current.path);
        }
        self.forward.clear();
        self.shown = Some(shown(page));
        true
    }

    pub(crate) fn update(&mut self, message: Message) -> Option<Request> {
        match message {
            Message::Query(query) => self.query = query,
            Message::Open(path) => {
                self.open(&path);
            }
            Message::Back => self.step(true),
            Message::Forward => self.step(false),
            Message::Contents => {
                self.query.clear();
                if let Some(current) = self.shown.take() {
                    self.back.push(current.path);
                    self.forward.clear();
                }
            }
            Message::Link(url) => match url.strip_prefix(DOCS_LINK) {
                Some(path) => {
                    self.open(path);
                }
                None => return Some(Request::Open(url)),
            },
        }
        None
    }

    fn step(&mut self, back: bool) {
        let (from, to) = if back {
            (&mut self.back, &mut self.forward)
        } else {
            (&mut self.forward, &mut self.back)
        };
        let Some(path) = from.pop() else {
            return;
        };
        if let Some(current) = self.shown.take() {
            to.push(current.path);
        }
        self.shown = reference().page(&path).map(shown);
        self.query.clear();
    }

    pub(crate) fn view<'a>(&'a self, localizer: &Localizer, theme: &Theme) -> Element<'a, Message> {
        let nav = row![
            nav_button(
                Icon::ArrowLeft,
                localizer.text("docs-back"),
                (!self.back.is_empty()).then_some(Message::Back)
            ),
            nav_button(
                Icon::ArrowRight,
                localizer.text("docs-forward"),
                (!self.forward.is_empty()).then_some(Message::Forward)
            ),
            nav_button(
                Icon::BookOpen,
                localizer.text("docs-contents"),
                Some(Message::Contents)
            ),
            text_input(&localizer.text("docs-search"), &self.query)
                .id(SEARCH_INPUT)
                .on_input(Message::Query)
                .size(font_size::SMALL)
                .padding([gap::XS, gap::S])
                .style(style::field),
        ]
        .spacing(gap::XS)
        .align_y(Vertical::Center);
        let body = if !self.query.trim().is_empty() {
            results(&self.query, localizer)
        } else if let Some(shown) = &self.shown {
            page_view(shown, localizer, theme)
        } else {
            contents(localizer)
        };
        column![
            container(nav).padding(gap::S),
            hairline(),
            scrollable(container(body).padding(gap::M).width(Fill)).height(Fill)
        ]
        .height(Fill)
        .into()
    }
}

fn results<'a>(query: &str, localizer: &Localizer) -> Element<'a, Message> {
    let found = reference().search(query);
    if found.is_empty() {
        return note(localizer.text("docs-none"));
    }
    column(found.into_iter().take(SHOWN).map(result_row))
        .spacing(gap::XXS)
        .into()
}

fn result_row<'a>(page: &'static DocPage) -> Element<'a, Message> {
    let content = column![
        row![
            text(&page.title).font(font::UI_SEMIBOLD),
            text(&page.path)
                .font(font::MONO)
                .size(font_size::SMALL)
                .style(style::muted)
        ]
        .spacing(gap::S)
        .align_y(Vertical::Center),
        text(&page.summary)
            .size(font_size::SMALL)
            .style(style::muted)
    ]
    .spacing(gap::XXS);
    button(content)
        .width(Fill)
        .padding([gap::XS, gap::S])
        .style(style::list_item(false))
        .on_press(Message::Open(page.path.clone()))
        .into()
}

fn contents<'a>(localizer: &Localizer) -> Element<'a, Message> {
    let mut args = FluentArgs::new();
    args.set("version", presswork_typst::version());
    let mut sections = column![
        text(localizer.text("docs-title"))
            .font(font::UI_SEMIBOLD)
            .size(font_size::TITLE),
        text(localizer.format("docs-intro", &args)).style(style::muted)
    ]
    .spacing(gap::M);
    for (category, pages) in reference().groups() {
        let links = row(pages
            .into_iter()
            .map(|page| link_button(&page.title, &page.path)))
        .spacing(gap::XS)
        .wrap()
        .vertical_spacing(gap::XS);
        sections = sections.push(
            column![
                text(localizer.text(&format!("docs-category-{category}"))).font(font::UI_SEMIBOLD),
                links
            ]
            .spacing(gap::S),
        );
    }
    sections.into()
}

fn page_view<'a>(shown: &'a Shown, localizer: &Localizer, theme: &Theme) -> Element<'a, Message> {
    let Some(page) = reference().page(&shown.path) else {
        return column![].into();
    };
    let kind = match page.kind {
        DocKind::Function => "docs-kind-function",
        DocKind::Type => "docs-kind-type",
        DocKind::Module => "docs-kind-module",
    };
    let mut content = column![
        row![
            text(&page.title)
                .font(font::UI_SEMIBOLD)
                .size(font_size::TITLE),
            badge(localizer.text(kind))
        ]
        .spacing(gap::S)
        .align_y(Vertical::Center),
        text(&page.path)
            .font(font::MONO)
            .size(font_size::SMALL)
            .style(style::muted)
    ]
    .spacing(gap::S);
    if !page.docs.is_empty() {
        content = content.push(crate::markdown::view(shown.docs.items(), theme).map(Message::Link));
    }
    if !page.params.is_empty() {
        content = content.push(heading(localizer.text("docs-parameters")));
        for (param, docs) in page.params.iter().zip(&shown.params) {
            content = content.push(param_view(param, docs, localizer, theme));
        }
    }
    if let Some(returns) = &page.returns {
        let mut args = FluentArgs::new();
        args.set("types", returns.clone());
        content = content.push(
            text(localizer.format("docs-returns", &args))
                .font(font::MONO)
                .size(font_size::SMALL),
        );
    }
    if !page.members.is_empty() {
        content = content.push(heading(localizer.text("docs-members")));
        content = content.push(
            row(page
                .members
                .iter()
                .map(|member| link_button(member.rsplit('.').next().unwrap_or(member), member)))
            .spacing(gap::XS)
            .wrap()
            .vertical_spacing(gap::XS),
        );
    }
    content.into()
}

fn param_view<'a>(
    param: &'static DocParam,
    docs: &'a markdown::Content,
    localizer: &Localizer,
    theme: &Theme,
) -> Element<'a, Message> {
    let mut title = row![
        text(&param.name).font(font::MONO_BOLD),
        text(&param.types)
            .font(font::MONO)
            .size(font_size::SMALL)
            .style(style::muted)
    ]
    .spacing(gap::S)
    .align_y(Vertical::Center);
    for flag in &param.flags {
        let id = match flag {
            ParamFlag::Required => "docs-required",
            ParamFlag::Positional => "docs-positional",
            ParamFlag::Variadic => "docs-variadic",
            ParamFlag::Settable => "docs-settable",
        };
        title = title.push(badge(localizer.text(id)));
    }
    let mut block = column![title].spacing(gap::XS);
    if let Some(default) = &param.default {
        let mut args = FluentArgs::new();
        args.set("value", default.clone());
        block = block.push(
            text(localizer.format("docs-default", &args))
                .font(font::MONO)
                .size(font_size::SMALL)
                .style(style::muted),
        );
    }
    column![
        hairline(),
        block.push(crate::markdown::view(docs.items(), theme).map(Message::Link))
    ]
    .spacing(gap::S)
    .into()
}

fn link_button<'a>(title: &'a str, path: &str) -> Element<'a, Message> {
    button(text(title).size(font_size::SMALL))
        .padding([gap::XXS, gap::S])
        .style(style::secondary)
        .on_press(Message::Open(path.to_owned()))
        .into()
}

fn nav_button<'a>(shape: Icon, words: String, message: Option<Message>) -> Element<'a, Message> {
    chrome::tip(
        button(icon(shape).size(size::ICON_SMALL))
            .padding(gap::XS)
            .style(style::ghost)
            .on_press_maybe(message),
        words,
        tooltip::Position::Bottom,
    )
}

fn badge<'a>(words: String) -> Element<'a, Message> {
    container(text(words).size(font_size::CAPTION))
        .padding([0.0, gap::XS])
        .style(style::badge(Tone::Neutral))
        .into()
}

fn heading<'a>(words: String) -> Element<'a, Message> {
    text(words)
        .font(font::UI_SEMIBOLD)
        .size(font_size::LARGE)
        .into()
}

fn note<'a>(words: String) -> Element<'a, Message> {
    text(words)
        .size(font_size::SMALL)
        .style(style::muted)
        .into()
}

fn hairline<'a>() -> Element<'a, Message> {
    rule::horizontal(1).style(style::hairline).into()
}

fn shown(page: &DocPage) -> Shown {
    Shown {
        path: page.path.clone(),
        docs: markdown::Content::parse(&page.docs),
        params: page
            .params
            .iter()
            .map(|param| markdown::Content::parse(&param.docs))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use iced_test::{Simulator, simulator};
    use presswork_ui::theme as ui_theme;

    use super::*;

    fn simulate(view: &DocsView) -> Simulator<'_, Message> {
        simulator::simulator(view.view(&Localizer::english(), &ui_theme::light()))
    }

    fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
        ui.into_messages().collect()
    }

    #[test]
    fn the_contents_list_the_pages_by_category() {
        let view = DocsView::default();

        let mut ui = simulate(&view);

        assert!(ui.find("Text").is_ok());
        assert!(ui.find("Layout").is_ok());
        ui.click("Heading").unwrap();
        assert!(matches!(messages(ui).as_slice(), [Message::Open(path)] if path == "heading"));
    }

    #[test]
    fn a_page_shows_its_parameters_and_definitions() {
        let mut view = DocsView::default();

        assert!(view.open("table"));

        let mut ui = simulate(&view);
        assert!(ui.find("Table").is_ok());
        assert!(ui.find("Parameters").is_ok());
        assert!(ui.find("columns").is_ok());
        assert!(ui.find("Definitions").is_ok());
        assert!(ui.find("cell").is_ok());
    }

    #[test]
    fn a_definition_link_opens_its_page() {
        let mut view = DocsView::default();
        view.open("calc");

        let mut ui = simulate(&view);
        ui.click("pow").unwrap();

        assert!(matches!(messages(ui).as_slice(), [Message::Open(path)] if path == "calc.pow"));
    }

    #[test]
    fn a_link_to_a_parameter_opens_its_page() {
        let mut view = DocsView::default();

        let request = view.update(Message::Link(format!("{DOCS_LINK}text.size")));

        assert_eq!(request, None);
        assert_eq!(view.current(), Some("text"));
    }

    #[test]
    fn a_web_link_asks_to_open_it() {
        let mut view = DocsView::default();
        let url = "https://typst.app/docs/guides/page-setup/".to_owned();

        let request = view.update(Message::Link(url.clone()));

        assert_eq!(request, Some(Request::Open(url)));
    }

    #[test]
    fn back_and_forward_move_through_the_opened_pages() {
        let mut view = DocsView::default();
        view.open("text");
        view.open("heading");

        let _ = view.update(Message::Back);
        assert_eq!(view.current(), Some("text"));
        let _ = view.update(Message::Forward);
        assert_eq!(view.current(), Some("heading"));
        let _ = view.update(Message::Contents);
        assert_eq!(view.current(), None);
        let _ = view.update(Message::Back);
        assert_eq!(view.current(), Some("heading"));
    }

    #[test]
    fn the_search_shows_the_pages_that_agree() {
        let mut view = DocsView::default();

        let _ = view.update(Message::Query("pow".to_owned()));

        let mut ui = simulate(&view);
        assert!(ui.find("calc.pow").is_ok());
        ui.click("calc.pow").unwrap();
        assert!(matches!(messages(ui).as_slice(), [Message::Open(path)] if path == "calc.pow"));
    }

    #[test]
    fn an_unknown_page_does_not_open() {
        let mut view = DocsView::default();

        assert!(!view.open("nothing.here"));
        assert_eq!(view.current(), None);
    }

    #[test]
    fn a_page_shows_its_text_as_markdown() {
        let page = reference().page("text").unwrap();

        let shown = shown(page);

        assert!(!shown.docs.items().is_empty());
        assert_eq!(shown.params.len(), page.params.len());
    }
}
