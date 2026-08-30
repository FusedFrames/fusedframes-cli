//! Rendering for a person reading a terminal.
//!
//! The API speaks JSON and so does this CLI whenever anything might be reading it
//! (a pipe, a file, a script, `--json`). But a human who runs `fusedframes search
//! "failed deployment"` and gets 60KB of compact JSON has been handed the raw
//! response and left to parse it themselves, which is why the CLI was hard to use
//! without `jq` next to it.
//!
//! Every renderer here is shape-driven: it recognises a response by the keys it
//! carries and falls back to indented JSON for anything it does not know, so a new
//! endpoint degrades to readable JSON rather than to nothing.

use std::fmt::Write as _;

use serde_json::Value;

const INDENT: &str = "  ";

/// Render a response for a terminal, or `None` when the shape is unrecognised and
/// the caller should print indented JSON instead.
pub fn render(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    // Ordered most specific first: a search response also has `guides`, and a
    // whoami response also has `libraries`.
    if object.contains_key("ok") && object.contains_key("libraryCount") {
        return Some(whoami(value));
    }
    if object.contains_key("libraries") {
        return Some(libraries(value));
    }
    if object.contains_key("sourceRecordings") {
        return Some(source_recordings(value));
    }
    if object.contains_key("nodes") && object.contains_key("edges") {
        return Some(traverse(value));
    }
    if object.contains_key("guides") && object.contains_key("edges") {
        return Some(graph(value));
    }
    if object.contains_key("guides") {
        return Some(guide_list(value));
    }
    // Before the vocabulary lists: a guide carries its own `tags` array, and
    // would otherwise be mistaken for one.
    if object.contains_key("content") && object.contains_key("title") {
        return Some(guide_detail(value));
    }
    if let Some(rendered) = name_counts(value) {
        return Some(rendered);
    }
    if object.contains_key("edgeCount")
        || (object.contains_key("name") && object.contains_key("id"))
    {
        return Some(library_detail(value));
    }
    if object.contains_key("apiKey") || object.contains_key("apiKeySource") {
        return Some(config(value));
    }
    if let Some(message) = object.get("message").and_then(Value::as_str) {
        let mut out = message.to_string();
        if let Some(warning) = object.get("warning").and_then(Value::as_str) {
            let _ = write!(out, "\n\n{warning}");
        }
        return Some(out);
    }
    None
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn count_at(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn list_at<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value.get(key).and_then(Value::as_array).map_or(&[], |v| v)
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    list_at(value, key)
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

/// "1 guide" / "4 guides", so no line ever reads "1 guides".
///
/// Adding an "s" is only right for the regular nouns used here. Anything else
/// (a library, a noun with a trailing phrase) spells both forms out via
/// [`plural_of`] rather than being quietly mangled into "librarys".
fn plural(count: i64, singular: &str) -> String {
    plural_of(count, singular, &format!("{singular}s"))
}

fn plural_of(count: i64, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

/// A list length as the `i64` every count in a response uses. Saturating rather
/// than wrapping: a page long enough to overflow cannot come back from a paginated
/// API, and a nonsense huge number beats a negative one if it ever did.
fn count_of(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

/// Left-aligned columns padded to the widest cell. Trailing padding is trimmed so
/// copying a line does not bring stray spaces with it.
fn table(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..columns)
        .map(|i| {
            rows.iter()
                .filter_map(|r| r.get(i))
                .map(|c| c.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    rows.iter()
        .map(|row| {
            let line: String = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let pad = widths[i].saturating_sub(cell.chars().count());
                    format!("{cell}{}", " ".repeat(pad))
                })
                .collect::<Vec<_>>()
                .join("  ");
            format!("{INDENT}{}", line.trim_end())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Wrap prose to a readable measure, indented. Long single words (ids, URLs) are
/// never broken.
fn wrap(text: &str, indent: &str, width: usize) -> String {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(format!("{indent}{line}"));
            line = String::new();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(format!("{indent}{line}"));
    }
    lines.join("\n")
}

// ─── shapes ──────────────────────────────────────────────────────────────────

fn libraries(value: &Value) -> String {
    let items = list_at(value, "libraries");
    if items.is_empty() {
        return "No guide libraries. This API key cannot see any.".into();
    }
    let mut rows = vec![vec![
        "NAME".into(),
        "GUIDES".into(),
        "CATEGORIES".into(),
        "ID".into(),
    ]];
    for library in items {
        rows.push(vec![
            str_at(library, "name").unwrap_or("(untitled)").to_string(),
            count_at(library, "guideCount").unwrap_or(0).to_string(),
            {
                let categories = strings(library, "categories");
                if categories.is_empty() {
                    "-".into()
                } else {
                    categories.join(", ")
                }
            },
            str_at(library, "id").unwrap_or("-").to_string(),
        ]);
    }
    format!(
        "{}\n\n{}",
        plural_of(count_of(items.len()), "guide library", "guide libraries"),
        table(&rows)
    )
}

fn library_detail(value: &Value) -> String {
    let mut out = vec![str_at(value, "name").unwrap_or("(untitled)").to_string()];
    if let Some(description) = str_at(value, "description") {
        out.push(wrap(description, "", 78));
    }
    out.push(String::new());
    let mut facts = vec![format!(
        "{INDENT}{}",
        plural(count_at(value, "guideCount").unwrap_or(0), "guide")
    )];
    if let Some(edges) = count_at(value, "edgeCount") {
        facts.push(format!("{INDENT}{} between guides", plural(edges, "link")));
    }
    let categories = strings(value, "categories");
    if !categories.is_empty() {
        facts.push(format!("{INDENT}Categories: {}", categories.join(", ")));
    }
    let tags = strings(value, "tags");
    if !tags.is_empty() {
        facts.push(format!("{INDENT}Tags: {}", tags.join(", ")));
    }
    if let Some(id) = str_at(value, "id") {
        facts.push(format!("{INDENT}ID: {id}"));
    }
    out.push(facts.join("\n"));
    out.join("\n")
}

/// The `{name, guideCount}` vocabulary lists. Matched on the SHAPE of the
/// entries, not just the key: a guide also has a `tags` array, but of bare
/// strings rather than counted names.
fn name_counts(value: &Value) -> Option<String> {
    let key = ["categories", "tags", "applications"]
        .into_iter()
        .find(|k| {
            value.get(k).and_then(Value::as_array).is_some_and(|items| {
                items
                    .first()
                    .is_none_or(|first| first.get("guideCount").is_some())
            })
        })?;
    let label = key;
    let items = list_at(value, key);
    if items.is_empty() {
        return Some(format!("No {label}."));
    }
    let mut rows = vec![vec![label.to_uppercase(), "GUIDES".into()]];
    for item in items {
        rows.push(vec![
            str_at(item, "name").unwrap_or("-").to_string(),
            count_at(item, "guideCount").unwrap_or(0).to_string(),
        ]);
    }
    Some(table(&rows))
}

/// One guide as a two-line entry: what it is, then how to reach it.
fn guide_line(guide: &Value) -> Vec<String> {
    let mut lines = vec![format!(
        "{INDENT}{}",
        str_at(guide, "title").unwrap_or("(untitled)")
    )];
    let mut facts: Vec<String> = Vec::new();
    if let Some(id) = str_at(guide, "id") {
        facts.push(id.to_string());
    }
    if let Some(category) = str_at(guide, "category") {
        facts.push(category.to_string());
    }
    if let Some(count) = count_at(guide, "recordingCount") {
        facts.push(plural(count, "recording"));
    }
    let applications = strings(guide, "applications");
    if !applications.is_empty() {
        facts.push(applications.join(", "));
    }
    if let Some(similarity) = guide
        .get("relevance")
        .and_then(|relevance| relevance.get("semanticSimilarity"))
        .and_then(Value::as_f64)
    {
        facts.push(format!("match {:.0}%", similarity * 100.0));
    }
    lines.push(format!("{INDENT}{INDENT}{}", facts.join("  ·  ")));
    let tags = strings(guide, "tags");
    if !tags.is_empty() {
        lines.push(format!("{INDENT}{INDENT}{}", tags.join(", ")));
    }
    lines
}

fn guide_list(value: &Value) -> String {
    let items = list_at(value, "guides");
    let total = count_at(value, "total").unwrap_or(count_of(items.len()));
    let mut out = Vec::new();

    if let Some(guidance) = str_at(value, "guidance") {
        out.push(wrap(guidance, "", 78));
        out.push(String::new());
    }
    if items.is_empty() {
        out.push("No guides matched.".into());
        return out.join("\n");
    }

    let page = count_at(value, "page").unwrap_or(1);
    let page_size = count_at(value, "pageSize")
        .unwrap_or(count_of(items.len()))
        .max(1);
    let pages = ((total + page_size - 1) / page_size).max(1);
    let mut header = plural(total, "guide");
    if pages > 1 {
        let _ = write!(header, " (page {page} of {pages})");
    }
    if value.get("lowConfidence") == Some(&Value::Bool(true)) {
        header.push_str("  [weak match]");
    }
    out.push(header);
    out.push(String::new());
    for guide in items {
        out.extend(guide_line(guide));
        out.push(String::new());
    }
    if pages > page {
        out.push(format!("Next page: --page {}", page + 1));
    }
    out.join("\n").trim_end().to_string()
}

/// A whole guide, rendered against the fixed guide structure the response
/// carries, so the sections come out in its order and under its labels.
fn guide_detail(value: &Value) -> String {
    let mut out = vec![str_at(value, "title").unwrap_or("(untitled)").to_string()];

    let mut facts: Vec<String> = Vec::new();
    if let Some(id) = str_at(value, "id") {
        facts.push(id.to_string());
    }
    if let Some(library) = value.get("library").and_then(|l| str_at(l, "name")) {
        facts.push(format!("in {library}"));
    }
    if let Some(category) = str_at(value, "category") {
        facts.push(category.to_string());
    }
    if let Some(count) = count_at(value, "recordingCount") {
        facts.push(plural(count, "recording"));
    }
    out.push(format!("{INDENT}{}", facts.join("  ·  ")));
    let tags = strings(value, "tags");
    if !tags.is_empty() {
        out.push(format!("{INDENT}{}", tags.join(", ")));
    }

    let content = value.get("content").cloned().unwrap_or(Value::Null);
    let sections = value
        .get("schema")
        .map_or(&[][..], |s| list_at(s, "sections"));
    for section in sections {
        let Some(key) = str_at(section, "key") else {
            continue;
        };
        let Some(body) = content.get(key) else {
            continue;
        };
        out.push(String::new());
        out.push(str_at(section, "title").unwrap_or(key).to_uppercase());
        out.push(render_section(section, body));
    }

    // Scenarios are the variant cases of this guide. They are derived when the
    // guide is read rather than stored, so they arrive beside `content` instead
    // of as one of its sections and would otherwise never be printed.
    let scenarios = list_at(value, "scenarios");
    if !scenarios.is_empty() {
        out.push(String::new());
        out.push("SCENARIOS".into());
        for scenario in scenarios {
            out.push(format!(
                "{INDENT}{} ({})",
                str_at(scenario, "title").unwrap_or("(untitled)"),
                str_at(scenario, "guideId").unwrap_or("-"),
            ));
            if let Some(trigger) = str_at(scenario, "trigger") {
                out.push(wrap(trigger, &format!("{INDENT}{INDENT}"), 74));
            }
        }
    }

    let edges = value.get("edges");
    let outgoing = edges.map_or(&[][..], |e| list_at(e, "outgoing"));
    let incoming = edges.map_or(&[][..], |e| list_at(e, "incoming"));
    if !outgoing.is_empty() || !incoming.is_empty() {
        out.push(String::new());
        out.push("RELATED".into());
        for edge in outgoing {
            out.push(format!(
                "{INDENT}{} → {} ({})",
                str_at(edge, "label").unwrap_or("related"),
                str_at(edge, "targetGuideTitle").unwrap_or("-"),
                str_at(edge, "targetGuideId").unwrap_or("-"),
            ));
        }
        for edge in incoming {
            out.push(format!(
                "{INDENT}{} ← {} ({})",
                str_at(edge, "label").unwrap_or("related"),
                str_at(edge, "sourceGuideTitle").unwrap_or("-"),
                str_at(edge, "sourceGuideId").unwrap_or("-"),
            ));
        }
    }
    out.join("\n")
}

/// One content section. A list of steps becomes a numbered walkthrough; a group of
/// fields becomes labelled prose; a prose section is one free-text value.
fn render_section(section: &Value, body: &Value) -> String {
    // A prose section (Trigger, Rules, Boundaries) is a bare string. Without this
    // it fell through to the unknown-shape branch and printed the JSON literal,
    // quotes and escaped newlines included.
    if let Some(text) = body.as_str() {
        return wrap(text, INDENT, 76);
    }

    if let Some(steps) = body.as_array() {
        return steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                let mut line = format!(
                    "{INDENT}{}. {}",
                    i + 1,
                    str_at(step, "instruction").unwrap_or("-")
                );
                if let Some(application) = str_at(step, "application") {
                    let _ = write!(line, " [{application}]");
                }
                for (key, prefix) in [("detail", "note:"), ("expected_result", "result:")] {
                    if let Some(text) = str_at(step, key) {
                        line.push('\n');
                        line.push_str(&wrap(
                            &format!("{prefix} {text}"),
                            &format!("{INDENT}{INDENT}  "),
                            74,
                        ));
                    }
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    let fields = list_at(section, "fields");
    let mut out = Vec::new();
    for field in fields {
        let Some(key) = str_at(field, "key") else {
            continue;
        };
        let Some(text) = body.get(key).and_then(Value::as_str) else {
            continue;
        };
        if fields.len() > 1 {
            out.push(format!("{INDENT}{}", str_at(field, "label").unwrap_or(key)));
            out.push(wrap(text, &format!("{INDENT}{INDENT}"), 74));
        } else {
            out.push(wrap(text, INDENT, 76));
        }
    }
    if out.is_empty() {
        // A section whose shape the schema did not describe still prints.
        out.push(wrap(&body.to_string(), INDENT, 76));
    }
    out.join("\n")
}

fn traverse(value: &Value) -> String {
    let nodes = list_at(value, "nodes");
    let root = str_at(value, "root").unwrap_or("-");
    let mut out = vec![format!(
        "{} reachable from {root} (depth {})",
        plural(count_of(nodes.len()), "guide"),
        count_at(value, "maxDepth").unwrap_or(1)
    )];
    out.push(String::new());
    let mut rows = vec![vec!["DEPTH".into(), "TITLE".into(), "ID".into()]];
    for node in nodes {
        rows.push(vec![
            count_at(node, "depth").unwrap_or(0).to_string(),
            str_at(node, "title").unwrap_or("(untitled)").to_string(),
            str_at(node, "id").unwrap_or("-").to_string(),
        ]);
    }
    out.push(table(&rows));

    let edges = list_at(value, "edges");
    if !edges.is_empty() {
        out.push(String::new());
        out.push(format!("{}:", plural(count_of(edges.len()), "link")));
        for edge in edges {
            let confidence = if edge.get("lowConfidence") == Some(&Value::Bool(true)) {
                "  [weak]"
            } else {
                ""
            };
            out.push(format!(
                "{INDENT}{} → {}  ({}, seen in {}){confidence}",
                str_at(edge, "sourceGuideId").unwrap_or("-"),
                str_at(edge, "targetGuideId").unwrap_or("-"),
                str_at(edge, "label").unwrap_or("related"),
                plural(count_at(edge, "support").unwrap_or(0), "recording"),
            ));
        }
    }
    out.join("\n")
}

fn graph(value: &Value) -> String {
    let guides = list_at(value, "guides");
    let edges = list_at(value, "edges");
    let mut out = vec![format!(
        "{} and {}",
        plural(count_of(guides.len()), "guide"),
        plural(count_of(edges.len()), "link")
    )];
    if value.get("truncated") == Some(&Value::Bool(true)) {
        out.push("This graph was too large to return whole, so it was truncated.".into());
    }
    out.push(String::new());
    let mut rows = vec![vec!["TITLE".into(), "RECORDINGS".into(), "ID".into()]];
    for guide in guides {
        rows.push(vec![
            str_at(guide, "title").unwrap_or("(untitled)").to_string(),
            count_at(guide, "recordingCount").unwrap_or(0).to_string(),
            str_at(guide, "id").unwrap_or("-").to_string(),
        ]);
    }
    out.push(table(&rows));
    let weak = edges
        .iter()
        .filter(|e| e.get("lowConfidence") == Some(&Value::Bool(true)))
        .count();
    if weak > 0 {
        out.push(String::new());
        out.push(format!(
            "{weak} of {} links are weakly supported and are hints rather than facts.",
            edges.len()
        ));
    }
    out.join("\n")
}

fn source_recordings(value: &Value) -> String {
    let items = list_at(value, "sourceRecordings");
    let mut out = Vec::new();
    if let Some(guidance) = str_at(value, "guidance") {
        out.push(wrap(guidance, "", 78));
        out.push(String::new());
    }
    if items.is_empty() {
        out.push("No recordings to show.".into());
        return out.join("\n").trim_end().to_string();
    }
    out.push(plural(count_at(value, "total").unwrap_or(0), "recording"));
    for recording in items {
        out.push(String::new());
        out.push(format!(
            "{INDENT}{}",
            str_at(recording, "title").unwrap_or("(untitled recording)")
        ));
        let mut facts = Vec::new();
        if let Some(id) = str_at(recording, "id") {
            facts.push(id.to_string());
        }
        if let Some(when) = str_at(recording, "clientCreatedAt") {
            facts.push(when.to_string());
        }
        out.push(format!("{INDENT}{INDENT}{}", facts.join("  ·  ")));
        for step in list_at(recording, "steps").iter().filter_map(Value::as_str) {
            out.push(wrap(step, &format!("{INDENT}{INDENT}"), 74));
        }
        for question in list_at(recording, "questions") {
            let asked = str_at(question, "question").unwrap_or("-");
            let answered = str_at(question, "response").unwrap_or("(unanswered)");
            out.push(wrap(
                &format!("Q: {asked}"),
                &format!("{INDENT}{INDENT}"),
                74,
            ));
            out.push(wrap(
                &format!("A: {answered}"),
                &format!("{INDENT}{INDENT}"),
                74,
            ));
        }
    }
    out.join("\n")
}

fn whoami(value: &Value) -> String {
    let source = match str_at(value, "apiKeySource") {
        Some("environment") => "from FUSEDFRAMES_API_KEY",
        Some("config") => "saved on this computer",
        _ => "not set",
    };
    let libraries = list_at(value, "libraries");
    let mut out = vec![
        "Your API key works.".to_string(),
        String::new(),
        format!(
            "{INDENT}Key:  {} ({source})",
            str_at(value, "apiKey").unwrap_or("not set")
        ),
        format!("{INDENT}API:  {}", str_at(value, "apiUrl").unwrap_or("-")),
        format!(
            "{INDENT}Sees: {} across {}",
            plural(count_at(value, "guideCount").unwrap_or(0), "guide"),
            plural_of(
                count_of(libraries.len()),
                "guide library",
                "guide libraries"
            ),
        ),
    ];
    if libraries.is_empty() {
        out.push(String::new());
        out.push(
            "This key is not scoped to any guide library, so every search will come \
             back empty. Add one where the key was created."
                .into(),
        );
        return out.join("\n");
    }
    out.push(String::new());
    let mut rows = vec![vec!["LIBRARY".into(), "GUIDES".into(), "ID".into()]];
    for library in libraries {
        rows.push(vec![
            str_at(library, "name").unwrap_or("(untitled)").to_string(),
            count_at(library, "guideCount").unwrap_or(0).to_string(),
            str_at(library, "id").unwrap_or("-").to_string(),
        ]);
    }
    out.push(table(&rows));
    out.join("\n")
}

fn config(value: &Value) -> String {
    let key = match str_at(value, "apiKey") {
        Some(key) => key.to_string(),
        None => "not set".to_string(),
    };
    let source = match str_at(value, "apiKeySource") {
        Some("environment") => " (from FUSEDFRAMES_API_KEY)",
        Some("config") => " (saved on this computer)",
        _ => "",
    };
    let mut out = vec![format!("API key:  {key}{source}")];
    out.push(format!(
        "API URL:  {}",
        str_at(value, "apiUrl").unwrap_or("-")
    ));
    if let Some(path) = str_at(value, "configPath") {
        out.push(format!("Config:   {path}"));
    }
    if str_at(value, "apiKeySource") == Some("none") {
        out.push(String::new());
        out.push("Set a key with:  echo \"ff_...\" | fusedframes config set-key".into());
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn counts_never_read_as_one_guides() {
        assert_eq!(plural(1, "guide"), "1 guide");
        assert_eq!(plural(0, "guide"), "0 guides");
        assert_eq!(plural(2, "guide"), "2 guides");
        // An irregular noun spells both forms out rather than gaining an "s".
        assert_eq!(
            plural_of(2, "guide library", "guide libraries"),
            "2 guide libraries"
        );
    }

    #[test]
    fn a_library_list_becomes_an_aligned_table() {
        let rendered = render(&json!({ "libraries": [
            { "name": "Support", "guideCount": 4, "categories": ["Billing"], "id": "lib_1" },
            { "name": "Ops", "guideCount": 12, "categories": [], "id": "lib_2" },
        ]}))
        .expect("libraries render");
        assert!(rendered.contains("2 guide libraries"));
        assert!(rendered.contains("NAME"));
        assert!(rendered.contains("Support"));
        // The id column lines up because the name column is padded.
        let lines: Vec<&str> = rendered.lines().collect();
        let header = lines.iter().find(|l| l.contains("NAME")).expect("header");
        let support = lines.iter().find(|l| l.contains("Support")).expect("row");
        // rfind: the ID header is the last column, and "GUIDES" also contains "ID".
        assert_eq!(
            header.rfind("ID"),
            support.find("lib_1"),
            "columns must align"
        );
    }

    #[test]
    fn an_empty_result_says_so_in_words() {
        let rendered = render(&json!({ "libraries": [] })).expect("renders");
        assert!(rendered.contains("No guide libraries"));
        let rendered = render(&json!({ "guides": [], "total": 0 })).expect("renders");
        assert!(rendered.contains("No guides matched"));
    }

    #[test]
    fn search_guidance_and_weak_matches_are_surfaced() {
        let rendered = render(&json!({
            "guides": [{ "title": "A", "id": "guide_1" }],
            "total": 1, "page": 1, "pageSize": 20,
            "lowConfidence": true,
            "guidance": "Nothing matched, try other words.",
        }))
        .expect("renders");
        assert!(rendered.contains("Nothing matched"));
        assert!(rendered.contains("[weak match]"), "got: {rendered}");
    }

    #[test]
    fn paging_tells_you_how_to_get_the_next_page() {
        let rendered = render(&json!({
            "guides": [{ "title": "A", "id": "guide_1" }],
            "total": 40, "page": 1, "pageSize": 20,
        }))
        .expect("renders");
        assert!(rendered.contains("page 1 of 2"), "got: {rendered}");
        assert!(rendered.contains("--page 2"), "got: {rendered}");
    }

    #[test]
    fn a_guide_renders_its_steps_against_its_schema() {
        let rendered = render(&json!({
            "title": "Refund a customer",
            "id": "guide_1",
            "recordingCount": 2,
            "schema": { "sections": [
                { "key": "steps", "title": "Steps", "type": "timeline", "fields": [] }
            ]},
            "content": { "steps": [
                { "instruction": "Open Stripe", "application": "Chrome",
                  "expected_result": "Payments are listed" }
            ]},
        }))
        .expect("renders");
        assert!(rendered.contains("STEPS"));
        assert!(
            rendered.contains("1. Open Stripe [Chrome]"),
            "got: {rendered}"
        );
        assert!(rendered.contains("result: Payments are listed"));
    }

    #[test]
    fn a_prose_section_reads_as_prose_not_as_json() {
        // Trigger, Rules and Boundaries are single free-text values, so a body
        // that is a bare string must lose its quotes and its escaped newlines.
        let rendered = render(&json!({
            "title": "Refund a customer",
            "id": "guide_1",
            "schema": { "sections": [
                { "key": "trigger", "title": "Trigger", "type": "prose", "fields": [] }
            ]},
            "content": { "trigger": "A customer asks for a refund.\nNot for a chargeback." },
        }))
        .expect("renders");
        assert!(rendered.contains("TRIGGER"), "got: {rendered}");
        assert!(
            rendered.contains("A customer asks for a refund."),
            "got: {rendered}"
        );
        assert!(!rendered.contains('"'), "got: {rendered}");
        assert!(!rendered.contains("\\n"), "got: {rendered}");
    }

    #[test]
    fn the_scenarios_index_is_printed() {
        // Scenarios are derived when the guide is read, so they sit beside
        // `content` rather than inside it and the section walk never sees them.
        let rendered = render(&json!({
            "title": "Refund a customer",
            "id": "guide_1",
            "content": {},
            "schema": { "sections": [] },
            "scenarios": [
                { "guideId": "guide_2", "title": "Refund a subscription",
                  "trigger": "The order is a recurring plan." }
            ],
        }))
        .expect("renders");
        assert!(rendered.contains("SCENARIOS"), "got: {rendered}");
        assert!(
            rendered.contains("Refund a subscription (guide_2)"),
            "got: {rendered}"
        );
        assert!(
            rendered.contains("The order is a recurring plan."),
            "got: {rendered}"
        );
    }

    #[test]
    fn a_guide_is_not_mistaken_for_a_tag_vocabulary() {
        // A guide carries `tags` too, but as bare strings. Rendering it as the
        // tag list produced a table of dashes and zeroes.
        let rendered = render(&json!({
            "title": "Refund a customer",
            "id": "guide_1",
            "tags": ["stripe", "refunds"],
            "content": { "steps": [] },
            "schema": { "sections": [] },
        }))
        .expect("renders");
        assert!(rendered.starts_with("Refund a customer"), "got: {rendered}");
        assert!(!rendered.contains("GUIDES"), "got: {rendered}");
    }

    #[test]
    fn a_tag_vocabulary_still_renders_as_counts() {
        let rendered = render(&json!({
            "tags": [{ "name": "stripe", "guideCount": 3 }]
        }))
        .expect("renders");
        assert!(rendered.contains("TAGS"));
        assert!(rendered.contains("stripe"));
    }

    #[test]
    fn an_unknown_shape_falls_back_rather_than_inventing() {
        assert!(render(&json!({ "somethingNew": [1, 2, 3] })).is_none());
        assert!(render(&json!([1, 2, 3])).is_none());
    }

    #[test]
    fn wrapping_never_breaks_an_id() {
        let id = "guide_01kzrrsky9edmreg60dxwgz2xz";
        let wrapped = wrap(&format!("see {id} for detail"), "", 10);
        assert!(wrapped.contains(id), "an id must survive wrapping intact");
    }
}
