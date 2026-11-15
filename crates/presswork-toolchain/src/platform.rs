#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    os: &'static str,
    arch: &'static str,
}

impl Platform {
    #[must_use]
    pub fn new(os: &'static str, arch: &'static str) -> Self {
        Self { os, arch }
    }

    #[must_use]
    pub fn current() -> Self {
        Self::new(std::env::consts::OS, std::env::consts::ARCH)
    }

    #[must_use]
    pub fn release_file(&self) -> Option<String> {
        let (target, extension) = match (self.os, self.arch) {
            ("windows", "x86_64" | "aarch64") => (format!("{}-pc-windows-msvc", self.arch), "zip"),
            ("macos", "x86_64" | "aarch64") => (format!("{}-apple-darwin", self.arch), "tar.xz"),
            ("linux", "x86_64" | "aarch64") => {
                (format!("{}-unknown-linux-musl", self.arch), "tar.xz")
            }
            ("linux", "arm") => ("armv7-unknown-linux-musleabi".to_owned(), "tar.xz"),
            ("linux", "riscv64") => ("riscv64gc-unknown-linux-gnu".to_owned(), "tar.xz"),
            _ => return None,
        };
        Some(format!("typst-{target}.{extension}"))
    }

    #[must_use]
    pub fn executable(&self) -> &'static str {
        if self.os == "windows" {
            "typst.exe"
        } else {
            "typst"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_supported_platform_has_its_release_file() {
        for (os, arch, file) in [
            ("windows", "x86_64", "typst-x86_64-pc-windows-msvc.zip"),
            ("windows", "aarch64", "typst-aarch64-pc-windows-msvc.zip"),
            ("macos", "x86_64", "typst-x86_64-apple-darwin.tar.xz"),
            ("macos", "aarch64", "typst-aarch64-apple-darwin.tar.xz"),
            ("linux", "x86_64", "typst-x86_64-unknown-linux-musl.tar.xz"),
            (
                "linux",
                "aarch64",
                "typst-aarch64-unknown-linux-musl.tar.xz",
            ),
            ("linux", "arm", "typst-armv7-unknown-linux-musleabi.tar.xz"),
            (
                "linux",
                "riscv64",
                "typst-riscv64gc-unknown-linux-gnu.tar.xz",
            ),
        ] {
            assert_eq!(
                Platform::new(os, arch).release_file().as_deref(),
                Some(file),
                "{os} {arch}"
            );
        }
    }

    #[test]
    fn an_unsupported_platform_has_no_release_file() {
        assert_eq!(Platform::new("freebsd", "x86_64").release_file(), None);
        assert_eq!(Platform::new("linux", "powerpc64").release_file(), None);
    }

    #[test]
    fn the_executable_has_the_extension_of_the_platform() {
        assert_eq!(Platform::new("windows", "x86_64").executable(), "typst.exe");
        assert_eq!(Platform::new("linux", "x86_64").executable(), "typst");
    }

    #[test]
    fn the_current_platform_is_supported() {
        assert!(Platform::current().release_file().is_some());
    }
}
