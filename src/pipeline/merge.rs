use anyhow::Result;

use crate::diagnostics::MergeError;
use crate::domain::{InvoiceDocument, InvoiceLineItem, InvoicePatch, LineItemPatch, Party};

pub fn merge(
    layers: impl IntoIterator<Item = InvoicePatch>,
    selected_client: Option<&str>,
    available_clients: &[String],
) -> Result<InvoiceDocument> {
    let mut merged = InvoicePatch::default();
    for layer in layers {
        merged.apply(layer);
    }

    if let Some(client) = selected_client
        && !available_clients
            .iter()
            .any(|candidate| candidate == client)
        && merged.bill_to.is_none()
    {
        return Err(MergeError::UnknownClient {
            client: client.to_string(),
            available: available_clients.to_vec(),
        }
        .into());
    }

    let bill_to = merged.bill_to.ok_or_else(|| MergeError::MissingField {
        field: "bill_to",
        help: Some(
            "Set bill_to in the invoice, or select a configured client that provides it."
                .to_string(),
        ),
    })?;

    let items = merged.items.take().unwrap_or_default();
    if items.is_empty() {
        return Err(MergeError::NoItems.into());
    }

    let default_rate = merged.default_rate;
    let mut resolved_items = Vec::with_capacity(items.len());
    for (i, item) in items.into_iter().enumerate() {
        resolved_items.push(resolve_line_item(i, item, default_rate)?);
    }

    let tax_rate = merged.tax_rate.ok_or_else(|| MergeError::MissingField {
        field: "tax_rate",
        help: Some("Set tax_rate in the invoice or provide a default in config.".to_string()),
    })?;
    if tax_rate.is_sign_negative() {
        return Err(MergeError::NegativeTaxRate.into());
    }

    Ok(InvoiceDocument {
        number: merged.number.ok_or_else(|| MergeError::MissingField {
            field: "number",
            help: Some("Add a number field to the invoice.".to_string()),
        })?,
        date: merged.date.ok_or_else(|| MergeError::MissingField {
            field: "date",
            help: Some("Add a date field to the invoice.".to_string()),
        })?,
        client: merged.client,
        po_number: merged.po_number,
        notes: merged.notes,
        sender: Party {
            name: merged.sender.name.unwrap_or_default(),
            address: merged.sender.address.unwrap_or_default(),
        },
        bill_to,
        ship_to: merged.ship_to.unwrap_or_default(),
        items: resolved_items,
        tax_rate,
        tax_note: merged.tax_note,
        currency: merged.currency.ok_or_else(|| MergeError::MissingField {
            field: "currency",
            help: Some("Set currency in config.defaults or in the invoice.".to_string()),
        })?,
        locale: merged.locale.ok_or_else(|| MergeError::MissingField {
            field: "locale",
            help: Some("Set locale in config.defaults or in the invoice.".to_string()),
        })?,
        date_format: merged.date_format.ok_or_else(|| MergeError::MissingField {
            field: "date_format",
            help: Some("Set date_format in config.defaults or in the invoice.".to_string()),
        })?,
        logo_path: merged.sender.logo_path,
    })
}

