//! The CLI's error type: a machine-readable `code` plus a human/agent-readable
//! `message`, emitted as `{"error":{"code","message"}}` by
//! [`crate::output::error`]. Rate-limited API responses additionally carry the
//! server's Retry-After value so agents know how long to back off.

#[derive(Debug)]
pub struct CliError {
    pub code: String,
    pub message: String,
    /// Seconds to wait before retrying, from the API's `Retry-After` header
    /// (always present on v2 `rate_limited` responses).
    pub retry_after: Option<u64>,
}

impl CliError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retry_after: None,
        }
    }

    #[must_use]
    pub fn with_retry_after(mut self, seconds: Option<u64>) -> Self {
        self.retry_after = seconds;
        self
    }
}
