//! `config set-key`, `config show` and `logout`.

use std::io::{IsTerminal, Read, Write};

use serde_json::json;

use crate::cli::ConfigCommand;
use crate::error::CliError;
use crate::output;

const ARGV_KEY_MESSAGE: &str = "Don't put your API key in the command itself. Your shell saves it in history and \
     other programs can see it. Pipe the key in instead: \
     echo \"ff_...\" | fusedframes config set-key. Or set the FUSEDFRAMES_API_KEY \
     environment variable.";

pub fn run(command: ConfigCommand) -> Result<(), CliError> {
    match command {
        ConfigCommand::SetKey { rejected } => set_key(&rejected),
        ConfigCommand::Show => {
            output::success(&crate::config::config_info());
            Ok(())
        }
    }
}

fn set_key(rejected: &[String]) -> Result<(), CliError> {
    // Never accept the key on the command line: argv is recorded in shell
    // history and visible to other processes via ps.
    if !rejected.is_empty() {
        return Err(CliError::new("validation_error", ARGV_KEY_MESSAGE));
    }

    let key = read_key_from_stdin()?;
    if key.is_empty() {
        return Err(CliError::new("validation_error", "No API key was given"));
    }
    validate_key_format(&key)?;

    let mut config = crate::config::read_config();
    config.api_key = Some(key);
    crate::config::write_config(&config)?;
    output::success(&json!({ "success": true, "message": "API key saved" }));
    Ok(())
}

/// The documented shape of a key: `ff_` followed by 64 hex characters.
///
/// Checked before the key is written, because a key is otherwise stored
/// unexamined and the mistake only surfaces later as an `unauthorised` on some
/// unrelated command. The usual causes are a partial paste, a copied prompt or
/// a shell that swallowed part of the string, and all of them are obvious the
/// moment the length is named.
fn validate_key_format(key: &str) -> Result<(), CliError> {
    const PREFIX: &str = "ff_";
    const BODY_LEN: usize = 64;

    let problem = if key.split_whitespace().count() > 1 {
        "it contains a space".to_string()
    } else if let Some(body) = key.strip_prefix(PREFIX) {
        if body.len() != BODY_LEN {
            format!(
                "it has {} character{} after ff_, not {BODY_LEN}",
                body.len(),
                if body.len() == 1 { "" } else { "s" }
            )
        } else if let Some(bad) = body.chars().find(|c| !c.is_ascii_hexdigit()) {
            format!("it contains \"{bad}\", which is not a hex character")
        } else {
            return Ok(());
        }
    } else {
        "it does not start with ff_".to_string()
    };

    Err(CliError::new(
        "validation_error",
        format!(
            "That does not look like a FusedFrames API key: {problem}. A key is ff_ \
             followed by {BODY_LEN} hex characters. Copy it again from the dashboard \
             or the desktop app. Nothing was saved."
        ),
    ))
}

fn read_key_from_stdin() -> Result<String, CliError> {
    let stdin = std::io::stdin();

    // Piped / redirected input (the documented
    // `echo "ff_..." | fusedframes config set-key` path): read to EOF.
    if !stdin.is_terminal() {
        let mut raw = Vec::new();
        stdin
            .lock()
            .read_to_end(&mut raw)
            .map_err(|err| CliError::new("error", format!("Could not read from stdin: {err}")))?;
        return Ok(String::from_utf8_lossy(&raw).trim().to_string());
    }

    // Interactive terminal: prompt on stderr (stdout stays clean JSON) and
    // read the line without echoing the key back to the screen.
    let mut stderr = std::io::stderr();
    let _ = write!(stderr, "Paste your API key and press Enter: ");
    let _ = stderr.flush();
    let entered = rpassword::read_password();
    let _ = writeln!(stderr);
    match entered {
        Ok(key) => Ok(key.trim().to_string()),
        Err(err) => Err(CliError::new(
            "error",
            format!("Could not read the API key from the terminal: {err}"),
        )),
    }
}

pub fn logout() -> Result<(), CliError> {
    let had = crate::config::has_stored_api_key();
    crate::config::clear_api_key()?;

    let message = if had {
        "Removed your saved API key."
    } else {
        "There was no saved API key to remove."
    };

    // The env var overrides the stored key, so clearing the file doesn't fully
    // log you out while it's set, so say so plainly.
    let env_key_set = std::env::var("FUSEDFRAMES_API_KEY").is_ok_and(|value| !value.is_empty());
    if env_key_set {
        output::success(&json!({
            "success": true,
            "message": message,
            "warning": "The FUSEDFRAMES_API_KEY environment variable is still set. It wins over \
                        the saved key. Unset it in your shell to fully sign out.",
        }));
    } else {
        output::success(&json!({ "success": true, "message": message }));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_key_format;

    const VALID: &str = "ff_287f70cd028ab08620d14a295dd5b6205f8c6cdf730f26845167aac995d748c7";

    fn rejection(key: &str) -> String {
        validate_key_format(key)
            .expect_err("key should be rejected")
            .message
    }

    #[test]
    fn a_real_key_is_accepted() {
        assert!(validate_key_format(VALID).is_ok());
        // Upper-case hex is still hex.
        assert!(validate_key_format(&format!("ff_{}", "A".repeat(64))).is_ok());
    }

    #[test]
    fn a_key_without_the_prefix_is_rejected() {
        assert!(rejection("not-a-key").contains("does not start with ff_"));
        // The 64 hex characters alone, prefix lost to a partial copy.
        assert!(rejection(&"a".repeat(64)).contains("does not start with ff_"));
    }

    #[test]
    fn a_truncated_key_names_the_length_it_got() {
        let short = &VALID[..40];
        let message = rejection(short);
        assert!(
            message.contains("37 characters after ff_"),
            "got: {message}"
        );
        assert!(message.contains("not 64"), "got: {message}");
    }

    #[test]
    fn a_key_with_a_non_hex_character_names_it() {
        // A 'z' where hex was expected: the classic OCR or hand-typed slip.
        let key = format!("ff_z{}", "a".repeat(63));
        assert!(rejection(&key).contains('z'));
    }

    #[test]
    fn a_pasted_shell_prompt_is_rejected_for_the_space_not_the_prefix() {
        // Copying the whole documented line, prompt and all.
        let message = rejection(&format!("$ {VALID}"));
        assert!(message.contains("contains a space"), "got: {message}");
    }

    #[test]
    fn nothing_is_saved_when_the_format_is_wrong() {
        // The message must say so, because the previous key is still in place
        // and the user needs to know which one is live.
        assert!(rejection("nope").contains("Nothing was saved"));
    }
}
