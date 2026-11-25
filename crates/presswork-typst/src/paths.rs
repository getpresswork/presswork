use std::ops::Range;
use std::path::{Path, PathBuf};

use typst::syntax::ast::{self, AstNode as _};
use typst::syntax::{Source, Span, SyntaxNode};

const READERS: [&str; 11] = [
    "bibliography",
    "cbor",
    "csv",
    "image",
    "json",
    "path",
    "plugin",
    "read",
    "toml",
    "xml",
    "yaml",
];

#[must_use]
pub fn renamed_paths(
    text: &str,
    file: &Path,
    old: &Path,
    new: &Path,
) -> Vec<(Range<usize>, String)> {
    let from = file.parent().unwrap_or(Path::new(""));
    let to = if file == old {
        new.parent().unwrap_or(Path::new(""))
    } else {
        from
    };
    let source = Source::detached(text);
    let mut literals = Vec::new();
    collect(source.root(), &mut literals);
    let mut edits: Vec<(Range<usize>, String)> = literals
        .into_iter()
        .filter_map(|(span, literal)| {
            let target = resolve(from, &literal)?;
            let target = if target == old {
                new.to_path_buf()
            } else if from != to && !literal.starts_with('/') {
                target
            } else {
                return None;
            };
            let written = written(&literal, to, &target);
            if written == literal {
                return None;
            }
            Some((source.find(span)?.range(), quoted(&written)))
        })
        .collect();
    edits.sort_by_key(|(range, _)| range.start);
    edits
}

fn collect(node: &SyntaxNode, found: &mut Vec<(Span, String)>) {
    if let Some(include) = node.cast::<ast::ModuleInclude<'_>>() {
        push(include.source(), found);
    } else if let Some(import) = node.cast::<ast::ModuleImport<'_>>() {
        push(import.source(), found);
    } else if let Some(call) = node.cast::<ast::FuncCall<'_>>()
        && let ast::Expr::Ident(name) = call.callee()
        && READERS.contains(&name.get().as_str())
        && let Some(first) = call.args().items().find_map(|arg| match arg {
            ast::Arg::Pos(expr) => Some(expr),
            _ => None,
        })
    {
        match first {
            ast::Expr::Array(array) => {
                for item in array.items() {
                    if let ast::ArrayItem::Pos(expr) = item {
                        push(expr, found);
                    }
                }
            }
            expr => push(expr, found),
        }
    }
    for child in node.children() {
        collect(child, found);
    }
}

fn push(expr: ast::Expr<'_>, found: &mut Vec<(Span, String)>) {
    if let ast::Expr::Str(literal) = expr {
        let value = literal.get();
        if !value.is_empty() && !value.starts_with('@') && !value.contains(':') {
            found.push((literal.span(), value.to_string()));
        }
    }
}

fn parts(path: &Path) -> Vec<String> {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect()
}

fn resolve(from: &Path, literal: &str) -> Option<PathBuf> {
    let (mut resolved, rest) = match literal.strip_prefix('/') {
        Some(rest) => (Vec::new(), rest),
        None => (parts(from), literal),
    };
    for segment in rest.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                resolved.pop()?;
            }
            name => resolved.push(name.to_owned()),
        }
    }
    Some(resolved.iter().collect())
}

fn written(literal: &str, folder: &Path, target: &Path) -> String {
    let target = parts(target);
    if literal.starts_with('/') {
        return format!("/{}", target.join("/"));
    }
    let folder = parts(folder);
    let common = folder
        .iter()
        .zip(&target)
        .take_while(|(one, other)| one == other)
        .count();
    let mut written = vec!["..".to_owned(); folder.len() - common];
    written.extend(target[common..].iter().cloned());
    let written = written.join("/");
    if literal.starts_with("./") && !written.starts_with("../") {
        format!("./{written}")
    } else {
        written
    }
}

fn quoted(path: &str) -> String {
    format!("\"{}\"", path.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn path(text: &str) -> PathBuf {
        text.split('/').collect()
    }

    fn renamed(text: &str, file: &str, old: &str, new: &str) -> String {
        let edits = renamed_paths(text, &path(file), &path(old), &path(new));
        let mut result = text.to_owned();
        for (range, literal) in edits.into_iter().rev() {
            result.replace_range(range, &literal);
        }
        result
    }

    #[test]
    fn an_include_of_the_renamed_file_gets_the_new_path() {
        assert_eq!(
            renamed(
                "= Book\n#include \"chapters/one.typ\"\n",
                "main.typ",
                "chapters/one.typ",
                "chapters/first.typ"
            ),
            "= Book\n#include \"chapters/first.typ\"\n"
        );
    }

    #[test]
    fn a_path_keeps_its_form() {
        let text =
            "#import \"/lib/utils.typ\": *\n#include \"./one.typ\"\n#include \"../appendix.typ\"";

        assert_eq!(
            renamed(text, "chapters/two.typ", "lib/utils.typ", "lib/tools.typ"),
            "#import \"/lib/tools.typ\": *\n#include \"./one.typ\"\n#include \"../appendix.typ\""
        );
        assert_eq!(
            renamed(
                text,
                "chapters/two.typ",
                "chapters/one.typ",
                "chapters/first.typ"
            ),
            "#import \"/lib/utils.typ\": *\n#include \"./first.typ\"\n#include \"../appendix.typ\""
        );
        assert_eq!(
            renamed(
                text,
                "chapters/two.typ",
                "appendix.typ",
                "back/appendix.typ"
            ),
            "#import \"/lib/utils.typ\": *\n#include \"./one.typ\"\n#include \"../back/appendix.typ\""
        );
    }

    #[test]
    fn files_that_functions_read_get_the_new_path() {
        let text = "#bibliography((\"a.bib\", \"refs.bib\"))\n#read(\"refs.bib\")\n\
                    #let source = path(\"refs.bib\")\n#figure(image(\"refs.bib\"))\n\
                    #json(\"refs.bib\") #csv(\"other.csv\")";

        assert_eq!(
            renamed(text, "main.typ", "refs.bib", "sources/refs.bib"),
            "#bibliography((\"a.bib\", \"sources/refs.bib\"))\n#read(\"sources/refs.bib\")\n\
             #let source = path(\"sources/refs.bib\")\n#figure(image(\"sources/refs.bib\"))\n\
             #json(\"sources/refs.bib\") #csv(\"other.csv\")"
        );
    }

    #[test]
    fn a_moved_file_keeps_the_targets_of_its_relative_paths() {
        let text = "#image(\"fig.png\")\n#import \"/lib.typ\": x\n\
                    #import \"@preview/cetz:0.3.4\"\n#include \"one-a.typ\"";

        assert_eq!(
            renamed(
                text,
                "chapters/one.typ",
                "chapters/one.typ",
                "parts/one.typ"
            ),
            "#image(\"../chapters/fig.png\")\n#import \"/lib.typ\": x\n\
             #import \"@preview/cetz:0.3.4\"\n#include \"../chapters/one-a.typ\""
        );
    }

    #[test]
    fn other_text_and_other_paths_do_not_change() {
        let text = "The file chapters/one.typ.\n#let name = \"chapters/one.typ\"\n\
                    #include name\n#include \"chapters/\" + \"one.typ\"\n#include \"../../one.typ\"\n\
                    `#include \"chapters/one.typ\"`";

        assert_eq!(
            renamed(text, "main.typ", "chapters/one.typ", "chapters/first.typ"),
            text
        );
    }
}
