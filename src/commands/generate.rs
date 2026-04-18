use anyhow::{Context, Result};
use std::fs;

use crate::cli::GenerateArgs;
use crate::config::{default_config_path, load as load_config};
use crate::invoice_input::load as load_invoice;
use crate::paths::invoice_dir;
use crate::render::render_pdf;
use crate::resolve::resolve;

pub fn run(args: GenerateArgs) -> Result<()> {
    let config_path = default_config_path()?;
    let config =
        load_config(&config_path).with_context(|| "load config (run `invoice init` first)")?;

    let invoice = load_invoice(&args.file)?;
    let dir = invoice_dir(&args.file)?;

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
