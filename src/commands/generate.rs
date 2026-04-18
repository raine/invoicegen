use anyhow::{Context, Result, bail};
use std::fs;

use crate::cli::GenerateArgs;
use crate::config::{default_config_path, load_or_default};
use crate::invoice_input::{
    ClientOverride, InvoiceFile, LineItemInput, SenderOverride, load as load_invoice,
};
use crate::paths::invoice_dir;
use crate::render::render_pdf;
use crate::resolve::resolve;

pub fn run(args: GenerateArgs) -> Result<()> {
    let config_path = default_config_path()?;
    let config = load_or_default(&config_path)?;

    let (invoice, dir) = match &args.file {
        Some(path) => (load_invoice(path)?, invoice_dir(path)?),
        None => (build_from_args(&args)?, std::env::current_dir()?),
    };

    let resolved = resolve(&config, invoice, &args, &dir)?;

    let pdf = render_pdf(
        &resolved.render,
        resolved.logo_bytes,
        resolved.logo_virtual_name,
    )?;

    if let Some(parent) = resolved.output_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&resolved.output_path, pdf)
        .with_context(|| format!("writing {}", resolved.output_path.display()))?;

    println!("Wrote {}", resolved.output_path.display());
    Ok(())
}

fn build_from_args(args: &GenerateArgs) -> Result<InvoiceFile> {
    let number = args
        .number
        .context("--number is required without an input file")?;
    let date = args
        .date
        .context("--date is required without an input file")?;
    let client = args.client.clone().unwrap_or_default();
    if client.is_empty() && args.bill_to.is_none() {
        bail!("either --client or --bill-to is required without an input file");
    }
    let description = args
        .description
        .clone()
        .context("--description is required without an input file")?;
    let quantity = args
        .quantity
        .context("--quantity is required without an input file")?;

    if description.is_empty() {
        bail!("--description is empty");
    }

    Ok(InvoiceFile {
        number,
        date,
        client,
        po_number: None,
        notes: None,
        tax_rate: None,
        tax_note: None,
        sender_override: SenderOverride::default(),
        client_override: ClientOverride::default(),
        items: vec![LineItemInput {
            description,
            quantity,
            rate: args.rate,
        }],
    })
}
