# fusedframes-cli

Find and read the documents [FusedFrames](https://www.fusedframes.com) makes from your team's recorded work, straight from the command line. Designed for AI agents to explore document libraries, follow relationships between documents and retrieve source recordings.

A single native binary written in Rust, with no language runtime required. TLS trust comes from the operating system's certificate store, so enterprise CAs work out of the box.

## Install

No installation required. With Node 20 or later, run directly with `npx`:

```bash
npx @fusedframes/cli --help
```

Or install globally:

```bash
npm install -g @fusedframes/cli
```

npm delivers a prebuilt native binary for your platform (macOS arm64/x64, Linux x64/arm64, Windows x64). Node is only the delivery mechanism; the CLI itself is a single native executable.

### Standalone binaries

No Node? Download the binary for your platform from the [latest release](https://github.com/FusedFrames/fusedframes-cli/releases/latest), then place it on your `PATH`:

```bash
tar -xzf fusedframes-v*-aarch64-apple-darwin.tar.gz
sudo mv fusedframes /usr/local/bin/
```

Prebuilt targets: macOS (Apple Silicon and Intel), Linux (x86_64 and arm64, fully static musl builds) and Windows (x86_64).

Every release ships a `SHA256SUMS` file and [build provenance attestations](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations); npm packages are published with npm provenance. Verify a download with:

```bash
gh attestation verify fusedframes-v*-aarch64-apple-darwin.tar.gz --repo FusedFrames/fusedframes-cli
```

For an npm install, verify the registry signatures and provenance attestations with:

```bash
npm audit signatures
```

### From source

Build with a Rust toolchain (1.88 or later):

```bash
cargo install --git https://github.com/FusedFrames/fusedframes-cli
```

## Setup

Create an API key in your workspace settings → API keys at `www.fusedframes.com/workspace/<your-workspace>/api-keys`, then configure the CLI:

```bash
echo "ff_your_api_key" | fusedframes config set-key
```

The key is stored at `~/.config/fusedframes/config.json` with restricted file permissions. Keys are read from stdin only: never pass one as a command-line argument, and the CLI refuses them there, because argv is saved in shell history and visible in process listings.

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

### Find documents

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

### Follow the graph

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

Depth controls how many levels of connected documents to follow (1-3). Direction can be `outgoing`, `incoming` or `both`. The server validates depth and direction, not the CLI; the values pass through unchanged and an invalid one comes back as the API's own `bad_request` error.

Edge labels describe the relationship between documents:

| Label | Meaning |
|---|---|
| `often next` | What typically happens after |
| `often previous` | What typically happens before |
| `alternative to` | An alternative approach |

### Search

Search all the libraries you can see:

```bash
fusedframes search "failed deployment"
fusedframes search "onboarding" --category "HR"
fusedframes search "export invoices" --app "Xero"
fusedframes search "review" --library <library-id>
```

### Log out

Remove the saved API key from this computer:

```bash
fusedframes logout
```

`clear-key` is an alias for the same command. It removes the key from the config file (leaving any other settings in place) and succeeds even when there is no saved key. If the `FUSEDFRAMES_API_KEY` environment variable is still set the output includes a warning, because the variable wins over the config file and unsetting it in your shell is the only way to fully sign out.

## Pagination

`documents list`, `documents source-recordings` and `search` support `--page` and `--page-size`:

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

One exception: a bare `fusedframes`, or a bare subcommand group such as `fusedframes documents`, prints the help text and exits `1` without any JSON. `--help` and `--version` print their usual output and exit `0`.

### Error codes

Codes produced by the CLI itself:

| Code | Meaning |
|---|---|
| `validation_error` | Invalid command-line arguments, or an API key passed as an argument |
| `config_error` | Missing API key, invalid API URL or a non-HTTPS API URL |
| `network_error` | The API could not be reached (DNS, connection, TLS, timeout) |
| `server_error` | The API answered without the standard JSON error envelope |
| `error` | Unexpected local failure (e.g. the config file could not be written) |
| `unknown` | The API sent an error envelope with a message but no code |

API error codes pass through verbatim: `unauthorised`, `api_key_expired`, `api_key_device_removed`, `bad_request`, `forbidden`, `not_found`, `rate_limited`, `insufficient_ai_credit`, `subscription_suspended`, `internal_error`.

Rate-limited responses include a `retryAfter` field with the number of seconds to wait, taken from the API's `Retry-After` header:

```json
{ "error": { "code": "rate_limited", "message": "Rate limit exceeded", "retryAfter": 12 } }
```

## Environment variables

| Variable | Purpose |
|---|---|
| `FUSEDFRAMES_API_KEY` | API key. Overrides the saved config. |
| `FUSEDFRAMES_API_URL` | API base URL. Defaults to `https://api.fusedframes.com`. |

## Configuration

The CLI stores its configuration at `~/.config/fusedframes/config.json`. On macOS and Linux the directory is created with `700` permissions and the file with `600` permissions, re-tightened on every write. Windows has no Unix permission bits, so no tightening happens there.

Environment variables take precedence over the config file.

## Security

- The API key is only ever sent over HTTPS. Plain HTTP is refused unless the host is genuinely loopback (`localhost`, `127.0.0.0/8`, `::1`) for local development; redirects follow the same rule, so a downgrade redirect can never carry the key in clear text.
- Proxy environment variables (`http_proxy`, `https_proxy`, `all_proxy`) are deliberately ignored, so the key never travels through a proxy the URL checks haven't vetted.
- Keys are accepted via stdin or environment variable only, never as arguments.
- `fusedframes config show` prints a masked key, never the full value.
- `fusedframes logout` (alias `clear-key`) removes the stored key from the machine.

## AI agent usage

This CLI is designed to be called by AI agents (Claude Code, Cursor, Windsurf, Codex) via shell commands. Each command returns structured JSON that the agent can parse and act on.

A typical agent workflow:

1. `fusedframes search "deployment failure"` to find relevant documents
2. `fusedframes documents get <id>` to read the full document and its edges
3. `fusedframes traverse <id> --depth 2` to explore related documents
4. `fusedframes documents source-recordings <id>` to see the raw recordings the document is based on

The agent uses document edges to navigate between related behaviours and build context about how your team works.

## Requirements

- A FusedFrames account (API access is included on every plan)
- An API key created in your workspace settings → API keys

## Development

```bash
cargo build            # debug build
cargo test             # unit + end-to-end tests (spawns the real binary)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Enable the repo git hooks (branch guard + secret scan) once per clone:

```bash
git config core.hooksPath .githooks
```

## Links

- [FusedFrames](https://www.fusedframes.com)
- [Documentation](https://www.fusedframes.com/docs)

## Licence

Released under the [MIT licence](LICENSE).
