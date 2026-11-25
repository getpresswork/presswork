use iced::widget::markdown::{self, Highlight, Item, Settings, Style, Text, Uri};
use iced::widget::{Id, column, container, image, rich_text, text};
use iced::{Element, Fill, Theme, border, padding};
use presswork_typst::example_id;
use presswork_ui::theme as ui_theme;
use presswork_ui::{font, font_size, space as gap, style};

use crate::examples::{ExamplePage, Examples};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Viewer<'a> {
    examples: Option<&'a Examples>,
}

pub(crate) fn view<'a>(items: &'a [Item], theme: &Theme) -> Element<'a, Uri> {
    markdown::view_with(items, settings(theme), &Viewer { examples: None })
}

pub(crate) fn view_with_examples<'a>(
    items: &'a [Item],
    theme: &Theme,
    examples: &'a Examples,
) -> Element<'a, Uri> {
    markdown::view_with(
        items,
        settings(theme),
        &Viewer {
            examples: Some(examples),
        },
    )
}

pub(crate) fn example_widget(id: u128) -> Id {
    Id::from(format!("docs-example-{id:032x}"))
}

fn settings(theme: &Theme) -> Settings {
    let colors = ui_theme::colors(theme);
    let style = Style {
        font: font::UI,
        inline_code_highlight: Highlight {
            background: colors.chrome.into(),
            border: border::rounded(3),
        },
        inline_code_padding: padding::left(2).right(2),
        inline_code_color: colors.ink,
        inline_code_font: font::MONO,
        code_block_font: font::MONO,
        link_color: colors.accent,
    };
    Settings {
        h1_size: font_size::DISPLAY.into(),
        h2_size: font_size::TITLE.into(),
        h3_size: font_size::LARGE.into(),
        h4_size: font_size::BODY.into(),
        h5_size: font_size::BODY.into(),
        h6_size: font_size::BODY.into(),
        code_size: font_size::SMALL.into(),
        spacing: gap::M.into(),
        ..Settings::with_text_size(font_size::BODY, style)
    }
}

fn example_view<'a>(id: u128, pages: &[ExamplePage]) -> Element<'a, Uri> {
    container(
        column(pages.iter().map(|page| {
            container(image(page.handle.clone()).width(Fill))
                .padding(1)
                .max_width(page.width + 2.0)
                .style(style::outline)
                .into()
        }))
        .spacing(gap::XS),
    )
    .id(example_widget(id))
    .into()
}

impl<'a> markdown::Viewer<'a, Uri> for Viewer<'a> {
    fn on_link_click(url: Uri) -> Uri {
        url
    }

    fn image(
        &self,
        settings: Settings,
        _url: &'a Uri,
        _title: &'a str,
        alt: &Text,
    ) -> Element<'a, Uri> {
        container(rich_text(alt.spans(settings.style)).on_link_click(Self::on_link_click))
            .padding(gap::S)
            .style(style::code_block)
            .into()
    }

    fn code_block(
        &self,
        settings: Settings,
        language: Option<&'a str>,
        code: &'a str,
        _lines: &'a [Text],
    ) -> Element<'a, Uri> {
        let block = container(
            text(code.trim_end_matches('\n'))
                .font(settings.style.code_block_font)
                .size(settings.code_size),
        )
        .padding(gap::S)
        .width(Fill)
        .style(style::code_block);
        let example = language
            .and_then(example_id)
            .and_then(|id| Some((id, self.examples?.pages(id)?)));
        match example {
            Some((id, pages)) if !pages.is_empty() => column![block, example_view(id, pages)]
                .spacing(gap::XS)
                .into(),
            _ => block.into(),
        }
    }
}
