mod world;

use anyhow::Result;
use typst::layout::PagedDocument;

use crate::diagnostics::RenderError;
use crate::invoice::RenderContext;
use world::InvoiceWorld;

const TEMPLATE: &str = include_str!("../../templates/invoice-minimal.typ");

pub fn render_pdf(
    data: &RenderContext,
    logo_bytes: Option<Vec<u8>>,
    logo_virtual_name: Option<String>,
) -> Result<Vec<u8>> {
    let json =
        serde_json::to_string(data).map_err(|source| RenderError::SerializeContext { source })?;
    let world = InvoiceWorld::new(TEMPLATE.to_string(), json, logo_bytes, logo_virtual_name);

    let warned = typst::compile::<PagedDocument>(&world);
    let document = warned.output.map_err(|errs| RenderError::Compile {
        details: format_typst_errors(&errs),
    })?;

    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errs| {
        RenderError::Pdf {
            details: format_typst_errors(&errs),
        }
    })?;

    Ok(pdf)
}

fn format_typst_errors<T: std::fmt::Debug>(errors: &T) -> String {
    format!("{errors:#?}")
}
