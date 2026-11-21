use unicode_segmentation::UnicodeSegmentation;

pub fn words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split_word_bound_indices()
        .filter(|(_, word)| is_checked(word))
}

fn is_checked(word: &str) -> bool {
    let mut letters = 0;
    let mut capitals = 0;
    for character in word.chars() {
        if character.is_numeric() {
            return false;
        }
        if character.is_alphabetic() {
            letters += 1;
            if character.is_uppercase() {
                capitals += 1;
            }
        }
    }
    letters == 1 || (letters > 1 && capitals < letters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_have_letters_and_their_byte_offsets() {
        let found: Vec<(usize, &str)> =
            words("Hello, world! It's a cat's toy, and “don’t” too.").collect();

        assert_eq!(
            found,
            [
                (0, "Hello"),
                (7, "world"),
                (14, "It's"),
                (19, "a"),
                (21, "cat's"),
                (27, "toy"),
                (32, "and"),
                (39, "don’t"),
                (50, "too")
            ]
        );
    }

    #[test]
    fn numbers_and_abbreviations_in_capitals_are_not_words() {
        let found: Vec<&str> = words("The PDF of 3D models from 2026, version A.")
            .map(|(_, word)| word)
            .collect();

        assert_eq!(found, ["The", "of", "models", "from", "version", "A"]);
    }

    #[test]
    fn words_of_other_scripts_count() {
        let found: Vec<&str> = words("Straße – Привет мир").map(|(_, word)| word).collect();

        assert_eq!(found, ["Straße", "Привет", "мир"]);
    }
}
