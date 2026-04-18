use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Currency {
    Eur,
    Usd,
    Gbp,
    Jpy,
}

impl Currency {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Eur => "EUR",
            Self::Usd => "USD",
            Self::Gbp => "GBP",
            Self::Jpy => "JPY",
        }
    }

    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Eur => "€",
            Self::Usd => "$",
            Self::Gbp => "£",
            Self::Jpy => "¥",
        }
    }

    pub const fn minor_unit(self) -> u32 {
        match self {
            Self::Eur | Self::Usd | Self::Gbp => 2,
            Self::Jpy => 0,
        }
    }
}

impl TryFrom<String> for Currency {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.trim().to_ascii_uppercase().as_str() {
            "EUR" => Ok(Self::Eur),
            "USD" => Ok(Self::Usd),
            "GBP" => Ok(Self::Gbp),
            "JPY" => Ok(Self::Jpy),
            other => Err(format!(
                "unsupported currency '{other}'; supported: EUR, USD, GBP, JPY"
            )),
        }
    }
}

impl From<Currency> for String {
    fn from(c: Currency) -> Self {
        c.code().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_codes_case_insensitive() {
        assert_eq!(
            Currency::try_from("USD".to_string()).unwrap(),
            Currency::Usd
        );
        assert_eq!(
            Currency::try_from("eur".to_string()).unwrap(),
            Currency::Eur
        );
        assert_eq!(
            Currency::try_from(" jpy ".to_string()).unwrap(),
            Currency::Jpy
        );
    }

    #[test]
    fn rejects_unknown() {
        let err = Currency::try_from("XYZ".to_string()).unwrap_err();
        assert!(err.contains("XYZ"));
        assert!(err.contains("EUR"));
    }

    #[test]
    fn minor_units() {
        assert_eq!(Currency::Jpy.minor_unit(), 0);
        assert_eq!(Currency::Usd.minor_unit(), 2);
        assert_eq!(Currency::Eur.minor_unit(), 2);
    }
}
