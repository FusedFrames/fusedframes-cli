//! HTTP client for the FusedFrames API: URL validation, request execution and
//! error mapping.
//!
//! Responses are passed through as-is: the server's JSON (camelCase fields,
//! TypeID ids) is the contract, and the CLI must not reshape it. Parsing into
//! `serde_json::Value` (with the `preserve_order` feature) validates the body
//! is JSON while keeping fields in the order the server sent them, exactly as
//! the TypeScript CLI's `JSON.parse`/`JSON.stringify` round-trip did.

use std::time::Duration;

use serde_json::Value;
use url::{Host, Url};

use crate::config;
use crate::error::CliError;

const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_REDIRECTS: usize = 10;
const USER_AGENT: &str = concat!("fusedframes-cli/", env!("CARGO_PKG_VERSION"));

/// The API's error envelope: `{"error":{"code","message"}}`.
#[derive(serde::Deserialize)]
struct ApiErrorEnvelope {
    error: ApiErrorBody,
}

#[derive(serde::Deserialize)]
struct ApiErrorBody {
    code: Option<String>,
    message: Option<String>,
}

/// Perform a GET against the API and return the parsed response body.
///
/// `segments` are joined into the request path with each segment
/// percent-encoded individually, so an id can never smuggle extra path
/// segments or query text into the URL. `params` values that are `None` or
/// empty are omitted from the query string.
pub fn request(segments: &[&str], params: &[(&str, Option<&str>)]) -> Result<Value, CliError> {
    let api_key = config::require_api_key()?;
    let base_url = config::get_api_url();

    let parsed_base = Url::parse(&base_url)
        .map_err(|_| CliError::new("config_error", "API URL is not a valid URL."))?;

    // Require HTTPS so the API key is never sent in clear text. A plain-http
    // exemption is allowed ONLY for genuine loopback hosts, matched on the
    // parsed host. A substring or prefix check would also accept hosts like
    // `localhost.evil.com` or `http://localhost@evil.com` and leak the bearer
    // key to an attacker-controlled host.
    let loopback_http = parsed_base.scheme() == "http" && is_loopback(&parsed_base);
    if parsed_base.scheme() != "https" && !loopback_http {
        return Err(CliError::new(
            "config_error",
            "The API URL must use HTTPS. Your API key is secret and we can't send it \
             over a link that is not secure.",
        ));
    }

    let url = build_url(&parsed_base, segments, params)?;

    let response = http_client()?
        .get(url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {api_key}"))
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .map_err(|err| CliError::new("network_error", describe_network_error(&err, &base_url)))?;

    let status = response.status();
    if !status.is_success() {
        // The API sets Retry-After on every rate-limited response; surface it
        // so agents know exactly how long to back off.
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok());
        // Body read failures on an error response collapse to an empty body,
        // leaving the status to speak for itself.
        let body = response.text().unwrap_or_default();
        return Err(api_error(status, &body).with_retry_after(retry_after));
    }

    let body = response
        .text()
        .map_err(|err| CliError::new("network_error", describe_network_error(&err, &base_url)))?;
    serde_json::from_str(&body).map_err(|err| {
        CliError::new(
            "server_error",
            format!("The API reply was not valid JSON: {err}"),
        )
    })
}

/// True when the URL's host is genuinely this machine: the literal `localhost`
/// or a loopback IP. Slightly wider than the TypeScript CLI's exact-string
/// check (`127.0.0.1`/`::1`) in that any 127.0.0.0/8 address qualifies. Every
/// such address is loopback by definition, so the security boundary is
/// unchanged.
fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => domain == "localhost",
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

/// Resolve the request path against the base URL the way the TypeScript CLI's
/// `new URL(path, base)` did: an absolute path replaces any path on the base.
fn build_url(
    base: &Url,
    segments: &[&str],
    params: &[(&str, Option<&str>)],
) -> Result<Url, CliError> {
    let mut url = base.clone();
    url.set_fragment(None);
    url.set_query(None);
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|()| CliError::new("config_error", "API URL is not a valid URL."))?;
        path.clear();
        path.extend(segments);
    }
    let mut appended = false;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            if let Some(value) = value
                && !value.is_empty()
            {
                query.append_pair(key, value);
                appended = true;
            }
        }
    }
    if !appended {
        // Otherwise the serializer leaves a dangling `?` on the URL.
        url.set_query(None);
    }
    Ok(url)
}

fn http_client() -> Result<reqwest::blocking::Client, CliError> {
    // Redirects may only land on HTTPS (or loopback plain-http) targets: a
    // downgrade redirect must never cause the bearer key to travel in clear
    // text. reqwest additionally drops the Authorization header whenever a
    // redirect changes host.
    let policy = reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }
        let target_ok = attempt.url().scheme() == "https"
            || (attempt.url().scheme() == "http" && is_loopback(attempt.url()));
        if target_ok {
            attempt.follow()
        } else {
            attempt.error("redirect blocked: the new address is not HTTPS")
        }
    });

    reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .redirect(policy)
        .user_agent(USER_AGENT)
        // Ignore http_proxy/https_proxy/all_proxy: Node's fetch never honoured
        // them, and routing via a proxy would send the bearer key to a host
        // the loopback/HTTPS checks above never vetted.
        .no_proxy()
        .build()
        .map_err(|err| CliError::new("error", format!("Could not set up the HTTP client: {err}")))
}

