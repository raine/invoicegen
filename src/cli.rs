use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use jiff::civil::Date;
use rust_decimal::Decimal;
use std::path::PathBuf;

use crate::domain::{InvoicePatch, LineItemPatch};
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

impl GenerateArgs {
    pub fn invoice_patch(&self, require_complete_invoice: bool) -> Result<InvoicePatch> {
        let mut patch = InvoicePatch {
            number: self.number,
            date: self.date,
            client: self.client.clone(),
            po_number: self.po.clone(),
            notes: self.notes.clone(),
            tax_rate: self.tax_rate,
            tax_note: self.tax_note.clone(),
            bill_to: self.bill_to.clone(),
            ship_to: self.ship_to.clone(),
            ..InvoicePatch::default()
        };

        if !self.items.is_empty() {
            patch.items = Some(self.items.iter().map(LineItemPatch::from).collect());
        } else if require_complete_invoice {
            let description = self
                .description
                .clone()
                .context("either --item or --description is required without an input file")?;
            let quantity = self
                .quantity
                .context("either --item or --quantity is required without an input file")?;
            if description.is_empty() {
                bail!("--description is empty");
            }
            patch.items = Some(vec![LineItemPatch {
                description: Some(description),
                quantity: Some(quantity),
                rate: self.rate,
            }]);
        } else if self.description.is_some() || self.quantity.is_some() || self.rate.is_some() {
            patch.first_item = Some(LineItemPatch {
                description: self.description.clone(),
                quantity: self.quantity,
                rate: self.rate,
            });
        }

        if require_complete_invoice {
            patch.number = Some(
                patch
                    .number
                    .context("--number is required without an input file")?,
            );
            patch.date = Some(
                patch
                    .date
                    .context("--date is required without an input file")?,
            );
            if patch.client.is_none() && patch.bill_to.is_none() {
                bail!("either --client or --bill-to is required without an input file");
            }
        }

        Ok(patch)
    }
}

impl From<&LineItemInput> for LineItemPatch {
    fn from(value: &LineItemInput) -> Self {
        Self {
            description: Some(value.description.clone()),
            quantity: Some(value.quantity),
            rate: value.rate,
        }
    }
}
