pub(crate) fn line_column(text: &str, offset: usize) -> Option<(usize, usize)> {
    let before = text.get(..offset)?;
    let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
    Some((
        before.matches('\n').count() + 1,
        before[line_start..].chars().count() + 1,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_columns_start_at_one() {
        assert_eq!(line_column("Hello.", 0), Some((1, 1)));
        assert_eq!(line_column("One.\nTwo.", 6), Some((2, 2)));
    }

    #[test]
    fn columns_count_characters() {
        let text = "Größe: 10";

        assert_eq!(line_column(text, text.find(':').unwrap()), Some((1, 6)));
    }

    #[test]
    fn an_offset_inside_a_character_has_no_place() {
        assert_eq!(line_column("ö", 1), None);
        assert_eq!(line_column("Text.", 99), None);
    }
}
