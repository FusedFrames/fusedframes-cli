//! `whoami` — prove the key works, and say what it can reach.
//!
//! Without this the only way to test a key is to run a real query and infer the
//! answer from whether it failed, which is exactly the loop that makes a new CLI
//! feel opaque. A key can also fail in ways that look like an empty account
//! (expired, revoked, scoped to a library that was since deleted), so the check
//! reports what it CAN see rather than merely that a request succeeded.

use serde_json::{Value, json};

use crate::client;
use crate::error::CliError;
use crate::output;

pub fn run() -> Result<(), CliError> {
    // `/libraries` is the cheapest authenticated read, and its result doubles as
    // the answer to "what can this key see?".
    let data = client::request(&["libraries"], &[])?;
    let settings = crate::config::config_info();

    let libraries = data
        .get("libraries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let guides: i64 = libraries
        .iter()
        .filter_map(|library| library.get("guideCount").and_then(Value::as_i64))
        .sum();

    output::success(&json!({
        "ok": true,
        "apiKey": settings.get("apiKey"),
        "apiKeySource": settings.get("apiKeySource"),
        "apiUrl": settings.get("apiUrl"),
        "libraryCount": libraries.len(),
        "guideCount": guides,
        "libraries": libraries
            .iter()
            .map(|library| json!({
                "id": library.get("id"),
                "name": library.get("name"),
                "guideCount": library.get("guideCount"),
            }))
            .collect::<Vec<_>>(),
    }));
    Ok(())
}
