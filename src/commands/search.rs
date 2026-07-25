//! `search <query>` — search documents across all accessible libraries.

use crate::client::request;
use crate::error::CliError;
use crate::output;

pub struct Params {
    pub query: String,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub app: Option<String>,
    pub library: Option<String>,
    pub page: String,
    pub page_size: String,
}

pub fn run(params: &Params) -> Result<(), CliError> {
    let data = request(
        &["search", "documents"],
        &[
            ("q", Some(params.query.as_str())),
            ("category", params.category.as_deref()),
            ("tag", params.tag.as_deref()),
            ("application", params.app.as_deref()),
            ("libraryId", params.library.as_deref()),
            ("page", Some(params.page.as_str())),
            ("pageSize", Some(params.page_size.as_str())),
        ],
    )?;
    output::success(&data);
    Ok(())
}
