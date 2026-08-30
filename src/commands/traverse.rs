//! `traverse <guideId>`: follow edges from a guide.

use crate::client::request;
use crate::error::CliError;
use crate::output;

pub fn run(
    guide_id: &str,
    direction: &str,
    label: Option<&str>,
    depth: &str,
) -> Result<(), CliError> {
    let data = request(
        &["guides", guide_id, "traverse"],
        &[
            ("direction", Some(direction)),
            ("label", label),
            ("depth", Some(depth)),
        ],
    )?;
    output::success(&data);
    Ok(())
}
