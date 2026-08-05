//! End-to-end tests: spawn the real `fusedframes` binary against a local mock
//! API server and a temp-dir home, asserting the full observable contract —
//! JSON stdout, exit codes, request shapes, config file handling and the key
//! hygiene rules. Plain-HTTP loopback URLs are an intentional carve-out in the
//! client, which is what makes these tests possible without TLS fixtures.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};

/// A fresh CLI process with a hermetic environment rooted in `home`.
fn cli(home: &tempfile::TempDir) -> Command {
    let mut cmd = Command::cargo_bin("fusedframes").expect("binary builds");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    // Windows resolves the home directory from USERPROFILE instead.
    cmd.env("USERPROFILE", home.path());
    // Windows: a child process without SYSTEMROOT cannot initialise Winsock
    // (WSAEPROVIDERFAILEDINIT, os error 10106), so every request would fail
    // before reaching the mock server. Harmless on Unix, where it is unset.
    if let Ok(system_root) = std::env::var("SYSTEMROOT") {
        cmd.env("SYSTEMROOT", system_root);
    }
    cmd
}

fn home() -> tempfile::TempDir {
    tempfile::tempdir().expect("temp home dir")
}

fn config_path(home: &tempfile::TempDir) -> std::path::PathBuf {
    home.path()
        .join(".config")
        .join("fusedframes")
        .join("config.json")
}

fn write_config(home: &tempfile::TempDir, value: &Value) {
    let path = config_path(home);
    std::fs::create_dir_all(path.parent().expect("config dir")).expect("create config dir");
    std::fs::write(&path, value.to_string()).expect("write config");
}

fn read_config_file(home: &tempfile::TempDir) -> Value {
    let data = std::fs::read_to_string(config_path(home)).expect("config file exists");
    serde_json::from_str(&data).expect("config file is JSON")
}

fn stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout is one JSON document")
}

// ─── Argument handling ──────────────────────────────────────────────────────

#[test]
fn version_flag_exits_zero() {
    let home = home();
    cli(&home)
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_flag_exits_zero() {
    let home = home();
    cli(&home)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Find and read the documents FusedFrames makes from your recorded work",
        ));
}

#[test]
fn bare_invocation_prints_help_and_exits_one_without_json() {
    let home = home();
    cli(&home)
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn bare_subcommand_group_prints_help_and_exits_one_without_json() {
    let home = home();
    cli(&home)
        .arg("libraries")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn unknown_option_is_a_json_validation_error() {
    let home = home();
    let output = cli(&home)
        .args(["libraries", "list", "--nope"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "validation_error");
    assert!(
        body["error"]["message"]
            .as_str()
            .expect("message is a string")
            .contains("--nope")
    );
}

#[test]
fn unknown_command_is_a_json_validation_error() {
    let home = home();
    let output = cli(&home)
        .arg("bogus")
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "validation_error");
    assert!(
        body["error"]["message"]
            .as_str()
            .expect("message is a string")
            .contains("bogus")
    );
}

// ─── config set-key ─────────────────────────────────────────────────────────

#[test]
fn set_key_reads_stdin_trims_and_saves() {
    let home = home();
    cli(&home)
        .args(["config", "set-key"])
        .write_stdin("  ff_testkey123\n")
        .assert()
        .success()
        .stdout(predicate::str::diff(
            "{\"success\":true,\"message\":\"API key saved\"}\n",
        ));

    let config = read_config_file(&home);
    assert_eq!(config["apiKey"], "ff_testkey123");
}

#[cfg(unix)]
#[test]
fn set_key_writes_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let home = home();
    cli(&home)
        .args(["config", "set-key"])
        .write_stdin("ff_testkey123")
        .assert()
        .success();

    let file_mode = std::fs::metadata(config_path(&home))
        .expect("config file")
        .permissions()
        .mode();
    let dir_mode = std::fs::metadata(config_path(&home).parent().expect("dir"))
        .expect("config dir")
        .permissions()
        .mode();
    assert_eq!(file_mode & 0o777, 0o600, "config file must be 0600");
    assert_eq!(dir_mode & 0o777, 0o700, "config dir must be 0700");
}

#[cfg(unix)]
#[test]
fn set_key_tightens_pre_existing_loose_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let home = home();
    write_config(&home, &json!({ "apiKey": "ff_old" }));
    let path = config_path(&home);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("chmod");
    std::fs::set_permissions(
        path.parent().expect("dir"),
        std::fs::Permissions::from_mode(0o755),
    )
    .expect("chmod dir");

    cli(&home)
        .args(["config", "set-key"])
        .write_stdin("ff_new")
        .assert()
        .success();

    let file_mode = std::fs::metadata(&path).expect("file").permissions().mode();
    let dir_mode = std::fs::metadata(path.parent().expect("dir"))
        .expect("dir")
        .permissions()
        .mode();
    assert_eq!(file_mode & 0o777, 0o600);
    assert_eq!(dir_mode & 0o777, 0o700);
}

