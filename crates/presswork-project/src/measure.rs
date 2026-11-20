use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LengthUnit {
    Mm,
    Cm,
    In,
    Pt,
}

impl LengthUnit {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Cm => "cm",
            Self::In => "in",
            Self::Pt => "pt",
        }
    }

    #[must_use]
    pub fn format(self, micrometers: u32) -> String {
        let text = format!("{:.2}", f64::from(micrometers) / self.micrometers());
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    }

    fn micrometers(self) -> f64 {
        match self {
            Self::Mm => 1_000.0,
            Self::Cm => 10_000.0,
            Self::In => 25_400.0,
            Self::Pt => 25_400.0 / 72.0,
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "mm" => Some(Self::Mm),
            "cm" => Some(Self::Cm),
            "in" => Some(Self::In),
            "pt" => Some(Self::Pt),
            _ => None,
        }
    }
}

const LONGEST: f64 = 10_000_000.0;

fn length(text: &str) -> Option<(u32, LengthUnit)> {
    let text = text.trim();
    let (number, unit) =
        text.split_at(text.find(|character: char| character.is_ascii_alphabetic())?);
    let unit = LengthUnit::parse(unit.trim())?;
    let micrometers = (number.trim().parse::<f64>().ok()? * unit.micrometers()).round();
    (1.0..=LONGEST)
        .contains(&micrometers)
        .then(|| (whole(micrometers), unit))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the caller keeps the value between 1 and 10 000 000"
)]
fn whole(value: f64) -> u32 {
    value as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PageSize {
    pub width: u32,
    pub height: u32,
    pub unit: LengthUnit,
}

impl PageSize {
    pub fn parse(text: &str) -> Result<Self, String> {
        let invalid =
            || format!("the page size \"{text}\" is not valid, for example use \"148mm x 210mm\"");
        let (width, height) = text.split_once(['x', 'X', '×']).ok_or_else(invalid)?;
        let (width, unit) = length(width).ok_or_else(invalid)?;
        let (height, _) = length(height).ok_or_else(invalid)?;
        Ok(Self {
            width,
            height,
            unit,
        })
    }
}

impl TryFrom<String> for PageSize {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

impl From<PageSize> for String {
    fn from(size: PageSize) -> Self {
        size.to_string()
    }
}

impl fmt::Display for PageSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}{unit} x {}{unit}",
            self.unit.format(self.width),
            self.unit.format(self.height),
            unit = self.unit.name()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(width: u32, height: u32, unit: LengthUnit) -> PageSize {
        PageSize {
            width,
            height,
            unit,
        }
    }

    #[test]
    fn page_sizes_take_millimeters_inches_and_points() {
        assert_eq!(
            PageSize::parse("148mm x 210mm"),
            Ok(size(148_000, 210_000, LengthUnit::Mm))
        );
        assert_eq!(
            PageSize::parse("6in x 9in"),
            Ok(size(152_400, 228_600, LengthUnit::In))
        );
        assert_eq!(
            PageSize::parse("14.8 cm X 21 cm"),
            Ok(size(148_000, 210_000, LengthUnit::Cm))
        );
        assert_eq!(
            PageSize::parse("420pt x 595pt"),
            Ok(size(148_167, 209_903, LengthUnit::Pt))
        );
    }

    #[test]
    fn a_page_size_needs_two_lengths_with_units() {
        for text in [
            "148 x 210",
            "148mm",
            "a5",
            "0mm x 210mm",
            "-148mm x 210mm",
            "148km x 210mm",
        ] {
            assert!(PageSize::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn lengths_show_without_extra_zeros() {
        assert_eq!(LengthUnit::Mm.format(148_000), "148");
        assert_eq!(LengthUnit::Mm.format(148_250), "148.25");
        assert_eq!(LengthUnit::In.format(155_956), "6.14");
        assert_eq!(
            size(152_400, 228_600, LengthUnit::In).to_string(),
            "6in x 9in"
        );
    }
}
