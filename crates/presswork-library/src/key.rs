#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "a citation key must have 1 to 64 lowercase letters, digits, and single hyphens, and start with a letter"
)]
pub struct KeyError;

const MAX_CHARACTERS: usize = 64;
const FALLBACK: &str = "work";

pub fn parse_key(key: &str) -> Result<&str, KeyError> {
    let key = key.trim();
    let valid = key.len() <= MAX_CHARACTERS
        && key.starts_with(|character: char| character.is_ascii_lowercase())
        && !key.ends_with('-')
        && !key.contains("--")
        && key.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        });
    if valid { Ok(key) } else { Err(KeyError) }
}

#[must_use]
pub fn propose_key(title: &str) -> String {
    let mut key = String::new();
    for word in folded(title)
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
    {
        if key.len() + 1 + word.len() > MAX_CHARACTERS {
            break;
        }
        if !key.is_empty() {
            key.push('-');
        }
        key.push_str(&word.to_ascii_lowercase());
    }
    if key.is_empty() {
        return FALLBACK.to_owned();
    }
    if !key.starts_with(|character: char| character.is_ascii_lowercase()) {
        key = format!("{FALLBACK}-{key}");
    }
    while key.len() > MAX_CHARACTERS {
        let end = key.rfind('-').unwrap_or(MAX_CHARACTERS);
        key.truncate(end);
    }
    key
}

fn folded(title: &str) -> String {
    let mut text = String::with_capacity(title.len());
    for character in title.chars() {
        match character {
            'ß' => text.push_str("ss"),
            'æ' | 'Æ' => text.push_str("ae"),
            'œ' | 'Œ' => text.push_str("oe"),
            _ => text.push(base_letter(character)),
        }
    }
    text
}

fn base_letter(character: char) -> char {
    match character {
        'à'..='å' | 'À'..='Å' => 'a',
        'ç' | 'Ç' => 'c',
        'è'..='ë' | 'È'..='Ë' => 'e',
        'ì'..='ï' | 'Ì'..='Ï' => 'i',
        'ñ' | 'Ñ' => 'n',
        'ò'..='ö' | 'Ò'..='Ö' | 'ø' | 'Ø' => 'o',
        'ù'..='ü' | 'Ù'..='Ü' => 'u',
        'ý' | 'ÿ' | 'Ý' => 'y',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn a_key_has_lowercase_letters_digits_and_hyphens() {
        for key in ["war-and-peace", "tolstoy1869", "a", " field-notes "] {
            assert_eq!(parse_key(key), Ok(key.trim()), "{key}");
        }
    }

    #[test]
    fn a_key_with_other_characters_is_refused() {
        for key in [
            "",
            "War",
            "1984",
            "war--and-peace",
            "war-",
            "war and peace",
            "war_and_peace",
            "größe",
            &"a".repeat(65),
        ] {
            assert_eq!(parse_key(key), Err(KeyError), "{key}");
        }
        assert_eq!(parse_key(&"a".repeat(64)).map(str::len), Ok(64));
    }

    #[test]
    fn the_proposal_comes_from_the_words_of_the_title() {
        assert_eq!(propose_key("War and Peace"), "war-and-peace");
        assert_eq!(
            propose_key("  The Book: A History!  "),
            "the-book-a-history"
        );
        assert_eq!(propose_key("Größe und Gewicht"), "grosse-und-gewicht");
        assert_eq!(propose_key("Élan vital"), "elan-vital");
    }

    #[test]
    fn a_proposal_starts_with_a_letter() {
        assert_eq!(propose_key("1984"), "work-1984");
        assert_eq!(propose_key("Война и мир"), "work");
        assert_eq!(propose_key(""), "work");
    }

    #[test]
    fn a_long_title_gives_a_short_key() {
        let key = propose_key(&"paper ".repeat(40));

        assert!(key.len() <= 64, "{key}");
        assert!(key.ends_with("paper"), "{key}");
    }

    proptest! {
        #[test]
        fn each_proposal_is_a_valid_key(title in any::<String>()) {
            let key = propose_key(&title);
            prop_assert_eq!(parse_key(&key), Ok(key.as_str()));
        }
    }
}
