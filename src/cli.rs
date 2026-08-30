//! Command-line definitions. Option names and defaults mirror the TypeScript
//! CLI, so the flags and their values are unchanged. The command names and help
//! strings do not: the written artifact is a guide, so the command group is
//! `guides`, and the help was rewritten in plain language for v2.
//!
//! Pagination, depth and direction values are deliberately plain strings
//! passed through to the API: the server is the single source of truth for
//! validating and clamping them (e.g. `pageSize` is clamped server-side, and
//! an invalid `direction` produces the API's own `bad_request` message).

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "fusedframes",
    version,
    about = "Find and read the guides FusedFrames makes from your recorded work",
    after_help = "The hosted MCP server at mcp.fusedframes.com is the primary way to \
                  connect your agent. This CLI is the shell-based alternative for \
                  agents that work over shell commands.",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Print raw JSON, even when running in a terminal
    #[arg(long, global = true)]
    pub json: bool,
}

/// Whether `--json` was passed, read straight from the raw arguments.
///
/// The output format has to be settled before clap runs, because a parse error is
/// itself output and has to be printed in the right format. It is a plain flag with
/// no value, so scanning for it cannot misread a neighbouring argument.
pub fn wants_json() -> bool {
    std::env::args().any(|arg| arg == "--json")
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// See and change CLI settings
    #[command(arg_required_else_help = true)]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Remove the saved API key from this computer
    #[command(visible_alias = "clear-key")]
    Logout,

    /// Check that your API key works and see what it can read
    Whoami,

    /// Print a completion script for your shell
    ///
    /// Add it to your shell so command and option names complete with Tab.
    Completions {
        /// bash, zsh, fish, elvish or powershell
        #[arg(value_name = "shell")]
        shell: clap_complete::Shell,
    },

    /// Browse guide libraries
    #[command(arg_required_else_help = true)]
    Libraries {
        #[command(subcommand)]
        command: LibrariesCommand,
    },

    /// Find guides
    #[command(arg_required_else_help = true)]
    Guides {
        #[command(subcommand)]
        command: GuidesCommand,
    },

    /// Get the full guide graph for a library
    Graph {
        #[arg(value_name = "libraryId")]
        library_id: String,
    },

    /// Follow the edges from a guide
    Traverse {
        #[arg(value_name = "guideId")]
        guide_id: String,
        /// Which way to follow edges (outgoing, incoming, both)
        #[arg(
            long,
            allow_hyphen_values = true,
            value_name = "value",
            default_value = "both"
        )]
        direction: String,
        /// Filter by edge label
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        label: Option<String>,
        /// How many steps to follow (1-3)
        #[arg(
            long,
            allow_hyphen_values = true,
            value_name = "number",
            default_value = "1"
        )]
        depth: String,
    },

    /// Search guides in all the libraries you can see
    Search {
        #[arg(value_name = "query")]
        query: String,
        /// Filter by category
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        category: Option<String>,
        /// Filter by tag
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        tag: Option<String>,
        /// Filter by application (capital or small letters both work)
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        app: Option<String>,
        /// Filter by library ID
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        library: Option<String>,
        /// Page number
        #[arg(
            long,
            allow_hyphen_values = true,
            value_name = "number",
            default_value = "1"
        )]
        page: String,
        /// Results per page
        #[arg(
            long,
            allow_hyphen_values = true,
            value_name = "number",
            default_value = "20"
        )]
        page_size: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Set the API key (reads from stdin)
    SetKey {
        /// Hidden catch-all: API keys are never accepted as arguments. Anything
        /// captured here triggers the security explanation instead of a generic
        /// clap parse error.
        #[arg(hide = true, num_args = 0.., value_name = "rejected")]
        rejected: Vec<String>,
    },
    /// Show the current settings
    Show,
}

#[derive(Debug, Subcommand)]
pub enum LibrariesCommand {
    /// List all the guide libraries you can see
    List,
    /// Get guide library detail
    Get {
        #[arg(value_name = "id")]
        id: String,
    },
    /// List categories with guide counts
    Categories {
        #[arg(value_name = "id")]
        id: String,
    },
    /// List tags with guide counts
    Tags {
        #[arg(value_name = "id")]
        id: String,
    },
    /// List applications with guide counts
    Applications {
        #[arg(value_name = "id")]
        id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum GuidesCommand {
    /// List guides in a library
    List {
        #[arg(value_name = "libraryId")]
        library_id: String,
        /// Filter by category
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        category: Option<String>,
        /// Filter by tag
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        tag: Option<String>,
        /// Filter by application
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        app: Option<String>,
        /// Search term
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        search: Option<String>,
        /// Page number
        #[arg(
            long,
            allow_hyphen_values = true,
            value_name = "number",
            default_value = "1"
        )]
        page: String,
        /// Results per page
        #[arg(
            long,
            allow_hyphen_values = true,
            value_name = "number",
            default_value = "20"
        )]
        page_size: String,
    },
    /// Get the full guide, with its edges included
    Get {
        #[arg(value_name = "id")]
        id: String,
    },
    /// Get the source recordings behind a guide
    ///
    /// On every plan your API key reads the steps and answers from your own
    /// recordings. Other people's recordings never appear.
    SourceRecordings {
        #[arg(value_name = "id")]
        id: String,
        /// Page number
        #[arg(
            long,
            allow_hyphen_values = true,
            value_name = "number",
            default_value = "1"
        )]
        page: String,
        /// Results per page
        #[arg(
            long,
            allow_hyphen_values = true,
            value_name = "number",
            default_value = "20"
        )]
        page_size: String,
    },
}

#[cfg(test)]
mod tests {
    use super::Cli;

    #[test]
    fn cli_definition_is_internally_consistent() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
