//! `traverse <documentId>`: follow edges from a document.

use crate::client::request;
use crate::error::CliError;
use crate::output;

pub fn run(
    document_id: &str,
    direction: &str,
    label: Option<&str>,
    depth: &str,
) -> Result<(), CliError> {
    let data = request(
        &["documents", document_id, "traverse"],
        &[
            ("direction", Some(direction)),
            ("label", label),
            ("depth", Some(depth)),
        ],
    )?;
    output::success(&data);
    Ok(())
}
