//! `guides list|get|source-recordings`.

use crate::cli::GuidesCommand;
use crate::client::request;
use crate::error::CliError;
use crate::output;

pub fn run(command: GuidesCommand) -> Result<(), CliError> {
    let data = match command {
        GuidesCommand::List {
            library_id,
            category,
            tag,
            app,
            search,
            page,
            page_size,
        } => request(
            &["libraries", &library_id, "guides"],
            &[
                ("category", category.as_deref()),
                ("tag", tag.as_deref()),
                ("application", app.as_deref()),
                ("search", search.as_deref()),
                ("page", Some(page.as_str())),
                ("pageSize", Some(page_size.as_str())),
            ],
        )?,
        GuidesCommand::Get { id } => request(&["guides", &id], &[])?,
        GuidesCommand::SourceRecordings {
            id,
            page,
            page_size,
        } => request(
            &["guides", &id, "source-recordings"],
            &[
                ("page", Some(page.as_str())),
                ("pageSize", Some(page_size.as_str())),
            ],
        )?,
    };
    output::success(&data);
    Ok(())
}
