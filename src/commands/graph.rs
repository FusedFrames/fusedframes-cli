//! `graph <libraryId>`: the full document graph for a library.

use crate::client::request;
use crate::error::CliError;
use crate::output;

pub fn run(library_id: &str) -> Result<(), CliError> {
    let data = request(&["libraries", library_id, "graph"], &[])?;
    output::success(&data);
    Ok(())
}
