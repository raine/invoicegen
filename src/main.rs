use anyhow::Error;
use clap::Parser;
use invoicegen::cli::{Cli, Command};
use invoicegen::commands;
use invoicegen::diagnostics::{LineItemInputError, MergeError, RenderError};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Init(args) => commands::init::run(args),
        Command::Generate(args) => commands::generate::run(*args),
        Command::Docs => commands::docs::run(),
    };

    if let Err(err) = result {
        print_error(&err);
        std::process::exit(1);
    }
}

fn print_error(err: &Error) {
    eprintln!("error: {err}");

    if let Some(err) = err.downcast_ref::<MergeError>() {
        if let Some(details) = err.details() {
            eprintln!("details: {details}");
        }
        if let Some(help) = err.help() {
            eprintln!("help: {help}");
        }
        return;
    }

    if let Some(err) = err.downcast_ref::<RenderError>() {
        if let Some(details) = err.details() {
            eprintln!("details: {details}");
        }
        if let Some(help) = err.help() {
            eprintln!("help: {help}");
        }
        return;
    }

    if let Some(err) = err.downcast_ref::<LineItemInputError>() {
        eprintln!("help: {}", err.help());
        return;
    }

    for cause in err.chain().skip(1) {
        eprintln!("caused by: {cause}");
    }
}
