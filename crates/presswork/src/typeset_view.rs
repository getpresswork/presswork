use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, scrollable, text, tooltip};
use iced::{Element, Fill, Length};
use presswork_typst::Problem;
use presswork_ui::icon::{Icon, icon};
use presswork_ui::{font, font_size, size, space as gap, style};

use crate::chrome;
use crate::i18n::{FluentArgs, Localizer};
use crate::problems::problem_sentence;

pub(crate) const PANEL_WIDTH: f32 = 340.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Message {
    Show(usize),
    Source(usize),
}

pub(crate) fn view<'a>(
    localizer: &Localizer,
    problems: &[Problem],
    available: bool,
    shown: Option<usize>,
    width: Length,
) -> Element<'a, Message> {
    let note = |id: &str| {
        text(localizer.text(id))
            .size(font_size::SMALL)
            .style(style::muted)
    };
    let mut content =
        column![text(localizer.text("typeset-title")).font(font::UI_SEMIBOLD)].spacing(gap::S);
    if !available {
        content = content.push(note("typeset-needs-engine"));
    } else if problems.is_empty() {
        content = content.push(note("typeset-empty"));
    } else {
        let mut args = FluentArgs::new();
        args.set("count", problems.len());
        let rows = problems
            .iter()
            .enumerate()
            .map(|(index, problem)| problem_row(index, problem, shown == Some(index), localizer));
        content = content
            .push(
                text(localizer.format("typeset-count", &args))
                    .size(font_size::SMALL)
                    .style(style::muted),
            )
            .push(scrollable(column(rows).spacing(gap::XXS)).height(Fill));
    }
    container(content)
        .padding([gap::S, gap::S])
        .width(width)
        .height(Fill)
        .style(style::chrome)
        .into()
}

fn problem_row<'a>(
    index: usize,
    problem: &Problem,
    shown: bool,
    localizer: &Localizer,
) -> Element<'a, Message> {
    let page = problem.page.map_or_default(|page| (page + 1).to_string());
    let content = row![
        text(problem_sentence(problem, localizer))
            .size(font_size::SMALL)
            .width(Fill),
        text(page)
            .font(font::MONO)
            .size(font_size::SMALL)
            .style(style::muted)
    ]
    .spacing(gap::S);
    let show = button(content)
        .padding([gap::XS, gap::S])
        .width(Fill)
        .style(style::list_item(shown))
        .on_press(Message::Show(index));
    if problem.source.is_none() {
        return show.into();
    }
    let source = container(chrome::tip(
        button(icon(Icon::SquarePen).size(size::ICON_SMALL))
            .padding(gap::XS)
            .style(style::ghost)
            .on_press(Message::Source(index)),
        localizer.text("typeset-source"),
        tooltip::Position::Left,
    ))
    .id(format!("qa-source-{index}"));
    row![show, source]
        .spacing(gap::XXS)
        .align_y(Vertical::Center)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::Id;
    use iced_test::simulator;
    use presswork_typst::{ProblemKind, SourceRange};

    use super::*;

    const SENTENCE: &str = "The last line of a paragraph has only 3 characters.";

    fn short_line(source: Option<SourceRange>) -> Problem {
        Problem {
            page: Some(0),
            kind: ProblemKind::ShortLastLine { characters: 3 },
            source,
            area: None,
        }
    }

    fn panel<'a>(localizer: &Localizer, problems: &[Problem]) -> Element<'a, Message> {
        view(localizer, problems, true, None, Length::Fill)
    }

    #[test]
    fn a_layout_without_problems_says_so() {
        let localizer = Localizer::english();

        let mut ui = simulator(panel(&localizer, &[]));

        assert!(
            ui.find("Presswork found no problems in the layout.")
                .is_ok()
        );
    }

    #[test]
    fn each_problem_shows_its_sentence_and_its_page() {
        let localizer = Localizer::english();
        let problems = [short_line(Some(SourceRange::Main(17..20)))];

        let mut ui = simulator(panel(&localizer, &problems));

        assert!(ui.find("1 problem in the layout").is_ok());
        assert!(ui.find(SENTENCE).is_ok());
        assert!(ui.find("1").is_ok());
    }

    #[test]
    fn a_click_on_a_problem_asks_to_show_it() {
        let localizer = Localizer::english();
        let problems = [short_line(Some(SourceRange::Main(17..20)))];
        let mut ui = simulator(panel(&localizer, &problems));

        ui.click(SENTENCE).unwrap();

        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Message::Show(0)]);
    }

    #[test]
    fn the_source_button_of_a_problem_asks_for_its_source() {
        let localizer = Localizer::english();
        let problems = [short_line(Some(SourceRange::Main(17..20)))];
        let mut ui = simulator(panel(&localizer, &problems));

        ui.click(Id::from("qa-source-0".to_owned())).unwrap();

        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Message::Source(0)]);
    }

    #[test]
    fn a_problem_without_a_source_has_no_source_button() {
        let localizer = Localizer::english();
        let problems = [short_line(None)];

        let mut ui = simulator(panel(&localizer, &problems));

        assert!(ui.find(Id::from("qa-source-0".to_owned())).is_err());
        assert!(ui.find(SENTENCE).is_ok());
    }

    #[test]
    fn without_the_built_in_engine_the_report_tells_why() {
        let localizer = Localizer::english();
        let problems = [short_line(Some(SourceRange::Main(17..20)))];

        let mut ui = simulator(view(&localizer, &problems, false, None, Length::Fill));

        assert!(
            ui.find("The QA report is available only with the built-in Typst engine.")
                .is_ok()
        );
    }

    #[test]
    fn the_source_button_has_english_text() {
        let localizer = Localizer::english();

        assert_ne!(localizer.text("typeset-source"), "typeset-source");
    }
}
