use clap::{Parser, Subcommand};
use jiff::civil::Date;
use rust_decimal::Decimal;
use std::path::PathBuf;

use crate::invoice_input::LineItemInput;

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
    Generate(Box<GenerateArgs>),
}

#[derive(Parser)]
pub struct InitArgs {
    /// Overwrite an existing config file
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser)]
pub struct GenerateArgs {
    /// Path to the invoice YAML file. Omit to build the invoice entirely from CLI flags.
    pub file: Option<PathBuf>,

    /// Invoice number
    #[arg(long)]
    pub number: Option<u32>,

    /// Invoice date (YYYY-MM-DD)
    #[arg(long)]
    pub date: Option<Date>,

    /// PO number
    #[arg(long)]
    pub po: Option<String>,

    /// Client template key
    #[arg(long)]
    pub client: Option<String>,

    /// Notes
    #[arg(long)]
    pub notes: Option<String>,

    /// Add a line item. Format: "DESCRIPTION: QUANTITY [@ RATE]"
    /// (e.g. --item "Consulting: 10 @ 150"). Repeat for multiple items.
    /// When provided, replaces items from the YAML file entirely.
    #[arg(long = "item", conflicts_with_all = ["description", "quantity", "rate"])]
    pub items: Vec<LineItemInput>,

    /// First item description (shortcut; overrides YAML first item)
    #[arg(long)]
    pub description: Option<String>,

    /// First item quantity (shortcut; overrides YAML first item)
    #[arg(long)]
    pub quantity: Option<Decimal>,

    /// First item rate (shortcut; overrides YAML first item)
    #[arg(long)]
    pub rate: Option<Decimal>,

    /// Tax rate in percent (e.g. 24)
    #[arg(long)]
    pub tax_rate: Option<Decimal>,

    /// Tax note printed below totals
    #[arg(long)]
    pub tax_note: Option<String>,

    /// Override client bill_to (multi-line supported)
    #[arg(long)]
    pub bill_to: Option<String>,

    /// Override client ship_to (multi-line supported)
    #[arg(long)]
    pub ship_to: Option<String>,

    /// Output PDF path (default: <output_dir>/invoice-<number>.pdf)
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}
