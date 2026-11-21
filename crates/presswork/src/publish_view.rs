use std::collections::HashMap;
use std::path::Path;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, scrollable, space, text};
use iced::{Element, Fill};
use presswork_project::{ExportFormat, ExportProfile};
use presswork_typst::Problem;
use presswork_ui::style::{self, Tone};
use presswork_ui::{font, font_size, space as gap};

use crate::compiler::{Export, ExportFailure};
use crate::i18n::{FluentArgs, Localizer};
use crate::problems::problem_text;

const LIST_WIDTH: f32 = 640.0;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Export(usize),
    Exported {
        index: usize,
        run: u64,
        result: Result<Export, ExportFailure>,
    },
    ShowProblem(usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    Running(u64),
    Done(Export),
    Failed(ExportFailure),
}

#[derive(Debug, Default)]
pub(crate) struct PublishView {
    states: HashMap<usize, State>,
    runs: u64,
}

impl PublishView {
    pub(crate) fn start(&mut self, index: usize) -> Option<u64> {
        if self.is_running(index) {
            return None;
        }
        self.runs += 1;
        self.states.insert(index, State::Running(self.runs));
        Some(self.runs)
    }

    pub(crate) fn finish(&mut self, index: usize, run: u64, result: Result<Export, ExportFailure>) {
        if self.states.get(&index) != Some(&State::Running(run)) {
            return;
        }
        let state = match result {
            Ok(report) => State::Done(report),
            Err(failure) => State::Failed(failure),
        };
        self.states.insert(index, state);
    }

    pub(crate) fn problem(&self, index: usize, problem: usize) -> Option<&Problem> {
        match self.states.get(&index)? {
            State::Done(report) => report.problems.get(problem),
            State::Running(_) | State::Failed(_) => None,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.states.clear();
    }

    fn is_running(&self, index: usize) -> bool {
        matches!(self.states.get(&index), Some(State::Running(_)))
    }

    pub(crate) fn view<'a>(
        &'a self,
        localizer: &Localizer,
        profiles: &[ExportProfile],
        main: &Path,
        from_manifest: bool,
    ) -> Element<'a, Message> {
        let mut content = column![
            text(localizer.text("publish-title"))
                .font(font::UI_SEMIBOLD)
                .size(font_size::TITLE)
        ]
        .spacing(gap::M);
        if !from_manifest {
            content = content.push(
                text(localizer.text("publish-default-note"))
                    .size(font_size::SMALL)
                    .style(style::muted),
            );
        }
        let rows = profiles
            .iter()
            .enumerate()
            .map(|(index, profile)| self.profile_view(index, profile, main, localizer));
        container(
            content
                .push(scrollable(column(rows).spacing(gap::L)).height(Fill))
                .width(LIST_WIDTH),
        )
        .center_x(Fill)
        .padding(gap::XL)
        .height(Fill)
        .style(style::page)
        .into()
    }

    fn profile_view<'a>(
        &'a self,
        index: usize,
        profile: &ExportProfile,
        main: &Path,
        localizer: &Localizer,
    ) -> Element<'a, Message> {
        let export = container(
            button(text(localizer.text("publish-export")))
                .padding([gap::XS + gap::XXS, gap::M])
                .style(style::primary)
                .on_press_maybe((!self.is_running(index)).then_some(Message::Export(index))),
        )
        .id(export_id(index));
        let output = profile.template(main).display().to_string();
        let heading = row![
            column![
                row![
                    text(profile.name.clone()).font(font::UI_SEMIBOLD),
                    text(summary(profile, localizer))
                        .size(font_size::SMALL)
                        .style(style::muted),
                ]
                .spacing(gap::S)
                .align_y(Vertical::Center),
                text(output)
                    .font(font::MONO)
                    .size(font_size::SMALL)
                    .style(style::muted),
            ]
            .spacing(gap::XXS),
            space().width(Fill),
            export,
        ]
        .align_y(Vertical::Center);
        let mut lines = column![heading].spacing(gap::XS);
        match self.states.get(&index) {
            None => {}
            Some(State::Running(_)) => {
                lines = lines.push(
                    text(localizer.text("publish-exporting"))
                        .size(font_size::SMALL)
                        .style(style::muted),
                );
            }
            Some(State::Failed(failure)) => {
                lines = lines.push(
                    text(failure_text(failure, localizer))
                        .size(font_size::SMALL)
                        .style(style::tone(Tone::Danger)),
                );
            }
            Some(State::Done(report)) => {
                lines = lines.push(
                    text(done_text(report, localizer))
                        .size(font_size::SMALL)
                        .style(style::tone(Tone::Success)),
                );
                for (number, problem) in report.problems.iter().enumerate() {
                    lines = lines.push(
                        button(
                            text(problem_text(problem, localizer))
                                .size(font_size::SMALL)
                                .style(style::tone(Tone::Warning)),
                        )
                        .padding([gap::XXS, 0.0])
                        .style(style::ghost)
                        .on_press_maybe(
                            problem
                                .source
                                .is_some()
                                .then_some(Message::ShowProblem(index, number)),
                        ),
                    );
                }
            }
        }
        container(lines)
            .padding(gap::M)
            .width(Fill)
            .style(style::chrome)
            .into()
    }
}

