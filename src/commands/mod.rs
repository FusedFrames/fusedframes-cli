//! Command handlers: each maps parsed arguments onto one API request and
//! prints the response, mirroring the `src/commands/` layout of the
//! TypeScript CLI.

mod config;
mod documents;
mod graph;
mod libraries;
mod search;
mod traverse;

use crate::cli::{Cli, Command};
use crate::error::CliError;

pub fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Config { command } => config::run(command),
        Command::Logout => config::logout(),
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
