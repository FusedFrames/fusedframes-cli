//! Configuration storage and lookup.
//!
//! The API key lives either in the `FUSEDFRAMES_API_KEY` environment variable
//! (which always wins) or in `~/.config/fusedframes/config.json`. The path is
//! the same on every platform — deliberately not the XDG/OS-native config dir —
//! so configs written by earlier CLI versions carry over unchanged.
//!
//! The config file holds a plaintext key, so it is kept owner-only: the
//! directory is created `0700`, the file `0600`, and both are re-tightened on
//! every write in case a pre-existing file carried looser permissions.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::CliError;

pub const DEFAULT_API_URL: &str = "https://api.fusedframes.com";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Unknown fields round-trip untouched so a config written by a newer CLI
    /// is not silently stripped by an older one.
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}

fn config_dir() -> Option<PathBuf> {
    std::env::home_dir().map(|home| home.join(".config").join("fusedframes"))
}

pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("config.json"))
}

pub fn read_config() -> Config {
    let Some(path) = config_path() else {
        return Config::default();
    };
    // No config file yet (or unreadable): treat as empty config.
    let Ok(data) = std::fs::read_to_string(&path) else {
        return Config::default();
    };
    if let Ok(value) = serde_json::from_str::<Value>(&data) {
        // Valid JSON of an unexpected shape (e.g. a bare string) reads as an
        // empty config rather than a warning, matching the TypeScript CLI.
        config_from_value(value)
    } else {
        // The file exists but is corrupt. Don't silently pretend there is no
        // config — warn (on stderr, so JSON stdout stays clean) and continue
        // without it.
        crate::output::warn(&format!(
            "Warning: config file at {} is not valid JSON and was ignored. \
             Re-set your key with: echo \"ff_...\" | fusedframes config set-key",
            path.display()
        ));
        Config::default()
    }
}

/// Split a parsed config document into the known `apiKey` and everything else.
///
/// Done by hand rather than via typed deserialization so that a malformed
/// `apiKey` (wrong JSON type) never discards the rest of the file: the other
/// fields must survive a rewrite by `set-key`/`logout` regardless. A
/// non-string `apiKey` itself is dropped — both writers replace or remove it
/// anyway, and it could never authenticate.
fn config_from_value(value: Value) -> Config {
    let Value::Object(mut map) = value else {
        return Config::default();
    };
    let api_key = match map.shift_remove("apiKey") {
        Some(Value::String(key)) => Some(key),
        _ => None,
    };
    Config { api_key, rest: map }
}

pub fn write_config(config: &Config) -> Result<(), CliError> {
    let Some(dir) = config_dir() else {
        return Err(CliError::new(
            "error",
            "Could not determine the home directory to store configuration in.",
        ));
    };
    create_private_dir(&dir)?;

    let path = dir.join("config.json");
    let mut body = serde_json::to_string_pretty(config)
        .map_err(|err| CliError::new("error", format!("Could not serialise config: {err}")))?;
    body.push('\n');

    // Write to a same-directory temp file and rename it into place: the config
    // can never be observed truncated, and — because the temp file is created
    // owner-only — the plaintext key is never on disk with looser permissions.
    let tmp = dir.join(format!("config.json.{}.tmp", std::process::id()));
    write_private_file(&tmp, &body).map_err(|err| {
        CliError::new(
            "error",
            format!("Could not write config file at {}: {err}", tmp.display()),
        )
    })?;
    std::fs::rename(&tmp, &path).map_err(|err| {
        let _ = std::fs::remove_file(&tmp);
        CliError::new(
            "error",
            format!("Could not write config file at {}: {err}", path.display()),
        )
    })?;

    // A directory or file that already existed may carry looser permissions
    // (user-created, or restored from a backup that dropped perms) — force
    // owner-only on every write. Best-effort: some filesystems (and Windows)
    // don't support Unix modes.
    harden_permissions(&dir, &path);
    Ok(())
}

fn create_private_dir(dir: &Path) -> Result<(), CliError> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(dir).map_err(|err| {
        CliError::new(
            "error",
            format!(
                "Could not create config directory at {}: {err}",
                dir.display()
            ),
        )
    })
}

fn write_private_file(path: &Path, body: &str) -> std::io::Result<()> {
    use std::io::Write;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    // A stale temp file from a crashed earlier run (same pid, so vanishingly
    // rare) would make `create_new` fail — clear it and retry once.
    let mut file = match options.open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            std::fs::remove_file(path)?;
            options.open(path)?
        }
        Err(err) => return Err(err),
    };
    file.write_all(body.as_bytes())
}

#[cfg(unix)]
fn harden_permissions(dir: &Path, file: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    let _ = std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn harden_permissions(_dir: &Path, _file: &Path) {}

/// True if the stored config file currently holds an API key. Used by `logout`.
pub fn has_stored_api_key() -> bool {
    read_config().api_key.is_some_and(|key| !key.is_empty())
}

/// Remove the stored API key, preserving any other config. Used by `logout`.
pub fn clear_api_key() -> Result<(), CliError> {
    let mut config = read_config();
    config.api_key = None;
    write_config(&config)
}

/// An environment variable, with empty values counting as unset — the same
/// semantics as JS truthiness in the TypeScript CLI.
fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn get_api_key() -> Option<String> {
    // Env var takes precedence over the stored key.
    env_nonempty("FUSEDFRAMES_API_KEY")
        .or_else(|| read_config().api_key.filter(|key| !key.is_empty()))
}

pub fn get_api_url() -> String {
    env_nonempty("FUSEDFRAMES_API_URL").unwrap_or_else(|| DEFAULT_API_URL.to_string())
}

pub fn require_api_key() -> Result<String, CliError> {
    // A missing key is a configuration problem — same family as an invalid or
    // non-HTTPS API URL.
    get_api_key().ok_or_else(|| {
        CliError::new(
            "config_error",
            "API key not configured. Run: echo \"ff_...\" | fusedframes config set-key, \
             or set the FUSEDFRAMES_API_KEY environment variable.",
        )
    })
}

/// Show the first 8 characters of a key, never the whole thing.
fn mask_key(key: &str) -> String {
    let prefix: String = key.chars().take(8).collect();
    format!("{prefix}...")
}

/// The payload for `config show`: masked key, where each setting came from,
/// and where the config file lives.
pub fn config_info() -> Value {
    let env_key = env_nonempty("FUSEDFRAMES_API_KEY");
    let file_key = read_config().api_key.filter(|key| !key.is_empty());

    let (api_key, api_key_source) = match (&env_key, &file_key) {
        (Some(key), _) => (Value::String(mask_key(key)), "environment"),
        (None, Some(key)) => (Value::String(mask_key(key)), "config"),
        (None, None) => (Value::Null, "none"),
    };

    serde_json::json!({
        "apiKey": api_key,
        "apiKeySource": api_key_source,
        "apiUrl": get_api_url(),
        "apiUrlSource": if env_nonempty("FUSEDFRAMES_API_URL").is_some() { "environment" } else { "default" },
        "configPath": config_path().map_or_else(String::new, |path| path.display().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::mask_key;

    #[test]
    fn mask_shows_only_the_first_eight_characters() {
        assert_eq!(mask_key("ff_1234567890abcdef"), "ff_12345...");
    }

    #[test]
    fn mask_of_a_short_key_is_the_whole_key() {
        assert_eq!(mask_key("ff_1"), "ff_1...");
    }
}
