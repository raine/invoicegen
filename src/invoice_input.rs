use anyhow::{Context, Result};
use jiff::civil::Date;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, path::Path, path::PathBuf};

use crate::domain::{InvoicePatch, LineItemPatch, PartyPatch};
use crate::paths::resolve_relative;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceFile {
    pub number: u32,
    pub date: Date,
    #[serde(default)]
    pub client: ClientInput,
    pub po_number: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub tax_rate: Option<Decimal>,
    #[serde(default)]
    pub tax_note: Option<String>,
    #[serde(default, alias = "sender_override")]
    pub sender: SenderOverride,
    #[serde(default)]
    pub client_override: ClientOverride,
    pub items: Vec<LineItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum ClientInput {
    #[default]
    Missing,
    Key(String),
    Inline(ClientReference),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ClientReference {
    pub template: Option<String>,
    pub bill_to: Option<String>,
    pub ship_to: Option<String>,
    pub default_rate: Option<Decimal>,
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
    let text = if path == Path::new("-") {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .context("reading invoice YAML from stdin")?;
        text
    } else {
        fs::read_to_string(path)
            .with_context(|| format!("reading invoice file {}", path.display()))?
    };
    let de = serde_yml::Deserializer::from_str(&text);
    let inv: InvoiceFile = serde_path_to_error::deserialize(de)
        .with_context(|| format!("parsing invoice {}", display_path(path)))?;
    Ok(inv)
}

fn display_path(path: &Path) -> String {
    if path == Path::new("-") {
        "stdin".to_string()
    } else {
        path.display().to_string()
    }
}

impl InvoiceFile {
    pub fn into_patch(self, invoice_dir: &Path) -> InvoicePatch {
        let mut patch = InvoicePatch {
            number: Some(self.number),
            date: Some(self.date),
            client: None,
            po_number: self.po_number,
            notes: self.notes,
            sender: PartyPatch {
                name: self.sender.name,
                address: self.sender.address,
                logo_path: self.sender.logo.map(|p| resolve_relative(invoice_dir, &p)),
            },
            bill_to: None,
            ship_to: None,
            items: Some(
                self.items
                    .into_iter()
                    .map(|item| LineItemPatch {
                        description: Some(item.description),
                        quantity: Some(item.quantity),
                        rate: item.rate,
                    })
                    .collect(),
            ),
            default_rate: None,
            tax_rate: self.tax_rate,
            tax_note: self.tax_note,
            ..InvoicePatch::default()
        };

        match self.client {
            ClientInput::Missing => {}
            ClientInput::Key(key) => patch.client = Some(key),
            ClientInput::Inline(client) => {
                patch.client = client.template;
                patch.bill_to = client.bill_to;
                patch.ship_to = client.ship_to;
                patch.default_rate = client.default_rate;
            }
        }

        patch.apply(InvoicePatch {
            bill_to: self.client_override.bill_to,
            ship_to: self.client_override.ship_to,
            default_rate: self.client_override.default_rate,
            ..InvoicePatch::default()
        });

        patch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rust_decimal_macros::dec;
    use std::path::Path;

    fn parse(yaml: &str) -> Result<InvoiceFile> {
        let de = serde_yml::Deserializer::from_str(yaml);
        Ok(serde_path_to_error::deserialize(de)?)
    }

    #[test]
    fn parses_inline_sender_and_client_blocks() {
        let invoice = parse(
            r#"number: 17
date: 2026-04-18
sender:
  name: Your Company Ltd.
client:
  template: example-client
  bill_to: One-off client
  default_rate: 150
items:
  - description: Consulting
    quantity: 1
"#,
        )
        .unwrap();

        let patch = invoice.into_patch(Path::new("/tmp"));
        assert_eq!(patch.sender.name.as_deref(), Some("Your Company Ltd."));
        assert_eq!(patch.client.as_deref(), Some("example-client"));
        assert_eq!(patch.bill_to.as_deref(), Some("One-off client"));
        assert_eq!(patch.default_rate, Some(dec!(150)));
    }

    #[test]
    fn legacy_override_keys_still_parse() {
        let invoice = parse(
            r#"number: 17
date: 2026-04-18
client: example-client
sender_override:
  name: Your Company Ltd.
client_override:
  bill_to: One-off client
items:
  - description: Consulting
    quantity: 1
"#,
        )
        .unwrap();

        let patch = invoice.into_patch(Path::new("/tmp"));
        assert_eq!(patch.sender.name.as_deref(), Some("Your Company Ltd."));
        assert_eq!(patch.client.as_deref(), Some("example-client"));
        assert_eq!(patch.bill_to.as_deref(), Some("One-off client"));
    }
}
