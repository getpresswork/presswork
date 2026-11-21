use std::collections::{HashMap, HashSet};
use std::ops::Range;

use typst::World as _;
use typst::layout::{Frame, FrameItem};
use typst::syntax::{FileId, LinkedNode, Source, Span, SyntaxKind};
use typst_layout::PagedDocument;

use crate::ide::{Analysis, project_path};
use crate::outline::SourceRange;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Language {
    pub(crate) language: String,
    pub(crate) region: Option<String>,
}

impl Language {
    pub(crate) fn tag(&self) -> String {
        match &self.region {
            Some(region) => format!("{}-{region}", self.language),
            None => self.language.clone(),
        }
    }
}

pub(crate) struct Run {
    pub(crate) range: Range<usize>,
    pub(crate) language: Language,
}

pub(crate) struct Prose {
    pub(crate) source: Source,
    main: bool,
    pub(crate) runs: Vec<Run>,
}

impl Prose {
    pub(crate) fn text(&self, run: &Run) -> &str {
        &self.source.text()[run.range.clone()]
    }

    pub(crate) fn place(&self, run: &Run, range: Range<usize>) -> Option<SourceRange> {
        let range = run.range.start + range.start..run.range.start + range.end;
        if self.main {
            return Some(SourceRange::Main(range));
        }
        Some(SourceRange::File(project_path(self.source.id())?, range))
    }
}

impl Analysis {
    pub(crate) fn prose(&self) -> Vec<Prose> {
        let Some(document) = self.document() else {
            return Vec::new();
        };
        let languages = languages(document);
        let main = self.world().main();
        let mut files: Vec<FileId> = languages
            .keys()
            .filter_map(|span| span.id())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        files.sort_by_key(|id| (*id != main, project_path(*id)));
        files
            .into_iter()
            .filter_map(|id| {
                let source = self.world().source(id).ok()?;
                let runs = runs(&source)
                    .into_iter()
                    .filter_map(|(range, spans)| {
                        let language = spans.iter().find_map(|span| languages.get(span))?;
                        Some(Run {
                            range,
                            language: language.clone(),
                        })
                    })
                    .collect();
                Some(Prose {
                    source,
                    main: id == main,
                    runs,
                })
            })
            .collect()
    }
}

fn languages(document: &PagedDocument) -> HashMap<Span, Language> {
    let mut languages = HashMap::new();
    for page in document.pages() {
        collect_languages(&page.frame, &mut languages);
    }
    languages
}

fn collect_languages(frame: &Frame, languages: &mut HashMap<Span, Language>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => collect_languages(&group.frame, languages),
            FrameItem::Text(text) => {
                for glyph in &text.glyphs {
                    languages.entry(glyph.span.0).or_insert_with(|| Language {
                        language: text.lang.as_str().to_owned(),
                        region: text.region.map(|region| region.as_str().to_owned()),
                    });
                }
            }
            FrameItem::Shape(..)
            | FrameItem::Image(..)
            | FrameItem::Link(..)
            | FrameItem::Tag(_) => {}
        }
    }
}

fn runs(source: &Source) -> Vec<(Range<usize>, Vec<Span>)> {
    let mut runs = Vec::new();
    collect_runs(&LinkedNode::new(source.root()), &mut runs);
    runs
}

fn collect_runs(node: &LinkedNode<'_>, runs: &mut Vec<(Range<usize>, Vec<Span>)>) {
    if matches!(node.kind(), SyntaxKind::Raw | SyntaxKind::Equation) {
        return;
    }
    let markup = node.kind() == SyntaxKind::Markup;
    let mut current: Option<(Range<usize>, Vec<Span>)> = None;
    for child in node.children() {
        let prose = markup
            && matches!(
                child.kind(),
                SyntaxKind::Text | SyntaxKind::SmartQuote | SyntaxKind::Space
            );
        if !prose {
            runs.extend(current.take());
            collect_runs(&child, runs);
            continue;
        }
        let range = child.range();
        let span = (child.kind() == SyntaxKind::Text).then(|| child.span());
        match &mut current {
            Some((run, spans)) if run.end == range.start => {
                run.end = range.end;
                spans.extend(span);
            }
            _ => {
                runs.extend(current.take());
                current = Some((range, span.into_iter().collect()));
            }
        }
    }
    runs.extend(current);
}
