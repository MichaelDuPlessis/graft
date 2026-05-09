mod cli;
mod commands;
mod config;
mod error;
mod install;
mod link;
mod platform;
mod resolve;

use std::process;

use clap::Parser;
use colored::Colorize;

use cli::{Cli, Command};
use error::Result;

fn main() {
    let cli = Cli::parse();
    let config_path = cli.config.as_deref();

    let result: Result<()> = match &cli.command {
        Command::Apply(args) => commands::apply::run(args, config_path),
        Command::Remove(args) => commands::remove::run(args, config_path),
        Command::Add(args) => commands::add::run(args, config_path),
        Command::Status(args) => commands::status::run(args, config_path),
        Command::List(args) => commands::list::run(args, config_path),
    };

    if let Err(e) = result {
        eprintln!("{}", format!("error: {e}").red());
        process::exit(1);
    }
}
