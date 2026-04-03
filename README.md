# @fusedframes/cli

Query behavioural patterns captured by [FusedFrames](https://fusedframes.com) from the command line. Designed for AI agents to traverse pattern libraries, follow relationships between patterns and retrieve evidence.

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

Create an API key in your FusedFrames team settings under **Integrations > API keys**, then configure the CLI:

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

List all pattern libraries your API key has access to:

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

### Query patterns

List patterns in a library with optional filters:

```bash
fusedframes patterns list <library-id>
fusedframes patterns list <library-id> --category "Deployment"
fusedframes patterns list <library-id> --tag "rollback" --app "Terminal"
fusedframes patterns list <library-id> --search "failed health check"
```

Get full detail for a pattern, including its relationships:

```bash
fusedframes patterns get <pattern-id>
```

This returns the pattern's behaviour, reasoning, trigger, outcome, category, tags, standard operating procedure steps, and all incoming and outgoing edges to other patterns.

Get the evidence actions that support a pattern:

```bash
fusedframes patterns evidence <pattern-id>
```

Each evidence action includes the original question, response, and a formatted event timeline showing exactly what happened.

### Traverse the graph

Get the full pattern graph for a library in a single call:

```bash
fusedframes graph <library-id>
```

Returns all patterns and all edges. Useful for building a complete picture of a library.

Follow relationships from a specific pattern:

```bash
fusedframes traverse <pattern-id>
fusedframes traverse <pattern-id> --depth 2
fusedframes traverse <pattern-id> --direction outgoing --label "often next"
fusedframes traverse <pattern-id> --depth 3 --direction both
```

Depth controls how many levels of connected patterns to follow (1-3). Direction can be `outgoing`, `incoming`, or `both`.

Edge labels describe the relationship between patterns:

| Label | Meaning |
|---|---|
| `often next` | What typically happens after |
| `often previous` | What typically happens before |
| `variation to` | An alternative approach |
| `contradicts` | Conflicts with this pattern |
| `often occurs with` | Usually happening alongside |
| `exception to` | Edge case where a pattern doesn't apply |

### Search

Search across all accessible libraries:

```bash
fusedframes search "failed deployment"
fusedframes search "onboarding" --category "HR"
fusedframes search "review" --library <library-id>
```

## Pagination

Commands that return lists support `--page` and `--page-size`:

```bash
fusedframes patterns list <library-id> --page 2 --page-size 50
fusedframes patterns evidence <pattern-id> --page 1 --page-size 10
```

Defaults: page 1, 20 results per page.

## Output

All commands output JSON to stdout. Errors are also JSON:

```json
{ "error": { "code": "unauthorised", "message": "Invalid or missing API key" } }
```

Exit codes: `0` for success, `1` for errors, `2` for invalid arguments.

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

1. `fusedframes search "deployment failure"` to find relevant patterns
2. `fusedframes patterns get <id>` to read the full pattern and its edges
3. `fusedframes traverse <id> --depth 2` to explore related patterns
4. `fusedframes patterns evidence <id>` to see the raw actions that support the pattern

The agent uses pattern edges to navigate between related behaviours and build context about how your team works.

## Requirements

- Node.js 18 or later
- A FusedFrames account with a Pro or Enterprise plan
- An API key created in your team's integration settings

## Links

- [FusedFrames](https://fusedframes.com)
- [API reference](https://api.fusedframes.com)
