use presswork_api::{LabelStyle, PageRange};

const ROMAN: [(u32, &str); 13] = [
    (1000, "m"),
    (900, "cm"),
    (500, "d"),
    (400, "cd"),
    (100, "c"),
    (90, "xc"),
    (50, "l"),
    (40, "xl"),
    (10, "x"),
    (9, "ix"),
    (5, "v"),
    (4, "iv"),
    (1, "i"),
];
const MAX_ROMAN: u32 = 3999;
const MAX_LETTER_REPEATS: u32 = 10;

#[must_use]
pub fn page_labels(pdf: &[PageRange], corrections: &[PageRange], pages: u32) -> Vec<String> {
    let mut ranges: Vec<&PageRange> = pdf.iter().chain(corrections).collect();
    ranges.sort_by_key(|range| range.first_page);
    (1..=pages)
        .map(|page| {
            ranges
                .iter()
                .rev()
                .find(|range| range.first_page <= page)
                .map_or_else(|| page.to_string(), |range| label(range, page))
        })
        .collect()
}

fn label(range: &PageRange, page: u32) -> String {
    let number = range.start.saturating_add(page - range.first_page);
    let text = match range.style {
        LabelStyle::Arabic => number.to_string(),
        LabelStyle::UpperRoman => roman(number).to_uppercase(),
        LabelStyle::LowerRoman => roman(number),
        LabelStyle::UpperLetters => letters(number).to_uppercase(),
        LabelStyle::LowerLetters => letters(number),
        LabelStyle::None => String::new(),
    };
    format!("{}{text}", range.prefix)
}

fn roman(number: u32) -> String {
    if number == 0 || number > MAX_ROMAN {
        return number.to_string();
    }
    let mut rest = number;
    let mut text = String::new();
    for (value, digits) in ROMAN {
        while rest >= value {
            text.push_str(digits);
            rest -= value;
        }
    }
    text
}

fn letters(number: u32) -> String {
    let Some(index) = number.checked_sub(1) else {
        return number.to_string();
    };
    let repeats = index / 26 + 1;
    if repeats > MAX_LETTER_REPEATS {
        return number.to_string();
    }
    let letter = char::from(b'a' + u8::try_from(index % 26).unwrap_or(0));
    std::iter::repeat_n(letter, usize::try_from(repeats).unwrap_or(1)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(first_page: u32, style: LabelStyle, prefix: &str, start: u32) -> PageRange {
        PageRange {
            first_page,
            style,
            prefix: prefix.to_owned(),
            start,
        }
    }

    #[test]
    fn pages_without_ranges_have_their_pdf_numbers() {
        assert_eq!(page_labels(&[], &[], 3), ["1", "2", "3"]);
    }

    #[test]
    fn the_ranges_of_the_pdf_file_give_the_labels() {
        let pdf = [
            range(1, LabelStyle::LowerRoman, "", 1),
            range(5, LabelStyle::Arabic, "", 1),
            range(7, LabelStyle::UpperRoman, "", 4),
        ];

        assert_eq!(
            page_labels(&pdf, &[], 8),
            ["i", "ii", "iii", "iv", "1", "2", "IV", "V"]
        );
    }

    #[test]
    fn a_label_can_have_a_prefix_or_letters() {
        let pdf = [
            range(1, LabelStyle::Arabic, "A-", 8),
            range(3, LabelStyle::LowerLetters, "", 25),
            range(7, LabelStyle::None, "Cover", 1),
        ];

        assert_eq!(
            page_labels(&pdf, &[], 7),
            ["A-8", "A-9", "y", "z", "aa", "bb", "Cover"]
        );
    }

    #[test]
    fn a_correction_takes_the_place_of_the_labels_from_its_page() {
        let pdf = [range(1, LabelStyle::Arabic, "", 1)];
        let corrections = [range(15, LabelStyle::Arabic, "", 1)];

        let labels = page_labels(&pdf, &corrections, 16);

        assert_eq!(labels[13], "14");
        assert_eq!(labels[14..], ["1", "2"]);
    }

    #[test]
    fn a_correction_wins_over_a_range_of_the_pdf_file_on_the_same_page() {
        let pdf = [range(1, LabelStyle::Arabic, "", 1)];
        let corrections = [range(1, LabelStyle::LowerRoman, "", 1)];

        assert_eq!(page_labels(&pdf, &corrections, 2), ["i", "ii"]);
    }

    #[test]
    fn numbers_without_a_short_form_use_digits() {
        let pdf = [
            range(1, LabelStyle::UpperRoman, "", 3999),
            range(3, LabelStyle::LowerLetters, "", 26 * 10 + 1),
        ];

        assert_eq!(page_labels(&pdf, &[], 3), ["MMMCMXCIX", "4000", "261"]);
    }
}
