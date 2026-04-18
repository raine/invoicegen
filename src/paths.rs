use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn expand_tilde(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    p.to_path_buf()
}

pub fn resolve_relative(base: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

pub fn invoice_dir(invoice_file: &Path) -> Result<PathBuf> {
    Ok(invoice_file
        .parent()
        .context("invoice file has no parent dir")?
        .to_path_buf())
}
