use presswork_project::Colors;
use presswork_typst::{Problem, ProblemKind};

use crate::i18n::{FluentArgs, Localizer};

pub(crate) fn problem_text(problem: &Problem, localizer: &Localizer) -> String {
    let text = problem_sentence(problem, localizer);
    let Some(page) = problem.page else {
        return text;
    };
    let mut args = FluentArgs::new();
    args.set("page", page + 1);
    args.set("problem", text);
    localizer.format("problem-on-page", &args)
}

pub(crate) fn problem_sentence(problem: &Problem, localizer: &Localizer) -> String {
    let mut args = FluentArgs::new();
    let id = match &problem.kind {
        ProblemKind::LowResolution { ppi, minimum } => {
            args.set("ppi", *ppi);
            args.set("minimum", *minimum);
            "problem-resolution"
        }
        ProblemKind::MissingAltText => "problem-alt",
        ProblemKind::MissingFont { family } => {
            args.set("family", family.clone());
            "problem-font"
        }
        ProblemKind::MissingGlyphs { characters } => {
            args.set("characters", characters.clone());
            "problem-glyphs"
        }
        ProblemKind::PageSize {
            width,
            height,
            needed,
        } => {
            args.set("width", needed.unit.format(*width));
            args.set("height", needed.unit.format(*height));
            args.set("needed_width", needed.unit.format(needed.width));
            args.set("needed_height", needed.unit.format(needed.height));
            args.set("unit", needed.unit.name());
            "problem-page-size"
        }
        ProblemKind::Color { color, permitted } => {
            args.set("color", color.clone());
            args.set("permitted", localizer.text(colors_id(*permitted)));
            "problem-color"
        }
        ProblemKind::GradientSpace { space, permitted } => {
            args.set("space", space.clone());
            args.set("permitted", localizer.text(colors_id(*permitted)));
            "problem-gradient"
        }
        ProblemKind::Widow => "problem-widow",
        ProblemKind::Orphan => "problem-orphan",
        ProblemKind::StackedHyphens { lines } => {
            args.set("lines", *lines);
            "problem-hyphens"
        }
        ProblemKind::ShortLastLine { characters } => {
            args.set("characters", *characters);
            "problem-short-line"
        }
        ProblemKind::LooseLine { spacing } => {
            args.set("spacing", *spacing);
            "problem-loose-line"
        }
    };
    localizer.format(id, &args)
}

fn colors_id(colors: Colors) -> &'static str {
    match colors {
        Colors::Cmyk => "colors-cmyk",
        Colors::Gray => "colors-gray",
    }
}

#[cfg(test)]
mod tests {
    use presswork_project::PageSize;

    use super::*;

    fn problem(page: Option<usize>, kind: ProblemKind) -> String {
        problem_text(
            &Problem {
                page,
                kind,
                source: None,
                area: None,
            },
            &Localizer::english(),
        )
    }

    #[test]
    fn each_preflight_problem_has_a_sentence() {
        let needed = PageSize::parse("6in x 9in").unwrap();

        assert_eq!(
            problem(
                None,
                ProblemKind::MissingFont {
                    family: "garamond".to_owned()
                }
            ),
            "The font family garamond is not available. Typst uses a different font."
        );
        assert_eq!(
            problem(
                Some(0),
                ProblemKind::MissingGlyphs {
                    characters: "日 本".to_owned()
                }
            ),
            "Page 1: No font has these characters: 日 本"
        );
        assert_eq!(
            problem(
                Some(4),
                ProblemKind::PageSize {
                    width: 155_956,
                    height: 228_600,
                    needed
                }
            ),
            "Page 5: The page is 6.14 × 9 in. The profile needs 6 × 9 in."
        );
        assert_eq!(
            problem(
                Some(1),
                ProblemKind::Color {
                    color: "rgb(\"#ff0000\")".to_owned(),
                    permitted: Colors::Cmyk
                }
            ),
            "Page 2: The document uses the color rgb(\"#ff0000\"). The profile permits only CMYK, gray, and spot colors."
        );
        assert_eq!(
            problem(
                Some(1),
                ProblemKind::GradientSpace {
                    space: "oklab".to_owned(),
                    permitted: Colors::Gray
                }
            ),
            "Page 2: A gradient uses the color space oklab. The profile permits only gray colors."
        );
    }

    #[test]
    fn each_layout_problem_has_a_sentence() {
        assert_eq!(
            problem(Some(2), ProblemKind::Widow),
            "Page 3: The last line of a paragraph is alone at the top of a page or column."
        );
        assert_eq!(
            problem(Some(2), ProblemKind::Orphan),
            "Page 3: The first line of a paragraph is alone at the bottom of a page or column."
        );
        assert_eq!(
            problem(Some(0), ProblemKind::StackedHyphens { lines: 4 }),
            "Page 1: 4 lines in sequence end with a hyphen."
        );
        assert_eq!(
            problem(Some(0), ProblemKind::ShortLastLine { characters: 1 }),
            "Page 1: The last line of a paragraph has only 1 character."
        );
        assert_eq!(
            problem(Some(0), ProblemKind::LooseLine { spacing: 186 }),
            "Page 1: The word spaces in this line are 186% of their normal width."
        );
    }
}
