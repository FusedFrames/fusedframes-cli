# @fusedframes/cli

Query documents written by [FusedFrames](https://fusedframes.com) from your team's recorded work, straight from the command line. Designed for AI agents to traverse document libraries, follow relationships between documents and retrieve source recordings.

## Install

No installation required. Run directly with `npx`:

```bash
npx @fusedframes/cli --help
```

Or install globally:

```bash
npm install -g @fusedframes/cli
```

## Setup

Create an API key in your FusedFrames team settings at `fusedframes.com/team/<your-team>/api-keys`, then configure the CLI:

```bash
echo "ff_your_api_key" | fusedframes config set-key
```

The key is stored at `~/.config/fusedframes/config.json` with restricted file permissions.

You can also set the key via environment variable:

```bash
export FUSEDFRAMES_API_KEY=ff_your_api_key
```

Verify your configuration:

```bash
fusedframes config show
```

## Commands

### Browse libraries

List all document libraries your API key has access to:

```bash
fusedframes libraries list
```

Get detail for a specific library:

```bash
fusedframes libraries get <library-id>
```

Discover the vocabulary of a library:

```bash
fusedframes libraries categories <library-id>
fusedframes libraries tags <library-id>
fusedframes libraries applications <library-id>
```

### Query documents

List documents in a library with optional filters:

```bash
fusedframes documents list <library-id>
fusedframes documents list <library-id> --category "Deployment"
fusedframes documents list <library-id> --tag "rollback" --app "Terminal"
fusedframes documents list <library-id> --search "failed health check"
```

Get full detail for a document, including its relationships:

```bash
fusedframes documents get <document-id>
```

This returns the document's structured content (for the default library structure: behaviour, reasoning, trigger, outcome and standard operating procedure steps), the library schema that shapes it, its category, tags and all incoming and outgoing edges to other documents.

Get the source recordings a document is based on:

```bash
fusedframes documents source-recordings <document-id>
```

Each source recording includes the original question, response and the formatted steps showing exactly what happened.

### Traverse the graph

Get the full document graph for a library in a single call:

```bash
fusedframes graph <library-id>
```

Returns all documents and all edges. Useful for building a complete picture of a library.

Follow relationships from a specific document:

```bash
fusedframes traverse <document-id>
fusedframes traverse <document-id> --depth 2
fusedframes traverse <document-id> --direction outgoing --label "often next"
fusedframes traverse <document-id> --depth 3 --direction both
```

Depth controls how many levels of connected documents to follow (1-3). Direction can be `outgoing`, `incoming`, or `both`.

Edge labels describe the relationship between documents:

| Label | Meaning |
|---|---|
| `often next` | What typically happens after |
| `often previous` | What typically happens before |
| `alternative to` | An alternative approach |

### Search

Search across all accessible libraries:

```bash
fusedframes search "failed deployment"
fusedframes search "onboarding" --category "HR"
fusedframes search "export invoices" --app "Xero"
fusedframes search "review" --library <library-id>
```

## Pagination

Commands that return lists support `--page` and `--page-size`:

```bash
fusedframes documents list <library-id> --page 2 --page-size 50
fusedframes documents source-recordings <document-id> --page 1 --page-size 10
```

Defaults: page 1, 20 results per page.

## Output

All commands output JSON to stdout. Errors are also JSON:

```json
{ "error": { "code": "unauthorised", "message": "Invalid or missing API key" } }
```

Exit codes: `0` for success, `1` for errors (including invalid arguments).

## Environment variables

| Variable | Purpose |
|---|---|
| `FUSEDFRAMES_API_KEY` | API key. Overrides the saved config. |
| `FUSEDFRAMES_API_URL` | API base URL. Defaults to `https://api.fusedframes.com`. |

## Configuration

The CLI stores its configuration at `~/.config/fusedframes/config.json`. The directory is created with `700` permissions and the file with `600` permissions.

Environment variables take precedence over the config file.

## AI agent usage

This CLI is designed to be called by AI agents (Claude Code, Cursor, Windsurf, Codex) via shell commands. Each command returns structured JSON that the agent can parse and act on.

A typical agent workflow:

1. `fusedframes search "deployment failure"` to find relevant documents
2. `fusedframes documents get <id>` to read the full document and its edges
3. `fusedframes traverse <id> --depth 2` to explore related documents
4. `fusedframes documents source-recordings <id>` to see the raw recordings the document is based on

The agent uses document edges to navigate between related behaviours and build context about how your team works.

## Requirements

- Node.js 20 or later
- A FusedFrames account (API access is included on every plan)
- An API key created in your team's integration settings

## Links

- [FusedFrames](https://fusedframes.com)
- [API reference](https://api.fusedframes.com)
