use presswork_language::HouseStyle;

use crate::ide::Analysis;
use crate::outline::SourceRange;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleProblem {
    pub text: String,
    pub rule: usize,
    pub replacement: Option<String>,
    pub source: SourceRange,
}

impl Analysis {
    #[must_use]
    pub fn style(&self, style: &HouseStyle) -> Vec<StyleProblem> {
        if style.is_empty() {
            return Vec::new();
        }
        let mut problems = Vec::new();
        for prose in self.prose() {
            for run in &prose.runs {
                let text = prose.text(run);
                for found in style.check(text) {
                    let Some(source) = prose.place(run, found.range.clone()) else {
                        continue;
                    };
                    problems.push(StyleProblem {
                        text: text[found.range].to_owned(),
                        rule: found.rule,
                        replacement: found.replacement,
                        source,
                    });
                }
            }
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use typst::foundations::Datetime;

    use super::*;
    use crate::fonts::Fonts;
    use crate::job::analyze;
    use crate::world::World;

    const STYLE: &str = "[[rule]]\ntext = \"organise\"\nreplacement = \"organize\"\n\n\
        [[rule]]\ntext = \"in order to\"\nreplacement = \"to\"\n";

    fn check(text: &str) -> Vec<StyleProblem> {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("main.typ"), text).unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 20),
            folder.path().to_path_buf(),
            Path::new("main.typ"),
        ));
        analyze(&world, text)
            .analysis
            .unwrap()
            .style(&HouseStyle::parse(STYLE).unwrap())
    }

    #[test]
    fn a_style_problem_has_its_place_and_its_replacement() {
        let text = "= Books\nWe organise the books.";

        assert_eq!(
            check(text),
            [StyleProblem {
                text: "organise".to_owned(),
                rule: 0,
                replacement: Some("organize".to_owned()),
                source: SourceRange::Main(11..19),
            }]
        );
        assert_eq!(&text[11..19], "organise");
    }

    #[test]
    fn a_rule_with_more_words_finds_them_across_spaces_and_lines() {
        let found = check("We read in order\nto learn, *in order to* know.");

        let texts: Vec<&str> = found.iter().map(|problem| problem.text.as_str()).collect();
        assert_eq!(texts, ["in order\nto", "in order to"]);
    }

    #[test]
    fn code_math_and_raw_text_are_not_checked_for_style() {
        let found = check(
            "#let organise = 1\n`organise` $organise$ #\"organise\"\n#let unused = [organise]",
        );

        assert_eq!(found, []);
    }
}
