use anyhow::{Context, Result};
use jiff::civil::Date;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceFile {
    pub number: u32,
    pub date: Date,
    #[serde(default)]
    pub client: Option<String>,
    pub po_number: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub tax_rate: Option<Decimal>,
    #[serde(default)]
    pub tax_note: Option<String>,
    #[serde(default)]
    pub sender_override: SenderOverride,
    #[serde(default)]
    pub client_override: ClientOverride,
    pub items: Vec<LineItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct SenderOverride {
    pub name: Option<String>,
    pub address: Option<String>,
    pub logo: Option<PathBuf>,
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

impl std::str::FromStr for LineItemInput {
    type Err = anyhow::Error;

    // Parses "Description: Quantity [@ Rate]".
    // Description may contain ':' — we split on the LAST ':' so
    // "Refactor: auth module: 5 @ 100" works.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (desc, rest) = s.rsplit_once(':').context(
            "item must be 'DESCRIPTION: QUANTITY [@ RATE]' (e.g. 'Consulting: 10 @ 150')",
        )?;

        let description = desc.trim().to_string();
        if description.is_empty() {
            anyhow::bail!("item description is empty");
        }

        let (qty_str, rate_str) = match rest.split_once('@') {
            Some((q, r)) => (q.trim(), Some(r.trim())),
            None => (rest.trim(), None),
        };

        let quantity: Decimal = qty_str
            .parse()
            .with_context(|| format!("invalid item quantity '{qty_str}'"))?;

        let rate = match rate_str {
            Some(r) => Some(
                r.parse::<Decimal>()
                    .with_context(|| format!("invalid item rate '{r}'"))?,
            ),
            None => None,
        };

        Ok(Self {
            description,
            quantity,
            rate,
        })
    }
}

pub fn load(path: &Path) -> Result<InvoiceFile> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading invoice file {}", path.display()))?;
    let de = serde_yml::Deserializer::from_str(&text);
    let inv: InvoiceFile = serde_path_to_error::deserialize(de)
        .with_context(|| format!("parsing invoice {}", path.display()))?;
    Ok(inv)
}
