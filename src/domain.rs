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
pub struct InvoiceLineItem {
    pub description: String,
    pub quantity: Decimal,
    pub rate: Decimal,
}

#[derive(Debug, Clone)]
pub struct InvoiceDocument {
    pub number: u32,
    pub date: Date,
    pub client: Option<String>,
    pub po_number: Option<String>,
    pub notes: Option<String>,
    pub sender: Party,
    pub bill_to: String,
    pub ship_to: String,
    pub items: Vec<InvoiceLineItem>,
    pub tax_rate: Decimal,
    pub tax_note: Option<String>,
    pub currency: Currency,
    pub locale: Locale,
    pub date_format: String,
    pub logo_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct PartyPatch {
    pub name: Option<String>,
    pub address: Option<String>,
    pub logo_path: Option<PathBuf>,
}

impl PartyPatch {
    pub fn has_any(&self) -> bool {
        self.name.is_some() || self.address.is_some() || self.logo_path.is_some()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LineItemPatch {
    pub description: Option<String>,
    pub quantity: Option<Decimal>,
    pub rate: Option<Decimal>,
}

impl LineItemPatch {
    pub fn apply(&mut self, overlay: LineItemPatch) {
        if overlay.description.is_some() {
            self.description = overlay.description;
        }
        if overlay.quantity.is_some() {
            self.quantity = overlay.quantity;
        }
        if overlay.rate.is_some() {
            self.rate = overlay.rate;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InvoicePatch {
    pub number: Option<u32>,
    pub date: Option<Date>,
    pub client: Option<String>,
    pub po_number: Option<String>,
    pub notes: Option<String>,
    pub sender: PartyPatch,
    pub bill_to: Option<String>,
    pub ship_to: Option<String>,
    pub items: Option<Vec<LineItemPatch>>,
    pub first_item: Option<LineItemPatch>,
    pub default_rate: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub tax_note: Option<String>,
    pub currency: Option<Currency>,
    pub locale: Option<Locale>,
    pub date_format: Option<String>,
}

impl InvoicePatch {
    pub fn apply(&mut self, overlay: InvoicePatch) {
        if overlay.number.is_some() {
            self.number = overlay.number;
        }
        if overlay.date.is_some() {
            self.date = overlay.date;
        }
        if overlay.client.is_some() {
            self.client = overlay.client;
        }
        if overlay.po_number.is_some() {
            self.po_number = overlay.po_number;
        }
        if overlay.notes.is_some() {
            self.notes = overlay.notes;
        }
        if overlay.sender.has_any() {
            self.sender = overlay.sender;
        }
        if overlay.bill_to.is_some() {
            self.bill_to = overlay.bill_to;
        }
        if overlay.ship_to.is_some() {
            self.ship_to = overlay.ship_to;
        }
        if overlay.items.is_some() {
            self.items = overlay.items;
        }
        if let Some(first_item) = overlay.first_item {
            match self.items.as_mut() {
                Some(items) if !items.is_empty() => items[0].apply(first_item),
                Some(items) => items.push(first_item),
                None => self.items = Some(vec![first_item]),
            }
        }
        if overlay.default_rate.is_some() {
            self.default_rate = overlay.default_rate;
        }
        if overlay.tax_rate.is_some() {
            self.tax_rate = overlay.tax_rate;
        }
        if overlay.tax_note.is_some() {
            self.tax_note = overlay.tax_note;
        }
        if overlay.currency.is_some() {
            self.currency = overlay.currency;
        }
        if overlay.locale.is_some() {
            self.locale = overlay.locale;
        }
        if overlay.date_format.is_some() {
            self.date_format = overlay.date_format;
        }
    }
}

#[derive(Debug, Clone)]
pub struct CalculatedLineItem {
    pub description: String,
    pub quantity: Decimal,
    pub rate: Decimal,
    pub amount: Decimal,
}

#[derive(Debug, Clone)]
pub struct InvoiceTotals {
    pub items: Vec<CalculatedLineItem>,
    pub subtotal: Decimal,
    pub tax: Decimal,
    pub total: Decimal,
}
