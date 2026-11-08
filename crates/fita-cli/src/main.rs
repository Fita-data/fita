//! The `fita` binary.

mod cli;
mod commands;
mod demo;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Init(args) => commands::init::run(&args, cli.output),
        Command::Validate(args) => commands::validate::run(&args, cli.output),
        Command::Run(args) => commands::run::run(&args, cli.output),
        Command::Export(args) => commands::export::run(&args, cli.output),
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
