//! `documents list|get|source-recordings`.

use crate::cli::DocumentsCommand;
use crate::client::request;
use crate::error::CliError;
use crate::output;

pub fn run(command: DocumentsCommand) -> Result<(), CliError> {
    let data = match command {
        DocumentsCommand::List {
            library_id,
            category,
            tag,
            app,
            search,
            page,
            page_size,
        } => request(
            &["libraries", &library_id, "documents"],
            &[
                ("category", category.as_deref()),
                ("tag", tag.as_deref()),
                ("application", app.as_deref()),
                ("search", search.as_deref()),
                ("page", Some(page.as_str())),
                ("pageSize", Some(page_size.as_str())),
            ],
        )?,
        DocumentsCommand::Get { id } => request(&["documents", &id], &[])?,
        DocumentsCommand::SourceRecordings {
            id,
            page,
            page_size,
        } => request(
            &["documents", &id, "source-recordings"],
            &[
                ("page", Some(page.as_str())),
                ("pageSize", Some(page_size.as_str())),
            ],
        )?,
    };
    output::success(&data);
    Ok(())
}
