use clap::{Parser, Subcommand};
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
    /// Path to the invoice YAML file, or '-' to read YAML from stdin.
    pub file: PathBuf,

    /// Output PDF path (default: <input-filename>.pdf beside the invoice file)
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}
