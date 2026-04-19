use anyhow::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use crate::diagnostics::{ConfigError, format_yaml_path_error};

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
    pub output_dir: Option<PathBuf>,
    pub tax_rate: Decimal,
    pub tax_note: Option<String>,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            currency: Currency::Eur,
            locale: Locale::EnUs,
            date_format: "%b %-d, %Y".to_string(),
            output_dir: None,
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
    let text = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let de = serde_yml::Deserializer::from_str(&text);
    let cfg: AppConfig =
        serde_path_to_error::deserialize(de).map_err(|error| ConfigError::Parse {
            path: path.to_path_buf(),
            message: format_yaml_path_error(error),
        })?;
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
    use crate::diagnostics::ConfigError;

    fn parse(yaml: &str) -> Result<AppConfig> {
        let de = serde_yml::Deserializer::from_str(yaml);
        Ok(serde_path_to_error::deserialize(de)?)
    }

    fn load_from_str(yaml: &str) -> Result<AppConfig> {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        let path = std::env::temp_dir().join(format!(
            "invoicegen-config-test-{}-{}.yaml",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, yaml).unwrap();
        let result = load(&path);
        let _ = std::fs::remove_file(&path);
        result
    }

    #[test]
    fn load_reports_field_path_for_invalid_config() {
        let err = load_from_str("defaults:\n  locale: xx-XX\n").unwrap_err();
        let err = err.downcast::<ConfigError>().unwrap();
        assert!(err.to_string().contains("invalid config file"));
        assert!(err.to_string().contains("defaults.locale"));
        assert!(err.to_string().to_lowercase().contains("locale"));
    }

    #[test]
    fn load_reports_missing_config_file_path() {
        let missing = Path::new("definitely-missing-config.yaml");
        let err = load(missing).unwrap_err();
        let err = err.downcast::<ConfigError>().unwrap();
        assert!(err.to_string().contains("could not read config file"));
        assert!(err.to_string().contains("definitely-missing-config.yaml"));
    }

    #[test]
    fn parse_reports_unknown_field_path() {
        let err = load_from_str("defaults:\n  currency: EUR\nextra: 1\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("extra"), "got: {err}");
    }

    #[test]
    fn parse_reports_output_dir_field_path() {
        let err = load_from_str("defaults:\n  currency: EUR\n  output_dir: [1, 2]\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.output_dir"), "got: {err}");
    }

    #[test]
    fn parse_reports_client_default_rate_field_path() {
        let err = load_from_str(
            "defaults:\n  currency: EUR\nclients:\n  acme:\n    default_rate: nope\n",
        )
        .unwrap_err()
        .downcast::<ConfigError>()
        .unwrap()
        .to_string();
        assert!(err.contains("clients.acme.default_rate"), "got: {err}");
    }

    #[test]
    fn parse_reports_sender_logo_field_path() {
        let err = load_from_str("sender:\n  logo: [1, 2]\ndefaults:\n  currency: EUR\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("sender.logo"), "got: {err}");
    }

    #[test]
    fn parse_reports_clients_block_field_path() {
        let err = load_from_str("defaults:\n  currency: EUR\nclients: []\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("clients"), "got: {err}");
    }

    #[test]
    fn parse_reports_defaults_block_path() {
        let err = load_from_str("defaults: []\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults"), "got: {err}");
    }

    #[test]
    fn parse_reports_sender_block_path() {
        let err = load_from_str("sender: []\ndefaults:\n  currency: EUR\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("sender"), "got: {err}");
    }

    #[test]
    fn parse_reports_currency_field_path() {
        let err = load_from_str("defaults:\n  currency: XYZ\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.currency"), "got: {err}");
    }

    #[test]
    fn parse_reports_locale_field_path() {
        let err = load_from_str("defaults:\n  currency: EUR\n  locale: xx-XX\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.locale"), "got: {err}");
    }

    #[test]
    fn parse_reports_tax_note_field_path() {
        let err = load_from_str("defaults:\n  currency: EUR\n  tax_note: [1, 2]\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.tax_note"), "got: {err}");
    }

    #[test]
    fn parse_reports_address_field_path() {
        let err = load_from_str("sender:\n  address: [1, 2]\ndefaults:\n  currency: EUR\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("sender.address"), "got: {err}");
    }

    #[test]
    fn parse_reports_ship_to_field_path() {
        let err =
            load_from_str("defaults:\n  currency: EUR\nclients:\n  acme:\n    ship_to: [1, 2]\n")
                .unwrap_err()
                .downcast::<ConfigError>()
                .unwrap()
                .to_string();
        assert!(err.contains("clients.acme.ship_to"), "got: {err}");
    }

    #[test]
    fn parse_reports_bill_to_field_path() {
        let err =
            load_from_str("defaults:\n  currency: EUR\nclients:\n  acme:\n    bill_to: [1, 2]\n")
                .unwrap_err()
                .downcast::<ConfigError>()
                .unwrap()
                .to_string();
        assert!(err.contains("clients.acme.bill_to"), "got: {err}");
    }

    #[test]
    fn parse_reports_date_format_field_path() {
        let err = load_from_str("defaults:\n  currency: EUR\n  date_format: [1, 2]\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.date_format"), "got: {err}");
    }

    #[test]
    fn parse_reports_tax_rate_field_path() {
        let err = load_from_str("defaults:\n  currency: EUR\n  tax_rate: nope\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.tax_rate"), "got: {err}");
    }

    #[test]
    fn parse_reports_unknown_nested_field_path() {
        let err = load_from_str("defaults:\n  currency: EUR\nclients:\n  acme:\n    nope: 1\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(
            err.contains("clients.acme.nope") || err.contains("nope"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_reports_sender_unknown_field_path() {
        let err = load_from_str("sender:\n  nope: 1\ndefaults:\n  currency: EUR\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(
            err.contains("sender.nope") || err.contains("nope"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_reports_defaults_unknown_field_path() {
        let err = load_from_str("defaults:\n  currency: EUR\n  nope: 1\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(
            err.contains("defaults.nope") || err.contains("nope"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_reports_client_template_unknown_field_path() {
        let err = load_from_str(
            "defaults:\n  currency: EUR\nclients:\n  acme:\n    bill_to: x\n    nope: 1\n",
        )
        .unwrap_err()
        .downcast::<ConfigError>()
        .unwrap()
        .to_string();
        assert!(
            err.contains("clients.acme.nope") || err.contains("nope"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_reports_invalid_client_map_type() {
        let err = load_from_str("defaults:\n  currency: EUR\nclients: true\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("clients"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_sender_map_type() {
        let err = load_from_str("sender: true\ndefaults:\n  currency: EUR\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("sender"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_defaults_map_type() {
        let err = load_from_str("defaults: true\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_sender_name_type() {
        let err = load_from_str("sender:\n  name: [1, 2]\ndefaults:\n  currency: EUR\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("sender.name"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_defaults_locale_type() {
        let err = load_from_str("defaults:\n  currency: EUR\n  locale: [1, 2]\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.locale"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_defaults_currency_type() {
        let err = load_from_str("defaults:\n  currency: [1, 2]\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.currency"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_default_rate_type() {
        let err = load_from_str(
            "defaults:\n  currency: EUR\nclients:\n  acme:\n    default_rate: true\n",
        )
        .unwrap_err()
        .downcast::<ConfigError>()
        .unwrap()
        .to_string();
        assert!(err.contains("clients.acme.default_rate"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_tax_rate_type() {
        let err = load_from_str("defaults:\n  currency: EUR\n  tax_rate: true\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.tax_rate"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_nested_client_template_type() {
        let err = load_from_str("defaults:\n  currency: EUR\nclients:\n  acme: true\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("clients.acme"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_locale_bool() {
        let err = load_from_str("defaults:\n  currency: EUR\n  locale: true\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.locale"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_currency_bool() {
        let err = load_from_str("defaults:\n  currency: true\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.currency"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_client_default_rate_bool() {
        let err = load_from_str(
            "defaults:\n  currency: EUR\nclients:\n  acme:\n    default_rate: false\n",
        )
        .unwrap_err()
        .downcast::<ConfigError>()
        .unwrap()
        .to_string();
        assert!(err.contains("clients.acme.default_rate"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_tax_rate_bool() {
        let err = load_from_str("defaults:\n  currency: EUR\n  tax_rate: false\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults.tax_rate"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_defaults_bool() {
        let err = load_from_str("defaults: false\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("defaults"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_sender_bool() {
        let err = load_from_str("sender: false\ndefaults:\n  currency: EUR\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("sender"), "got: {err}");
    }

    #[test]
    fn parse_reports_invalid_clients_bool() {
        let err = load_from_str("defaults:\n  currency: EUR\nclients: false\n")
            .unwrap_err()
            .downcast::<ConfigError>()
            .unwrap()
            .to_string();
        assert!(err.contains("clients"), "got: {err}");
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
