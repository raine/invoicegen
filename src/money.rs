use rust_decimal::Decimal;

/// Format as "€14,600.00" with thousands separators.
pub fn format_eur(amount: Decimal) -> String {
    format_with_symbol("€", amount)
}

pub fn format_with_symbol(symbol: &str, amount: Decimal) -> String {
    let rounded = amount.round_dp(2);
    let sign = if rounded.is_sign_negative() { "-" } else { "" };
    let abs = rounded.abs();
    let s = format!("{:.2}", abs);
    let (int_part, frac_part) = s.split_once('.').unwrap_or((&s, "00"));
    let with_commas = insert_thousands(int_part);
    format!("{sign}{symbol}{with_commas}.{frac_part}")
}

fn insert_thousands(digits: &str) -> String {
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
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

pub fn currency_symbol(code: &str) -> &'static str {
    match code {
        "EUR" => "€",
        "USD" => "$",
        "GBP" => "£",
        _ => "",
    }
}
