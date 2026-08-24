# gmail-opencode-rust

High-performance Gmail API client with verified HTTP/3 transport and streaming I/O.

## Overview

A Rust rewrite of the original TypeScript `gmail-opencode` project, structured as a 3-crate workspace:

- **gmail-core** — Core API client, transport logic, streaming I/O, fs_io with optional io_uring support
- **gmail-cli** — Command-line interface for Gmail operations (send, import, auth, transport status)
- **gmail-skill-bindings** — NAPI bindings for Node.js integration (846 lines, no npm packaging yet)

## Features

- **Verified HTTP/3 transport** — reqwest per-request `HTTP_3` variant; builder probes once at build time, records `fell_back=true` permanently for the session (visible via `gmail transport` command)
- **Streaming media upload** — `mime_message_stream` via `futures::stream::unfold` state machine, CHUNK=192*1024, incremental `STANDARD.encode_slice` encoding
- **Linux file I/O with io_uring** — optional `io_uring` feature gates `tokio-uring` backend; otherwise `tokio::fs`; detection requires `cfg(all(target_os="linux", feature="io_uring"))`
- **OAuth 2.0 / PKCE** — config lives in `~/.gmail-opencode/config.toml` (snake_case aliases accepted alongside kebab-case); token stored at `~/.gmail-opencode/token.json`
- **Attachment pipeline** — single-allocation URL_SAFE_NO_PAD base64 decode; URL-safe base64 CLI bug fixed

## Quick Start

```bash
# Install
cargo install -p gmail-cli

# Authenticate (opens browser)
cargo run -p gmail-cli -- auth --force

# Check transport status
gmail transport

# Send an email
gmail send --to user@example.com --subject "Hello" --body "world"

# Import messages from RFC 822 file
gmail import --file message.rfc822
```

## Configuration

Config lives in `~/.gmail-opencode/config.toml` (home directory, **not** repo). Supports both snake_case and kebab-case key aliases:

```toml
[OAuth]
client_id = "your-client-id.apps.googleusercontent.com"
client_secret = "your-client-secret"
redirect_uri = "urn:ietf:wg:oauth:2.0:oob"
use_pkce = true
```

See [config.toml.example](./config.toml.example) for the full schema.

## License

MIT

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).