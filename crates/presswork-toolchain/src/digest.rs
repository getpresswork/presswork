use std::fmt::{self, Write as _};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    #[must_use]
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let mut bytes = [0; 32];
        for (byte, pair) in bytes.iter_mut().zip(hex.as_bytes().chunks(2)) {
            *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
        }
        Some(Self(bytes))
    }

    #[must_use]
    pub fn from_github(text: &str) -> Option<Self> {
        text.strip_prefix("sha256:").and_then(Self::from_hex)
    }

    #[must_use]
    pub fn to_hex(&self) -> String {
        self.0
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            })
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Digest({})", self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "19ce3551153c2fe7ee9fa2f95208310c8f4d3209fedb699e0333faf8913f6736";

    #[test]
    fn a_digest_reads_64_hex_digits() {
        let digest = Digest::from_hex(HEX).unwrap();

        assert_eq!(digest.to_hex(), HEX);
        assert_eq!(Digest::from_hex(&HEX.to_uppercase()), Some(digest));
    }

    #[test]
    fn a_digest_with_a_wrong_length_or_letter_is_not_valid() {
        assert_eq!(Digest::from_hex(&HEX[1..]), None);
        assert_eq!(Digest::from_hex(&format!("{}g", &HEX[1..])), None);
    }

    #[test]
    fn a_github_digest_must_use_sha256() {
        assert_eq!(
            Digest::from_github(&format!("sha256:{HEX}")),
            Digest::from_hex(HEX)
        );
        assert_eq!(Digest::from_github(&format!("sha512:{HEX}")), None);
        assert_eq!(Digest::from_github(HEX), None);
    }
}
