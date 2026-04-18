use rust_decimal::Decimal;

use crate::currency::Currency;
use crate::locale::{Locale, SymbolPosition};

pub fn format_money(amount: Decimal, currency: Currency, locale: Locale) -> String {
    let decimals = currency.minor_unit();
    let rounded = amount.round_dp(decimals);
    let sign = if rounded.is_sign_negative() { "-" } else { "" };
    let abs = rounded.abs();

    let s = format!("{:.*}", decimals as usize, abs);
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s.as_str(), ""),
    };

    let grouped = insert_grouping(int_part, locale.group_sep());
    let number = if frac_part.is_empty() {
        grouped
    } else {
        format!("{grouped}{}{frac_part}", locale.decimal_sep())
    };

    let symbol = currency.symbol();
    match locale.symbol_position() {
        SymbolPosition::Prefix => format!("{sign}{symbol}{number}"),
        SymbolPosition::Suffix => format!("{sign}{number}\u{00A0}{symbol}"),
    }
}

fn insert_grouping(digits: &str, sep: char) -> String {
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(*b as char);
    }
    out
}

/// Quantity formatter — drop trailing zeros, keep up to 2 decimals.
pub fn format_quantity(q: Decimal) -> String {
    let r = q.round_dp(2).normalize();
    r.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn usd_en_us() {
        assert_eq!(
            format_money(d("1234.56"), Currency::Usd, Locale::EnUs),
            "$1,234.56"
        );
    }

    #[test]
    fn eur_fi_fi() {
        assert_eq!(
            format_money(d("1234.56"), Currency::Eur, Locale::FiFi),
            "1\u{00A0}234,56\u{00A0}€"
        );
    }

    #[test]
    fn eur_de_de() {
        assert_eq!(
            format_money(d("1234.56"), Currency::Eur, Locale::DeDe),
            "1.234,56\u{00A0}€"
        );
    }

    #[test]
    fn jpy_ja_jp_no_decimals() {
        assert_eq!(
            format_money(d("1234.567"), Currency::Jpy, Locale::JaJp),
            "¥1,235"
        );
        assert_eq!(format_money(d("100"), Currency::Jpy, Locale::JaJp), "¥100");
    }

    #[test]
    fn negative_prefix() {
        assert_eq!(
            format_money(d("-1234.5"), Currency::Usd, Locale::EnUs),
            "-$1,234.50"
        );
    }

    #[test]
    fn negative_suffix() {
        assert_eq!(
            format_money(d("-1234.5"), Currency::Eur, Locale::FiFi),
            "-1\u{00A0}234,50\u{00A0}€"
        );
    }

    #[test]
    fn small_amount_no_grouping() {
        assert_eq!(
            format_money(d("12.34"), Currency::Usd, Locale::EnUs),
            "$12.34"
        );
    }

    #[test]
    fn millions() {
        assert_eq!(
            format_money(d("1234567.89"), Currency::Usd, Locale::EnUs),
            "$1,234,567.89"
        );
    }
}