pub(crate) fn export_id(index: usize) -> String {
    format!("export-{index}")
}

fn summary(profile: &ExportProfile, localizer: &Localizer) -> String {
    match profile.format {
        ExportFormat::Pdf if profile.standards.is_empty() => localizer.text("publish-format-pdf"),
        ExportFormat::Pdf => profile
            .standards
            .iter()
            .map(|name| standard_label(name))
            .collect::<Vec<_>>()
            .join(", "),
        ExportFormat::Png => {
            let mut args = FluentArgs::new();
            args.set("ppi", profile.resolution());
            localizer.format("publish-format-png", &args)
        }
        ExportFormat::Svg => localizer.text("publish-format-svg"),
    }
}

fn standard_label(name: &str) -> String {
    if name.starts_with(|character: char| character.is_ascii_digit()) {
        return format!("PDF {name}");
    }
    match name.split_once('-') {
        Some((family, version)) => format!("PDF/{}-{version}", family.to_uppercase()),
        None => format!("PDF/{}", name.to_uppercase()),
    }
}

fn done_text(export: &Export, localizer: &Localizer) -> String {
    let mut args = FluentArgs::new();
    args.set("count", export.files.len());
    args.set("report", export.report.display().to_string());
    let first = export.files.first();
    args.set(
        "file",
        first.map_or_else(String::new, |path| path.display().to_string()),
    );
    args.set(
        "folder",
        first
            .and_then(|path| path.parent())
            .map(|folder| folder.display().to_string())
            .filter(|folder| !folder.is_empty())
            .unwrap_or_else(|| localizer.text("publish-project-folder")),
    );
    localizer.format("publish-exported", &args)
}

fn failure_text(failure: &ExportFailure, localizer: &Localizer) -> String {
    let mut args = FluentArgs::new();
    let id = match failure {
        ExportFailure::NeedsEngine => "publish-needs-engine",
        ExportFailure::NoPages => "publish-no-pages",
        ExportFailure::Stopped => "publish-stopped",
        ExportFailure::Outside(path) => {
            args.set("path", path.display().to_string());
            "publish-outside"
        }
        ExportFailure::Export(message) => {
            args.set("error", message.clone());
            "publish-failed"
        }
        ExportFailure::Write(path, message) => {
            args.set("path", path.display().to_string());
            args.set("error", message.clone());
            "publish-write-failed"
        }
    };
    localizer.format(id, &args)
}

