# gmail-cli

High-performance Gmail CLI in Rust: verified HTTP/3 transport, streaming I/O,
PKCE + device-code OAuth, and clean JSON output designed for both humans and
AI agents.

```
$ gmail message search "in:inbox" -m 5 | jq -r '.[].id'
$ gmail auth login                    # browser loopback flow (PKCE)
$ gmail auth login --device           # headless fallback (RFC 8628)
```

## Features

- **HTTP/3 by default** — QUIC via reqwest's unstable `http3` feature; probes
  once at startup, falls back to HTTP/2 silently (`gmail transport` shows what
  was negotiated)
- **Streaming uploads** — RFC 822 media upload via a `futures` state machine
  (192 KiB chunks, incremental base64), so large attachments never sit fully
  in memory
- **Auth that fits the environment** — PKCE browser loopback flow by default;
  RFC 8628 device flow for headless machines; `client_secret` sent only when
  configured (Google requires it, PKCE-only providers don't)
- **Agent-friendly output** — JSON on stdout, logs on stderr: `| jq` just
  works. `-f table|pretty|jsonl` for humans
- **Parallel batch operations** — semaphore-bounded concurrency for
  `msg batch-label` / `batch-delete`

## Install

### From source (any platform)

Requires Rust nightly and the MSVC toolchain (Windows) or Xcode CLT (macOS):

```sh
git clone https://github.com/debanjanbasu/gmail-cli.git
cd gmail-cli
cargo install --path gmail-cli --locked
gmail --version
```

### Prebuilt binaries

Download from [GitHub Releases](https://github.com/debanjanbasu/gmail-cli/releases)
— built for Windows x64, Linux x64, and macOS ARM by the release workflow.

## Setup

1. **Google Cloud Console** (same project, Gmail API enabled):
   - *Google Auth platform → Clients → Create client → Desktop app*
   - Copy the **client ID** (a `.apps.googleusercontent.com` string)
   - If your project is in **Testing** mode, add yourself under *Audience → Test users*
2. **Configure** — `~/.gmail-opencode/config.toml`:

   ```toml
   [oauth]
   client_id = "<your-client-id>.apps.googleusercontent.com"
   # Optional: Google mandates a secret even for Desktop clients.
   # Omit it entirely for PKCE-only providers.
   # client_secret = "<secret>"

   use_pkce = true
   ```

   See [`config.toml.example`](./config.toml.example) for the full schema
   (performance tuning, compression, cache). `GMAIL_CONFIG_PATH` overrides
   the file location; `GMAIL_OAUTH__CLIENT_ID` etc. override individual
   values.
3. **Authenticate**:

   ```sh
   gmail auth login            # opens your browser, loopback on :3434
   gmail auth login --device   # prints a URL + code for another device
   gmail profile               # verify: prints your account profile
   ```

Tokens auto-refresh and live in the platform cache dir
(`%LOCALAPPDATA%\gmail-opencode\token.json` on Windows). If the OAuth client
is in Testing mode, Google expires refresh tokens after ~7 days — just run
`gmail auth login` again. Rotating a reset client secret without echoing it:

```powershell
./scripts/set-client-secret.ps1   # hidden prompt; -EnvVarName for CI
```

## Usage

| Command group | Examples |
| --- | --- |
| `message` | `search "from:github.com" -m 10`, `get <id>`, `thread <id>`, `attachment <msgId> <attId> -o out.bin` |
| `msg` | `label <id> --add STARRED --remove UNREAD`, `trash`, `untrash`, `delete`, `batch-label --ids a,b --add READ`, `batch-delete --ids a,b` |
| `label` | `list`, `get`, `create "Name"`, `update`, `delete` |
| `draft` | `create <to> <subject> <body>`, `list`, `get`, `update`, `delete`, `send` |
| `send` | `send <to> <subject> <body> [--cc] [--bcc]`, `send-attach <to> <subject> <body> --attachments a.pdf,b.png` |
| `thread` | `label`, `trash`, `untrash`, `delete` |
| `history` | `history <startHistoryId> [--label-id] [-m N]` |
| `send-as` | `list`, `get`, `create`, `update`, `delete` |
| `watch` | `start <topicName>`, `stop` (Gmail push notifications) |
| `import` | `import --file message.rfc822 [--deleted]` |
| `profile` / `transport` / `auth` | account info, negotiated protocol info, login/logout |

Every subcommand takes `-f json|jsonl|table|pretty` (default `json`).
Logs go to stderr — stdout is always parseable:

```sh
gmail message search "in:inbox" -m 1 | jq -r '.[0].id'
```

## Development

Workspace layout:

```
gmail-cli/            # the binary (clap v4, one file per command group)
gmail-core/           # library: auth, client, models, streaming, config
gmail-skill-bindings/ # NAPI bindings for Node.js integration (WIP, unpackaged)
```

```sh
cargo build -p gmail-cli            # dev build
cargo test --workspace              # unit + integration tests
cargo clippy --workspace --all-targets
cargo run -p gmail-cli -- auth login
```

Notes:

- **Nightly is required** — the `http3` reqwest feature needs
  `--cfg reqwest_unstable` (set for you in [`.cargo/config.toml`](./.cargo/config.toml)),
  and the build-std config needs the `rust-src` component. First build takes
  a few minutes; release profile uses fat LTO.
- **Logs** — `RUST_LOG` controls verbosity (default `info`, with quinn's
  harmless IPv6 warnings muted; try `RUST_LOG=debug` to trace HTTP/2 frames).
- **Tests** never touch real credentials — token paths are injected, and
  wiremock serves the API endpoints.

## Security notes

- The OAuth token cache is plaintext JSON in your user cache dir; treat the
  machine's disk security as the boundary.
- `client_secret` is only sent when configured; it is never logged.
- Never commit `config.toml` with real credentials — `.gitignore` covers it.

## Roadmap

- [ ] `--search` flag on `msg batch-label` / `batch-delete` (query-driven bulk ops)
- [ ] NAPI skill bindings packaged for npm
- [ ] crates.io publish of `gmail-core` (blocked: unstable `http3` feature)
- [ ] OS keyring storage for tokens

## License

MIT — see [LICENSE](./LICENSE).
