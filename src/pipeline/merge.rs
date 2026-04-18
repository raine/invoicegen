use anyhow::{Context, Result, bail};
use jiff::civil::Date;
use rust_decimal::Decimal;
use std::path::{Path, PathBuf};

use crate::cli::GenerateArgs;
use crate::config::AppConfig;
use crate::domain::{DomainInvoice, DomainLineItem, Party};
use crate::invoice_input::{InvoiceFile, LineItemInput};
use crate::paths::{expand_tilde, resolve_relative};

#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    pub number: Option<u32>,
    pub date: Option<Date>,
    pub po: Option<String>,
    pub client: Option<String>,
    pub notes: Option<String>,
    pub first_description: Option<String>,
    pub first_quantity: Option<Decimal>,
    pub first_rate: Option<Decimal>,
    /// When non-empty, replaces invoice.items entirely.
    pub items: Vec<LineItemInput>,
    pub tax_rate: Option<Decimal>,
    pub tax_note: Option<String>,
    pub bill_to: Option<String>,
    pub ship_to: Option<String>,
}

impl From<&GenerateArgs> for CliOverrides {
    fn from(args: &GenerateArgs) -> Self {
        Self {
            number: args.number,
            date: args.date,
            po: args.po.clone(),
            client: args.client.clone(),
            notes: args.notes.clone(),
            first_description: args.description.clone(),
            first_quantity: args.quantity,
            first_rate: args.rate,
            items: args.items.clone(),
            tax_rate: args.tax_rate,
            tax_note: args.tax_note.clone(),
            bill_to: args.bill_to.clone(),
            ship_to: args.ship_to.clone(),
        }
    }
}

pub fn merge(
    mut invoice: InvoiceFile,
    config: &AppConfig,
    overrides: CliOverrides,
    invoice_dir: &Path,
) -> Result<DomainInvoice> {
    apply_cli_overrides(&mut invoice, overrides);

    let template = invoice
        .client
        .as_deref()
        .and_then(|c| config.clients.get(c));

    let bill_to = invoice
        .client_override
        .bill_to
        .clone()
        .or_else(|| template.and_then(|t| t.bill_to.clone()))
        .with_context(|| {
            let keys: Vec<_> = config.clients.keys().cloned().collect();
            match &invoice.client {
                Some(c) => format!(
                    "bill_to missing. Set it under client_override in the invoice, or define client '{c}' in config (available: {keys:?})"
                ),
                None => "bill_to missing. Set it under client_override in the invoice, or set 'client' to a key defined in config".to_string(),
            }
        })?;
    let ship_to = invoice
        .client_override
        .ship_to
        .clone()
        .or_else(|| template.and_then(|t| t.ship_to.clone()))
        .unwrap_or_default();
    let default_rate = invoice
        .client_override
        .default_rate
        .or_else(|| template.and_then(|t| t.default_rate));

    if invoice.items.is_empty() {
        bail!("invoice has no items");
    }

    let mut items = Vec::with_capacity(invoice.items.len());
    for (i, item) in invoice.items.iter().enumerate() {
        let rate = item.rate.or(default_rate).with_context(|| {
            format!(
                "item #{}: no rate (set item.rate or client default_rate)",
                i + 1
            )
        })?;
        if item.quantity.is_sign_negative() {
            bail!("item #{}: negative quantity", i + 1);
        }
        if rate.is_sign_negative() {
            bail!("item #{}: negative rate", i + 1);
        }
        items.push(DomainLineItem {
            description: item.description.clone(),
            quantity: item.quantity,
            rate,
        });
    }

    let tax_rate = invoice.tax_rate.unwrap_or(config.defaults.tax_rate);
    if tax_rate.is_sign_negative() {
        bail!("tax_rate is negative");
    }
    let tax_note = invoice
        .tax_note
        .clone()
        .or_else(|| config.defaults.tax_note.clone());

    let sender_name = invoice
        .sender_override
        .name
        .clone()
        .unwrap_or_else(|| config.sender.name.clone());
    let sender_address = invoice
        .sender_override
        .address
        .clone()
        .or_else(|| config.sender.address.clone())
        .unwrap_or_default();

    let logo_path: Option<PathBuf> = if let Some(p) = &invoice.sender_override.logo {
        Some(resolve_relative(invoice_dir, &expand_tilde(p)))
    } else {
        config.sender.logo.as_ref().map(|p| expand_tilde(p))
    };

    Ok(DomainInvoice {
        number: invoice.number,
        date: invoice.date,
        po_number: invoice.po_number,
        notes: invoice.notes,
        sender: Party {
            name: sender_name,
            address: sender_address,
        },
        bill_to,
        ship_to,
        items,
        tax_rate,
        tax_note,
        currency: config.defaults.currency.clone(),
        date_format: config.defaults.date_format.clone(),
        logo_path,
    })
}