/// Network-level failures (DNS, refused connection, TLS, timeout) come wrapped
/// in layers of reqwest errors. Surface the innermost reason and where to look
/// so the user isn't left guessing.
fn describe_network_error(err: &reqwest::Error, base_url: &str) -> String {
    if err.is_timeout() {
        return format!(
            "Request to {base_url} timed out after 30s. Check your connection, \
             or the FUSEDFRAMES_API_URL setting."
        );
    }
    let mut source: &dyn std::error::Error = err;
    while let Some(inner) = source.source() {
        source = inner;
    }
    format!(
        "Could not reach the FusedFrames API at {base_url} ({source}). \
         Check your internet connection and the FUSEDFRAMES_API_URL setting."
    )
}

fn api_error(status: reqwest::StatusCode, raw_body: &str) -> CliError {
    // A well-formed API error: pass its code and message straight through.
    if let Ok(parsed) = serde_json::from_str::<ApiErrorEnvelope>(raw_body) {
        let code = parsed.error.code.filter(|code| !code.is_empty());
        let message = parsed.error.message.filter(|message| !message.is_empty());
        if code.is_some() || message.is_some() {
            return CliError::new(
                code.unwrap_or_else(|| "unknown".to_string()),
                message.unwrap_or_else(|| format!("HTTP {}", status.as_u16())),
            );
        }
    }

    // Non-JSON error body (HTML error page, plain text, an axum extractor
    // rejection, empty): surface the raw body instead of collapsing it to just
    // "HTTP <status>".
    let detail: String = raw_body.trim().chars().take(500).collect();
    let mut message = if detail.is_empty() {
        format!("HTTP {}", status.as_u16())
    } else {
        format!("HTTP {}: {detail}", status.as_u16())
    };
    if status == reqwest::StatusCode::NOT_FOUND {
        message.push_str(" The API may have changed. Update the CLI to the latest release.");
    }
    CliError::new("server_error", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(url: &str) -> Url {
        Url::parse(url).expect("test URL parses")
    }

    #[test]
    fn loopback_hosts_are_recognised_exactly() {
        assert!(is_loopback(&parse("http://localhost:8081")));
        assert!(is_loopback(&parse("http://127.0.0.1:8081")));
        assert!(is_loopback(&parse("http://127.0.0.2")));
        assert!(is_loopback(&parse("http://[::1]:8081")));
        assert!(!is_loopback(&parse("http://localhost.evil.com")));
        assert!(!is_loopback(&parse("http://localhost@evil.com")));
        assert!(!is_loopback(&parse("http://foo.localhost")));
        assert!(!is_loopback(&parse("http://example.com")));
        assert!(!is_loopback(&parse("http://[::2]")));
    }

    #[test]
    fn build_url_percent_encodes_path_segments() {
        let base = parse("https://api.fusedframes.com");
        let url = build_url(&base, &["documents", "doc x/../y?z"], &[]).expect("builds");
        assert_eq!(
            url.as_str(),
            "https://api.fusedframes.com/documents/doc%20x%2F..%2Fy%3Fz"
        );
    }

    #[test]
    fn build_url_replaces_any_base_path_and_skips_empty_params() {
        let base = parse("http://localhost:8081/some/prefix");
        let url = build_url(
            &base,
            &["libraries"],
            &[
                ("category", None),
                ("search", Some("")),
                ("page", Some("1")),
                ("pageSize", Some("20")),
            ],
        )
        .expect("builds");
        assert_eq!(
            url.as_str(),
            "http://localhost:8081/libraries?page=1&pageSize=20"
        );
    }

    #[test]
    fn build_url_without_params_has_no_query() {
        let base = parse("https://api.fusedframes.com");
        let url = build_url(&base, &["libraries"], &[("category", None)]).expect("builds");
        assert_eq!(url.as_str(), "https://api.fusedframes.com/libraries");
    }

    #[test]
    fn build_url_form_encodes_query_values() {
        let base = parse("https://api.fusedframes.com");
        let url = build_url(
            &base,
            &["search", "documents"],
            &[("q", Some("failed deploy"))],
        )
        .expect("builds");
        assert_eq!(
            url.as_str(),
            "https://api.fusedframes.com/search/documents?q=failed+deploy"
        );
    }

    #[test]
    fn api_error_passes_the_envelope_through() {
        let err = api_error(
            reqwest::StatusCode::UNAUTHORIZED,
            r#"{"error":{"code":"unauthorised","message":"Invalid or missing API key"}}"#,
        );
        assert_eq!(err.code, "unauthorised");
        assert_eq!(err.message, "Invalid or missing API key");
    }

    #[test]
    fn api_error_fills_missing_envelope_fields() {
        let err = api_error(
            reqwest::StatusCode::IM_A_TEAPOT,
            r#"{"error":{"code":"teapot"}}"#,
        );
        assert_eq!(err.code, "teapot");
        assert_eq!(err.message, "HTTP 418");
    }

    #[test]
    fn api_error_surfaces_plain_text_bodies_truncated() {
        let long_body = "x".repeat(600);
        let err = api_error(reqwest::StatusCode::BAD_REQUEST, &long_body);
        assert_eq!(err.code, "server_error");
        assert_eq!(err.message, format!("HTTP 400: {}", "x".repeat(500)));
    }

    #[test]
    fn api_error_on_404_suggests_updating() {
        let err = api_error(reqwest::StatusCode::NOT_FOUND, "");
        assert_eq!(err.code, "server_error");
        assert_eq!(
            err.message,
            "HTTP 404 The API may have changed. Update the CLI to the latest release."
        );
    }
}
