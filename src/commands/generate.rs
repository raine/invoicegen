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
    let cli_patch = args.invoice_patch(args.file.is_none())?;

    let (invoice_patch, dir) = match &args.file {
        Some(path) => {
            let dir = invoice_dir(path)?;
            (Some(load_invoice(path)?.into_patch(&dir)), dir)
        }
        None => (None, std::env::current_dir()?),
    };

    let selected_client = cli_patch.client.clone().or_else(|| {
        invoice_patch
            .as_ref()
            .and_then(|patch| patch.client.clone())
    });

    let mut layers = vec![config.defaults_patch()];
    if let Some(client) = selected_client.as_deref()
        && let Some(client_patch) = config.client_patch(client)
    {
        layers.push(client_patch);
    }
    if let Some(invoice_patch) = invoice_patch {
        layers.push(invoice_patch);
    }
    layers.push(cli_patch);

    let invoice = merge(layers, selected_client.as_deref(), &config.client_keys())?;
    let calc = calculate(invoice);
    let render_ctx = present(&calc)?;

    let (logo_bytes, logo_virtual_name) = match &calc.invoice.logo_path {
        Some(path) => {
            let bytes =
                fs::read(path).with_context(|| format!("reading logo {}", path.display()))?;
            (Some(bytes), render_ctx.logo_path.clone())
        }
        None => (None, None),
    };

    let output_path = resolve_output_path(&args.output, &config, &dir, calc.invoice.number);

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
