//! FusedFrames CLI: find and read the documents FusedFrames makes from your
//! recorded work.
//!
//! A Rust rebuild of the original TypeScript CLI with an unchanged observable
//! contract: every command prints a single line of JSON to stdout (the API
//! response passed through verbatim on success, `{"error":{"code","message"}}`
//! on failure), with exit code 0 on success and 1 on any error.

mod cli;
mod client;
mod commands;
mod config;
mod error;
mod output;

use clap::Parser;
use clap::error::ErrorKind;

use crate::error::CliError;

fn main() {
    match cli::Cli::try_parse() {
        Ok(parsed) => {
            if let Err(err) = commands::run(parsed) {
                output::error(&err);
            }
        }
        Err(err) => handle_parse_error(&err),
    }
}

/// Mirror the argument-error handling of the TypeScript CLI: `--help` and
/// `--version` exit 0 after clap's own output; a bare `fusedframes` (or a bare
/// subcommand group) prints help and exits 1 without emitting a JSON error on
/// top of it; every other parse problem becomes a JSON `validation_error` on
/// stdout so callers always get machine-readable errors.
fn handle_parse_error(err: &clap::Error) -> ! {
    use std::io::Write;

    match err.kind() {
        ErrorKind::DisplayHelp => {
            let _ = err.print();
            std::process::exit(0);
        }
        ErrorKind::DisplayVersion => {
            // The TypeScript CLI printed the bare version with no program-name
            // prefix; keep that exact output for anything parsing `--version`.
            let _ = writeln!(std::io::stdout(), "{}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            let _ = err.print();
            std::process::exit(1);
        }
        _ => {
            // clap renders multi-line messages (the problem, then tips and
            // usage in later paragraphs). The JSON error carries the first
            // paragraph collapsed onto one line, so a missing-argument error
            // still names the argument.
            let rendered = err.to_string();
            let mut message = rendered
                .lines()
                .take_while(|line| !line.trim().is_empty())
                .map(str::trim)
                .collect::<Vec<_>>()
                .join(" ");
            if message.is_empty() {
                message = "the arguments are not valid".to_string();
            }
            output::error(&CliError::new("validation_error", message));
        }
    }
}
