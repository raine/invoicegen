use rust_decimal::Decimal;

use crate::domain::{CalculatedInvoice, CalculatedLineItem, DomainInvoice};

pub fn calculate(invoice: DomainInvoice) -> CalculatedInvoice {
    let mut items = Vec::with_capacity(invoice.items.len());
    let mut subtotal = Decimal::ZERO;
    for item in invoice.items {
        let amount = (item.quantity * item.rate).round_dp(2);
        subtotal += amount;
        items.push(CalculatedLineItem {
            description: item.description,
            quantity: item.quantity,
            rate: item.rate,
            amount,
        });
    }
    let tax = (subtotal * invoice.tax_rate / Decimal::from(100)).round_dp(2);
    let total = subtotal + tax;

    CalculatedInvoice {
        number: invoice.number,
        date: invoice.date,
        po_number: invoice.po_number,
        notes: invoice.notes,
        sender: invoice.sender,
        bill_to: invoice.bill_to,
        ship_to: invoice.ship_to,
        items,
        subtotal,
        tax_rate: invoice.tax_rate,
        tax,
        total,
        tax_note: invoice.tax_note,
        currency: invoice.currency,
        date_format: invoice.date_format,
        logo_path: invoice.logo_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainLineItem, Party};
    use jiff::civil::date;
    use rust_decimal_macros::dec;

    fn inv(items: Vec<DomainLineItem>, tax_rate: Decimal) -> DomainInvoice {
        DomainInvoice {
            number: 1,
            date: date(2026, 1, 1),
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
            currency: "EUR".into(),
            date_format: "%Y".into(),
            logo_path: None,
        }
    }

    fn item(q: Decimal, r: Decimal) -> DomainLineItem {
        DomainLineItem {
            description: "x".into(),
            quantity: q,
            rate: r,
        }
    }

    #[test]
    fn single_line_totals() {
        let c = calculate(inv(vec![item(dec!(2), dec!(100))], dec!(0)));
        assert_eq!(c.items[0].amount, dec!(200.00));
        assert_eq!(c.subtotal, dec!(200.00));
        assert_eq!(c.tax, dec!(0.00));
        assert_eq!(c.total, dec!(200.00));
    }

    #[test]
    fn tax_rounding() {
        let c = calculate(inv(vec![item(dec!(1), dec!(100))], dec!(24)));
        assert_eq!(c.subtotal, dec!(100.00));
        assert_eq!(c.tax, dec!(24.00));
        assert_eq!(c.total, dec!(124.00));
    }

    #[test]
    fn rounds_per_line_before_summing() {
        // 0.333 * 3 = 0.999 -> rounds to 1.00; two such lines -> 2.00
        let c = calculate(inv(
            vec![item(dec!(0.333), dec!(3)), item(dec!(0.333), dec!(3))],
            dec!(0),
        ));
        assert_eq!(c.items[0].amount, dec!(1.00));
        assert_eq!(c.subtotal, dec!(2.00));
    }

    #[test]
    fn fractional_tax() {
        // subtotal 100, tax 7.5% = 7.50
        let c = calculate(inv(vec![item(dec!(1), dec!(100))], dec!(7.5)));
        assert_eq!(c.tax, dec!(7.50));
        assert_eq!(c.total, dec!(107.50));
    }
}
