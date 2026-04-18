use clap::{Parser, Subcommand};
use jiff::civil::Date;
use rust_decimal::Decimal;
use std::path::PathBuf;

use clap::builder::styling::{AnsiColor, Effects, Styles};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(name = "invoice")]
#[command(about = "Generate PDF invoices from YAML")]
#[command(styles = STYLES)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Scaffold a starter config at ~/.config/invoice/config.yaml
    Init(InitArgs),
    /// Render an invoice YAML file to PDF
    Generate(GenerateArgs),
}

#[derive(Parser)]
pub struct InitArgs {
    /// Overwrite an existing config file
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser)]
pub struct GenerateArgs {
    /// Path to the invoice YAML file
    pub file: PathBuf,

    /// Override invoice number
    #[arg(long)]
    pub number: Option<u32>,

    /// Override invoice date (YYYY-MM-DD)
    #[arg(long)]
    pub date: Option<Date>,

    /// Override PO number
    #[arg(long)]
    pub po: Option<String>,

    /// Override client template key
    #[arg(long)]
    pub client: Option<String>,

    /// Override notes
    #[arg(long)]
    pub notes: Option<String>,

    /// Shortcut: override first item quantity
    #[arg(long)]
    pub hours: Option<Decimal>,

    /// Shortcut: override first item rate
    #[arg(long)]
    pub rate: Option<Decimal>,

    /// Output PDF path (default: <output_dir>/invoice-<number>.pdf)
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}
