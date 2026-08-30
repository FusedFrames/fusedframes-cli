# fusedframes-cli

Your agent does the task the way you do it. [FusedFrames](https://www.fusedframes.com) turns your recorded work into guides, and this CLI lets any agent that can run shell commands read those guides, follow the links between them and pull your own source recordings for extra detail.

The hosted MCP server at `mcp.fusedframes.com` is the primary way to connect an agent. This CLI is the shell-based alternative for agents that work over shell commands.

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

npm delivers a prebuilt native binary for your platform (macOS arm64/x64, Linux x64/arm64, Windows x64/arm64). Node is only the delivery mechanism; the CLI itself is a single native executable.

### Standalone binaries

No Node? Download the binary for your platform from the [latest release](https://github.com/FusedFrames/fusedframes-cli/releases/latest), then place it on your `PATH`:

```bash
tar -xzf fusedframes-v*-darwin-arm64.tar.gz
sudo mv fusedframes /usr/local/bin/
```

Assets are named for the platform and architecture Node reports, so the file you
want matches the npm package you would otherwise have installed:

| Machine | Asset |
| --- | --- |
| macOS, Apple Silicon | `fusedframes-v*-darwin-arm64.tar.gz` |
| macOS, Intel | `fusedframes-v*-darwin-x64.tar.gz` |
| Windows, arm64 | `fusedframes-v*-win32-arm64.zip` |
| Windows, x86_64 | `fusedframes-v*-win32-x64.zip` |
| Linux, arm64 | `fusedframes-v*-linux-arm64.tar.gz` |
| Linux, x86_64 | `fusedframes-v*-linux-x64.tar.gz` |

The Linux builds are fully static musl binaries, so they run on any distribution
with no glibc dependency, Alpine included.

Every release ships a `SHA256SUMS` file and [build provenance attestations](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations); npm packages are published with npm provenance. Verify a download with:

```bash
gh attestation verify fusedframes-v*-darwin-arm64.tar.gz --repo FusedFrames/fusedframes-cli
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

Create an API key in the FusedFrames desktop app (API keys, in the sidebar), then configure the CLI:

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

A guide is what FusedFrames writes from your recorded work, so `guides` is the command group that reads them.

### Browse libraries

List all the guide libraries your API key can see:

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

### Find guides

List guides in a library with optional filters:

```bash
fusedframes guides list <library-id>
fusedframes guides list <library-id> --category "Deployment"
fusedframes guides list <library-id> --tag "rollback" --app "Terminal"
fusedframes guides list <library-id> --search "failed health check"
```

Get full detail for a guide, including its relationships:

```bash
fusedframes guides get <guide-id>
```

This returns the guide's content, the schema that shapes it, its category, tags and all incoming and outgoing edges to other guides. Every guide has the same fixed structure:

| Section | What it holds |
|---|---|
| Trigger | When the guide applies and when it does not |
| Steps | What to do, in order |
| Rules | The judgement notes that shape the steps |
| Scenarios | Links to guides for variant cases |
| Boundaries | When to stop and ask |

Get the source recordings a guide is based on:

```bash
fusedframes guides source-recordings <guide-id>
```

Each source recording includes the original question, response and the formatted steps showing exactly what happened. Recordings are private: on every plan your API key reads the steps and answers from your own recordings, and other people's recordings never appear.

### Follow the graph

Get the full guide graph for a library in a single call:

```bash
fusedframes graph <library-id>
```

Returns all guides and all edges. Useful for building a complete picture of a library.

Follow relationships from a specific guide:

```bash
fusedframes traverse <guide-id>
fusedframes traverse <guide-id> --depth 2
fusedframes traverse <guide-id> --direction outgoing --label "often next"
fusedframes traverse <guide-id> --depth 3 --direction both
```

Depth controls how many levels of connected guides to follow (1-3). Direction can be `outgoing`, `incoming` or `both`. The server validates depth and direction, not the CLI; the values pass through unchanged and an invalid one comes back as the API's own `bad_request` error.

Edge labels describe the relationship between guides:

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

### Check your key

Confirm the key works and see what it can read:

```bash
fusedframes whoami
```

It names the key in use and where it came from, then lists every guide library the key can reach with a guide count for each. An expired, revoked or wrongly scoped key shows up straight away rather than later as an empty search.

### Shell completions

Complete commands and options with Tab:

```bash
fusedframes completions zsh > ~/.zsh/completions/_fusedframes   # zsh
fusedframes completions bash > /etc/bash_completion.d/fusedframes
fusedframes completions fish > ~/.config/fish/completions/fusedframes.fish
```

`elvish` and `powershell` are also supported.

### Log out

Remove the saved API key from this computer:

```bash
fusedframes logout
```

`clear-key` is an alias for the same command. It removes the key from the config file (leaving any other settings in place) and succeeds even when there is no saved key. If the `FUSEDFRAMES_API_KEY` environment variable is still set the output includes a warning, because the variable wins over the config file and unsetting it in your shell is the only way to fully sign out.

## Pagination

`guides list`, `guides source-recordings` and `search` support `--page` and `--page-size`:

```bash
fusedframes guides list <library-id> --page 2 --page-size 50
fusedframes guides source-recordings <guide-id> --page 1 --page-size 10
```

Defaults: page 1, 20 results per page.

## Output

The format follows where the output is going.

**Piped, redirected or captured** (a script, an agent, `| jq`, `> file`): one line of compact JSON, the API response passed through verbatim. That is the contract to build on and it does not change.

**Straight to a terminal**: the same response rendered to read, with aligned tables and a guide's steps laid out under its own section headings. It is the same data, not a subset.

`--json` forces the machine format anywhere, which is what you want when checking by hand what a script will receive.

```bash
fusedframes libraries list          # a table, when you are at a terminal
fusedframes libraries list --json   # one line of JSON, always
fusedframes libraries list | jq     # JSON, because it is piped
```

Errors are the same JSON on stdout in machine mode, and a plain message on stderr for a person:

```json
{ "error": { "code": "unauthorised", "message": "Invalid or missing API key" } }
```

Exit codes: `0` for success, `1` for errors (including invalid arguments).

One exception: a bare `fusedframes`, or a bare subcommand group such as `fusedframes guides`, prints the help text and exits `1` without any JSON. `--help` and `--version` print their usual output and exit `0`.

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

This CLI is built to be called by AI agents (Claude Code, Cursor, Windsurf, Codex) via shell commands. Each command returns structured JSON that the agent can parse and act on. If your agent speaks MCP, the hosted server at `mcp.fusedframes.com` is the primary connection; use the CLI when shell commands are what your agent has.

A typical agent workflow:

1. `fusedframes search "deployment failure"` to find relevant guides
2. `fusedframes guides get <id>` to read the full guide and its edges
3. `fusedframes traverse <id> --depth 2` to explore related guides
4. `fusedframes guides source-recordings <id>` to pull your own recordings behind the guide

The agent follows guide edges to move between related guides, so it can do the task your way, exceptions included.

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
