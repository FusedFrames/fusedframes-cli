//! All user-visible output.
//!
//! Two audiences read this CLI and they want opposite things. A script or an agent
//! wants one line of compact JSON it can parse. A person wants to be able to read
//! the answer. So the format follows the destination: **anything other than a
//! terminal gets exactly the JSON it always got**, byte for byte, and only an
//! interactive terminal gets the rendered view. `--json` forces the machine format
//! everywhere, for the case where someone is debugging a pipeline by hand.
//!
//! Warnings always go to stderr so stdout stays parseable either way.

use std::io::{IsTerminal, Write};
use std::sync::OnceLock;

use serde_json::Value;

use crate::error::CliError;
use crate::human;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    /// One line of compact JSON: what a pipe, a file or an agent reads.
    Json,
    /// Rendered for a person at a terminal.
    Human,
}

static FORMAT: OnceLock<Format> = OnceLock::new();

/// Decide once, at startup, how this run will print.
///
/// `--json` wins; otherwise a terminal on stdout means a person is reading. Piped,
/// redirected and captured output all fall to JSON, which is what keeps every
/// existing script and `| jq` invocation working exactly as before.
pub fn init(force_json: bool) {
    let format = if force_json || !std::io::stdout().is_terminal() {
        Format::Json
    } else {
        Format::Human
    };
    let _ = FORMAT.set(format);
}

fn format() -> Format {
    // Defaulting to JSON matters: anything printed before `init` (or in a test that
    // never calls it) must be the machine-readable form.
    *FORMAT.get().unwrap_or(&Format::Json)
}

/// Print a success payload: compact JSON, or a rendered view for a terminal.
///
/// Write failures (e.g. a closed pipe when output is piped into `head`) are
/// deliberately ignored: there is nobody left to report to, and a panic would
/// turn a benign broken pipe into a crash.
pub fn success(value: &Value) {
    let mut stdout = std::io::stdout();
    match format() {
        Format::Json => {
            let _ = writeln!(stdout, "{value}");
        }
        Format::Human => {
            // An unrecognised shape prints as indented JSON rather than as nothing:
            // a new endpoint stays readable without this module knowing about it.
            let rendered = human::render(value).unwrap_or_else(|| {
                serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
            });
            let _ = writeln!(stdout, "{rendered}");
        }
    }
}

/// Report a failure and exit 1.
///
/// JSON goes to stdout, where every version of this CLI has put it. For a person
/// the message goes to stderr instead, which is where a shell expects errors and
/// which keeps a half-finished stdout from being mistaken for a result.
pub fn error(err: &CliError) -> ! {
    match format() {
        Format::Json => {
            let mut payload =
                serde_json::json!({ "error": { "code": err.code, "message": err.message } });
            if let Some(seconds) = err.retry_after {
                payload["error"]["retryAfter"] = serde_json::json!(seconds);
            }
            let _ = writeln!(std::io::stdout(), "{payload}");
        }
        Format::Human => {
            let _ = writeln!(std::io::stderr(), "Error: {}", err.message);
            if let Some(seconds) = err.retry_after {
                let _ = writeln!(
                    std::io::stderr(),
                    "Too many requests. Try again in {seconds} seconds."
                );
            }
        }
    }
    std::process::exit(1);
}

/// Print a warning line on stderr, keeping stdout clean.
pub fn warn(message: &str) {
    let _ = writeln!(std::io::stderr(), "{message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_defaults_to_json_when_nothing_decided() {
        // Anything printed before `init` must be machine-readable, never a
        // half-rendered human view.
        assert_eq!(format(), Format::Json);
    }
}
