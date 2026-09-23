# grr

[![CI](https://github.com/debanjanbasu/grr-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/debanjanbasu/grr-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![crates.io](https://img.shields.io/badge/crates.io-pending-orange)](https://github.com/debanjanbasu/grr-cli/releases)

**Zero-config, maximum-performance Google tools from the terminal.** Gmail is the first namespaced service: every mail command lives under `grr gmail ...`, stdout is always clean machine-readable output, and the only thing you ever configure is one OAuth client ID.

```sh
grr auth login
grr gmail message search "in:inbox" --max 5
grr gmail message get 191f8ab2 --body
grr schema
```

## Install

**Prebuilt binaries** — Windows x64, Linux x64, macOS ARM:
[GitHub Releases](https://github.com/debanjanbasu/grr-cli/releases)

**From source** (requires Rust nightly — see [Development](#development)):

```sh
cargo install --git https://github.com/debanjanbasu/grr-cli --locked
```

or from a local clone:

```sh
git clone https://github.com/debanjanbasu/grr-cli
cargo install --path grr --locked
```

**Package managers**:

```sh
winget install debanjanbasu.grr       # Windows (submitted)
brew install debanjanbasu/tap/grr     # macOS (Linux) tap: debanjanbasu/homebrew-grr
cargo install grr-cli                 # crates.io (pending; binary installs as `grr`)
```

See [Packaging & status](#packaging--status) for details.

## 60-second quickstart

1. **One-time Google Cloud setup** (~5 min): create a Desktop OAuth client and copy its client ID.
   Follow [docs/gcp-setup.md](docs/gcp-setup.md), or run [scripts/setup-gcp.ps1](scripts/setup-gcp.ps1) (Windows) / [scripts/setup-gcp.sh](scripts/setup-gcp.sh) (macOS/Linux) — they automate the gcloud parts and print the console links for the browser steps.

2. **Configure** — `~/.grr/config.toml` (Windows: `%USERPROFILE%\.grr\config.toml`):

   ```toml
   [oauth]
   client_id = "123456789-abc.apps.googleusercontent.com"
   # Optional — Google shows it next to the client ID. Sent at the token
   # endpoint only when present; PKCE is always on.
   # client_secret = "GOCSPX-..."
   ```

3. **Authenticate and verify**:

   ```sh
   grr auth login
   grr gmail profile
   ```

   Headless machine? `grr auth login --device` prints a URL + code instead of opening a browser.

## Usage highlights

```sh
grr gmail message search "from:github.com" --max 10
grr gmail message get 191f8ab2 --body --max-length 2000
grr gmail message batch-read --ids 191f8ab2,191f8cd4 --body
grr gmail msg batch-label --search "from:linkedin.com" --remove INBOX --dry-run
grr gmail msg batch-label --search "from:linkedin.com" --remove INBOX
grr gmail send send you@example.com "Quick question" "Body text"
grr gmail send send-attach you@example.com "Invoice" "See attached" --attachments invoice.pdf
grr transport
grr schema --format pretty
```

Every data command takes `-f/--format json|jsonl|table|pretty` (default `json`; on `message get` the output flag is `-o` because `-f` there selects the Gmail message format). Logs go to stderr, so stdout is always parseable:

```sh
grr gmail message search "in:inbox" --max 1 | jq -r '.[0].id'
```

### Command reference

| Group | Commands |
| --- | --- |
| `grr auth` | `login [--device]`, `status` |
| `grr gmail message` | `search <query> [--max N]`, `get <id> [--format full\|metadata\|minimal\|raw] [--body] [--max-length N]`, `thread <id>`, `batch-read --ids a,b,c [--body]`, `attachment <msg-id> <att-id> [-o FILE]` |
| `grr gmail msg` | `label <id> --add L1 --remove L2`, `trash <id>`, `untrash <id>`, `delete <id>`, `batch-label (--ids a,b \| --search "query") --add/--remove ... [--max N] [--dry-run]`, `batch-delete (--ids a,b \| --search "query") [--max N] [--dry-run]` |
| `grr gmail label` | `list`, `get <id>`, `create <name>`, `update <id>`, `delete <id>` |
| `grr gmail draft` | `create <to> <subject> <body>`, `list`, `get <id>`, `update <id> <to> <subject> <body>`, `delete <id>`, `send <id>` |
| `grr gmail send` | `send <to> <subject> <body> [--cc] [--bcc]`, `send-attach <to> <subject> <body> --attachments f1,f2 [--thread-id] [--cc] [--bcc]` |
| `grr gmail thread` | `label <id> --add/--remove ...`, `trash <id>`, `untrash <id>`, `delete <id>` |
| `grr gmail history` | `history <start-history-id> [--label-id] [--max N]` |
| `grr gmail send-as` | `list`, `get <email>`, `create <email>`, `update <email>`, `delete <email>` |
| `grr gmail profile` | mailbox profile |
| `grr gmail watch` | `start <topic> [--label-ids]`, `stop` (push notifications) |
| `grr gmail import` | `import <rfc822-file> [--deleted]` |
| `grr calendar` | `list` (calendars), `events [cal] [--time-min] [--time-max] [--query] [--max]`, `get <cal> <id>`, `create <cal> --summary --start --end [--location] [--attendees]`, `update <cal> <id> ...`, `delete <cal> <id>`, `free-busy --calendars a,b --time-min --time-max` |
| `grr drive` | `list [--query] [--max]`, `search --query <q>`, `get <id>`, `download <id> -o FILE`, `upload <file> [--name] [--parent]`, `rename <id> <name>`, `delete <id>`, `quota` |
| `grr contacts` | `list [--query-name]`, `search <query>`, `get <people/123>`, `create --given-name --family-name [--email] [--phone]`, `update`, `delete` |
| `grr chat` | `spaces [--max]`, `space <id>`, `messages <space> [--max]`, `send <space> --text "..."` |
| `grr forms` | `get <form-id>`, `responses <form-id> [--max]` |
| `grr transport` | negotiated HTTP version + runtime features |
| `grr schema` | the full command tree as JSON |

Details worth knowing:

- **`--body`** returns `{message, body}` where `body` is the decoded text (`text/plain` preferred, `text/html` fallback), truncated to `--max-length` (default 800) with a `...[truncated]` marker — sized for LLM context windows.
- **Batch ops** accept `--ids a,b,c` or `--search "query"` (resolves IDs by running the query, capped by `--max`, default 10000), plus `--dry-run`. Calls are chunked at 1000 IDs (Gmail API batch limit) and fanned out in parallel.
- **`attachment`** prints base64 (URL-safe) to stdout when no `-o FILE` is given, so piping is always binary-safe.

## Design philosophy

- **Zero-config.** `~/.grr/config.toml` holds exactly one thing: an OAuth client ID (and optionally a secret). Scopes, redirect URI, pool sizes, timeouts, and retry policy are compile-time constants tuned for Google's frontends ([grr-core/src/client](grr-core/src/client/mod.rs)). `GRR_CONFIG_PATH` overrides the file location, `RUST_LOG` the log level (`GRR_OAUTH__*` env vars exist for headless overrides) — nothing else is configurable, on purpose.
- **Namespaced services.** Mail is `grr gmail ...`; Calendar, Drive, Contacts, Chat, and Forms live alongside it (`grr calendar ...`, `grr drive ...`, ...), and account-level concerns stay top-level (`grr auth`, `grr transport`, `grr schema`). One login covers every service; each service client builds lazily so running one never probes another's endpoints. (Keep has no public API.)
- **stdout purity.** Logs go to stderr, results go to stdout, so `| jq` always works. `-f jsonl` streams arrays one object per line.
- **Keyring-first token storage.** Tokens live in the OS keyring (Windows Credential Manager, macOS Keychain, Linux Secret Service via D-Bus), with automatic fallback to `<cache dir>/grr/token.json` on headless systems. A token found in the fallback file auto-imports into the keyring on first sight.
- **Agent-first.** `grr schema` dumps the complete command tree as JSON with zero configuration — the machine-readable contract for AI agents, discoverable without touching a config file or scraping `--help`. One fast CLI replaces per-service MCP servers: no MCP setup, just `grr schema`.

## Performance

- **HTTP/3 (QUIC) by default** — prior-knowledge h3 with one authenticated probe at startup and silent HTTP/2 fallback; `grr transport` shows what was actually negotiated.
- **Tokio multi-threaded runtime**, auto-sized to cores — no thread pool to tune.
- **In-flight request valve** — a semaphore (not a thread pool) caps concurrent HTTP requests at 64, staying under Gmail's per-user rate limits so bursts don't self-DOS into 429s. 429s are retried honoring `Retry-After` (waits capped at 30s); whole-request timeout is 30s.
- **Compression always on** — gzip, deflate, zstd, and brotli response decompression.
- **Streaming uploads** — RFC 822 media upload streams 192 KiB chunks with incremental base64, so large attachments never sit fully in memory.
- **io_uring file I/O** on Linux, behind a feature gate and detected at runtime.

## Architecture

One binary, layered crates — a service client per crate, all on one shared core:

```
grr/          the CLI: clap v4 command tree (one module per command group),
              output formats, lazy per-service dispatch
grr-core/     the shared base: OAuth (PKCE + device flow), keyring token
              store, HttpCore (transport probe, retries, rate-limit valve),
              config, runtime probes
grr-gmail/    Gmail: messages, labels, drafts, threads, history, settings,
              watch, streaming upload
grr-calendar/ grr-drive/  grr-people/  grr-chat/  grr-forms/
```

The CLI is a thin shell; each service crate adds typed endpoints over `grr-core`'s `HttpCore`. The core's default features build on **stable** Rust — only the CLI (and service crates) opt into the unstable HTTP/3 path.

## Development

Requires Rust **nightly**: [rust-toolchain.toml](rust-toolchain.toml) pins it, and the `grr` binary enables the `http3` features, which need the `--cfg reqwest_unstable` that [.cargo/config.toml](.cargo/config.toml) sets for you (plus `rustup component add rust-src` for the build-std config). `grr-core` alone builds on stable.

```sh
cargo build --workspace
cargo test --workspace --locked
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p grr-cli -- gmail profile
```

- Tests never touch real credentials — token paths are injected, and wiremock/mockito serve the API endpoints.
- `RUST_LOG=debug` traces requests; quinn's harmless IPv6 warnings are muted by default.

## Packaging & status

| Channel | Install | Status |
| --- | --- | --- |
| GitHub Releases | 3-platform binaries (Windows x64, Linux x64, macOS ARM) built on `v*` tags | live — [releases](https://github.com/debanjanbasu/grr-cli/releases) |
| crates.io | `cargo install grr-cli` (binary installs as `grr`) | pending — the `grr` name is taken by an unrelated crate, so the package publishes as `grr-cli` |
| winget | `winget install debanjanbasu.grr` | submitted (0.2.0) |
| Homebrew | `brew install debanjanbasu/tap/grr` (tap: [debanjanbasu/homebrew-grr](https://github.com/debanjanbasu/homebrew-grr)) | live (0.2.0, arm64 macOS + x86_64 Linux) |

Publishing model: `v*` tags trigger the release workflow (3-platform binaries); crates.io will use trusted publishing (no tokens).

## License

MIT — see [LICENSE](./LICENSE).
