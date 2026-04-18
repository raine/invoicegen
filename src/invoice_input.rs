use anyhow::{Context, Result};
use jiff::civil::Date;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceFile {
    pub number: u32,
    pub date: Date,
    pub client: String,
    pub po_number: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub tax_rate: Option<Decimal>,
    #[serde(default)]
    pub tax_note: Option<String>,
    #[serde(default)]
    pub client_override: ClientOverride,
    pub items: Vec<LineItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ClientOverride {
    pub bill_to: Option<String>,
    pub ship_to: Option<String>,
    pub default_rate: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineItemInput {
    pub description: String,
    pub quantity: Decimal,
    pub rate: Option<Decimal>,
}

pub fn load(path: &Path) -> Result<InvoiceFile> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading invoice file {}", path.display()))?;
    let de = serde_yml::Deserializer::from_str(&text);
    let inv: InvoiceFile = serde_path_to_error::deserialize(de)
        .with_context(|| format!("parsing invoice {}", path.display()))?;
    Ok(inv)
}