#[test]
fn set_key_rejects_keys_passed_as_arguments() {
    let home = home();
    let output = cli(&home)
        .args(["config", "set-key", "ff_leaked_via_argv"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "validation_error");
    let message = body["error"]["message"].as_str().expect("message");
    assert!(message.contains("Your shell saves it in history"));
    assert!(message.contains("other programs can see it"));
    // The rejected key must not be written anywhere.
    assert!(!config_path(&home).exists());
}

#[test]
fn set_key_with_empty_stdin_is_a_validation_error() {
    let home = home();
    let output = cli(&home)
        .args(["config", "set-key"])
        .write_stdin("\n")
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "validation_error");
    assert_eq!(body["error"]["message"], "No API key was given");
}

#[test]
fn set_key_preserves_unknown_config_fields() {
    let home = home();
    write_config(
        &home,
        &json!({ "apiKey": "ff_old", "future": { "keep": 1 } }),
    );

    cli(&home)
        .args(["config", "set-key"])
        .write_stdin("ff_new")
        .assert()
        .success();

    let config = read_config_file(&home);
    assert_eq!(config["apiKey"], "ff_new");
    assert_eq!(config["future"]["keep"], 1);
}

// ─── config show ────────────────────────────────────────────────────────────

#[test]
fn config_show_masks_the_stored_key() {
    let home = home();
    write_config(&home, &json!({ "apiKey": "ff_secretvalue123" }));

    let output = cli(&home)
        .args(["config", "show"])
        .assert()
        .success()
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["apiKey"], "ff_secre...");
    assert_eq!(body["apiKeySource"], "config");
    assert_eq!(body["apiUrl"], "https://api.fusedframes.com");
    assert_eq!(body["apiUrlSource"], "default");
    assert!(
        body["configPath"]
            .as_str()
            .expect("configPath")
            .ends_with("config.json")
    );
    // The full key never appears anywhere in the output.
    assert!(!String::from_utf8_lossy(&output.stdout).contains("ff_secretvalue123"));
}

#[test]
fn config_show_prefers_the_environment_key() {
    let home = home();
    write_config(&home, &json!({ "apiKey": "ff_storedkey" }));

    let output = cli(&home)
        .args(["config", "show"])
        .env("FUSEDFRAMES_API_KEY", "ff_envkey999")
        .env("FUSEDFRAMES_API_URL", "https://staging.example.com")
        .assert()
        .success()
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["apiKey"], "ff_envke...");
    assert_eq!(body["apiKeySource"], "environment");
    assert_eq!(body["apiUrl"], "https://staging.example.com");
    assert_eq!(body["apiUrlSource"], "environment");
}

#[test]
fn config_show_with_no_key_reports_none() {
    let home = home();
    let output = cli(&home)
        .args(["config", "show"])
        .assert()
        .success()
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["apiKey"], Value::Null);
    assert_eq!(body["apiKeySource"], "none");
}

#[test]
fn empty_environment_key_falls_back_to_the_stored_key() {
    let home = home();
    write_config(&home, &json!({ "apiKey": "ff_storedkey" }));

    let output = cli(&home)
        .args(["config", "show"])
        .env("FUSEDFRAMES_API_KEY", "")
        .assert()
        .success()
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["apiKeySource"], "config");
}

#[test]
fn corrupt_config_warns_on_stderr_and_continues() {
    let home = home();
    let path = config_path(&home);
    std::fs::create_dir_all(path.parent().expect("dir")).expect("create dir");
    std::fs::write(&path, "{ not json").expect("write corrupt config");

    let output = cli(&home)
        .args(["config", "show"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "is not valid JSON, so the CLI skipped it",
        ))
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["apiKeySource"], "none");
}

// ─── logout ─────────────────────────────────────────────────────────────────

#[test]
fn logout_removes_the_stored_key_and_preserves_other_fields() {
    let home = home();
    write_config(&home, &json!({ "apiKey": "ff_stored", "future": true }));

    cli(&home)
        .arg("logout")
        .assert()
        .success()
        .stdout(predicate::str::diff(
            "{\"success\":true,\"message\":\"Removed your saved API key.\"}\n",
        ));

    let config = read_config_file(&home);
    assert_eq!(config.get("apiKey"), None);
    assert_eq!(config["future"], true);
}

