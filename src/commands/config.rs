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

    let mut config = crate::config::read_config();
    config.api_key = Some(key);
    crate::config::write_config(&config)?;
    output::success(&json!({ "success": true, "message": "API key saved" }));
    Ok(())
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
    // log you out while it's set — say so plainly.
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
