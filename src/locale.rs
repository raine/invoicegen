use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolPosition {
    Prefix,
    Suffix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Locale {
    EnUs,
    FiFi,
    DeDe,
    JaJp,
}

impl Locale {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::FiFi => "fi-FI",
            Self::DeDe => "de-DE",
            Self::JaJp => "ja-JP",
        }
    }

    pub const fn group_sep(self) -> char {
        match self {
            Self::EnUs | Self::JaJp => ',',
            Self::FiFi => '\u{00A0}',
            Self::DeDe => '.',
        }
    }

    pub const fn decimal_sep(self) -> char {
        match self {
            Self::EnUs | Self::JaJp => '.',
            Self::FiFi | Self::DeDe => ',',
        }
    }

    pub const fn symbol_position(self) -> SymbolPosition {
        match self {
            Self::EnUs | Self::JaJp => SymbolPosition::Prefix,
            Self::FiFi | Self::DeDe => SymbolPosition::Suffix,
        }
    }
}

impl TryFrom<String> for Locale {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let normalized: String = value
            .trim()
            .chars()
            .map(|c| if c == '_' { '-' } else { c })
            .collect();
        match normalized.to_ascii_lowercase().as_str() {
            "en-us" => Ok(Self::EnUs),
            "fi-fi" => Ok(Self::FiFi),
            "de-de" => Ok(Self::DeDe),
            "ja-jp" => Ok(Self::JaJp),
            other => Err(format!(
                "unsupported locale '{other}'; supported: en-US, fi-FI, de-DE, ja-JP"
            )),
        }
    }
}

impl From<Locale> for String {
    fn from(l: Locale) -> Self {
        l.code().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_locales() {
        assert_eq!(Locale::try_from("en-US".to_string()).unwrap(), Locale::EnUs);
        assert_eq!(Locale::try_from("fi-FI".to_string()).unwrap(), Locale::FiFi);
        assert_eq!(Locale::try_from("en_US".to_string()).unwrap(), Locale::EnUs);
        assert_eq!(Locale::try_from("ja-jp".to_string()).unwrap(), Locale::JaJp);
    }

    #[test]
    fn rejects_unknown_locale() {
        assert!(Locale::try_from("xx-XX".to_string()).is_err());
    }
}
