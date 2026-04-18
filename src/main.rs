use anyhow::Result;
use clap::Parser;
use invoice::cli::{Cli, Command};
use invoice::commands;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init(args) => commands::init::run(args),
        Command::Generate(args) => commands::generate::run(*args),
    }
}
