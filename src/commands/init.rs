use anyhow::{Context, Result, bail};
use std::fs;

use crate::cli::InitArgs;
use crate::config::default_config_path;

const STARTER: &str = r#"sender:
  name: "Your Company Ltd."
  address: |
    123 Main Street
    City, Country
  # logo: ~/.config/invoice/logo.svg

defaults:
  currency: EUR
  locale: en-US
  date_format: "%b %-d, %Y"
  output_dir: ./pdf
  tax_rate: 0
  tax_note: "VAT 0%, Export of goods or services"

clients:
  example-client:
    bill_to: |
      Example Client Inc.
      123 Example St
      City, State 12345
      Country
    default_rate: 100.00
"#;

pub fn run(args: InitArgs) -> Result<()> {
    let path = default_config_path()?;
    if path.exists() && !args.force {
        bail!(
            "config already exists at {}. Pass --force to overwrite.",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&path, STARTER).with_context(|| format!("writing {}", path.display()))?;
    println!("Wrote starter config to {}", path.display());
    Ok(())
}
