//! Centralized XDG Base Directory resolution.
//!
//! All invoicegen-owned paths should go through this module rather than
//! resolving `dirs::home_dir()` directly.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Resolve an XDG base directory.
///
/// Checks the given env var first (empty or relative values are treated as
/// unset per the XDG spec), then falls back to `$HOME/<default_suffix>`.
fn base_dir(env_var: &str, default_suffix: &str) -> Result<PathBuf> {
    if let Some(val) = std::env::var_os(env_var).filter(|v| !v.is_empty()) {
        let path = PathBuf::from(val);
        if path.is_absolute() {
            return Ok(path);
        }
    }
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(default_suffix))
}

/// `$XDG_CONFIG_HOME/invoicegen` (default: `~/.config/invoicegen`)
pub fn config_dir() -> Result<PathBuf> {
    Ok(base_dir("XDG_CONFIG_HOME", ".config")?.join("invoicegen"))
}
