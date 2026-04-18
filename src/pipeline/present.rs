use anyhow::{Context, Result};
use jiff::fmt::strtime;
use rust_decimal::Decimal;

use crate::domain::CalculatedInvoice;
use crate::invoice::{RenderContext, RenderLineItem, RenderParty};
use crate::money::{format_money, format_quantity};

pub fn present(calc: &CalculatedInvoice) -> Result<RenderContext> {
    let currency = calc.currency;
    let locale = calc.locale;
    let fmt = |d: Decimal| format_money(d, currency, locale);

    let date_display = strtime::format(&calc.date_format, calc.date)
        .with_context(|| format!("formatting date with '{}'", calc.date_format))?;

    let items = calc
        .items
        .iter()
        .map(|item| RenderLineItem {
            description: item.description.clone(),
            quantity_display: format_quantity(item.quantity),
            rate_display: fmt(item.rate),
            amount_display: fmt(item.amount),
        })
        .collect();

    let tax_label = if calc.tax_rate.is_zero() {
        "Tax".to_string()
    } else {
        format!("Tax ({}%)", calc.tax_rate.normalize())
    };

    let logo_virtual_path = calc.logo_path.as_ref().map(|p| {
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("svg")
            .to_lowercase();
        format!("/logo.{ext}")
    });

    Ok(RenderContext {
        number: calc.number.to_string(),
        date_display,
        po_number: calc.po_number.clone().unwrap_or_default(),
        balance_due_display: fmt(calc.total),
        tax_label,
        tax_note: calc.tax_note.clone(),
        logo_path: logo_virtual_path,
        sender: RenderParty {
            name: calc.sender.name.clone(),
            address_lines: split_lines(&calc.sender.address),
        },
        bill_to_lines: split_lines(&calc.bill_to),
        ship_to_lines: split_lines(&calc.ship_to),
        notes_lines: split_lines(calc.notes.as_deref().unwrap_or("")),
        items,
        subtotal_display: fmt(calc.subtotal),
        tax_display: fmt(calc.tax),
        total_display: fmt(calc.total),
    })
}

fn split_lines(s: &str) -> Vec<String> {
    s.lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CalculatedLineItem, Party};
    use jiff::civil::date;
    use rust_decimal_macros::dec;
    use std::path::PathBuf;

    fn base() -> CalculatedInvoice {
        CalculatedInvoice {
            number: 7,
            date: date(2026, 4, 18),
            po_number: None,
            notes: None,
            sender: Party {
                name: "Me".into(),
                address: "1 Home\n\nCity".into(),
            },
            bill_to: "Acme\n1 Main".into(),
            ship_to: "".into(),
            items: vec![CalculatedLineItem {
                description: "work".into(),
                quantity: dec!(2),
                rate: dec!(100),
                amount: dec!(200),
            }],
            subtotal: dec!(200),
            tax_rate: dec!(0),
            tax: dec!(0),
            total: dec!(200),
            tax_note: None,
            currency: crate::currency::Currency::Eur,
            locale: crate::locale::Locale::EnUs,
            date_format: "%Y-%m-%d".into(),
            logo_path: None,
        }
    }

    #[test]
    fn formats_eur() {
        let r = present(&base()).unwrap();
        assert_eq!(r.total_display, "€200.00");
        assert_eq!(r.subtotal_display, "€200.00");
        assert_eq!(r.items[0].rate_display, "€100.00");
    }

    #[test]
    fn formats_fi_fi_suffix() {
        let mut c = base();
        c.locale = crate::locale::Locale::FiFi;
        let r = present(&c).unwrap();
        assert_eq!(r.total_display, "200,00\u{00A0}€");
    }

    #[test]
    fn jpy_has_no_decimals() {
        let mut c = base();
        c.currency = crate::currency::Currency::Jpy;
        c.locale = crate::locale::Locale::JaJp;
        c.total = dec!(1234.56);
        let r = present(&c).unwrap();
        assert_eq!(r.total_display, "¥1,235");
    }

    #[test]
    fn tax_label_zero_vs_nonzero() {
        let mut c = base();
        assert_eq!(present(&c).unwrap().tax_label, "Tax");
        c.tax_rate = dec!(24);
        assert_eq!(present(&c).unwrap().tax_label, "Tax (24%)");
        c.tax_rate = dec!(7.5);
        assert_eq!(present(&c).unwrap().tax_label, "Tax (7.5%)");
    }

    #[test]
    fn date_formatting() {
        let r = present(&base()).unwrap();
        assert_eq!(r.date_display, "2026-04-18");
    }

    #[test]
    fn multiline_splitting_drops_blanks() {
        let r = present(&base()).unwrap();
        assert_eq!(r.sender.address_lines, vec!["1 Home", "City"]);
        assert_eq!(r.bill_to_lines, vec!["Acme", "1 Main"]);
        assert!(r.ship_to_lines.is_empty());
        assert!(r.notes_lines.is_empty());
    }

    #[test]
    fn logo_virtual_path_from_extension() {
        let mut c = base();
        c.logo_path = Some(PathBuf::from("/x/y/brand.PNG"));
        let r = present(&c).unwrap();
        assert_eq!(r.logo_path.as_deref(), Some("/logo.png"));
    }

    #[test]
    fn logo_virtual_path_none_when_no_logo() {
        let r = present(&base()).unwrap();
        assert_eq!(r.logo_path, None);
    }
}
