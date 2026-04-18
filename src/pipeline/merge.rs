use anyhow::{Context, Result, bail};

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

    let bill_to = merged.bill_to.with_context(|| match selected_client {
        Some(client) => format!(
            "bill_to missing. Set it in the invoice, or define client '{client}' in config (available: {available_clients:?})"
        ),
        None => {
            "bill_to missing. Set it in the invoice, or set 'client' to a key defined in config"
                .to_string()
        }
    })?;

    let items = merged.items.take().unwrap_or_default();
    if items.is_empty() {
        bail!("invoice has no items");
    }

    let default_rate = merged.default_rate;
    let mut resolved_items = Vec::with_capacity(items.len());
    for (i, item) in items.into_iter().enumerate() {
        resolved_items.push(resolve_line_item(i, item, default_rate)?);
    }

    let tax_rate = merged.tax_rate.context("tax_rate missing")?;
    if tax_rate.is_sign_negative() {
        bail!("tax_rate is negative");
    }

    Ok(InvoiceDocument {
        number: merged.number.context("invoice number missing")?,
        date: merged.date.context("invoice date missing")?,
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
        currency: merged.currency.context("currency missing")?,
        locale: merged.locale.context("locale missing")?,
        date_format: merged.date_format.context("date_format missing")?,
        logo_path: merged.sender.logo_path,
    })
}

fn resolve_line_item(
    index: usize,
    item: LineItemPatch,
    default_rate: Option<rust_decimal::Decimal>,
) -> Result<InvoiceLineItem> {
    let quantity = item
        .quantity
        .with_context(|| format!("item #{}: quantity missing", index + 1))?;
    let rate = item.rate.or(default_rate).with_context(|| {
        format!(
            "item #{}: no rate (set item.rate or client default_rate)",
            index + 1
        )
    })?;
    if quantity.is_sign_negative() {
        bail!("item #{}: negative quantity", index + 1);
    }
    if rate.is_sign_negative() {
        bail!("item #{}: negative rate", index + 1);
    }

    let description = item
        .description
        .with_context(|| format!("item #{}: description missing", index + 1))?;
    if description.is_empty() {
        bail!("item #{}: description is empty", index + 1);
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
                output_dir: PathBuf::from("pdf"),
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
    fn missing_bill_to_fails() {
        let mut invoice = base_invoice_patch();
        invoice.client = Some("unknown".to_string());
        let err = merge_with_layers(vec![invoice]).unwrap_err().to_string();
        assert!(err.contains("bill_to"), "got: {err}");
    }

    #[test]
    fn missing_rate_fails() {
        let mut invoice = base_invoice_patch();
        invoice.client = Some("unknown".to_string());
        invoice.bill_to = Some("Ad Hoc".to_string());
        let err = merge_with_layers(vec![invoice]).unwrap_err().to_string();
        assert!(err.contains("no rate"), "got: {err}");
    }

    #[test]
    fn no_items_fails() {
        let mut invoice = base_invoice_patch();
        invoice.items = Some(Vec::new());
        let err = merge_with_layers(vec![invoice]).unwrap_err().to_string();
        assert!(err.contains("no items"), "got: {err}");
    }

    #[test]
    fn negative_quantity_fails() {
        let mut invoice = base_invoice_patch();
        invoice.items = Some(vec![LineItemPatch {
            description: Some("work".to_string()),
            quantity: Some(dec!(-1)),
            rate: Some(dec!(100)),
        }]);
        let err = merge_with_layers(vec![invoice]).unwrap_err().to_string();
        assert!(err.contains("negative quantity"), "got: {err}");
    }

    #[test]
    fn negative_tax_rate_fails() {
        let mut invoice = base_invoice_patch();
        invoice.tax_rate = Some(dec!(-5));
        let err = merge_with_layers(vec![invoice]).unwrap_err().to_string();
        assert!(err.contains("tax_rate"), "got: {err}");
    }

    #[test]
    fn sender_override_partial() {
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
        assert_eq!(d.sender.address, "Home 1");
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
