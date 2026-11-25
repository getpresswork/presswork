use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use typst::foundations::Repr;
use typst::syntax::{FileId, Side, VirtualRoot};
use typst::{World as _, WorldExt as _};

use crate::ide::{Analysis, project_path};
use crate::outline::LabelTarget;
use crate::package::PackageName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tooltip {
    Text(String),
    Code(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionKind {
    Syntax,
    Function,
    Type,
    Parameter,
    Constant,
    Path,
    Package,
    Label,
    Font,
    Symbol(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub kind: CompletionKind,
    pub label: String,
    pub detail: Option<String>,
    pub text: String,
    pub select: Range<usize>,
    pub target: Option<LabelTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completions {
    pub from: usize,
    pub items: Vec<Completion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Definition {
    Main(Range<usize>),
    File(PathBuf, Option<Range<usize>>),
    Package(PackageName),
    Library(String),
}

impl Analysis {
    #[must_use]
    pub fn tooltip(&self, file: Option<&Path>, text: &str, offset: usize) -> Option<Tooltip> {
        let (world, source) = self.source_with(file, text)?;
        match typst_ide::tooltip(&world, self.document(), &source, offset, Side::After)? {
            typst_ide::Tooltip::Text(text) => Some(Tooltip::Text(text.to_string())),
            typst_ide::Tooltip::Code(code) => Some(Tooltip::Code(code.to_string())),
        }
    }

    #[must_use]
    pub fn completions(
        &self,
        file: Option<&Path>,
        text: &str,
        offset: usize,
        explicit: bool,
    ) -> Option<Completions> {
        let (world, source) = self.source_with(file, text)?;
        let (from, items) =
            typst_ide::autocomplete(&world, self.document(), &source, offset, explicit)?;
        if items.is_empty() {
            return None;
        }
        let mut targets: HashMap<String, LabelTarget> = if items
            .iter()
            .any(|item| item.kind == typst_ide::CompletionKind::Label)
        {
            self.labels()
                .into_iter()
                .map(|target| (target.name.clone(), target))
                .collect()
        } else {
            HashMap::new()
        };
        Some(Completions {
            from,
            items: items
                .into_iter()
                .map(|item| {
                    let mut found = completion(item);
                    if found.kind == CompletionKind::Label {
                        found.target = targets.remove(&found.label);
                    }
                    found
                })
                .collect(),
        })
    }

    #[must_use]
    pub fn definition(&self, file: Option<&Path>, text: &str, offset: usize) -> Option<Definition> {
        let (world, source) = self.source_with(file, text)?;
        let found = typst_ide::definition(&world, self.document(), &source, offset, Side::After)
            .or_else(|| {
                typst_ide::definition(&world, self.document(), &source, offset, Side::Before)
            })?;
        match found {
            typst_ide::Definition::Span(span) => {
                let id = span.id()?;
                if let Some(package) = package_of(id) {
                    return Some(Definition::Package(package));
                }
                let range = world.range(span)?;
                if id == world.main() {
                    Some(Definition::Main(range))
                } else {
                    Some(Definition::File(project_path(id)?, Some(range)))
                }
            }
            typst_ide::Definition::File(id) => match package_of(id) {
                Some(package) => Some(Definition::Package(package)),
                None => Some(Definition::File(project_path(id)?, None)),
            },
            typst_ide::Definition::Std(value) => {
                Some(Definition::Library(value.repr().to_string()))
            }
        }
    }
}

fn package_of(id: FileId) -> Option<PackageName> {
    match id.get().root() {
        VirtualRoot::Package(spec) => Some(PackageName::from(spec)),
        VirtualRoot::Project => None,
    }
}

fn completion(completion: typst_ide::Completion) -> Completion {
    let (text, select) = expand(completion.apply.as_deref().unwrap_or(&completion.label));
    Completion {
        kind: kind(completion.kind),
        label: completion.label.to_string(),
        detail: completion.detail.map(|detail| detail.to_string()),
        text,
        select,
        target: None,
    }
}

fn kind(kind: typst_ide::CompletionKind) -> CompletionKind {
    match kind {
        typst_ide::CompletionKind::Syntax => CompletionKind::Syntax,
        typst_ide::CompletionKind::Func => CompletionKind::Function,
        typst_ide::CompletionKind::Type => CompletionKind::Type,
        typst_ide::CompletionKind::Param => CompletionKind::Parameter,
        typst_ide::CompletionKind::Constant => CompletionKind::Constant,
        typst_ide::CompletionKind::Path => CompletionKind::Path,
        typst_ide::CompletionKind::Package => CompletionKind::Package,
        typst_ide::CompletionKind::Label => CompletionKind::Label,
        typst_ide::CompletionKind::Font => CompletionKind::Font,
        typst_ide::CompletionKind::Symbol(symbol) => CompletionKind::Symbol(symbol.to_string()),
    }
}

fn expand(snippet: &str) -> (String, Range<usize>) {
    let mut text = String::with_capacity(snippet.len());
    let mut first: Option<Range<usize>> = None;
    let mut rest = snippet;
    while let Some(start) = rest.find("${") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            break;
        };
        text.push_str(&rest[..start]);
        let at = text.len();
        text.push_str(&after[..end]);
        first.get_or_insert(at..text.len());
        rest = &after[end + 1..];
    }
    text.push_str(rest);
    let select = first.unwrap_or(text.len()..text.len());
    (text, select)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use typst::foundations::Datetime;

    use super::*;
    use crate::fonts::Fonts;
    use crate::job::analyze;
    use crate::world::World;

    fn analysis(text: &str) -> Analysis {
        let today = Datetime::from_ymd(2026, 11, 19);
        let world = Mutex::new(World::new(Arc::new(Fonts::embedded()), today));
        analyze(&world, text).analysis.unwrap()
    }

    fn inside(text: &str, word: &str) -> usize {
        text.find(word).unwrap() + 1
    }

    #[test]
    fn a_tooltip_tells_about_a_function() {
        let text = "#text(fill: red)[Hi]";

        let tooltip = analysis(text).tooltip(None, text, inside(text, "text"));

        assert!(
            matches!(&tooltip, Some(Tooltip::Text(about)) if !about.is_empty()),
            "{tooltip:?}"
        );
    }

    #[test]
    fn a_tooltip_converts_a_length() {
        let text = "#let gap = 10pt";

        let tooltip = analysis(text).tooltip(None, text, inside(text, "10pt"));

        assert!(
            matches!(&tooltip, Some(Tooltip::Code(code)) if code.starts_with("10pt =")),
            "{tooltip:?}"
        );
    }

    #[test]
    fn a_tooltip_uses_the_text_of_the_editor() {
        let newer = "#let gap = 2cm";

        let tooltip = analysis("Hello.").tooltip(None, newer, inside(newer, "2cm"));

        assert!(
            matches!(&tooltip, Some(Tooltip::Code(code)) if code.contains("20mm")),
            "{tooltip:?}"
        );
    }

    #[test]
    fn plain_text_has_no_tooltip() {
        let text = "Hello world.";

        assert_eq!(analysis(text).tooltip(None, text, 3), None);
    }

    #[test]
    fn completions_after_a_hash_offer_functions() {
        let text = "#te";

        let completions = analysis(text)
            .completions(None, text, text.len(), false)
            .unwrap();

        assert_eq!(completions.from, 1);
        assert!(
            completions
                .items
                .iter()
                .any(|item| item.label == "text" && item.kind == CompletionKind::Function),
            "{completions:?}"
        );
    }

    #[test]
    fn plain_text_gives_no_completions_without_a_request() {
        let text = "Hello";

        assert_eq!(
            analysis(text).completions(None, text, text.len(), false),
            None
        );
    }

    #[test]
    fn a_request_in_plain_text_gives_completions() {
        let text = "Hello ";

        let completions = analysis(text).completions(None, text, text.len(), true);

        assert!(
            completions.is_some_and(|completions| !completions.items.is_empty()),
            "no completions"
        );
    }

    #[test]
    fn a_label_completion_gives_the_kind_and_the_page_of_its_element() {
        let compiled = "= Start <start>\n#pagebreak()\n= End <end>\nSee .";
        let typed = "= Start <start>\n#pagebreak()\n= End <end>\nSee @";

        let completions = analysis(compiled)
            .completions(None, typed, typed.len(), false)
            .unwrap();

        let end = completions
            .items
            .iter()
            .find(|item| item.label == "end")
            .unwrap();
        assert_eq!(
            end.target
                .as_ref()
                .map(|target| (target.kind, target.page.as_deref())),
            Some((crate::LabelKind::Heading, Some("2")))
        );
    }

    #[test]
    fn other_completions_have_no_label_target() {
        let text = "#te";

        let completions = analysis(text)
            .completions(None, text, text.len(), false)
            .unwrap();

        assert!(completions.items.iter().all(|item| item.target.is_none()));
    }

    #[test]
    fn the_definition_of_a_variable_is_its_name_in_the_binding() {
        let text = "#let gap = 1pt\n#gap";
        let used = text.rfind("gap").unwrap() + 1;

        let definition = analysis(text).definition(None, text, used);

        assert!(
            matches!(&definition, Some(Definition::Main(range)) if &text[range.clone()] == "gap" && range.start < used),
            "{definition:?}"
        );
    }

    #[test]
    fn the_end_of_a_name_also_finds_its_definition() {
        let text = "#let gap = 1pt\n#gap";

        let definition = analysis(text).definition(None, text, text.len());

        assert!(
            matches!(&definition, Some(Definition::Main(range)) if &text[range.clone()] == "gap"),
            "{definition:?}"
        );
    }

    #[test]
    fn the_definition_of_a_reference_is_the_labeled_element() {
        let text = "#set heading(numbering: \"1.\")\n= Start <start>\nSee @start.";

        let definition = analysis(text).definition(None, text, text.rfind("start").unwrap() + 1);

        assert!(
            matches!(&definition, Some(Definition::Main(range)) if text[range.clone()].starts_with("= Start")),
            "{definition:?}"
        );
    }

    #[test]
    fn a_function_of_the_library_is_in_the_library() {
        let text = "#text[Hi]";

        let definition = analysis(text).definition(None, text, inside(text, "text"));

        assert_eq!(definition, Some(Definition::Library("text".to_owned())));
    }

    #[test]
    fn an_included_path_gives_its_file() {
        let folder = tempfile::tempdir().unwrap();
        let text = "#include \"part.typ\"";
        std::fs::write(folder.path().join("main.typ"), text).unwrap();
        std::fs::write(folder.path().join("part.typ"), "Part.").unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            None,
            folder.path().to_path_buf(),
            std::path::Path::new("main.typ"),
        ));
        let analysis = analyze(&world, text).analysis.unwrap();

        let definition = analysis.definition(None, text, inside(text, "part"));

        assert_eq!(
            definition,
            Some(Definition::File(PathBuf::from("part.typ"), None))
        );
    }

    #[test]
    fn the_definition_of_a_name_in_another_file_is_in_that_file() {
        let folder = tempfile::tempdir().unwrap();
        let main = "#include \"part.typ\"";
        let part = "#let gap = 1pt\n#gap";
        std::fs::write(folder.path().join("main.typ"), main).unwrap();
        std::fs::write(folder.path().join("part.typ"), part).unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            None,
            folder.path().to_path_buf(),
            Path::new("main.typ"),
        ));
        let analysis = analyze(&world, main).analysis.unwrap();

        let definition = analysis.definition(
            Some(Path::new("part.typ")),
            part,
            part.rfind("gap").unwrap() + 1,
        );

        assert_eq!(
            definition,
            Some(Definition::File(PathBuf::from("part.typ"), Some(5..8)))
        );
    }

    #[test]
    fn an_imported_package_gives_the_package() {
        let packages = tempfile::tempdir().unwrap();
        let folder = packages.path().join("local/greeting/0.1.0");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            folder.join("typst.toml"),
            "[package]\nname = \"greeting\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n",
        )
        .unwrap();
        std::fs::write(folder.join("lib.typ"), "#let greet = [Hello]").unwrap();
        let text = "#import \"@local/greeting:0.1.0\": greet\n#greet";
        let world = Mutex::new(
            World::new(Arc::new(Fonts::embedded()), None)
                .with_packages(vec![packages.path().to_path_buf()]),
        );
        let analysis = analyze(&world, text).analysis.unwrap();

        let definition = analysis.definition(None, text, inside(text, "greeting"));

        assert_eq!(
            definition,
            Some(Definition::Package(PackageName {
                namespace: "local".to_owned(),
                name: "greeting".to_owned(),
                version: "0.1.0".to_owned(),
            }))
        );
    }

    #[test]
    fn plain_text_has_no_definition() {
        let text = "Hello world.";

        assert_eq!(analysis(text).definition(None, text, 3), None);
    }

    #[test]
    fn a_snippet_without_placeholders_puts_the_cursor_at_its_end() {
        assert_eq!(expand("text"), ("text".to_owned(), 4..4));
    }

    #[test]
    fn an_empty_placeholder_puts_the_cursor_at_its_place() {
        assert_eq!(expand("text(${})"), ("text()".to_owned(), 5..5));
    }

    #[test]
    fn the_first_named_placeholder_is_selected() {
        assert_eq!(
            expand("rgb(${r}, ${g}, ${b})"),
            ("rgb(r, g, b)".to_owned(), 4..5)
        );
    }
}
