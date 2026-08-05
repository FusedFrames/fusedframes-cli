//! `libraries list|get|categories|tags|applications`.

use crate::cli::LibrariesCommand;
use crate::client::request;
use crate::error::CliError;
use crate::output;

pub fn run(command: LibrariesCommand) -> Result<(), CliError> {
    let data = match command {
        LibrariesCommand::List => request(&["libraries"], &[])?,
        LibrariesCommand::Get { id } => request(&["libraries", &id], &[])?,
        LibrariesCommand::Categories { id } => request(&["libraries", &id, "categories"], &[])?,
        LibrariesCommand::Tags { id } => request(&["libraries", &id, "tags"], &[])?,
        LibrariesCommand::Applications { id } => request(&["libraries", &id, "applications"], &[])?,
    };
    output::success(&data);
    Ok(())
}
