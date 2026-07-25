//! All user-visible output. Data goes to stdout as one line of compact JSON;
//! warnings go to stderr so stdout stays parseable by agents and pipelines.

use std::io::Write;

use serde_json::Value;

use crate::error::CliError;

/// Print a success payload as one line of compact JSON on stdout.
///
/// Write failures (e.g. a closed pipe when output is piped into `head`) are
/// deliberately ignored: there is nobody left to report to, and a panic would
/// turn a benign broken pipe into a crash.
pub fn success(value: &Value) {
    let _ = writeln!(std::io::stdout(), "{value}");
}

/// Print `{"error":{"code","message"[,"retryAfter"]}}` on stdout and exit 1.
pub fn error(err: &CliError) -> ! {
    let mut payload = serde_json::json!({ "error": { "code": err.code, "message": err.message } });
    if let Some(seconds) = err.retry_after {
        payload["error"]["retryAfter"] = serde_json::json!(seconds);
    }
    let _ = writeln!(std::io::stdout(), "{payload}");
    std::process::exit(1);
}

/// Print a warning line on stderr, keeping stdout clean.
pub fn warn(message: &str) {
    let _ = writeln!(std::io::stderr(), "{message}");
}