fn resolve_line_item(
    index: usize,
    item: LineItemPatch,
    default_rate: Option<rust_decimal::Decimal>,
) -> Result<InvoiceLineItem> {
    let item_number = index + 1;
    let quantity = item.quantity.ok_or_else(|| MergeError::MissingItemField {
        index: item_number,
        field: "quantity",
        help: Some("Add quantity to the item.".to_string()),
    })?;
    let rate = item
        .rate
        .or(default_rate)
        .ok_or_else(|| MergeError::MissingItemField {
            index: item_number,
            field: "rate",
            help: Some("Set item.rate or provide a client/default_rate in config.".to_string()),
        })?;
    if quantity.is_sign_negative() {
        return Err(MergeError::NegativeItemValue {
            index: item_number,
            field: "quantity",
        }
        .into());
    }
    if rate.is_sign_negative() {
        return Err(MergeError::NegativeItemValue {
            index: item_number,
            field: "rate",
        }
        .into());
    }

    let description = item
        .description
        .ok_or_else(|| MergeError::MissingItemField {
            index: item_number,
            field: "description",
            help: Some("Add description to the item.".to_string()),
        })?;
    if description.is_empty() {
        return Err(MergeError::EmptyItemDescription { index: item_number }.into());
    }

    Ok(InvoiceLineItem {
        description,
        quantity,
        rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, ClientTemplate, DefaultsConfig, SenderConfig};
    use crate::currency::Currency;
    use crate::domain::{LineItemPatch, PartyPatch};
    use crate::locale::Locale;
    use jiff::civil::date;
    use rust_decimal_macros::dec;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn base_config() -> AppConfig {
        let mut clients = BTreeMap::new();
        clients.insert(
            "acme".to_string(),
            ClientTemplate {
                bill_to: Some("Acme Inc\n1 Main St".to_string()),
                ship_to: Some("Acme Dock".to_string()),
                default_rate: Some(dec!(100)),
            },
        );
        AppConfig {
            sender: SenderConfig {
                name: "Me Co".to_string(),
                address: Some("Home 1".to_string()),
                logo: None,
            },
            defaults: DefaultsConfig {
                currency: Currency::Eur,
                locale: Locale::EnUs,
                date_format: "%Y-%m-%d".to_string(),
                output_dir: None,
                tax_rate: dec!(0),
                tax_note: Some("vat note".to_string()),
            },
            clients,
        }
    }

    fn base_invoice_patch() -> InvoicePatch {
        InvoicePatch {
            number: Some(1),
            date: Some(date(2026, 4, 18)),
            client: Some("acme".to_string()),
            items: Some(vec![LineItemPatch {
                description: Some("work".to_string()),
                quantity: Some(dec!(2)),
                rate: None,
            }]),
            ..InvoicePatch::default()
        }
    }

    fn merge_with_layers(mut layers: Vec<InvoicePatch>) -> Result<InvoiceDocument> {
        let config = base_config();
        let selected_client = layers.iter().rev().find_map(|patch| patch.client.clone());
        let mut all_layers = vec![config.defaults_patch()];
        if let Some(client) = selected_client.as_deref()
            && let Some(client_patch) = config.client_patch(client)
        {
            all_layers.push(client_patch);
        }
        all_layers.append(&mut layers);
        merge(
            all_layers,
            selected_client.as_deref(),
            &config.client_keys(),
        )
    }

    #[test]
    fn merges_from_template() {
        let d = merge_with_layers(vec![base_invoice_patch()]).unwrap();
        assert_eq!(d.bill_to, "Acme Inc\n1 Main St");
        assert_eq!(d.ship_to, "Acme Dock");
        assert_eq!(d.items[0].rate, dec!(100));
        assert_eq!(d.sender.name, "Me Co");
        assert_eq!(d.currency, Currency::Eur);
    }

    #[test]
    fn cli_overrides_win() {
        let overrides = InvoicePatch {
            number: Some(42),
            po_number: Some("PO-9".to_string()),
            notes: Some("n".to_string()),
            first_item: Some(LineItemPatch {
                description: Some("override".to_string()),
                quantity: Some(dec!(3)),
                rate: Some(dec!(200)),
            }),
            tax_rate: Some(dec!(24)),
            tax_note: Some("tn".to_string()),
            bill_to: Some("Override Ltd".to_string()),
            ship_to: Some("Override Dock".to_string()),
            ..InvoicePatch::default()
        };
        let d = merge_with_layers(vec![base_invoice_patch(), overrides]).unwrap();
        assert_eq!(d.number, 42);
        assert_eq!(d.po_number.as_deref(), Some("PO-9"));
        assert_eq!(d.items[0].description, "override");
        assert_eq!(d.items[0].quantity, dec!(3));
        assert_eq!(d.items[0].rate, dec!(200));
        assert_eq!(d.tax_rate, dec!(24));
        assert_eq!(d.tax_note.as_deref(), Some("tn"));
        assert_eq!(d.bill_to, "Override Ltd");
        assert_eq!(d.ship_to, "Override Dock");
        assert_eq!(d.notes.as_deref(), Some("n"));
    }

    #[test]
    fn unknown_client_with_bill_to_override_ok() {
        let mut invoice = base_invoice_patch();
        invoice.client = Some("unknown".to_string());
        invoice.bill_to = Some("Ad Hoc".to_string());
        invoice.default_rate = Some(dec!(50));
        let d = merge_with_layers(vec![invoice]).unwrap();
        assert_eq!(d.bill_to, "Ad Hoc");
        assert_eq!(d.items[0].rate, dec!(50));
    }

    #[test]
    fn unknown_client_reports_available_clients() {
        let mut invoice = base_invoice_patch();
        invoice.client = Some("unknown".to_string());
        let err = merge_with_layers(vec![invoice]).unwrap_err();
        let err = err.downcast::<MergeError>().unwrap();
        assert_eq!(err.to_string(), "unknown client 'unknown' in invoice input");
        assert!(err.details().unwrap().contains("acme"));
        assert!(err.help().unwrap().contains("configured client keys"));
    }

    #[test]
    fn missing_rate_reports_item_and_help() {
        let mut invoice = base_invoice_patch();
        invoice.client = Some("unknown".to_string());
        invoice.bill_to = Some("Ad Hoc".to_string());
        let err = merge_with_layers(vec![invoice]).unwrap_err();
        let err = err.downcast::<MergeError>().unwrap();
        assert_eq!(err.to_string(), "item #1 is missing rate");
        assert!(err.help().unwrap().contains("default_rate"));
    }

    #[test]
    fn no_items_fails() {
        let mut invoice = base_invoice_patch();
        invoice.items = Some(Vec::new());
        let err = merge_with_layers(vec![invoice]).unwrap_err();
        let err = err.downcast::<MergeError>().unwrap();
        assert_eq!(err.to_string(), "invoice must include at least one item");
        assert_eq!(err.help(), Some("Add at least one entry under items."));
    }

    #[test]
    fn negative_quantity_fails() {
        let mut invoice = base_invoice_patch();
        invoice.items = Some(vec![LineItemPatch {
            description: Some("work".to_string()),
            quantity: Some(dec!(-1)),
            rate: Some(dec!(100)),
        }]);
        let err = merge_with_layers(vec![invoice]).unwrap_err();
        let err = err.downcast::<MergeError>().unwrap();
        assert_eq!(err.to_string(), "item #1 has a negative quantity");
        assert_eq!(
            err.help(),
            Some("Set the quantity to 0 or a positive number.")
        );
    }

    #[test]
    fn negative_tax_rate_fails() {
        let mut invoice = base_invoice_patch();
        invoice.tax_rate = Some(dec!(-5));
        let err = merge_with_layers(vec![invoice]).unwrap_err();
        let err = err.downcast::<MergeError>().unwrap();
        assert_eq!(err.to_string(), "invalid tax_rate: must be zero or greater");
        assert_eq!(err.help(), Some("Set tax_rate to 0 or a positive number."));
    }

    #[test]
    fn missing_bill_to_reports_help() {
        let mut invoice = base_invoice_patch();
        invoice.client = None;
        let err = merge_with_layers(vec![invoice]).unwrap_err();
        let err = err.downcast::<MergeError>().unwrap();
        assert_eq!(err.to_string(), "missing bill_to");
        assert!(err.help().unwrap().contains("configured client"));
    }

    #[test]
    fn empty_description_reports_item_number() {
        let mut invoice = base_invoice_patch();
        invoice.items = Some(vec![LineItemPatch {
            description: Some(String::new()),
            quantity: Some(dec!(1)),
            rate: Some(dec!(100)),
        }]);
        let err = merge_with_layers(vec![invoice]).unwrap_err();
        let err = err.downcast::<MergeError>().unwrap();
        assert_eq!(err.to_string(), "item #1 description is empty");
    }

    #[test]
    fn missing_description_reports_item_number() {
        let mut invoice = base_invoice_patch();
        invoice.items = Some(vec![LineItemPatch {
            description: None,
            quantity: Some(dec!(1)),
            rate: Some(dec!(100)),
        }]);
        let err = merge_with_layers(vec![invoice]).unwrap_err();
        let err = err.downcast::<MergeError>().unwrap();
        assert_eq!(err.to_string(), "item #1 is missing description");
    }

    #[test]
    fn missing_quantity_reports_item_number() {
        let mut invoice = base_invoice_patch();
        invoice.items = Some(vec![LineItemPatch {
            description: Some("work".to_string()),
            quantity: None,
            rate: Some(dec!(100)),
        }]);
        let err = merge_with_layers(vec![invoice]).unwrap_err();
        let err = err.downcast::<MergeError>().unwrap();
        assert_eq!(err.to_string(), "item #1 is missing quantity");
    }

    #[test]
    fn negative_rate_reports_item_number() {
        let mut invoice = base_invoice_patch();
        invoice.items = Some(vec![LineItemPatch {
            description: Some("work".to_string()),
            quantity: Some(dec!(1)),
            rate: Some(dec!(-100)),
        }]);
        let err = merge_with_layers(vec![invoice]).unwrap_err();
        let err = err.downcast::<MergeError>().unwrap();
        assert_eq!(err.to_string(), "item #1 has a negative rate");
    }

    #[test]
    fn sender_override_replaces_global_sender_block() {
        let invoice = InvoicePatch {
            sender: PartyPatch {
                name: Some("Other".to_string()),
                address: None,
                logo_path: None,
            },
            ..base_invoice_patch()
        };
        let d = merge_with_layers(vec![invoice]).unwrap();
        assert_eq!(d.sender.name, "Other");
        assert_eq!(d.sender.address, "");
    }

    #[test]
    fn logo_from_invoice_patch_wins() {
        let invoice = InvoicePatch {
            sender: PartyPatch {
                name: None,
                address: None,
                logo_path: Some(PathBuf::from("/inv/dir/logo.svg")),
            },
            ..base_invoice_patch()
        };
        let d = merge_with_layers(vec![invoice]).unwrap();
        assert_eq!(d.logo_path, Some(PathBuf::from("/inv/dir/logo.svg")));
    }
}
