const TWO_LETTER: &[(&str, &str)] = &[
    ("af", "afr"),
    ("am", "amh"),
    ("ar", "ara"),
    ("as", "asm"),
    ("az", "aze"),
    ("be", "bel"),
    ("bg", "bul"),
    ("bn", "ben"),
    ("bo", "bod"),
    ("br", "bre"),
    ("bs", "bos"),
    ("ca", "cat"),
    ("co", "cos"),
    ("cs", "ces"),
    ("cy", "cym"),
    ("da", "dan"),
    ("de", "deu"),
    ("dv", "div"),
    ("dz", "dzo"),
    ("el", "ell"),
    ("en", "eng"),
    ("eo", "epo"),
    ("es", "spa"),
    ("et", "est"),
    ("eu", "eus"),
    ("fa", "fas"),
    ("fi", "fin"),
    ("fo", "fao"),
    ("fr", "fra"),
    ("fy", "fry"),
    ("ga", "gle"),
    ("gd", "gla"),
    ("gl", "glg"),
    ("gu", "guj"),
    ("he", "heb"),
    ("hi", "hin"),
    ("hr", "hrv"),
    ("ht", "hat"),
    ("hu", "hun"),
    ("hy", "hye"),
    ("id", "ind"),
    ("is", "isl"),
    ("it", "ita"),
    ("iu", "iku"),
    ("ja", "jpn"),
    ("jv", "jav"),
    ("ka", "kat"),
    ("kk", "kaz"),
    ("km", "khm"),
    ("kn", "kan"),
    ("ko", "kor"),
    ("ku", "kmr"),
    ("ky", "kir"),
    ("la", "lat"),
    ("lb", "ltz"),
    ("lo", "lao"),
    ("lt", "lit"),
    ("lv", "lav"),
    ("mi", "mri"),
    ("mk", "mkd"),
    ("ml", "mal"),
    ("mn", "mon"),
    ("mr", "mar"),
    ("ms", "msa"),
    ("mt", "mlt"),
    ("my", "mya"),
    ("nb", "nor"),
    ("ne", "nep"),
    ("nl", "nld"),
    ("nn", "nor"),
    ("no", "nor"),
    ("oc", "oci"),
    ("or", "ori"),
    ("pa", "pan"),
    ("pl", "pol"),
    ("ps", "pus"),
    ("pt", "por"),
    ("qu", "que"),
    ("ro", "ron"),
    ("ru", "rus"),
    ("sa", "san"),
    ("sd", "snd"),
    ("si", "sin"),
    ("sk", "slk"),
    ("sl", "slv"),
    ("sq", "sqi"),
    ("sr", "srp"),
    ("su", "sun"),
    ("sv", "swe"),
    ("sw", "swa"),
    ("ta", "tam"),
    ("te", "tel"),
    ("tg", "tgk"),
    ("th", "tha"),
    ("ti", "tir"),
    ("tl", "tgl"),
    ("to", "ton"),
    ("tr", "tur"),
    ("tt", "tat"),
    ("ug", "uig"),
    ("uk", "ukr"),
    ("ur", "urd"),
    ("uz", "uzb"),
    ("vi", "vie"),
    ("yi", "yid"),
    ("yo", "yor"),
];

const BIBLIOGRAPHIC: &[(&str, &str)] = &[
    ("alb", "sqi"),
    ("arm", "hye"),
    ("baq", "eus"),
    ("bur", "mya"),
    ("cze", "ces"),
    ("dut", "nld"),
    ("fre", "fra"),
    ("geo", "kat"),
    ("ger", "deu"),
    ("gre", "ell"),
    ("ice", "isl"),
    ("mac", "mkd"),
    ("mao", "mri"),
    ("may", "msa"),
    ("per", "fas"),
    ("rum", "ron"),
    ("slo", "slk"),
    ("tib", "bod"),
    ("wel", "cym"),
];

const TRADITIONAL: &[&str] = &["hant", "tw", "hk", "mo", "tra"];

fn lookup(table: &[(&str, &str)], code: &str) -> Option<String> {
    table
        .iter()
        .find(|(key, _)| *key == code)
        .map(|(_, tesseract)| (*tesseract).to_owned())
}

#[must_use]
pub fn tesseract_language(language: &str) -> Option<String> {
    let tag = language.trim().to_ascii_lowercase().replace('_', "-");
    let mut subtags = tag.split('-');
    let primary = subtags.next()?;
    let rest: Vec<&str> = subtags.collect();
    let has = |subtag: &str| rest.contains(&subtag);
    let special = match primary {
        "zh" | "chi" | "zho" if rest.iter().any(|subtag| TRADITIONAL.contains(subtag)) => {
            Some("chi_tra")
        }
        "zh" | "chi" | "zho" => Some("chi_sim"),
        "sr" | "srp" if has("latn") => Some("srp_latn"),
        "uz" | "uzb" if has("cyrl") => Some("uzb_cyrl"),
        "az" | "aze" if has("cyrl") => Some("aze_cyrl"),
        _ => None,
    };
    special
        .map(str::to_owned)
        .or_else(|| lookup(TWO_LETTER, primary))
        .or_else(|| lookup(BIBLIOGRAPHIC, primary))
        .or_else(|| {
            (primary.len() == 3 && primary.bytes().all(|byte| byte.is_ascii_lowercase()))
                .then(|| primary.to_owned())
        })
}

#[must_use]
pub fn languages_for(language: &str, installed: &[String], fallback: &[String]) -> Vec<String> {
    let exact = language.trim().to_ascii_lowercase();
    let wanted = [Some(exact), tesseract_language(language)];
    wanted
        .into_iter()
        .flatten()
        .find(|code| installed.contains(code))
        .map_or_else(|| fallback.to_vec(), |code| vec![code])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(languages: &[&str]) -> Vec<String> {
        languages
            .iter()
            .map(|&language| language.to_owned())
            .collect()
    }

    #[test]
    fn an_edition_language_gives_its_tesseract_language() {
        for (language, expected) in [
            ("de", "deu"),
            ("de-AT", "deu"),
            ("EN_us", "eng"),
            (" la ", "lat"),
            ("nb", "nor"),
            ("zh", "chi_sim"),
            ("zh-Hans-CN", "chi_sim"),
            ("zh-Hant", "chi_tra"),
            ("zh-TW", "chi_tra"),
            ("sr-Latn", "srp_latn"),
            ("sr", "srp"),
            ("ger", "deu"),
            ("deu", "deu"),
            ("grc", "grc"),
        ] {
            assert_eq!(
                tesseract_language(language).as_deref(),
                Some(expected),
                "{language}"
            );
        }
        for language in ["", "x", "German", "1234", "de!"] {
            assert_eq!(tesseract_language(language), None, "{language}");
        }
    }

    #[test]
    fn an_installed_edition_language_has_priority_over_the_fallback() {
        let installed = owned(&["deu", "deu_latf", "eng"]);
        let fallback = owned(&["eng"]);

        assert_eq!(languages_for("de", &installed, &fallback), ["deu"]);
        assert_eq!(
            languages_for("deu_latf", &installed, &fallback),
            ["deu_latf"]
        );
        assert_eq!(languages_for("fr", &installed, &fallback), ["eng"]);
        assert_eq!(languages_for("", &installed, &fallback), ["eng"]);
        assert_eq!(
            languages_for("Latin", &installed, &owned(&["eng", "lat"])),
            ["eng", "lat"]
        );
    }
}
