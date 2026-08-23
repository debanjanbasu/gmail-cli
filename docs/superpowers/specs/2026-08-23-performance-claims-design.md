# Performance Claims — Make Them Real

**Date:** 2026-08-23
**Status:** Approved (design reviewed in session)
**Scope:** Sub-project 1 of 3 (performance claims). Sub-projects 2 (packaging/docs) and 3 (quality infra) are tracked separately as bounded work.

## Problem

The workspace markets itself as "High-performance Gmail API client with HTTP/3, io_uring, and zero-copy streaming" but:

1. **HTTP/3** rides reqwest's unstable feature with a **mutual-exclusion bug**: `build_http_client()` can set both `http3_prior_knowledge()` and `http2_prior_knowledge()` under default config (`enable_http3=true`, `enable_http2=true`). Negotiated protocol is never verified or observable.
2. **io_uring** is detection-only. `tokio-uring` is a Linux-target dependency used by nothing.
3. **Zero-copy streaming** does not exist: attachments buffer 3× (JSON string → struct field → CLI re-decode); attachment uploads build entire MIME in memory via `Vec<String>` join; imports take `&str`.
4. `simd` feature flag is empty vapor code.
5. `Cargo.lock` is gitignored (wrong for a binary project).

## Goals

Every shipped claim must be observable and true:

| Claim | Becomes |
|---|---|
| HTTP/3 | Real quinn-based h3 prior-knowledge transport, verified negotiation with transparent fallback, observable via `gmail transport` |
| Zero-copy streaming | Single-allocation `Bytes` decode paths, chunked disk I/O, incremental base64 streaming uploads — no full-payload buffering |
| io_uring | File-I/O acceleration (attachment read/write, import) on Linux behind `spawn_blocking(tokio_uring::start(...))`, tokio fallback elsewhere |

## Non-goals

- Custom network stack on glommio/monoio (rejected: ecosystem immaturity, loses h2/h3 fallback).
- rkyv zero-copy deserialization (abandoned upstream due to recursive types; not needed for honest claims).
- simd anything.

## Design

### 1. Transport layer

**File:** `gmail-core/src/client/mod.rs`

- Fix `build_http_client(perf)`:
  - `enable_http3 == true` → `.http3_prior_knowledge()`, never h2 prior knowledge.
  - else if `enable_http2 == true` → `.http2_prior_knowledge()`.
  - else → ALPN default (no prior knowledge calls).
- New `TransportInfo { negotiated_version: String, http3_requested: bool, http3_effective: bool, fell_back: bool }` stored on `GmailClient`.
- `GmailClientBuilder::build()`: when h3 requested, perform one `GET users/me/profile` probe through the built client. On connect error matching QUIC/h3 failure modes → rebuild client without h3, set `fell_back = true`, log warning. Capture `response.version()` from probe into `TransportInfo`.
- New public method `GmailClient::transport_info() -> &TransportInfo`.

**CLI:** new `gmail-cli/src/commands/transport.rs` — top-level `gmail transport` prints negotiated protocol, requested/effective h3, runtime features (cpus, concurrency, pool), pool size. Registered in `main.rs` alongside existing commands.

**Config:** unchanged shape (`[performance] enable_http3`, `enable_http2`).

### 2. Streaming pipeline

**Files:** `gmail-core/src/client/messages.rs`, new helpers in `gmail-core/src/fs_io.rs`

- `get_attachment_bytes(mid, aid) -> Result<bytes::Bytes>`: fetch JSON, extract `data` field, base64-decode directly into `bytes::BytesMut` sized from base64 length estimate, `freeze()`. One heap allocation for payload. Existing `get_attachment()` and the `Attachment` model stay unchanged (JSON/NAPI serialization compatibility); CLI attachment paths switch to `get_attachment_bytes`.
- `download_attachment_to(mid, aid, path) -> Result<u64>`: uses `get_attachment_bytes` + `fs_io::write_file`. Returns bytes written.
- `send_attachments_from_disk(to, subject, body, paths, thread_id)`: MIME assembled incrementally into a writer chain; each file streamed through `base64::write::EncoderWriter` into `reqwest::Body::wrap_stream`; no whole-file buffering. Boundary generation unchanged.
- `import_from_file(path, deleted)`: reads RFC822 via `fs_io::read_file`, encodes, posts. (Gmail import API requires base64-in-JSON, so file streams to encode, single `Bytes` out.)
- `get_message_raw_bytes(id) -> Result<Bytes>` replacing String return (callers updated).

### 3. io_uring file I/O

**New file:** `gmail-core/src/fs_io.rs`

```rust
pub async fn write_file(path: &Path, data: Bytes) -> Result<()>
pub async fn read_file(path: &Path) -> Result<Bytes>
```

- Linux + `io_uring` cargo feature enabled + `runtime::has_io_uring()` → `tokio::task::spawn_blocking(move || tokio_uring::start(...))` doing the ring-backed op.
- Otherwise → `tokio::fs` equivalents.
- Identical signatures regardless of backend; callers never branch.
- `tokio-uring` dep stays `[target.'cfg(target_os = "linux")']`-gated; add optional feature `io_uring` (default ON for Linux builds).

### 4. Honesty cleanup

- Workspace description → `"Gmail API client with verified HTTP/3 transport and streaming I/O"`; keywords: drop `zero-copy`, `napi` from hype positions where untrue post-change (keep accurate ones).
- Delete: `simd` feature (Cargo.toml ×3), `RuntimeFeatures.simd`, `has_simd()`, any config references.
- Remove `Cargo.lock` from `.gitignore`; commit lockfile.

### 5. Testing & verification

- Unit (wiremock): `get_attachment_bytes` round-trip incl. URL-safe-no-pad decode correctness; `build_http_client` mutual-exclusion matrix (h3 on/off × h2 on/off).
- Unit (tempfile): `fs_io::read/write` both backends (backend-specific tests cfg-gated).
- Integration (assert_cmd): `gmail transport` smoke test (parses, prints valid fields offline-safe).
- Live e2e (`#[ignore]`, gated by `LIVE_E2E=1` env): auth via existing `~/.gmail-opencode/token.json` → profile → search max=1 → download_attachment_to(tmpfile) → import_from_file → cleanup; asserts transport info reports effective protocol.
- All 21 existing tests remain green.

## Acceptance criteria

1. `cargo test --workspace` green; live e2e passes with `LIVE_E2E=1`.
2. `gmail transport` output shows actual negotiated protocol against gmail.googleapis.com.
3. No code path buffers a full attachment twice; upload path streams from disk.
4. On macOS: binary builds clean without tokio-uring; on Linux CI-less check: `cargo check --target x86_64-unknown-linux-gnu --features io_uring` (or documented manual step).
5. Descriptions/keywords match reality; no dead `simd` symbols.
