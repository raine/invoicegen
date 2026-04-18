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
    let p = expand_tilde(p);
    if p.is_absolute() { p } else { base.join(p) }
}

pub fn invoice_dir(invoice_file: &Path) -> Result<PathBuf> {
    Ok(invoice_file
        .parent()
        .context("invoice file has no parent dir")?
        .to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_relative_expands_tilde_before_joining() {
        let home = dirs::home_dir().expect("home dir available for test");
        assert_eq!(
            resolve_relative(
                Path::new("/tmp/invoices"),
                Path::new("~/.config/invoice/logo.png")
            ),
            home.join(".config/invoice/logo.png")
        );
    }

    #[test]
    fn resolve_relative_joins_non_absolute_paths() {
        assert_eq!(
            resolve_relative(Path::new("/tmp/invoices"), Path::new("logo.png")),
            PathBuf::from("/tmp/invoices/logo.png")
        );
    }
}
