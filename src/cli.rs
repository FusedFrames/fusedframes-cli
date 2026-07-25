//! Command-line definitions. Command names, option names, defaults and help
//! strings mirror the TypeScript CLI verbatim so agents and scripts keep
//! working unchanged.
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
    about = "Query documents FusedFrames writes from recorded work",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage CLI configuration
    #[command(arg_required_else_help = true)]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Remove the stored API key from this machine
    #[command(visible_alias = "clear-key")]
    Logout,

    /// Browse document libraries
    #[command(arg_required_else_help = true)]
    Libraries {
        #[command(subcommand)]
        command: LibrariesCommand,
    },

    /// Query documents
    #[command(arg_required_else_help = true)]
    Documents {
        #[command(subcommand)]
        command: DocumentsCommand,
    },

    /// Get the full document graph for a library
    Graph {
        #[arg(value_name = "libraryId")]
        library_id: String,
    },

    /// Traverse edges from a document
    Traverse {
        #[arg(value_name = "documentId")]
        document_id: String,
        /// Traversal direction (outgoing, incoming, both)
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
        /// Traversal depth (1-3)
        #[arg(
            long,
            allow_hyphen_values = true,
            value_name = "number",
            default_value = "1"
        )]
        depth: String,
    },

    /// Search documents across all accessible libraries
    Search {
        #[arg(value_name = "query")]
        query: String,
        /// Filter by category
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        category: Option<String>,
        /// Filter by tag
        #[arg(long, allow_hyphen_values = true, value_name = "value")]
        tag: Option<String>,
        /// Filter by application (case-insensitive)
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
    /// Show current configuration
    Show,
}

#[derive(Debug, Subcommand)]
pub enum LibrariesCommand {
    /// List all accessible document libraries
    List,
    /// Get document library detail
    Get {
        #[arg(value_name = "id")]
        id: String,
    },
    /// List categories with document counts
    Categories {
        #[arg(value_name = "id")]
        id: String,
    },
    /// List tags with document counts
    Tags {
        #[arg(value_name = "id")]
        id: String,
    },
    /// List applications with document counts
    Applications {
        #[arg(value_name = "id")]
        id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum DocumentsCommand {
    /// List documents in a library
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
    /// Get full document detail with inline edges
    Get {
        #[arg(value_name = "id")]
        id: String,
    },
    /// Get the source recordings behind a document
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
