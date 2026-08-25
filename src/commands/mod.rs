//! Command handlers: each maps parsed arguments onto one API request and
//! prints the response, mirroring the `src/commands/` layout of the
//! TypeScript CLI.

mod config;
mod documents;
mod graph;
mod libraries;
mod search;
mod traverse;
mod whoami;

use std::io::Write;

use clap::CommandFactory;

use crate::cli::{Cli, Command};
use crate::error::CliError;

pub fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Config { command } => config::run(command),
        Command::Logout => config::logout(),
        Command::Whoami => whoami::run(),
        Command::Completions { shell } => {
            // Straight to stdout: a completion script is meant to be sourced or
            // redirected, so it is the one command whose output is a shell script
            // rather than a result.
            let mut command = Cli::command();
            let name = command.get_name().to_string();
            clap_complete::generate(shell, &mut command, name, &mut std::io::stdout());
            let _ = std::io::stdout().flush();
            Ok(())
        }
        Command::Libraries { command } => libraries::run(command),
        Command::Documents { command } => documents::run(command),
        Command::Graph { library_id } => graph::run(&library_id),
        Command::Traverse {
            document_id,
            direction,
            label,
            depth,
        } => traverse::run(&document_id, &direction, label.as_deref(), &depth),
        Command::Search {
            query,
            category,
            tag,
            app,
            library,
            page,
            page_size,
        } => search::run(&search::Params {
            query,
            category,
            tag,
            app,
            library,
            page,
            page_size,
        }),
    }
}
