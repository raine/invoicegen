use jiff::civil::Date;
use rust_decimal::Decimal;
use std::path::PathBuf;

use crate::currency::Currency;
use crate::locale::Locale;

#[derive(Debug, Clone)]
pub struct Party {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone)]
pub struct DomainLineItem {
    pub description: String,
    pub quantity: Decimal,
    pub rate: Decimal,
}

#[derive(Debug, Clone)]
pub struct DomainInvoice {
    pub number: u32,
    pub date: Date,
    pub po_number: Option<String>,
    pub notes: Option<String>,
    pub sender: Party,
    pub bill_to: String,
    pub ship_to: String,
    pub items: Vec<DomainLineItem>,
    pub tax_rate: Decimal,
    pub tax_note: Option<String>,
    pub currency: Currency,
    pub locale: Locale,
    pub date_format: String,
    pub logo_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CalculatedLineItem {
    pub description: String,
    pub quantity: Decimal,
    pub rate: Decimal,
    pub amount: Decimal,
}

#[derive(Debug, Clone)]
pub struct CalculatedInvoice {
    pub number: u32,
    pub date: Date,
    pub po_number: Option<String>,
    pub notes: Option<String>,
    pub sender: Party,
    pub bill_to: String,
    pub ship_to: String,
    pub items: Vec<CalculatedLineItem>,
    pub subtotal: Decimal,
    pub tax_rate: Decimal,
    pub tax: Decimal,
    pub total: Decimal,
    pub tax_note: Option<String>,
    pub currency: Currency,
    pub locale: Locale,
    pub date_format: String,
    pub logo_path: Option<PathBuf>,
}
