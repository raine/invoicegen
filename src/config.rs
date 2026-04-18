use anyhow::{Context, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

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
    pub currency: String,
    pub date_format: String,
    pub output_dir: PathBuf,
    pub tax_rate: Decimal,
    pub tax_note: Option<String>,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            currency: "EUR".to_string(),
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

pub fn default_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home dir")?;
    Ok(home.join(".config").join("invoice").join("config.yaml"))
}

pub fn load(path: &Path) -> Result<AppConfig> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading config {}", path.display()))?;
    let de = serde_yml::Deserializer::from_str(&text);
    let cfg: AppConfig = serde_path_to_error::deserialize(de)
        .with_context(|| format!("parsing config {}", path.display()))?;
    Ok(cfg)
}