#[test]
fn logout_without_a_stored_key_says_so() {
    let home = home();
    let output = cli(&home)
        .arg("logout")
        .assert()
        .success()
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["message"], "There was no saved API key to remove.");
}

#[test]
fn logout_warns_when_the_env_key_is_still_set() {
    let home = home();
    write_config(&home, &json!({ "apiKey": "ff_stored" }));

    let output = cli(&home)
        .arg("logout")
        .env("FUSEDFRAMES_API_KEY", "ff_env")
        .assert()
        .success()
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["message"], "Removed your saved API key.");
    assert!(
        body["warning"]
            .as_str()
            .expect("warning")
            .contains("It wins over the saved key")
    );
}

#[test]
fn clear_key_is_an_alias_for_logout() {
    let home = home();
    write_config(&home, &json!({ "apiKey": "ff_stored" }));

    let output = cli(&home)
        .arg("clear-key")
        .assert()
        .success()
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["message"], "Removed your saved API key.");
}

// ─── Transport security ─────────────────────────────────────────────────────

#[test]
fn missing_api_key_is_reported_before_any_request() {
    let home = home();
    let output = cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_URL", "http://127.0.0.1:1")
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "config_error");
    assert!(
        body["error"]["message"]
            .as_str()
            .expect("message")
            .contains("No API key is set")
    );
}

#[test]
fn plain_http_to_a_non_loopback_host_is_refused() {
    let home = home();
    let output = cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", "http://api.fusedframes.com")
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "config_error");
    assert!(
        body["error"]["message"]
            .as_str()
            .expect("message")
            .contains("must use HTTPS")
    );
}

#[test]
fn lookalike_loopback_hosts_are_refused() {
    for url in [
        "http://localhost.evil.com",
        "http://localhost@evil.com",
        "http://foo.localhost",
    ] {
        let home = home();
        let output = cli(&home)
            .args(["libraries", "list"])
            .env("FUSEDFRAMES_API_KEY", "ff_key")
            .env("FUSEDFRAMES_API_URL", url)
            .assert()
            .failure()
            .code(1)
            .get_output()
            .clone();
        let body = stdout_json(&output);
        assert_eq!(body["error"]["code"], "config_error", "URL: {url}");
    }
}

#[test]
fn an_invalid_api_url_is_a_config_error() {
    let home = home();
    let output = cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", "not a url")
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "config_error");
    assert_eq!(body["error"]["message"], "API URL is not a valid URL.");
}

#[test]
fn unreachable_server_is_a_network_error() {
    let home = home();
    // Port 1 on loopback: nothing listens there.
    let output = cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", "http://127.0.0.1:1")
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "network_error");
    assert!(
        body["error"]["message"]
            .as_str()
            .expect("message")
            .contains("Could not reach the FusedFrames API")
    );
}

// ─── API requests ───────────────────────────────────────────────────────────

