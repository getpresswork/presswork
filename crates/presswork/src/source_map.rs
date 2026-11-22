use std::ops::Range;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceMap {
    removed: Vec<Range<usize>>,
}

impl SourceMap {
    pub(crate) fn removing(ranges: impl IntoIterator<Item = Range<usize>>) -> Self {
        let mut ranges: Vec<Range<usize>> = ranges
            .into_iter()
            .filter(|range| range.start < range.end)
            .collect();
        ranges.sort_by_key(|range| (range.start, range.end));
        let mut removed: Vec<Range<usize>> = Vec::new();
        for range in ranges {
            match removed.last_mut() {
                Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
                _ => removed.push(range),
            }
        }
        Self { removed }
    }

    pub(crate) fn text(&self, editor_text: &str) -> String {
        let mut text = String::with_capacity(editor_text.len());
        let mut at = 0;
        for range in &self.removed {
            text.push_str(&editor_text[at..range.start.min(editor_text.len())]);
            at = range.end.min(editor_text.len());
        }
        text.push_str(&editor_text[at..]);
        text
    }

    pub(crate) fn to_compiled(&self, offset: usize) -> usize {
        let removed: usize = self
            .removed
            .iter()
            .take_while(|range| range.start < offset)
            .map(|range| range.end.min(offset) - range.start)
            .sum();
        offset - removed
    }

    pub(crate) fn to_editor(&self, offset: usize, after_removed: bool) -> usize {
        let mut shift = 0;
        for range in &self.removed {
            let at = range.start - shift;
            if at < offset || (after_removed && at == offset) {
                shift += range.len();
            } else {
                break;
            }
        }
        offset + shift
    }

    pub(crate) fn range_to_editor(&self, range: Range<usize>) -> Range<usize> {
        let start = self.to_editor(range.start, true);
        start..self.to_editor(range.end, false).max(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EDITOR: &str = "The big cat sat.";

    fn without_big() -> SourceMap {
        SourceMap::removing(std::iter::once(4..8))
    }

    #[test]
    fn the_compiled_text_has_no_removed_text() {
        assert_eq!(without_big().text(EDITOR), "The cat sat.");
        assert_eq!(SourceMap::removing([12..16, 0..4]).text(EDITOR), "big cat ");
        assert_eq!(SourceMap::default().text(EDITOR), EDITOR);
    }

    #[test]
    fn offsets_move_over_the_removed_text() {
        let map = without_big();

        assert_eq!(map.to_compiled(2), 2);
        assert_eq!(map.to_compiled(6), 4);
        assert_eq!(map.to_compiled(9), 5);
        assert_eq!(map.to_editor(5, true), 9);
        assert_eq!(map.to_editor(2, false), 2);
    }

    #[test]
    fn a_range_at_removed_text_stays_outside_of_it() {
        let map = without_big();

        assert_eq!(map.range_to_editor(4..7), 8..11);
        assert_eq!(map.range_to_editor(0..4), 0..4);
        assert_eq!(map.to_editor(4, false), 4);
        assert_eq!(map.to_editor(4, true), 8);
    }
}
