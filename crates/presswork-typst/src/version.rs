#[must_use]
pub fn version() -> &'static str {
    typst::utils::version().raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_version_has_three_numbers() {
        let numbers: Vec<&str> = version().split('.').collect();

        assert_eq!(numbers.len(), 3);
        assert!(numbers.iter().all(|number| number.parse::<u32>().is_ok()));
    }
}