pub(crate) fn no_project<'a>(localizer: &Localizer) -> Element<'a, Message> {
    container(
        text(localizer.text("publish-no-project"))
            .style(style::muted)
            .align_x(Horizontal::Center),
    )
    .center(Fill)
    .style(style::page)
    .into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::{selector, simulator};
    use presswork_typst::ProblemKind;

    use super::*;

    fn pdf() -> ExportProfile {
        ExportProfile::named("PDF", ExportFormat::Pdf)
    }

    fn print() -> ExportProfile {
        ExportProfile {
            standards: vec!["a-2b".to_owned()],
            output: Some(PathBuf::from("out/book.pdf")),
            ..ExportProfile::named("print", ExportFormat::Pdf)
        }
    }

    fn report() -> Export {
        Export {
            files: vec![PathBuf::from("out/book.pdf")],
            problems: vec![Problem {
                page: Some(2),
                kind: ProblemKind::LowResolution {
                    ppi: 120,
                    minimum: 300,
                },
                source: None,
                area: None,
            }],
            report: PathBuf::from("out/print.report.json"),
        }
    }

    #[test]
    fn a_project_without_profiles_offers_one_pdf_file() {
        let view = PublishView::default();
        let localizer = Localizer::english();

        let mut ui = simulator(view.view(&localizer, &[pdf()], Path::new("main.typ"), false));

        assert!(ui.find("main.pdf").is_ok());
        assert!(ui.find(localizer.text("publish-default-note")).is_ok());
    }

    #[test]
    fn a_profile_shows_its_standards_and_its_file() {
        let view = PublishView::default();
        let localizer = Localizer::english();

        let mut ui = simulator(view.view(&localizer, &[print()], Path::new("main.typ"), true));

        assert!(ui.find("PDF/A-2b").is_ok());
        assert!(ui.find("out/book.pdf").is_ok());
    }

    #[test]
    fn page_images_show_the_pattern_of_their_file_names() {
        let view = PublishView::default();
        let localizer = Localizer::english();
        let pages = ExportProfile {
            output: Some(PathBuf::from("out/page-{0p}.png")),
            ..ExportProfile::named("Pages", ExportFormat::Png)
        };

        let mut ui = simulator(view.view(&localizer, &[pages], Path::new("main.typ"), true));

        assert!(ui.find("PNG, 144 ppi").is_ok());
        assert!(ui.find("out/page-{0p}.png").is_ok());
    }

    #[test]
    fn the_export_button_asks_for_its_profile() {
        let view = PublishView::default();
        let localizer = Localizer::english();
        let mut ui =
            simulator(view.view(&localizer, &[pdf(), print()], Path::new("main.typ"), true));

        ui.click(selector::id(export_id(1))).unwrap();

        assert!(matches!(
            ui.into_messages().collect::<Vec<_>>().as_slice(),
            [Message::Export(1)]
        ));
    }

    #[test]
    fn a_running_export_does_not_start_again() {
        let mut view = PublishView::default();

        assert!(view.start(0).is_some());
        assert_eq!(view.start(0), None);
        let localizer = Localizer::english();
        let mut ui = simulator(view.view(&localizer, &[pdf()], Path::new("main.typ"), true));
        assert!(ui.find("Exporting…").is_ok());
    }

    #[test]
    fn a_finished_export_shows_the_file_and_the_problems() {
        let mut view = PublishView::default();
        let run = view.start(0).unwrap();
        view.finish(0, run, Ok(report()));
        let localizer = Localizer::english();

        let mut ui = simulator(view.view(&localizer, &[print()], Path::new("main.typ"), true));

        assert!(
            ui.find("Presswork wrote out/book.pdf and the build report out/print.report.json.")
                .is_ok()
        );
        assert!(
            ui.find("Page 3: The image has 120 ppi. The profile needs 300 ppi.")
                .is_ok()
        );
    }

    #[test]
    fn a_result_of_an_earlier_run_is_ignored() {
        let mut view = PublishView::default();
        let earlier = view.start(0).unwrap();
        view.clear();
        let later = view.start(0).unwrap();

        view.finish(0, earlier, Ok(report()));

        assert_eq!(view.states.get(&0), Some(&State::Running(later)));
    }

    #[test]
    fn a_failed_export_tells_why() {
        let mut view = PublishView::default();
        let run = view.start(0).unwrap();
        view.finish(0, run, Err(ExportFailure::NoPages));
        let localizer = Localizer::english();

        let mut ui = simulator(view.view(&localizer, &[pdf()], Path::new("main.typ"), true));

        assert!(ui.find(localizer.text("publish-no-pages")).is_ok());
    }

    #[test]
    fn standards_have_the_names_of_the_standards() {
        assert_eq!(standard_label("a-2b"), "PDF/A-2b");
        assert_eq!(standard_label("ua-1"), "PDF/UA-1");
        assert_eq!(standard_label("1.7"), "PDF 1.7");
    }

    #[test]
    fn each_text_of_the_workspace_has_english_text() {
        let localizer = Localizer::english();

        for id in [
            "publish-title",
            "publish-default-note",
            "publish-no-project",
            "publish-export",
            "publish-exporting",
            "publish-exported",
            "publish-project-folder",
            "publish-needs-engine",
            "publish-no-pages",
            "publish-stopped",
            "publish-outside",
            "publish-failed",
            "publish-write-failed",
            "publish-format-pdf",
            "publish-format-png",
            "publish-format-svg",
        ] {
            assert_ne!(localizer.text(id), id);
        }
    }
}
