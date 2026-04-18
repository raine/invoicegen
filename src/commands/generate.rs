use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::GenerateArgs;
use crate::config::{AppConfig, default_config_path, load_or_default};
use crate::invoice_input::{
    ClientOverride, InvoiceFile, LineItemInput, SenderOverride, load as load_invoice,
};
use crate::paths::{invoice_dir, resolve_relative};
use crate::pipeline::{CliOverrides, calculate, merge, present};
use crate::render::render_pdf;

pub fn run(args: GenerateArgs) -> Result<()> {
    let config_path = default_config_path()?;
    let config = load_or_default(&config_path)?;

    let (invoice, dir) = match &args.file {
        Some(path) => (load_invoice(path)?, invoice_dir(path)?),
        None => (build_from_args(&args)?, std::env::current_dir()?),
    };

    let overrides = CliOverrides::from(&args);
    let domain = merge(invoice, &config, overrides, &dir)?;
    let calc = calculate(domain);
    let render_ctx = present(&calc)?;

    let (logo_bytes, logo_virtual_name) = match &calc.logo_path {
        Some(path) => {
            let bytes =
                fs::read(path).with_context(|| format!("reading logo {}", path.display()))?;
            (Some(bytes), render_ctx.logo_path.clone())
        }
        None => (None, None),
    };

    let output_path = resolve_output_path(&args.output, &config, &dir, calc.number);

    let pdf = render_pdf(&render_ctx, logo_bytes, logo_virtual_name)?;

    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output dir {}", parent.display()))?;
    }
    fs::write(&output_path, pdf).with_context(|| format!("writing {}", output_path.display()))?;

    println!("Wrote {}", output_path.display());
    Ok(())
}

fn resolve_output_path(
    cli_output: &Option<PathBuf>,
    config: &AppConfig,
    invoice_dir: &Path,
    number: u32,
) -> PathBuf {
    match cli_output {
        Some(p) => p.clone(),
        None => {
            let dir = resolve_relative(invoice_dir, &config.defaults.output_dir);
            dir.join(format!("invoice-{number}.pdf"))
        }
    }
}

fn build_from_args(args: &GenerateArgs) -> Result<InvoiceFile> {
    let number = args
        .number
        .context("--number is required without an input file")?;
    let date = args
        .date
        .context("--date is required without an input file")?;
    let client = args.client.clone();
    if client.is_none() && args.bill_to.is_none() {
        bail!("either --client or --bill-to is required without an input file");
    }
    let items = if !args.items.is_empty() {
        args.items.clone()
    } else {
        let description = args
            .description
            .clone()
            .context("either --item or --description is required without an input file")?;
        let quantity = args
            .quantity
            .context("either --item or --quantity is required without an input file")?;
        if description.is_empty() {
            bail!("--description is empty");
        }
        vec![LineItemInput {
            description,
            quantity,
            rate: args.rate,
        }]
    };

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
        items,
    })
}
