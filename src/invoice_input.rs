use anyhow::Result;
use jiff::civil::Date;
use rust_decimal::Decimal;
use serde::de::{self, MapAccess, Visitor, value::MapAccessDeserializer};
use serde::{Deserialize, Deserializer, Serialize};
use std::{fs, io::Read, path::Path, path::PathBuf};

use crate::diagnostics::{
    InvoiceInputError, LineItemInputError, display_input_path, format_yaml_path_error,
};
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

#[derive(Debug, Clone, Serialize, Default)]
pub enum ClientInput {
    #[default]
    Missing,
    Key(String),
    Inline(ClientReference),
}

impl<'de> Deserialize<'de> for ClientInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ClientInputVisitor;

        impl<'de> Visitor<'de> for ClientInputVisitor {
            type Value = ClientInput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a client key string or inline client mapping")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ClientInput::Missing)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ClientInput::Missing)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ClientInput::Key(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ClientInput::Key(value))
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let reference = ClientReference::deserialize(MapAccessDeserializer::new(map))?;
                Ok(ClientInput::Inline(reference))
            }
        }

        deserializer.deserialize_any(ClientInputVisitor)
    }
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
        let (desc, rest) = s
            .rsplit_once(':')
            .ok_or(LineItemInputError::MissingSeparator)?;

        let description = desc.trim().to_string();
        if description.is_empty() {
            return Err(LineItemInputError::EmptyDescription.into());
        }

        let (qty_str, rate_str) = match rest.split_once('@') {
            Some((q, r)) => (q.trim(), Some(r.trim())),
            None => (rest.trim(), None),
        };

        let quantity: Decimal =
            qty_str
                .parse()
                .map_err(|source| LineItemInputError::InvalidQuantity {
                    value: qty_str.to_string(),
                    source,
                })?;

        let rate =
            match rate_str {
                Some(r) => Some(r.parse::<Decimal>().map_err(|source| {
                    LineItemInputError::InvalidRate {
                        value: r.to_string(),
                        source,
                    }
                })?),
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
            .map_err(|source| InvoiceInputError::ReadStdin { source })?;
        text
    } else {
        fs::read_to_string(path).map_err(|source| InvoiceInputError::ReadFile {
            path: path.to_path_buf(),
            source,
        })?
    };
    let de = serde_yml::Deserializer::from_str(&text);
    let inv: InvoiceFile =
        serde_path_to_error::deserialize(de).map_err(|error| InvoiceInputError::Parse {
            path: display_input_path(path),
            message: format_yaml_path_error(error),
        })?;
    Ok(inv)
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
    fn line_item_reports_format_help() {
        let err = "Consulting".parse::<LineItemInput>().unwrap_err();
        let err = err.downcast::<LineItemInputError>().unwrap();
        assert_eq!(
            err.to_string(),
            "item must be 'DESCRIPTION: QUANTITY [@ RATE]'"
        );
        assert!(err.help().contains("DESCRIPTION: QUANTITY"));
    }

    #[test]
    fn line_item_reports_invalid_quantity() {
        let err = "Consulting: nope".parse::<LineItemInput>().unwrap_err();
        let err = err.downcast::<LineItemInputError>().unwrap();
        assert!(err.to_string().contains("invalid item quantity 'nope'"));
    }

    #[test]
    fn line_item_reports_invalid_rate() {
        let err = "Consulting: 2 @ nope".parse::<LineItemInput>().unwrap_err();
        let err = err.downcast::<LineItemInputError>().unwrap();
        assert!(err.to_string().contains("invalid item rate 'nope'"));
    }

    #[test]
    fn line_item_reports_empty_description() {
        let err = ": 2 @ 150".parse::<LineItemInput>().unwrap_err();
        let err = err.downcast::<LineItemInputError>().unwrap();
        assert_eq!(err.to_string(), "item description is empty");
    }

    #[test]
    fn invoice_parse_reports_field_path() {
        let path = Path::new("invoice.yaml");
        let yaml = "number: 17\ndate: 2026-04-18\nitems:\n  - description: Consulting\n    quantity: nope\n";
        let de = serde_yml::Deserializer::from_str(yaml);
        let err = serde_path_to_error::deserialize::<_, InvoiceFile>(de)
            .map_err(|error| InvoiceInputError::Parse {
                path: display_input_path(path),
                message: format_yaml_path_error(error),
            })
            .unwrap_err();
        assert!(err.to_string().contains("invoice.yaml"));
        assert!(err.to_string().contains("items[0].quantity"));
    }

    #[test]
    fn invoice_parse_reports_unknown_field() {
        let path = Path::new("invoice.yaml");
        let yaml = "number: 17\ndate: 2026-04-18\nunknown: true\nitems: []\n";
        let de = serde_yml::Deserializer::from_str(yaml);
        let err = serde_path_to_error::deserialize::<_, InvoiceFile>(de)
            .map_err(|error| InvoiceInputError::Parse {
                path: display_input_path(path),
                message: format_yaml_path_error(error),
            })
            .unwrap_err();
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn display_path_uses_stdin_label() {
        assert_eq!(display_input_path(Path::new("-")), "stdin");
    }

    #[test]
    fn display_path_uses_file_name() {
        assert_eq!(
            display_input_path(Path::new("invoice.yaml")),
            "invoice.yaml"
        );
    }

    #[test]
    fn parse_error_format_without_field_path_is_still_readable() {
        let de = serde_yml::Deserializer::from_str("[");
        let err = serde_path_to_error::deserialize::<_, InvoiceFile>(de).unwrap_err();
        let message = format_yaml_path_error(err);
        assert!(!message.is_empty());
    }

    #[test]
    fn parse_error_format_includes_nested_path() {
        let de = serde_yml::Deserializer::from_str(
            "number: 17\ndate: 2026-04-18\nclient:\n  default_rate: nope\nitems: []\n",
        );
        let err = serde_path_to_error::deserialize::<_, InvoiceFile>(de).unwrap_err();
        let message = format_yaml_path_error(err);
        assert!(message.contains("client.default_rate"), "got: {message}");
    }

    #[test]
    fn parse_error_format_includes_array_index() {
        let de = serde_yml::Deserializer::from_str(
            "number: 17\ndate: 2026-04-18\nitems:\n  - description: Consulting\n    quantity: 1\n    rate: nope\n",
        );
        let err = serde_path_to_error::deserialize::<_, InvoiceFile>(de).unwrap_err();
        let message = format_yaml_path_error(err);
        assert!(message.contains("items[0].rate"), "got: {message}");
    }

    #[test]
    fn line_item_still_parses_valid_input() {
        let item = "Consulting: 2 @ 150".parse::<LineItemInput>().unwrap();
        assert_eq!(item.description, "Consulting");
        assert_eq!(item.quantity, dec!(2));
        assert_eq!(item.rate, Some(dec!(150)));
    }

    #[test]
    fn line_item_supports_missing_rate() {
        let item = "Consulting: 2".parse::<LineItemInput>().unwrap();
        assert_eq!(item.rate, None);
    }

    #[test]
    fn line_item_supports_colons_in_description() {
        let item = "Refactor: auth module: 5 @ 100"
            .parse::<LineItemInput>()
            .unwrap();
        assert_eq!(item.description, "Refactor: auth module");
    }

    #[test]
    fn line_item_trims_values() {
        let item = " Consulting : 2 @ 150 ".parse::<LineItemInput>().unwrap();
        assert_eq!(item.description, "Consulting");
        assert_eq!(item.quantity, dec!(2));
        assert_eq!(item.rate, Some(dec!(150)));
    }

    #[test]
    fn line_item_rejects_bad_rate_with_helpful_message() {
        let err = "Consulting: 2 @ bad".parse::<LineItemInput>().unwrap_err();
        assert!(err.to_string().contains("invalid item rate 'bad'"));
    }

    #[test]
    fn line_item_rejects_bad_quantity_with_helpful_message() {
        let err = "Consulting: bad".parse::<LineItemInput>().unwrap_err();
        assert!(err.to_string().contains("invalid item quantity 'bad'"));
    }

    #[test]
    fn line_item_rejects_empty_description_with_helpful_message() {
        let err = " : 2".parse::<LineItemInput>().unwrap_err();
        assert!(err.to_string().contains("description is empty"));
    }

    #[test]
    fn parse_invoice_error_mentions_stdin() {
        let err = InvoiceInputError::Parse {
            path: display_input_path(Path::new("-")),
            message: "at items[0].quantity: bad value".to_string(),
        };
        assert!(err.to_string().contains("stdin"));
    }

    #[test]
    fn parse_invoice_error_mentions_file_path() {
        let err = InvoiceInputError::Parse {
            path: display_input_path(Path::new("example.yaml")),
            message: "at items[0].quantity: bad value".to_string(),
        };
        assert!(err.to_string().contains("example.yaml"));
    }

    #[test]
    fn parse_invoice_error_keeps_message_body() {
        let err = InvoiceInputError::Parse {
            path: "example.yaml".to_string(),
            message: "at items[0].quantity: bad value".to_string(),
        };
        assert!(err.to_string().contains("items[0].quantity"));
    }

    #[test]
    fn read_invoice_error_mentions_path() {
        let err = InvoiceInputError::ReadFile {
            path: Path::new("example.yaml").to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        };
        assert!(err.to_string().contains("example.yaml"));
    }

    #[test]
    fn read_stdin_error_has_clear_message() {
        let err = InvoiceInputError::ReadStdin {
            source: std::io::Error::other("broken pipe"),
        };
        assert_eq!(err.to_string(), "could not read invoice from stdin");
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