fn apply_cli_overrides(invoice: &mut InvoiceFile, o: CliOverrides) {
    if let Some(n) = o.number {
        invoice.number = n;
    }
    if let Some(d) = o.date {
        invoice.date = d;
    }
    if let Some(p) = o.po {
        invoice.po_number = Some(p);
    }
    if let Some(c) = o.client {
        invoice.client = Some(c);
    }
    if !o.items.is_empty() {
        invoice.items = o.items;
    }
    if let Some(n) = o.notes {
        invoice.notes = Some(n);
    }
    if let Some(q) = o.first_quantity
        && let Some(first) = invoice.items.first_mut()
    {
        first.quantity = q;
    }
    if let Some(r) = o.first_rate
        && let Some(first) = invoice.items.first_mut()
    {
        first.rate = Some(r);
    }
    if let Some(d) = o.first_description
        && let Some(first) = invoice.items.first_mut()
    {
        first.description = d;
    }
    if let Some(t) = o.tax_rate {
        invoice.tax_rate = Some(t);
    }
    if let Some(t) = o.tax_note {
        invoice.tax_note = Some(t);
    }
    if let Some(b) = o.bill_to {
        invoice.client_override.bill_to = Some(b);
    }
    if let Some(s) = o.ship_to {
        invoice.client_override.ship_to = Some(s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ClientTemplate, DefaultsConfig, SenderConfig};
    use crate::invoice_input::{ClientOverride, LineItemInput, SenderOverride};
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
                currency: "EUR".to_string(),
                date_format: "%Y-%m-%d".to_string(),
                output_dir: PathBuf::from("pdf"),
                tax_rate: dec!(0),
                tax_note: Some("vat note".to_string()),
            },
            clients,
        }
    }

    fn base_invoice() -> InvoiceFile {
        InvoiceFile {
            number: 1,
            date: date(2026, 4, 18),
            client: Some("acme".to_string()),
            po_number: None,
            notes: None,
            tax_rate: None,
            tax_note: None,
            sender_override: SenderOverride::default(),
            client_override: ClientOverride::default(),
            items: vec![LineItemInput {
                description: "work".to_string(),
                quantity: dec!(2),
                rate: None,
            }],
        }
    }

    #[test]
    fn merges_from_template() {
        let d = merge(
            base_invoice(),
            &base_config(),
            CliOverrides::default(),
            Path::new("/tmp"),
        )
        .unwrap();
        assert_eq!(d.bill_to, "Acme Inc\n1 Main St");
        assert_eq!(d.ship_to, "Acme Dock");
        assert_eq!(d.items[0].rate, dec!(100));
        assert_eq!(d.sender.name, "Me Co");
        assert_eq!(d.currency, "EUR");
    }

    #[test]
    fn cli_overrides_win() {
        let overrides = CliOverrides {
            number: Some(42),
            po: Some("PO-9".to_string()),
            notes: Some("n".to_string()),
            first_quantity: Some(dec!(3)),
            first_rate: Some(dec!(200)),
            first_description: Some("override".to_string()),
            tax_rate: Some(dec!(24)),
            tax_note: Some("tn".to_string()),
            bill_to: Some("Override Ltd".to_string()),
            ship_to: Some("Override Dock".to_string()),
            ..Default::default()
        };
        let d = merge(base_invoice(), &base_config(), overrides, Path::new("/tmp")).unwrap();
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
        let mut inv = base_invoice();
        inv.client = Some("unknown".to_string());
        inv.client_override.bill_to = Some("Ad Hoc".to_string());
        inv.client_override.default_rate = Some(dec!(50));
        inv.items[0].rate = None;
        let d = merge(
            inv,
            &base_config(),
            CliOverrides::default(),
            Path::new("/tmp"),
        )
        .unwrap();
        assert_eq!(d.bill_to, "Ad Hoc");
        assert_eq!(d.items[0].rate, dec!(50));
    }

    #[test]
    fn missing_bill_to_fails() {
        let mut inv = base_invoice();
        inv.client = Some("unknown".to_string());
        let err = merge(
            inv,
            &base_config(),
            CliOverrides::default(),
            Path::new("/tmp"),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("bill_to"), "got: {err}");
    }

    #[test]
    fn missing_rate_fails() {
        let mut cfg = base_config();
        cfg.clients.get_mut("acme").unwrap().default_rate = None;
        let inv = base_invoice();
        let err = merge(inv, &cfg, CliOverrides::default(), Path::new("/tmp"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("no rate"), "got: {err}");
    }

    #[test]
    fn no_items_fails() {
        let mut inv = base_invoice();
        inv.items.clear();
        let err = merge(
            inv,
            &base_config(),
            CliOverrides::default(),
            Path::new("/tmp"),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("no items"), "got: {err}");
    }

    #[test]
    fn negative_quantity_fails() {
        let mut inv = base_invoice();
        inv.items[0].quantity = dec!(-1);
        let err = merge(
            inv,
            &base_config(),
            CliOverrides::default(),
            Path::new("/tmp"),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("negative quantity"), "got: {err}");
    }

    #[test]
    fn negative_tax_rate_fails() {
        let mut inv = base_invoice();
        inv.tax_rate = Some(dec!(-5));
        let err = merge(
            inv,
            &base_config(),
            CliOverrides::default(),
            Path::new("/tmp"),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("tax_rate"), "got: {err}");
    }

    #[test]
    fn sender_override_partial() {
        let mut inv = base_invoice();
        inv.sender_override.name = Some("Other".to_string());
        let d = merge(
            inv,
            &base_config(),
            CliOverrides::default(),
            Path::new("/tmp"),
        )
        .unwrap();
        assert_eq!(d.sender.name, "Other");
        assert_eq!(d.sender.address, "Home 1");
    }

    #[test]
    fn logo_from_invoice_is_relative_to_invoice_dir() {
        let mut inv = base_invoice();
        inv.sender_override.logo = Some(PathBuf::from("logo.svg"));
        let d = merge(
            inv,
            &base_config(),
            CliOverrides::default(),
            Path::new("/inv/dir"),
        )
        .unwrap();
        assert_eq!(d.logo_path, Some(PathBuf::from("/inv/dir/logo.svg")));
    }
}
