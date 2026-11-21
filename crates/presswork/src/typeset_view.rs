use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Fill};
use presswork_typst::Problem;
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, space as gap};

use crate::i18n::{FluentArgs, Localizer};
use crate::problems::problem_text;

const LIST_WIDTH: f32 = 640.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Message {
    Show(usize),
}

pub(crate) fn view<'a>(
    localizer: &Localizer,
    problems: &[Problem],
    available: bool,
) -> Element<'a, Message> {
    let note = |id: &str| {
        text(localizer.text(id))
            .size(font_size::SMALL)
            .style(style::muted)
    };
    let mut content = column![
        text(localizer.text("typeset-title"))
            .font(font::UI_SEMIBOLD)
            .size(font_size::TITLE)
    ]
    .spacing(gap::M);
    if !available {
        content = content.push(note("typeset-needs-engine"));
    } else if problems.is_empty() {
        content = content.push(note("typeset-empty"));
    } else {
        let mut args = FluentArgs::new();
        args.set("count", problems.len());
        let rows = problems.iter().enumerate().map(|(index, problem)| {
            button(
                text(problem_text(problem, localizer))
                    .size(font_size::SMALL)
                    .style(style::tone(Tone::Warning)),
            )
            .padding([gap::XXS, 0.0])
            .style(style::ghost)
            .on_press_maybe(problem.source.is_some().then_some(Message::Show(index)))
            .into()
        });
        content = content
            .push(
                text(localizer.format("typeset-count", &args))
                    .size(font_size::SMALL)
                    .style(style::muted),
            )
            .push(scrollable(column(rows).spacing(gap::XXS)).height(Fill));
    }
    container(content.width(LIST_WIDTH))
        .center_x(Fill)
        .padding(gap::XL)
        .height(Fill)
        .style(style::page)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use presswork_typst::{ProblemKind, SourceRange};

    use super::*;

    fn short_line() -> Problem {
        Problem {
            page: Some(0),
            kind: ProblemKind::ShortLastLine { characters: 3 },
            source: Some(SourceRange::Main(17..20)),
            area: None,
        }
    }

    #[test]
    fn a_layout_without_problems_says_so() {
        let localizer = Localizer::english();

        let mut ui = simulator(view(&localizer, &[], true));

        assert!(
            ui.find("Presswork found no problems in the layout.")
                .is_ok()
        );
    }

    #[test]
    fn each_problem_shows_its_page_and_its_sentence() {
        let localizer = Localizer::english();

        let mut ui = simulator(view(&localizer, &[short_line()], true));

        assert!(ui.find("1 problem in the layout").is_ok());
        assert!(
            ui.find("Page 1: The last line of a paragraph has only 3 characters.")
                .is_ok()
        );
    }

    #[test]
    fn a_click_on_a_problem_asks_for_its_source() {
        let localizer = Localizer::english();
        let mut ui = simulator(view(&localizer, &[short_line()], true));

        ui.click("Page 1: The last line of a paragraph has only 3 characters.")
            .unwrap();

        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Message::Show(0)]);
    }

    #[test]
    fn without_the_built_in_engine_the_report_tells_why() {
        let localizer = Localizer::english();

        let mut ui = simulator(view(&localizer, &[short_line()], false));

        assert!(
            ui.find("The QA report is available only with the built-in Typst engine.")
                .is_ok()
        );
    }
}
