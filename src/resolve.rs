use anyhow::{Context, Result, bail};
use jiff::fmt::strtime;
use rust_decimal::Decimal;
use std::path::Path;

use crate::cli::GenerateArgs;
use crate::config::AppConfig;
use crate::invoice::{RenderInvoice, RenderLineItem, RenderParty};
use crate::invoice_input::InvoiceFile;
use crate::money::{currency_symbol, format_quantity, format_with_symbol};
use crate::paths::{expand_tilde, resolve_relative};

pub struct Resolved {
    pub render: RenderInvoice,
    pub logo_bytes: Option<Vec<u8>>,
    pub logo_virtual_name: Option<String>,
    pub output_path: std::path::PathBuf,
}

pub fn resolve(
    config: &AppConfig,
    mut invoice: InvoiceFile,
    args: &GenerateArgs,
    invoice_dir: &Path,
) -> Result<Resolved> {
    // 1. CLI overrides onto invoice
    if let Some(n) = args.number {
        invoice.number = n;
    }
    if let Some(d) = args.date {
        invoice.date = d;
    }
    if let Some(p) = args.po.clone() {
        invoice.po_number = Some(p);
    }
    if let Some(c) = args.client.clone() {
        invoice.client = c;
    }
    if let Some(n) = args.notes.clone() {
        invoice.notes = Some(n);
    }
    if let Some(q) = args.hours
        && let Some(first) = invoice.items.first_mut()
    {
        first.quantity = q;
    }
    if let Some(r) = args.rate
        && let Some(first) = invoice.items.first_mut()
    {
        first.rate = Some(r);
    }

    // 2. Client template
    let template = config.clients.get(&invoice.client).with_context(|| {
        let keys: Vec<_> = config.clients.keys().cloned().collect();
        format!("unknown client '{}'. Available: {:?}", invoice.client, keys)
    })?;

    // 3. Resolve client block (override wins)
    let bill_to = invoice
        .client_override
        .bill_to
        .clone()
        .or_else(|| template.bill_to.clone())
        .context("bill_to missing (set in client template or client_override)")?;
    let ship_to = invoice
        .client_override
        .ship_to
        .clone()
        .or_else(|| template.ship_to.clone())
        .unwrap_or_default();
    let default_rate = invoice
        .client_override
        .default_rate
        .or(template.default_rate);

    // 4. Validate items
    if invoice.items.is_empty() {
        bail!("invoice has no items");
    }

    // 5. Compute money
    let currency = &config.defaults.currency;
    let symbol = currency_symbol(currency);
    let fmt = |d: Decimal| format_with_symbol(symbol, d);

    let mut render_items = Vec::with_capacity(invoice.items.len());
    let mut subtotal = Decimal::ZERO;
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
        let amount = (item.quantity * rate).round_dp(2);
        subtotal += amount;
        render_items.push(RenderLineItem {
            description: item.description.clone(),
            quantity_display: format_quantity(item.quantity),
            rate_display: fmt(rate),
            amount_display: fmt(amount),
        });
    }

    let tax_rate = invoice.tax_rate.unwrap_or(config.defaults.tax_rate);
    if tax_rate.is_sign_negative() {
        bail!("tax_rate is negative");
    }
    let tax = (subtotal * tax_rate / Decimal::from(100)).round_dp(2);
    let total = subtotal + tax;
    let tax_note = invoice
        .tax_note
        .clone()
        .or_else(|| config.defaults.tax_note.clone());

    // 6. Date formatting
    let date_display = strtime::format(&config.defaults.date_format, invoice.date)
        .with_context(|| format!("formatting date with '{}'", config.defaults.date_format))?;

    // 7. Sender
    let sender_name = config.sender.name.clone();
    let sender_address = config.sender.address.clone().unwrap_or_default();

    // 8. Logo
    let (logo_bytes, logo_virtual_name) = match &config.sender.logo {
        Some(p) => {
            let expanded = expand_tilde(p);
            let bytes = std::fs::read(&expanded)
                .with_context(|| format!("reading logo {}", expanded.display()))?;
            let ext = expanded
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("svg")
                .to_lowercase();
            (Some(bytes), Some(format!("/logo.{ext}")))
        }
        None => (None, None),
    };

    // 9. Output path
    let output_path = match &args.output {
        Some(p) => p.clone(),
        None => {
            let dir = resolve_relative(invoice_dir, &config.defaults.output_dir);
            std::fs::create_dir_all(&dir).ok();
            dir.join(format!("invoice-{}.pdf", invoice.number))
        }
    };

    let render = RenderInvoice {
        number: invoice.number.to_string(),
        date_display,
        po_number: invoice.po_number.unwrap_or_default(),
        balance_due_display: fmt(total),
        tax_label: if tax_rate.is_zero() {
            "Tax".to_string()
        } else {
            format!("Tax ({}%)", tax_rate.normalize())
        },
        tax_note,
        logo_path: logo_virtual_name.clone(),
        sender: RenderParty {
            name: sender_name,
            address_lines: split_lines(&sender_address),
        },
        bill_to_lines: split_lines(&bill_to),
        ship_to_lines: split_lines(&ship_to),
        notes_lines: split_lines(invoice.notes.as_deref().unwrap_or("")),
        items: render_items,
        subtotal_display: fmt(subtotal),
        tax_display: fmt(tax),
        total_display: fmt(total),
    };

    Ok(Resolved {
        render,
        logo_bytes,
        logo_virtual_name,
        output_path,
    })
}

fn split_lines(s: &str) -> Vec<String> {
    s.lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}
