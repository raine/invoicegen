use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::GenerateArgs;
use crate::config::{AppConfig, default_config_path, load_or_default};
use crate::diagnostics::GenerateError;
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
            let bytes = fs::read(path).map_err(|source| GenerateError::ReadLogo {
                path: path.clone(),
                source,
            })?;
            (Some(bytes), render_ctx.logo_path.clone())
        }
        None => (None, None),
    };

    let output_path = resolve_output_path(&args.output, &config, &args.file, &dir, invoice.number);

    let pdf = render_pdf(&render_ctx, logo_bytes, logo_virtual_name)?;

    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| GenerateError::CreateOutputDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(&output_path, pdf).map_err(|source| GenerateError::WriteOutput {
        path: output_path.clone(),
        source,
    })?;

    println!("Wrote {}", output_path.display());
    Ok(())
}

fn invoice_base_dir(invoice_file: &Path) -> Result<PathBuf> {
    if invoice_file == Path::new("-") {
        Ok(
            std::env::current_dir().map_err(|source| GenerateError::StdinBaseDir {
                source: source.into(),
            })?,
        )
    } else {
        Ok(
            invoice_dir(invoice_file).map_err(|source| GenerateError::InvoiceBaseDir {
                path: invoice_file.to_path_buf(),
                source,
            })?,
        )
    }
}

fn resolve_output_path(
    cli_output: &Option<PathBuf>,
    config: &AppConfig,
    invoice_file: &Path,
    invoice_dir: &Path,
    number: u32,
) -> PathBuf {
    match cli_output {
        Some(p) => p.clone(),
        None => match &config.defaults.output_dir {
            Some(dir) => {
                resolve_relative(invoice_dir, dir).join(default_output_name(invoice_file, number))
            }
            None => invoice_dir.join(default_output_name(invoice_file, number)),
        },
    }
}

fn default_output_name(invoice_file: &Path, number: u32) -> String {
    if invoice_file == Path::new("-") {
        return format!("invoice-{number}.pdf");
    }

    invoice_file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .map(|stem| format!("{stem}.pdf"))
        .unwrap_or_else(|| format!("invoice-{number}.pdf"))
}

#[cfg(test)]
mod tests {
    use super::default_output_name;
    use std::path::Path;

    #[test]
    fn output_name_uses_invoice_basename() {
        assert_eq!(
            default_output_name(Path::new("/tmp/2026-02_INV-017.yaml"), 17),
            "2026-02_INV-017.pdf"
        );
    }

    #[test]
    fn stdin_falls_back_to_invoice_number() {
        assert_eq!(default_output_name(Path::new("-"), 17), "invoice-17.pdf");
    }
}
