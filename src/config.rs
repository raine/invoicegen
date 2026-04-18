use anyhow::{Context, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use crate::currency::Currency;
use crate::domain::{InvoicePatch, PartyPatch};
use crate::locale::Locale;
use crate::paths::expand_tilde;
use crate::xdg;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub sender: SenderConfig,
    pub defaults: DefaultsConfig,
    pub clients: BTreeMap<String, ClientTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct SenderConfig {
    pub name: String,
    pub address: Option<String>,
    pub logo: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DefaultsConfig {
    pub currency: Currency,
    pub locale: Locale,
    pub date_format: String,
    pub output_dir: PathBuf,
    pub tax_rate: Decimal,
    pub tax_note: Option<String>,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            currency: Currency::Eur,
            locale: Locale::EnUs,
            date_format: "%b %-d, %Y".to_string(),
            output_dir: PathBuf::from("pdf"),
            tax_rate: Decimal::ZERO,
            tax_note: Some("VAT 0%, Export of goods or services".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ClientTemplate {
    pub bill_to: Option<String>,
    pub ship_to: Option<String>,
    pub default_rate: Option<Decimal>,
}

impl AppConfig {
    pub fn defaults_patch(&self) -> InvoicePatch {
        InvoicePatch {
            sender: PartyPatch {
                name: Some(self.sender.name.clone()),
                address: self.sender.address.clone(),
                logo_path: self.sender.logo.as_ref().map(|p| expand_tilde(p)),
            },
            tax_rate: Some(self.defaults.tax_rate),
            tax_note: self.defaults.tax_note.clone(),
            currency: Some(self.defaults.currency),
            locale: Some(self.defaults.locale),
            date_format: Some(self.defaults.date_format.clone()),
            ..InvoicePatch::default()
        }
    }

    pub fn client_patch(&self, key: &str) -> Option<InvoicePatch> {
        let template = self.clients.get(key)?;
        Some(InvoicePatch {
            bill_to: template.bill_to.clone(),
            ship_to: template.ship_to.clone(),
            default_rate: template.default_rate,
            ..InvoicePatch::default()
        })
    }

    pub fn client_keys(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }
}

pub fn default_config_path() -> Result<PathBuf> {
    Ok(xdg::config_dir()?.join("config.yaml"))
}

pub fn load(path: &Path) -> Result<AppConfig> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading config {}", path.display()))?;
    let de = serde_yml::Deserializer::from_str(&text);
    let cfg: AppConfig = serde_path_to_error::deserialize(de)
        .with_context(|| format!("parsing config {}", path.display()))?;
    Ok(cfg)
}

/// Load the config if it exists; otherwise return defaults.
pub fn load_or_default(path: &Path) -> Result<AppConfig> {
    if path.exists() {
        load(path)
    } else {
        Ok(AppConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Result<AppConfig> {
        let de = serde_yml::Deserializer::from_str(yaml);
        Ok(serde_path_to_error::deserialize(de)?)
    }

    #[test]
    fn missing_locale_defaults_to_en_us() {
        let cfg = parse("defaults:\n  currency: EUR\n").unwrap();
        assert_eq!(cfg.defaults.currency, Currency::Eur);
        assert_eq!(cfg.defaults.locale, Locale::EnUs);
    }

    #[test]
    fn existing_currency_strings_still_work() {
        for code in ["USD", "EUR", "GBP"] {
            let yaml = format!("defaults:\n  currency: {code}\n");
            let cfg = parse(&yaml).unwrap_or_else(|e| panic!("{code}: {e}"));
            assert_eq!(cfg.defaults.currency.code(), code);
        }
    }

    #[test]
    fn unknown_currency_is_rejected() {
        let err = parse("defaults:\n  currency: XYZ\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("XYZ"), "got: {err}");
    }

    #[test]
    fn locale_parses() {
        let cfg = parse("defaults:\n  currency: EUR\n  locale: fi-FI\n").unwrap();
        assert_eq!(cfg.defaults.locale, Locale::FiFi);
    }

    #[test]
    fn unknown_locale_is_rejected() {
        let err = parse("defaults:\n  currency: EUR\n  locale: xx-XX\n")
            .unwrap_err()
            .to_string();
        assert!(err.to_lowercase().contains("locale"), "got: {err}");
    }
}
