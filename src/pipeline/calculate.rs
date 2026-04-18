use rust_decimal::Decimal;

use crate::domain::{CalculatedLineItem, InvoiceDocument, InvoiceTotals};

pub fn calculate(invoice: &InvoiceDocument) -> InvoiceTotals {
    let minor = invoice.currency.minor_unit();
    let mut items = Vec::with_capacity(invoice.items.len());
    let mut subtotal = Decimal::ZERO;
    for item in &invoice.items {
        let amount = (item.quantity * item.rate).round_dp(minor);
        subtotal += amount;
        items.push(CalculatedLineItem {
            description: item.description.clone(),
            quantity: item.quantity,
            rate: item.rate,
            amount,
        });
    }
    let tax = (subtotal * invoice.tax_rate / Decimal::from(100)).round_dp(minor);
    let total = subtotal + tax;

    InvoiceTotals {
        items,
        subtotal,
        tax,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use crate::domain::{InvoiceLineItem, Party};
    use crate::locale::Locale;
    use jiff::civil::date;
    use rust_decimal_macros::dec;

    fn inv(items: Vec<InvoiceLineItem>, tax_rate: Decimal) -> InvoiceDocument {
        InvoiceDocument {
            number: 1,
            date: date(2026, 1, 1),
            client: None,
            po_number: None,
            notes: None,
            sender: Party {
                name: "S".into(),
                address: "".into(),
            },
            bill_to: "B".into(),
            ship_to: "".into(),
            items,
            tax_rate,
            tax_note: None,
            currency: Currency::Eur,
            locale: Locale::EnUs,
            date_format: "%Y".into(),
            logo_path: None,
        }
    }

    fn item(q: Decimal, r: Decimal) -> InvoiceLineItem {
        InvoiceLineItem {
            description: "x".into(),
            quantity: q,
            rate: r,
        }
    }

    #[test]
    fn single_line_totals() {
        let invoice = inv(vec![item(dec!(2), dec!(100))], dec!(0));
        let c = calculate(&invoice);
        assert_eq!(c.items[0].amount, dec!(200.00));
        assert_eq!(c.subtotal, dec!(200.00));
        assert_eq!(c.tax, dec!(0.00));
        assert_eq!(c.total, dec!(200.00));
    }

    #[test]
    fn tax_rounding() {
        let invoice = inv(vec![item(dec!(1), dec!(100))], dec!(24));
        let c = calculate(&invoice);
        assert_eq!(c.subtotal, dec!(100.00));
        assert_eq!(c.tax, dec!(24.00));
        assert_eq!(c.total, dec!(124.00));
    }

    #[test]
    fn rounds_per_line_before_summing() {
        // 0.333 * 3 = 0.999 -> rounds to 1.00; two such lines -> 2.00
        let invoice = inv(
            vec![item(dec!(0.333), dec!(3)), item(dec!(0.333), dec!(3))],
            dec!(0),
        );
        let c = calculate(&invoice);
        assert_eq!(c.items[0].amount, dec!(1.00));
        assert_eq!(c.subtotal, dec!(2.00));
    }

    #[test]
    fn fractional_tax() {
        // subtotal 100, tax 7.5% = 7.50
        let invoice = inv(vec![item(dec!(1), dec!(100))], dec!(7.5));
        let c = calculate(&invoice);
        assert_eq!(c.tax, dec!(7.50));
        assert_eq!(c.total, dec!(107.50));
    }
}