#[test]
fn libraries_list_passes_the_response_through_preserving_key_order() {
    let home = home();
    let mut server = mockito::Server::new();
    // Deliberately not alphabetical: proves the CLI re-emits keys in server
    // order rather than re-sorting them.
    let body = r#"{"zeta":1,"libraries":[{"id":"lib_1","name":"Ops"}],"alpha":{"b":2,"a":[1,2]}}"#;
    let mock = server
        .mock("GET", "/libraries")
        .match_header("authorization", "Bearer ff_key")
        .match_header("accept", "application/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success()
        .stdout(predicate::str::diff(format!("{body}\n")));
    mock.assert();
}

#[test]
fn libraries_subcommands_hit_their_endpoints() {
    for (args, path) in [
        (vec!["libraries", "get", "lib_1"], "/libraries/lib_1"),
        (
            vec!["libraries", "categories", "lib_1"],
            "/libraries/lib_1/categories",
        ),
        (vec!["libraries", "tags", "lib_1"], "/libraries/lib_1/tags"),
        (
            vec!["libraries", "applications", "lib_1"],
            "/libraries/lib_1/applications",
        ),
        (vec!["graph", "lib_1"], "/libraries/lib_1/graph"),
        (vec!["documents", "get", "doc_1"], "/documents/doc_1"),
    ] {
        let home = home();
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", path)
            .with_status(200)
            .with_body("{}")
            .create();

        cli(&home)
            .args(&args)
            .env("FUSEDFRAMES_API_KEY", "ff_key")
            .env("FUSEDFRAMES_API_URL", server.url())
            .assert()
            .success()
            .stdout(predicate::str::diff("{}\n"));
        mock.assert();
    }
}

#[test]
fn documents_list_sends_default_pagination_only() {
    let home = home();
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/libraries/lib_1/documents?page=1&pageSize=20")
        .with_status(200)
        .with_body("{}")
        .create();

    cli(&home)
        .args(["documents", "list", "lib_1"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success();
    mock.assert();
}

#[test]
fn documents_list_sends_all_filters_in_order() {
    let home = home();
    let mut server = mockito::Server::new();
    let mock = server
        .mock(
            "GET",
            "/libraries/lib_1/documents?category=Deployment&tag=rollback&application=Terminal&search=failed+health+check&page=2&pageSize=50",
        )
        .with_status(200)
        .with_body("{}")
        .create();

    cli(&home)
        .args([
            "documents",
            "list",
            "lib_1",
            "--category",
            "Deployment",
            "--tag",
            "rollback",
            "--app",
            "Terminal",
            "--search",
            "failed health check",
            "--page",
            "2",
            "--page-size",
            "50",
        ])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success();
    mock.assert();
}

#[test]
fn source_recordings_sends_pagination() {
    let home = home();
    let mut server = mockito::Server::new();
    let mock = server
        .mock(
            "GET",
            "/documents/doc_1/source-recordings?page=1&pageSize=10",
        )
        .with_status(200)
        .with_body("{}")
        .create();

    cli(&home)
        .args([
            "documents",
            "source-recordings",
            "doc_1",
            "--page-size",
            "10",
        ])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success();
    mock.assert();
}

#[test]
fn traverse_sends_defaults_and_omits_label() {
    let home = home();
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/documents/doc_1/traverse?direction=both&depth=1")
        .with_status(200)
        .with_body("{}")
        .create();

    cli(&home)
        .args(["traverse", "doc_1"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success();
    mock.assert();
}

#[test]
fn traverse_sends_explicit_options() {
    let home = home();
    let mut server = mockito::Server::new();
    let mock = server
        .mock(
            "GET",
            "/documents/doc_1/traverse?direction=outgoing&label=often+next&depth=2",
        )
        .with_status(200)
        .with_body("{}")
        .create();

    cli(&home)
        .args([
            "traverse",
            "doc_1",
            "--direction",
            "outgoing",
            "--label",
            "often next",
            "--depth",
            "2",
        ])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success();
    mock.assert();
}

#[test]
fn search_sends_query_and_filters() {
    let home = home();
    let mut server = mockito::Server::new();
    let mock = server
        .mock(
            "GET",
            "/search/documents?q=deploy+failed&libraryId=lib_9&page=1&pageSize=20",
        )
        .with_status(200)
        .with_body("{}")
        .create();

    cli(&home)
        .args(["search", "deploy failed", "--library", "lib_9"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success();
    mock.assert();
}

#[test]
fn ids_are_percent_encoded_into_the_path() {
    let home = home();
    let mut server = mockito::Server::new();
    // An id with a space and a slash cannot smuggle extra path segments.
    let mock = server
        .mock("GET", "/documents/doc%20x%2Fy")
        .with_status(200)
        .with_body("{}")
        .create();

    cli(&home)
        .args(["documents", "get", "doc x/y"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success();
    mock.assert();
}

#[test]
fn the_environment_key_wins_over_the_stored_key() {
    let home = home();
    write_config(&home, &json!({ "apiKey": "ff_stored" }));
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/libraries")
        .match_header("authorization", "Bearer ff_env")
        .with_status(200)
        .with_body("{}")
        .create();

    cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_env")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success();
    mock.assert();
}

// ─── API error handling ─────────────────────────────────────────────────────

#[test]
fn api_error_envelopes_pass_through_unchanged() {
    let home = home();
    let mut server = mockito::Server::new();
    let body = r#"{"error":{"code":"unauthorised","message":"Invalid or missing API key"}}"#;
    server
        .mock("GET", "/libraries")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::diff(format!("{body}\n")));
}

#[test]
fn plain_text_error_bodies_are_surfaced() {
    let home = home();
    let mut server = mockito::Server::new();
    // Mirrors an axum Query-extractor rejection, which bypasses the JSON envelope.
    server
        .mock("GET", "/libraries")
        .with_status(400)
        .with_body("Failed to deserialize query string: invalid digit")
        .create();

    let output = cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "server_error");
    assert_eq!(
        body["error"]["message"],
        "HTTP 400: Failed to deserialize query string: invalid digit"
    );
}

#[test]
fn not_found_suggests_updating_the_cli() {
    let home = home();
    let mut server = mockito::Server::new();
    server.mock("GET", "/libraries").with_status(404).create();

    let output = cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "server_error");
    assert!(
        body["error"]["message"]
            .as_str()
            .expect("message")
            .contains("Update the CLI")
    );
}

#[test]
fn rate_limited_errors_include_retry_after() {
    let home = home();
    let mut server = mockito::Server::new();
    server
        .mock("GET", "/libraries")
        .with_status(429)
        .with_header("retry-after", "7")
        .with_header("content-type", "application/json")
        .with_body(r#"{"error":{"code":"rate_limited","message":"Rate limit exceeded"}}"#)
        .create();

    cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::diff(concat!(
            r#"{"error":{"code":"rate_limited","message":"Rate limit exceeded","retryAfter":7}}"#,
            "\n"
        )));
}

#[test]
fn non_json_success_bodies_are_a_server_error() {
    let home = home();
    let mut server = mockito::Server::new();
    server
        .mock("GET", "/libraries")
        .with_status(200)
        .with_body("<html>totally not json</html>")
        .create();

    let output = cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "server_error");
    assert!(
        body["error"]["message"]
            .as_str()
            .expect("message")
            .contains("The API reply was not valid JSON")
    );
}

// ─── Parity regressions ─────────────────────────────────────────────────────

#[test]
fn version_output_is_the_bare_version() {
    // The TypeScript CLI printed just "2.0.0", no program-name prefix.
    let home = home();
    cli(&home)
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::diff(concat!(
            env!("CARGO_PKG_VERSION"),
            "\n"
        )));
}

#[test]
fn missing_argument_errors_name_the_argument() {
    let home = home();
    let output = cli(&home)
        .args(["documents", "list"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    let body = stdout_json(&output);
    assert_eq!(body["error"]["code"], "validation_error");
    assert!(
        body["error"]["message"]
            .as_str()
            .expect("message")
            .contains("libraryId"),
        "message must say which argument is missing: {}",
        body["error"]["message"]
    );
}

#[test]
fn option_values_may_start_with_a_hyphen() {
    let home = home();
    let mut server = mockito::Server::new();
    let mock = server
        .mock(
            "GET",
            "/libraries/lib_1/documents?search=-rf&page=-1&pageSize=20",
        )
        .with_status(200)
        .with_body("{}")
        .create();

    cli(&home)
        .args([
            "documents",
            "list",
            "lib_1",
            "--search",
            "-rf",
            "--page",
            "-1",
        ])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success();
    mock.assert();
}

#[test]
fn proxy_environment_variables_are_ignored() {
    let home = home();
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/libraries")
        .with_status(200)
        .with_body("{}")
        .create();

    // If any of these were honoured, the request would go to the dead proxy
    // port (and carry the bearer key to a host the URL checks never vetted).
    cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .env("http_proxy", "http://127.0.0.1:9")
        .env("HTTP_PROXY", "http://127.0.0.1:9")
        .env("https_proxy", "http://127.0.0.1:9")
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env("all_proxy", "http://127.0.0.1:9")
        .env("ALL_PROXY", "http://127.0.0.1:9")
        .assert()
        .success()
        .stdout(predicate::str::diff("{}\n"));
    mock.assert();
}

#[test]
fn numbers_pass_through_byte_exact() {
    let home = home();
    let mut server = mockito::Server::new();
    // 1.0 must not normalise to 1, and integers above 2^53 must not lose
    // precision (both happened in the TypeScript CLI's JSON round-trip).
    let body = r#"{"score":1.0,"big":9007199254740993,"ratio":0.30000000000000004}"#;
    server
        .mock("GET", "/libraries")
        .with_status(200)
        .with_body(body)
        .create();

    cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success()
        .stdout(predicate::str::diff(format!("{body}\n")));
}

#[test]
fn user_agent_identifies_the_cli_and_version() {
    let home = home();
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/libraries")
        .match_header(
            "user-agent",
            format!("fusedframes-cli/{}", env!("CARGO_PKG_VERSION")).as_str(),
        )
        .with_status(200)
        .with_body("{}")
        .create();

    cli(&home)
        .args(["libraries", "list"])
        .env("FUSEDFRAMES_API_KEY", "ff_key")
        .env("FUSEDFRAMES_API_URL", server.url())
        .assert()
        .success();
    mock.assert();
}

#[test]
fn malformed_api_key_type_still_preserves_other_config_fields() {
    let home = home();
    write_config(&home, &json!({ "apiKey": 123, "other": true }));

    cli(&home).arg("logout").assert().success();

    let config = read_config_file(&home);
    assert_eq!(config.get("apiKey"), None);
    assert_eq!(
        config["other"], true,
        "unknown fields must survive a rewrite"
    );
}
