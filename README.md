<p align="center"><img src="assets/logo-wordmark.svg" width="420" alt="grr — Google Rust Rewrite"></p>
<p align="center"><img src="assets/favicon.svg" width="32" height="32" alt="grr favicon"></p>

# grr

[![CI](https://github.com/debanjanbasu/grr-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/debanjanbasu/grr-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![crates.io](https://img.shields.io/crates/v/grr-cli.svg)](https://crates.io/crates/grr-cli)

**Google tools from the terminal, at maximum performance.** `grr-cli` is one published Rust package with the `grr` command-line binary and the `grr_cli` library behind it. Gmail, Calendar, Drive, Contacts, Chat, and Forms share one OAuth login, while stdout stays clean and machine-readable.

Project site: [grr-cli.pages.dev](https://grr-cli.pages.dev/) · [Privacy](https://grr-cli.pages.dev/privacy/)

grr is an independent project and is not affiliated with or endorsed by Google.

```sh
grr auth login
grr gmail message search "in:inbox" --max 5
grr gmail message get 191f8ab2 --body
grr schema
```

## Install

**Prebuilt binaries** — Windows x64, Linux x64, macOS ARM:
[GitHub Releases](https://github.com/debanjanbasu/grr-cli/releases)

**From source** (the default CLI build requires Rust nightly — see [Development](#development)):

```sh
cargo install --git https://github.com/debanjanbasu/grr-cli --locked
```

or from a local clone at the repository root:

```sh
git clone https://github.com/debanjanbasu/grr-cli
cd grr-cli
cargo install --path . --locked
```

**Package managers**:

```sh
winget install debanjanbasu.grr       # Windows (0.2.0 live; 0.3.0 update PR pending)
brew install debanjanbasu/tap/grr     # macOS + Linux (tap: debanjanbasu/homebrew-grr)
cargo install grr-cli                 # crates.io (0.3.0 live; 0.4.0 in preparation)
```

The crates.io CLI build needs nightly Rust and `RUSTFLAGS="--cfg reqwest_unstable"` for HTTP/3; the prebuilt releases avoid that source-build step. Library consumers pick services with cargo features; every build, library or CLI, requires Rust nightly (see [Library use](#library-use)).

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

   The project/fork is **Google Rust Rewrite**; the Google consent-screen application is named **Rust Rewrite**. The consent screen is where that shorter name appears.

   **0.4 re-consent:** if you used a pre-0.4 token, run `grr auth login` again. The new service permissions include `chat.delete`, `chat.memberships`, `chat.messages.reactions`, and `contacts.other.readonly`; an existing grant does not pick them up automatically.

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

Every data command takes `-f/--format json|jsonl|table|pretty` (default `json`). For `gmail message get`, `--message-format` selects the stored Gmail MIME format (`full`, `metadata`, `minimal`, or `raw`) while `-f/--format` still selects the printed output format. Logs go to stderr, so stdout is always parseable:

```sh
grr gmail message search "in:inbox" --max 1 | jq -r '.[0].id'
```

### Command reference

| Group | Commands |
| --- | --- |
| `grr auth` | `login [--device]`, `status` |
| `grr gmail` | `message`, `label`, `draft`, `send`, `thread`, `history`, `send-as`, `profile`, `watch`, `import`, `msg` |
| `grr gmail message` | `search <query>`, `get <id> [--message-format full\|metadata\|minimal\|raw] [--body] [--max-length N]`, `thread <id>`, `batch-read --ids a,b,c [--body] [--max-length N]`, `attachment <msg-id> <att-id> [-o FILE]` |
| `grr gmail label` | `list`, `get <id>`, `create <name>`, `update <id>` (the CLI spelling for label updates; the library also exposes `GmailClient::patch_label`), `delete <id>` |
| `grr gmail draft` | `create <to> <subject> <body>`, `list`, `get <id>`, `update <id> <to> <subject> <body>`, `delete <id>`, `send <id>` |
| `grr gmail send` | `send <to> <subject> <body> [--cc] [--bcc]`, `send-attach <to> <subject> <body> --attachments f1,f2 [--thread-id] [--cc] [--bcc]` |
| `grr gmail thread` | `list`, `label <id> --add/--remove ...`, `trash <id>`, `untrash <id>`, `delete <id>` |
| `grr gmail history` | `<start-history-id> [--label-id] [--max N]`; the client follows Gmail history pages up to the limit |
| `grr gmail send-as` | `list`, `get <email>`, `create <email>`, `update <email>`, `delete <email>`, `verify <email>` |
| `grr gmail profile` | mailbox profile |
| `grr gmail watch` | `start <topic>`, `stop` (push notifications) |
| `grr gmail import` | `<rfc822-file> [--deleted]` |
| `grr gmail msg` | `label`, `trash`, `untrash`, `delete`, `batch-label`, `batch-delete`, `filter-list`, `filter`, `filter-create`, `filter-delete`, `forwarding-list`, `forwarding-create`, `forwarding-delete`, `autoforwarding`, `autoforwarding-set`, `pop`, `pop-set`, `imap`, `imap-set` |
| `grr calendar` | `list`, `events`, `get`, `new`, `create`, `update`, `delete`, `free-busy`, `set`, `instances`, `patch`, `move`, `watch`, `stop`, `colors`, `settings`, `share`, `shares`, `unshare` |
| `grr drive` | `list [--trashed]`, `search`, `get`, `mkdir`, `trash`, `restore`, `copy`, `empty-trash`, `download`, `upload`, `rename`, `delete`, `export`, `share`, `shares`, `unshare`, `comments`, `comment`, `comment-add`, `comment-delete`, `revisions`, `revision`, `quota` |
| `grr contacts` | `list`, `search`, `get`, `create`, `update`, `delete`, `groups`, `group`, `group-new`, `group-rename`, `group-delete`, `group-add`, `group-remove`, `get-batch`, `others`, `adopt`, `photos`, `photo-set`, `photo-remove` |
| `grr chat` | `spaces`, `space`, `space-new`, `space-rename`, `space-delete`, `members`, `member`, `member-add`, `member-remove`, `messages`, `send`, `react`, `reactions`, `unreact` |
| `grr forms` | `get`, `responses`, `new`, `update`, `watch`, `watches`, `watch-delete`, `watch-renew` |
| `grr transport` | negotiated HTTP version + runtime features |
| `grr schema` | the full command tree as JSON |

Details worth knowing:

- **`--body`** returns `{message, body}` where `body` is the decoded text (`text/plain` preferred, `text/html` fallback), truncated to `--max-length` (default 800) with a `...[truncated]` marker — sized for LLM context windows.
- **Batch ops** accept `--ids a,b,c` or `--search "query"` (resolves IDs by running the query, capped by `--max`, default 10000), plus `--dry-run`. `batch-label`/`batch-delete` calls are chunked at 1000 IDs (Gmail's batchModify/batchDelete limit); `batch-read` fans its fetches out in parallel, preserving input order.
- **`attachment`** prints base64 (URL-safe) to stdout when no `-o FILE` is given, so piping is always binary-safe.
- **Gmail settings and routing** live under `grr gmail msg`: filters (`filter-list`, `filter`, `filter-create`, `filter-delete`), forwarding addresses, auto-forwarding, POP, and IMAP. `grr gmail thread list` lists threads, `grr gmail send-as verify <email>` verifies an alias, and `grr gmail history` follows Gmail's paginated history response.
- **Label PATCH** is available to library users through `GmailClient::patch_label`; the CLI exposes label updates as `grr gmail label update <id>`.

## Design philosophy

- **Zero-config.** `~/.grr/config.toml` holds exactly one thing: an OAuth client ID (and optionally a secret). Scopes, redirect URI, pool sizes, timeouts, and retry policy are compile-time constants tuned for Google's frontends ([src/core/http.rs](src/core/http.rs)). `GRR_CONFIG_PATH` overrides the file location, `RUST_LOG` the log level (`GRR_OAUTH__*` env vars exist for headless overrides) — nothing else is configurable, on purpose.
- **Namespaced services.** Mail is `grr gmail ...`; Calendar, Drive, Contacts, Chat, and Forms live alongside it (`grr calendar ...`, `grr drive ...`, ...), and account-level concerns stay top-level (`grr auth`, `grr transport`, `grr schema`). One login covers every service; each service client builds lazily so running one never probes another's endpoints. (Keep has no consumer-facing API.)
- **stdout purity.** Logs go to stderr, results go to stdout, so `| jq` always works. `-f jsonl` streams arrays one object per line.
- **Keyring-first token storage.** Tokens live in the OS keyring (Windows Credential Manager, macOS Keychain, Linux Secret Service via D-Bus), with automatic fallback to `<cache dir>/grr/token.json` on headless systems. A token found in the fallback file auto-imports into the keyring on first sight.
- **Agent-first.** `grr schema` dumps the complete command tree as JSON with zero configuration — the machine-readable contract for AI agents, discoverable without touching a config file or scraping `--help`. One fast CLI replaces per-service MCP servers: no MCP setup, just `grr schema`.

## Performance

- **HTTP/3 (QUIC) by default** — prior-knowledge h3 with one authenticated probe at startup and silent HTTP/2 fallback; `grr transport` shows what was actually negotiated.
- **Tokio multi-threaded runtime**, auto-sized to cores — no thread pool to tune.
- **In-flight request valve** — a semaphore (not a thread pool) caps concurrent HTTP requests at 64, staying under Gmail's per-user rate limits so bursts don't self-DOS into 429s. 429s are retried honoring `Retry-After` (waits capped at 30s); whole-request timeout is 30s.
- **Compression always on** — gzip, deflate, zstd, and brotli response decompression.
- **Streaming uploads** — RFC 822 media upload streams 192 KiB chunks with incremental base64, so large attachments never sit fully in memory.
- **io_uring file I/O** is a Linux-only target-specific dependency, auto-detected at runtime; it is not a Cargo feature.

## Architecture

`grr-cli` is one published crate at the repository root. `src/lib.rs` builds the library target `grr_cli`; `src/main.rs` is a thin wrapper over `src/cli.rs`, and the binary is named `grr`. The `grr` binary declares `required-features = ["cli"]`.

```text
.
├── src/
│   ├── core/                 # auth/device/oauth/server/store, http.rs,
│   │                         # config.rs, config_loader.rs, error.rs,
│   │                         # fs_io.rs, runtime.rs
│   ├── gmail/                # client/, models.rs, mod.rs
│   ├── calendar/             # client.rs, models.rs, mod.rs
│   ├── drive/                # client.rs, models.rs, mod.rs
│   ├── people/               # client.rs, models.rs, mod.rs
│   ├── chat/                 # client.rs, models.rs, mod.rs
│   ├── forms/                # client.rs, models.rs, mod.rs
│   ├── commands/             # one module per command group
│   ├── schema.rs
│   ├── output.rs
│   ├── cli.rs
│   ├── lib.rs
│   └── main.rs
└── tests/                    # 18 flattened integration test files
```

The service modules sit behind one shared core rather than separate published crates. The repository also contains the `assets/`, `site/` (the Astro GitHub Pages site with base `/grr-cli`), `packaging/`, and `scripts/` material used for the project site and distribution.

### Library use

The package exposes the same clients through the `grr_cli` library. The `cli` feature enables all six service features and is required by the `grr` binary; individual services can be selected independently with `gmail`, `calendar`, `drive`, `people`, `chat`, and `forms`. The Cargo feature declarations are:

```toml
default = ["cli"]
cli = ["gmail", "calendar", "drive", "people", "chat", "forms"]
gmail = []
calendar = []
drive = []
people = []
chat = []
forms = []
```

There is no `http3` feature: HTTP/3 (rustls + quinn, via reqwest's unstable http3 support) is **always compiled in**, and HTTP/2 exists only as a runtime fallback. A library dependency can select only the services it needs:

```toml
[dependencies]
grr-cli = { version = "0.4.0", default-features = false, features = ["gmail"] }
```

Every build — CLI or library, any feature subset — requires Rust **nightly** and the `reqwest_unstable` cfg (`.cargo/config.toml` supplies it for in-repo builds; downstream users need `RUSTFLAGS="--cfg reqwest_unstable"`). There is deliberately no stable-Rust path. The old per-crate `io_uring` feature is gone: io_uring is a Linux-only target-specific dependency that is detected at runtime.

## Development

grr requires Rust **nightly** — the build script fails with a clear message on any other toolchain. [rust-toolchain.toml](rust-toolchain.toml) pins it and supplies the components needed by the build, while [.cargo/config.toml](.cargo/config.toml) sets the `reqwest_unstable` cfg for HTTP/3.

```sh
cargo build
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
cargo run -- gmail profile
```

- Tests never touch real credentials — token paths are injected, and wiremock/mockito serve the API endpoints.
- `RUST_LOG=debug` traces requests; quinn's harmless IPv6 warnings are muted by default.

## Packaging & status

| Channel | Install | Status |
| --- | --- | --- |
| GitHub Releases | 3-platform binaries (Windows x64, Linux x64, macOS ARM) built on `v*` tags | live for existing releases; **0.4.0 release in preparation** — [releases](https://github.com/debanjanbasu/grr-cli/releases) |
| crates.io | `cargo install grr-cli` (binary installs as `grr`; needs nightly + `RUSTFLAGS="--cfg reqwest_unstable"` for the default CLI HTTP/3 build) | **0.3.0 live; 0.4.0 release in preparation** — one crate now; the 0.3.0 library crates are legacy/unpublished going forward. Trusted publishing uses OIDC (no stored API tokens) |
| winget | `winget install debanjanbasu.grr` | live at 0.2.0; update PR to 0.3.0 pending Microsoft review |
| Homebrew | `brew install debanjanbasu/tap/grr` (tap: [debanjanbasu/homebrew-grr](https://github.com/debanjanbasu/homebrew-grr)) | live (0.3.0, arm64 macOS + x86_64 Linux) |

The project publishes one package, `grr-cli` (library `grr_cli` plus binary `grr`). `v*` tags trigger the release workflow, and crates.io publishing is handled through trusted publishing; the 0.3.0 service/core crates do not receive new releases.

## License

MIT — see [LICENSE](./LICENSE).
