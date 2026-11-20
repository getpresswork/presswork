use std::collections::BTreeMap;
use std::ops::Range;

use presswork_text::{Buffer, Position};
use typst_syntax::{Lines, LinkedNode, Source, SyntaxKind, ast};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Region {
    pub(crate) line: usize,
    pub(crate) hidden: Range<usize>,
}

impl Region {
    pub(crate) fn positions(&self) -> Range<Position> {
        Position::new(self.hidden.start)..Position::new(self.hidden.end)
    }
}

pub(crate) fn header_line(buffer: &Buffer, fold: &Range<Position>) -> usize {
    buffer.line_of(fold.start).saturating_sub(1)
}

pub(crate) fn region_at(regions: &[Region], line: usize) -> Option<&Region> {
    regions
        .binary_search_by_key(&line, |region| region.line)
        .ok()
        .map(|index| &regions[index])
}

pub(crate) fn innermost<'a>(
    regions: &'a [Region],
    buffer: &Buffer,
    offset: usize,
    folds: &[Range<Position>],
) -> Option<&'a Region> {
    regions
        .iter()
        .filter(|region| {
            buffer.line_start(region.line).offset() <= offset && offset < region.hidden.end
        })
        .rfind(|region| !folds.contains(&region.positions()))
}

pub(crate) fn regions(source: &Source) -> Vec<Region> {
    let lines = source.lines();
    let mut last_lines = BTreeMap::new();
    headings(source, &mut last_lines);
    blocks(&LinkedNode::new(source.root()), lines, &mut last_lines);
    last_lines
        .into_iter()
        .filter_map(|(line, last)| {
            let start = lines.line_to_byte(line + 1)?;
            let end = lines.line_to_byte(last + 1).unwrap_or(source.text().len());
            (start < end).then_some(Region {
                line,
                hidden: start..end,
            })
        })
        .collect()
}

fn keep(last_lines: &mut BTreeMap<usize, usize>, line: usize, last: usize) {
    if last > line {
        let kept = last_lines.entry(line).or_insert(last);
        *kept = (*kept).max(last);
    }
}

fn headings(source: &Source, last_lines: &mut BTreeMap<usize, usize>) {
    let lines = source.lines();
    let marks: Vec<(usize, usize)> = LinkedNode::new(source.root())
        .children()
        .filter_map(|child| {
            let heading = child.cast::<ast::Heading<'_>>()?;
            Some((lines.byte_to_line(child.offset())?, heading.depth().get()))
        })
        .collect();
    for (index, (line, depth)) in marks.iter().enumerate() {
        let next = marks[index + 1..]
            .iter()
            .find(|(_, other)| other <= depth)
            .map_or(lines.len_lines(), |(next, _)| *next);
        let mut last = next.saturating_sub(1);
        while last > *line && is_blank(source, last) {
            last -= 1;
        }
        keep(last_lines, *line, last);
    }
}

fn is_blank(source: &Source, line: usize) -> bool {
    source
        .lines()
        .line_to_range(line)
        .is_none_or(|range| source.text()[range].trim().is_empty())
}

fn blocks(node: &LinkedNode<'_>, lines: &Lines<String>, last_lines: &mut BTreeMap<usize, usize>) {
    let foldable = matches!(
        node.kind(),
        SyntaxKind::CodeBlock
            | SyntaxKind::ContentBlock
            | SyntaxKind::Args
            | SyntaxKind::Params
            | SyntaxKind::Array
            | SyntaxKind::Dict
            | SyntaxKind::Parenthesized
            | SyntaxKind::Raw
            | SyntaxKind::Equation
            | SyntaxKind::BlockComment
    );
    if foldable
        && let (Some(first), Some(last)) = (
            lines.byte_to_line(node.offset()),
            lines.byte_to_line(node.range().end),
        )
    {
        keep(last_lines, first, last.saturating_sub(1));
    }
    for child in node.children() {
        blocks(&child, lines, last_lines);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folds(text: &str) -> Vec<(usize, &str)> {
        regions(&Source::detached(text))
            .into_iter()
            .map(|region| (region.line, &text[region.hidden]))
            .collect()
    }

    #[test]
    fn a_heading_folds_its_text_until_the_next_heading_of_its_level() {
        let text = "= A\ntext\n== B\nmore\n\n= C\nend";

        assert_eq!(
            folds(text),
            [(0, "text\n== B\nmore\n"), (2, "more\n"), (5, "end")]
        );
    }

    #[test]
    fn a_block_folds_the_lines_between_its_brackets() {
        let text = "#figure(\n  image(\"a.png\"),\n  caption: [A],\n)";

        assert_eq!(folds(text), [(0, "  image(\"a.png\"),\n  caption: [A],\n")]);
    }

    #[test]
    fn a_block_on_two_lines_does_not_fold() {
        assert_eq!(folds("#f(\n)"), Vec::<(usize, &str)>::new());
    }

    #[test]
    fn raw_blocks_and_comments_fold() {
        assert_eq!(folds("```\na\nb\n```"), [(0, "a\nb\n")]);
        assert_eq!(folds("/*\nx\n*/"), [(0, "x\n")]);
    }

    #[test]
    fn regions_that_start_on_one_line_keep_the_longest() {
        let text = "= A #[\nx\ny\n]\nmore";

        assert_eq!(folds(text), [(0, "x\ny\n]\nmore")]);
    }
}
