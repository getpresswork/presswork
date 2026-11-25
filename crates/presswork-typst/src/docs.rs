use std::collections::HashSet;
use std::fmt::Write as _;
use std::sync::LazyLock;

use typst::foundations::{CastInfo, ParamInfo, Repr, Scope, Value};
use typst::syntax::ast::{self, AstNode as _};
use typst::syntax::{SyntaxKind, SyntaxNode};
use typst::{Library, LibraryExt as _};

pub const DOCS_LINK: &str = "docs:";
const WEB_DOCS: &str = "https://typst.app/docs/";
const SKIPPED: [&str; 3] = ["std", "sym", "emoji"];
const DEEPEST: usize = 2;
const CATEGORIES: [&str; 14] = [
    "foundations",
    "model",
    "text",
    "math",
    "symbols",
    "layout",
    "visualize",
    "introspection",
    "data-loading",
    "pdf",
    "html",
    "svg",
    "png",
    "bundle",
];

static REFERENCE: LazyLock<Reference> = LazyLock::new(|| Reference::of(&Library::default()));

type Visit<'a> = dyn FnMut(&str, &Value, Option<&'static str>) + 'a;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Function,
    Type,
    Module,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamFlag {
    Required,
    Positional,
    Variadic,
    Settable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocParam {
    pub name: String,
    pub types: String,
    pub default: Option<String>,
    pub docs: String,
    pub flags: Vec<ParamFlag>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocPage {
    pub path: String,
    pub title: String,
    pub kind: DocKind,
    pub category: Option<String>,
    pub summary: String,
    pub docs: String,
    pub params: Vec<DocParam>,
    pub returns: Option<String>,
    pub members: Vec<String>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Reference {
    pages: Vec<DocPage>,
}

impl Reference {
    fn of(library: &Library) -> Self {
        let mut pages = Vec::new();
        let mut seen = HashSet::new();
        walk(library, &mut |path, value, category| {
            if let Some(page) = page_of(path, value, category)
                && seen.insert(page.path.clone())
            {
                pages.push(page);
            }
        });
        pages.sort_by(|one, other| one.path.cmp(&other.path));
        Self { pages }
    }

    #[must_use]
    pub fn page(&self, path: &str) -> Option<&DocPage> {
        self.pages
            .binary_search_by(|page| page.path.as_str().cmp(path))
            .ok()
            .map(|index| &self.pages[index])
    }

    #[must_use]
    pub fn resolve(&self, target: &str) -> Option<&DocPage> {
        let mut path = target.split('#').next()?;
        loop {
            if let Some(page) = self.page(path) {
                return Some(page);
            }
            path = path.rsplit_once('.')?.0;
        }
    }

    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&DocPage> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<(usize, &DocPage)> = self
            .pages
            .iter()
            .filter_map(|page| Some((rank(page, &words)?, page)))
            .collect();
        found.sort_by(|one, other| {
            one.0
                .cmp(&other.0)
                .then_with(|| one.1.path.len().cmp(&other.1.path.len()))
                .then_with(|| one.1.path.cmp(&other.1.path))
        });
        found.into_iter().map(|(_, page)| page).collect()
    }

    #[must_use]
    pub fn groups(&self) -> Vec<(&str, Vec<&DocPage>)> {
        CATEGORIES
            .iter()
            .filter_map(|category| {
                let pages: Vec<&DocPage> = self
                    .pages
                    .iter()
                    .filter(|page| {
                        !page.path.contains('.') && page.category.as_deref() == Some(category)
                    })
                    .collect();
                (!pages.is_empty()).then_some((*category, pages))
            })
            .collect()
    }
}

#[must_use]
pub fn reference() -> &'static Reference {
    &REFERENCE
}

pub(crate) fn library_path(value: &Value) -> Option<String> {
    let mut found = None;
    walk(&Library::default(), &mut |path, candidate, _| {
        if found.is_none() && candidate == value && reference().page(path).is_some() {
            found = Some(path.to_owned());
        }
    });
    found
}

fn walk(library: &Library, visit: &mut Visit<'_>) {
    for (name, binding) in library.global.scope().iter() {
        let category = binding.category().map(|category| category.name());
        walk_value(name, binding.read(), category, visit);
    }
}

fn walk_value(path: &str, value: &Value, category: Option<&'static str>, visit: &mut Visit<'_>) {
    if SKIPPED.contains(&path) {
        return;
    }
    visit(path, value, category);
    if path.matches('.').count() >= DEEPEST {
        return;
    }
    let scope = match value {
        Value::Func(func) => func.scope(),
        Value::Type(ty) => Some(ty.scope()),
        Value::Module(module) => Some(module.scope()),
        _ => None,
    };
    for (name, binding) in scope.into_iter().flat_map(Scope::iter) {
        walk_value(&format!("{path}.{name}"), binding.read(), category, visit);
    }
}

fn page_of(path: &str, value: &Value, category: Option<&'static str>) -> Option<DocPage> {
    let name = path.rsplit('.').next().unwrap_or(path);
    let category = category.map(str::to_owned);
    let page = match value {
        Value::Func(func) => {
            let docs = func.docs()?;
            DocPage {
                path: path.to_owned(),
                title: func.title().unwrap_or(name).to_owned(),
                kind: DocKind::Function,
                category,
                summary: summary(&clean_docs(docs)),
                docs: clean_docs(docs),
                params: func.params().filter_map(|info| param(&info)).collect(),
                returns: func.returns().map(cast_text),
                members: members(func.scope(), path),
                keywords: func
                    .keywords()
                    .iter()
                    .map(|word| (*word).to_owned())
                    .collect(),
            }
        }
        Value::Type(ty) => DocPage {
            path: path.to_owned(),
            title: ty.title().to_owned(),
            kind: DocKind::Type,
            category,
            summary: summary(&clean_docs(ty.docs())),
            docs: clean_docs(ty.docs()),
            params: ty.constructor().map_or_else(
                |_| Vec::new(),
                |constructor| {
                    constructor
                        .params()
                        .filter_map(|info| param(&info))
                        .collect()
                },
            ),
            returns: None,
            members: members(Some(ty.scope()), path),
            keywords: ty
                .keywords()
                .iter()
                .map(|word| (*word).to_owned())
                .collect(),
        },
        Value::Module(module) => {
            let members = members(Some(module.scope()), path);
            if members.is_empty() {
                return None;
            }
            DocPage {
                path: path.to_owned(),
                title: name.to_owned(),
                kind: DocKind::Module,
                category,
                summary: String::new(),
                docs: String::new(),
                params: Vec::new(),
                returns: None,
                members,
                keywords: Vec::new(),
            }
        }
        _ => return None,
    };
    Some(page)
}

fn members(scope: Option<&Scope>, path: &str) -> Vec<String> {
    if path.matches('.').count() >= DEEPEST {
        return Vec::new();
    }
    scope
        .into_iter()
        .flat_map(Scope::iter)
        .filter(|(_, binding)| match binding.read() {
            Value::Func(func) => func.docs().is_some(),
            Value::Type(_) => true,
            _ => false,
        })
        .map(|(name, _)| format!("{path}.{name}"))
        .collect()
}

fn param(info: &ParamInfo) -> Option<DocParam> {
    let native = info.to_native()?;
    Some(DocParam {
        name: native.name.to_owned(),
        types: cast_text(&native.input),
        default: native.default.map(|default| default().repr().to_string()),
        docs: clean_docs(native.docs),
        flags: [
            (native.required, ParamFlag::Required),
            (native.positional, ParamFlag::Positional),
            (native.variadic, ParamFlag::Variadic),
            (native.settable, ParamFlag::Settable),
        ]
        .into_iter()
        .filter_map(|(set, flag)| set.then_some(flag))
        .collect(),
    })
}

fn cast_text(info: &CastInfo) -> String {
    let mut parts: Vec<String> = Vec::new();
    info.walk(|info| {
        let part = match info {
            CastInfo::Any => "any".to_owned(),
            CastInfo::Value(value, _) => value.repr().to_string(),
            CastInfo::Type(ty) => ty.short_name().to_owned(),
            CastInfo::Union(_) => return,
        };
        if !parts.contains(&part) {
            parts.push(part);
        }
    });
    parts.join(" | ")
}

fn rank(page: &DocPage, words: &[String]) -> Option<usize> {
    let path = page.path.to_lowercase();
    let name = path.rsplit('.').next().unwrap_or(&path);
    let title = page.title.to_lowercase();
    let summary = page.summary.to_lowercase();
    words
        .iter()
        .map(|word| {
            let word = word.as_str();
            if name == word || title == word || path == word {
                Some(0)
            } else if name.starts_with(word) || title.starts_with(word) {
                Some(1)
            } else if path.contains(word) || title.contains(word) {
                Some(2)
            } else if page
                .keywords
                .iter()
                .any(|keyword| keyword.to_lowercase().contains(word))
            {
                Some(3)
            } else if summary.contains(word) {
                Some(4)
            } else {
                None
            }
        })
        .sum()
}

fn summary(docs: &str) -> String {
    let paragraph = docs
        .trim_start()
        .split("\n\n")
        .next()
        .unwrap_or_default()
        .replace('\n', " ");
    let plain = plain_text(&paragraph);
    let end = plain
        .match_indices(". ")
        .next()
        .map_or(plain.len(), |(index, _)| index + 1);
    plain[..end].trim().to_owned()
}

fn plain_text(markdown: &str) -> String {
    let mut text = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(open) = rest.find('[') {
        text.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        if let Some((middle, end)) = after.find("](").and_then(|middle| {
            let end = after[middle..].find(')')? + middle;
            Some((middle, end))
        }) {
            text.push_str(&after[..middle]);
            rest = &after[end + 1..];
        } else {
            text.push('[');
            rest = after;
        }
    }
    text.push_str(rest);
    text.retain(|character| !matches!(character, '`' | '*'));
    text
}

fn clean_docs(docs: &str) -> String {
    let mut markdown = String::with_capacity(docs.len());
    write_children(&typst::syntax::parse(docs), &mut markdown);
    markdown
}

fn write_children(node: &SyntaxNode, out: &mut String) {
    for child in node.children() {
        write_node(child, out);
    }
}

fn write_markup(markup: ast::Markup<'_>, out: &mut String) {
    write_children(markup.to_untyped(), out);
}

fn markup_text(markup: ast::Markup<'_>) -> String {
    let mut text = String::new();
    write_markup(markup, &mut text);
    text.trim().to_owned()
}

fn write_node(node: &SyntaxNode, out: &mut String) {
    match node.kind() {
        SyntaxKind::Hash => {}
        SyntaxKind::Label => {
            let kept = out.trim_end_matches(' ').len();
            out.truncate(kept);
        }
        SyntaxKind::Heading => {
            if let Some(heading) = ast::Heading::from_untyped(node) {
                out.push_str(&"#".repeat(heading.depth().get() + 2));
                out.push(' ');
                out.push_str(&markup_text(heading.body()));
            }
        }
        SyntaxKind::Ref => {
            if let Some(reference) = ast::Ref::from_untyped(node) {
                let target = reference.target();
                let text = reference.supplement().map_or_else(
                    || {
                        target
                            .rsplit(['.', ':'])
                            .next()
                            .unwrap_or(target)
                            .to_owned()
                    },
                    |supplement| markup_text(supplement.body()),
                );
                let _ = write!(out, "[{text}]({})", reference_target(target));
            }
        }
        SyntaxKind::Raw => {
            if let Some(raw) = ast::Raw::from_untyped(node) {
                write_raw(raw, out);
            }
        }
        SyntaxKind::Strong => {
            if let Some(strong) = ast::Strong::from_untyped(node) {
                let _ = write!(out, "**{}**", markup_text(strong.body()));
            }
        }
        SyntaxKind::Emph => {
            if let Some(emph) = ast::Emph::from_untyped(node) {
                let _ = write!(out, "*{}*", markup_text(emph.body()));
            }
        }
        SyntaxKind::ListItem => {
            if let Some(item) = ast::ListItem::from_untyped(node) {
                out.push_str("- ");
                write_markup(item.body(), out);
            }
        }
        SyntaxKind::EnumItem => {
            if let Some(item) = ast::EnumItem::from_untyped(node) {
                let _ = write!(out, "{}. ", item.number().unwrap_or(1));
                write_markup(item.body(), out);
            }
        }
        SyntaxKind::TermItem => {
            if let Some(item) = ast::TermItem::from_untyped(node) {
                let _ = write!(out, "- **{}**: ", markup_text(item.term()));
                write_markup(item.description(), out);
            }
        }
        SyntaxKind::Escape => {
            if let Some(escape) = ast::Escape::from_untyped(node) {
                let character = escape.get();
                if character.is_ascii_punctuation() {
                    out.push('\\');
                }
                out.push(character);
            }
        }
        SyntaxKind::Shorthand => {
            if let Some(shorthand) = ast::Shorthand::from_untyped(node) {
                out.push(shorthand.get());
            }
        }
        SyntaxKind::Link => {
            let url = node.leaf_text();
            let _ = write!(out, "[{url}]({url})");
        }
        SyntaxKind::Linebreak => out.push_str("  \n"),
        SyntaxKind::Equation => write_code(&node.full_text(), out),
        SyntaxKind::FuncCall => write_call(node, out),
        _ if node.children().len() == 0 => out.push_str(node.leaf_text()),
        _ => write_children(node, out),
    }
}

fn write_raw(raw: ast::Raw<'_>, out: &mut String) {
    let lines: Vec<&str> = raw.lines().map(|line| line.get().as_str()).collect();
    if !raw.block() {
        let code = lines.join("\n");
        let inner = code
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
            .or_else(|| {
                code.strip_prefix('[')
                    .and_then(|rest| rest.strip_suffix(']'))
            })
            .unwrap_or(&code);
        write_code(inner, out);
        return;
    }
    let language = raw.lang().map_or("", |lang| lang.get().as_str());
    let language = if language == "example" {
        "typ"
    } else {
        language
    };
    let _ = writeln!(out, "```{language}");
    for line in lines {
        if line.trim_start().starts_with(">>>") {
            continue;
        }
        let shown = line
            .strip_prefix("<<< ")
            .or_else(|| line.strip_prefix("<<<"))
            .unwrap_or(line);
        out.push_str(shown);
        out.push('\n');
    }
    out.push_str("```");
}

fn write_code(code: &str, out: &mut String) {
    if code.contains('`') {
        let _ = write!(out, "`` {code} ``");
    } else {
        let _ = write!(out, "`{code}`");
    }
}

fn write_call(node: &SyntaxNode, out: &mut String) {
    if let Some(call) = ast::FuncCall::from_untyped(node)
        && let ast::Expr::Ident(name) = call.callee()
        && name.get() == "link"
    {
        let mut url = None;
        let mut body = None;
        for argument in call.args().items() {
            match argument {
                ast::Arg::Pos(ast::Expr::Str(text)) => url = Some(text.get().to_string()),
                ast::Arg::Pos(ast::Expr::ContentBlock(block)) => body = Some(block),
                _ => {}
            }
        }
        if let Some(url) = url {
            let text = body.map_or_else(|| url.clone(), |block| markup_text(block.body()));
            let _ = write!(out, "[{text}]({url})");
            return;
        }
    }
    write_code(&format!("#{}", node.full_text()), out);
}

fn reference_target(target: &str) -> String {
    if !target.contains(':') {
        return format!("{DOCS_LINK}{target}");
    }
    let parts: Vec<&str> = target.split(':').collect();
    let (path, fragment) = parts.split_at(parts.len().min(2));
    let mut url = format!("{WEB_DOCS}{}/", path.join("/"));
    if !fragment.is_empty() {
        let _ = write!(url, "#{}", fragment.join("-"));
    }
    url
}
#[cfg(test)]
mod tests {
    use super::*;

    fn page(path: &str) -> &'static DocPage {
        reference()
            .page(path)
            .unwrap_or_else(|| panic!("no page {path}"))
    }

    fn paths(pages: &[&DocPage]) -> Vec<String> {
        pages.iter().map(|page| page.path.clone()).collect()
    }

    #[test]
    fn each_function_has_a_page_with_its_parameters() {
        let text = page("text");

        assert_eq!(text.kind, DocKind::Function);
        assert_eq!(text.title, "Text");
        assert_eq!(text.category.as_deref(), Some("text"));
        assert_ne!(text.summary, "");
        let size = text
            .params
            .iter()
            .find(|param| param.name == "size")
            .unwrap();
        assert!(size.types.contains("length"), "{}", size.types);
        assert!(size.flags.contains(&ParamFlag::Settable));
        assert_ne!(size.docs, "");
    }

    #[test]
    fn methods_and_definitions_have_their_own_pages() {
        assert_eq!(page("str").kind, DocKind::Type);
        assert!(page("str").members.contains(&"str.len".to_owned()));
        assert_eq!(page("str.len").kind, DocKind::Function);
        assert_eq!(page("calc.pow").kind, DocKind::Function);
        assert_eq!(page("calc").kind, DocKind::Module);
        assert_eq!(page("table.cell").kind, DocKind::Function);
    }

    #[test]
    fn example_blocks_become_typst_code_without_hidden_lines() {
        let docs = "Text.\n```example\n>>> #set page(width: 100pt)\n<<< #show: rest\n= Title\n```";

        assert_eq!(clean_docs(docs), "Text.\n```typ\n#show: rest\n= Title\n```");
    }

    #[test]
    fn references_become_page_links_and_other_links_web_links() {
        let docs = "See @text.size[`size`], @regex, @reference:context[context], \
                    and #link(\"https://typst.app/universe\")[Typst Universe].";

        assert_eq!(
            clean_docs(docs),
            "See [`size`](docs:text.size), [regex](docs:regex), \
             [context](https://typst.app/docs/reference/context/), \
             and [Typst Universe](https://typst.app/universe)."
        );
    }

    #[test]
    fn typst_markup_becomes_markdown() {
        let docs = "= Example <example>\n*Bold* and _slanted_ text.\n\n\
                    - Use `{auto}` or `[*x*]`.\n+ One";

        assert_eq!(
            clean_docs(docs),
            "### Example\n**Bold** and *slanted* text.\n\n\
             - Use `auto` or `*x*`.\n1. One"
        );
    }

    #[test]
    fn a_dotted_link_finds_the_nearest_page() {
        assert_eq!(
            reference()
                .resolve("text.size")
                .map(|page| page.path.as_str()),
            Some("text")
        );
        assert_eq!(
            reference()
                .resolve("calc.pow")
                .map(|page| page.path.as_str()),
            Some("calc.pow")
        );
        assert!(reference().resolve("nothing.here").is_none());
    }

    #[test]
    fn the_search_finds_pages_by_name_and_words() {
        let found = reference().search("heading");
        assert_eq!(
            found.first().map(|page| page.path.as_str()),
            Some("heading")
        );
        assert!(paths(&reference().search("pow")).contains(&"calc.pow".to_owned()));
        assert_eq!(
            paths(&reference().search("zzzz qqqq")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn the_contents_group_the_top_pages_by_category() {
        let groups = reference().groups();

        let text = groups
            .iter()
            .find(|(category, _)| *category == "text")
            .unwrap();
        assert!(text.1.iter().any(|page| page.path == "text"));
        assert!(
            groups
                .iter()
                .flat_map(|(_, pages)| pages)
                .all(|page| !page.path.contains('.'))
        );
    }

    #[test]
    fn a_value_of_the_library_has_its_page_path() {
        let calc = reference_value("calc");
        let Value::Module(module) = calc else {
            panic!("calc is not a module");
        };
        let pow = module.scope().get("pow").unwrap().read().clone();

        assert_eq!(library_path(&pow).as_deref(), Some("calc.pow"));
    }

    fn reference_value(name: &str) -> Value {
        use typst::LibraryExt as _;
        typst::Library::default()
            .global
            .scope()
            .get(name)
            .unwrap()
            .read()
            .clone()
    }
}
