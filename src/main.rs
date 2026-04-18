use anyhow::Result;
use clap::Parser;
use invoicegen::cli::{Cli, Command};
use invoicegen::commands;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init(args) => commands::init::run(args),
        Command::Generate(args) => commands::generate::run(*args),
        Command::Docs => commands::docs::run(),
    }
}
