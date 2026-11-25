use std::process::Command;

pub(crate) fn is_web_link(url: &str) -> bool {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .is_some_and(|rest| {
            !rest.is_empty()
                && !url
                    .chars()
                    .any(|character| character.is_whitespace() || character.is_control())
        })
}

pub(crate) fn open(url: &str) -> bool {
    if !is_web_link(url) {
        return false;
    }
    let mut command = opener();
    match command.arg(url).spawn() {
        Ok(_) => true,
        Err(error) => {
            tracing::warn!(%error, "The link did not open");
            false
        }
    }
}

#[cfg(windows)]
fn opener() -> Command {
    let mut command = Command::new("rundll32");
    command.arg("url.dll,FileProtocolHandler");
    command
}

#[cfg(target_os = "macos")]
fn opener() -> Command {
    Command::new("open")
}

#[cfg(not(any(windows, target_os = "macos")))]
fn opener() -> Command {
    Command::new("xdg-open")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_links_open() {
        for url in ["https://typst.app/universe", "http://example.com/a?b=c&d=e"] {
            assert!(is_web_link(url), "{url}");
        }
        for url in [
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "./docs/manual.md",
            "#usage",
            "https://example.com/a b",
            "https://example.com/\n",
            "https://",
            "",
        ] {
            assert!(!is_web_link(url), "{url:?}");
        }
    }
}
