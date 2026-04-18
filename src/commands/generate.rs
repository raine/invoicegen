use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::GenerateArgs;
use crate::config::{AppConfig, default_config_path, load_or_default};
use crate::invoice_input::load as load_invoice;
use crate::paths::{invoice_dir, resolve_relative};
use crate::pipeline::{calculate, merge, present};
use crate::render::render_pdf;

pub fn run(args: GenerateArgs) -> Result<()> {
    let config_path = default_config_path()?;
    let config = load_or_default(&config_path)?;
    let dir = invoice_base_dir(&args.file)?;
    let invoice_patch = load_invoice(&args.file)?.into_patch(&dir);
    let selected_client = invoice_patch.client.clone();

    let mut layers = vec![config.defaults_patch()];
    if let Some(client) = selected_client.as_deref()
        && let Some(client_patch) = config.client_patch(client)
    {
        layers.push(client_patch);
    }
    layers.push(invoice_patch);

    let invoice = merge(layers, selected_client.as_deref(), &config.client_keys())?;
    let totals = calculate(&invoice);
    let render_ctx = present(&invoice, &totals)?;

    let (logo_bytes, logo_virtual_name) = match &invoice.logo_path {
        Some(path) => {
            let bytes =
                fs::read(path).with_context(|| format!("reading logo {}", path.display()))?;
            (Some(bytes), render_ctx.logo_path.clone())
        }
        None => (None, None),
    };

    let output_path = resolve_output_path(&args.output, &config, &dir, invoice.number);

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

fn invoice_base_dir(invoice_file: &Path) -> Result<PathBuf> {
    if invoice_file == Path::new("-") {
        std::env::current_dir().context("reading current directory for stdin invoice")
    } else {
        invoice_dir(invoice_file)
    }
}

fn resolve_output_path(
    cli_output: &Option<PathBuf>,
    config: &AppConfig,
    invoice_dir: &Path,
    number: u32,
) -> PathBuf {
    match cli_output {
        Some(p) => p.clone(),
        None => match &config.defaults.output_dir {
            Some(dir) => resolve_relative(invoice_dir, dir).join(format!("invoice-{number}.pdf")),
            None => invoice_dir.join(format!("invoice-{number}.pdf")),
        },
    }
}
