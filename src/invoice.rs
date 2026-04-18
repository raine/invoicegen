use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RenderInvoice {
    pub number: String,
    pub date_display: String,
    pub po_number: String,
    pub balance_due_display: String,
    pub tax_label: String,
    pub tax_note: Option<String>,
    pub logo_path: Option<String>,
    pub sender: RenderParty,
    pub bill_to_lines: Vec<String>,
    pub ship_to_lines: Vec<String>,
    pub notes_lines: Vec<String>,
    pub items: Vec<RenderLineItem>,
    pub subtotal_display: String,
    pub tax_display: String,
    pub total_display: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderParty {
    pub name: String,
    pub address_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderLineItem {
    pub description: String,
    pub quantity_display: String,
    pub rate_display: String,
    pub amount_display: String,
}
