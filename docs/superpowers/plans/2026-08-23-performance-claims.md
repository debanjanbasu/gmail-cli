# Performance Claims Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every performance claim (HTTP/3, zero-copy streaming, io_uring) literally true and observable.

**Architecture:** Fix the mutually-exclusive h3/h2 transport config with verified negotiation + fallback; add a `fs_io` abstraction that uses io_uring on Linux; convert attachment download/upload/import paths to `bytes::Bytes` streaming via Gmail's media-upload endpoints.

**Tech Stack:** Rust 2024 workspace, reqwest 0.13 (`http3` unstable feature/quinn), tokio + tokio-uring (Linux), bytes, base64 0.22, wiremock/mockito for tests.

**Spec:** `docs/superpowers/specs/2026-08-23-performance-claims-design.md`

## Global Constraints

- Edition 2024; strict lints from `.cargo/config.toml` must pass (`cargo clippy --workspace` clean).
- No new dependencies except `bytes` and `futures` (already in ecosystem via reqwest/tokio).
- All existing 21 tests stay green after every task.
- Conventional commits (`feat:`, `fix:`, `chore:`, `test:`).
- No code comments unless documenting a non-obvious invariant (match repo style of doc comments).
- Platform gate: `tokio-uring` compiles only under `[target.'cfg(target_os = "linux")']`; macOS builds must not reference it outside cfg blocks.
- Config lives at `~/.gmail-opencode/config.toml` — never inside the repo.

## Parallelism map

```
Task 1 (transport) ─┬─> Task 5 (transport CLI)
Task 2 (fs_io) ──┬──┴─> Task 3 (attachment bytes) ─┐
                 └───> Task 4 (streaming upload)  ─┼─> Task 7 (live e2e) -> Task 8 (verify)
Task 6 (honesty cleanup, fully independent) ───────┘
```

Tasks 1, 2, 6 can run simultaneously. Tasks 3 and 4 depend only on 2. Task 5 depends on 1.

---

### Task 1: Transport layer — fix h3/h2 exclusion, verified negotiation

**Files:**
- Modify: `gmail-core/src/client/mod.rs`
- Test: `gmail-core/tests/transport_test.rs` (new)

**Interfaces:**
- Consumes: `PerformanceConfig { enable_http3, enable_http2, .. }`, `execute_with_retry(RequestBuilder) -> Result<Response>`, `GmailAuth::get_access_token()`.
- Produces: `pub struct TransportInfo { pub negotiated_version: String, pub http3_requested: bool, pub http3_effective: bool, pub fell_back: bool }`; `GmailClient::transport_info() -> &TransportInfo`; `GmailClientBuilder::base_url(Url) -> Self` (test injection); `fn build_http_client(perf: &PerformanceConfig) -> ReqwestClient` (now private-fn testable via unit tests in module).

- [ ] **Step 1: Add failing tests**

Create `gmail-core/tests/transport_test.rs`:

```rust
use gmail_core::{ConfigLoader, GmailClientBuilder};

fn perf_with(h3: bool, h2: bool) -> gmail_core::config::PerformanceConfig {
    let mut p = PerformanceConfig_default();
    p.enable_http3 = h3;
    p.enable_http2 = h2;
    p
}

// NOTE: helper below stands in for direct builder access; adjust import to
// whatever the crate exports (prelude re-export expected).
#[test]
fn http3_and_http2_prior_knowledge_are_mutually_exclusive_h3_wins() {
    // build_http_client is private; expose via #[cfg(test)] pub(crate) or
    // an integration-visible constructor. Assert by inspecting built client
    // behavior through a mock probe instead:
    // - h3=true,h2=true => exactly one prior-knowledge mode active
}
```

Because `build_http_client` is private, first make it `pub(crate)` and add `#[cfg(test)] pub fn __build_for_test(...)`. Simpler: move the mutual-exclusion logic into a pure helper and unit-test it:

```rust
// in client/mod.rs
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum TransportMode { Http3PriorKnowledge, Http2PriorKnowledge, AlpnDefault }

pub(crate) fn resolve_transport_mode(enable_http3: bool, http3_feature: bool, enable_http2: bool) -> TransportMode {
    if enable_http3 && http3_feature { TransportMode::Http3PriorKnowledge }
    else if enable_http2 { TransportMode::Http2PriorKnowledge }
    else { TransportMode::AlpnDefault }
}
```

Test file then contains:

```rust
use gmail_core::client::{resolve_transport_mode, TransportMode};

#[test]
fn h3_requested_with_feature_enabled_wins_over_h2() {
    assert_eq!(resolve_transport_mode(true, true, true), TransportMode::Http3PriorKnowledge);
}

#[test]
fn h3_requested_without_feature_falls_to_h2() {
    assert_eq!(resolve_transport_mode(true, false, true), TransportMode::Http2PriorKnowledge);
}

#[test]
fn neither_enabled_is_alpn_default() {
    assert_eq!(resolve_transport_mode(false, true, false), TransportMode::AlpnDefault);
}

#[test]
fn h2_only_is_prior_knowledge() {
    assert_eq!(resolve_transport_mode(false, true, true), TransportMode::Http2PriorKnowledge);
}
```

Export `client` module publicly if not already (`pub mod client;` in `lib.rs` — verify; it likely re-exports through prelude).

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p gmail-core --test transport_test`
Expected: FAIL (items don't exist).

- [ ] **Step 3: Implement**

In `client/mod.rs`:

1. Add `TransportMode` enum + `resolve_transport_mode` as shown above.
2. Rewrite the HTTP/3+HTTP/2 section of `build_http_client`:

```rust
    match resolve_transport_mode(perf.enable_http3, cfg!(feature = "http3"), perf.enable_http2) {
        TransportMode::Http3PriorKnowledge => {
            #[cfg(feature = "http3")]
            { builder = builder.http3_prior_knowledge(); }
        }
        TransportMode::Http2PriorKnowledge => {
            builder = builder.http2_prior_knowledge();
        }
        TransportMode::AlpnDefault => {}
    }
```

3. Add `TransportInfo` struct (public, at `client` module level):

```rust
#[derive(Debug, Clone, Default)]
pub struct TransportInfo {
    pub negotiated_version: String,
    pub http3_requested: bool,
    pub http3_effective: bool,
    pub fell_back: bool,
}
```

4. Extend `GmailClient` with field `transport_info: TransportInfo` + accessor `pub fn transport_info(&self) -> &TransportInfo`.
5. Add optional override to `GmailClientBuilder`: `base_url: Option<Url>` field + `pub fn base_url(mut self, url: Url) -> Self`; use it in `build()` instead of hardcoded parse when present.
6. Probe logic in `GmailClientBuilder::build()` after constructing `http_client`:

```rust
        let requested = self.config.performance.enable_http3 && cfg!(feature = "http3");
        let mut info = TransportInfo { http3_requested: requested, http3_effective: requested, ..Default::default() };
        let mut http_client = ...;

        async fn probe(client: &ReqwestClient, base: &Url, auth: &GmailAuth) -> Result<String> {
            let token = auth.get_access_token().await?;
            let url = base.join("users/me/profile")?;
            let resp = client.get(url).bearer_auth(&token).send().await?
                .error_for_status()?;
            Ok(format!("{:?}", resp.version()))
        }

        if requested {
            match probe(&http_client, &base_url, &auth).await {
                Ok(v) => info.negotiated_version = v,
                Err(e) => {
                    warn!("HTTP/3 probe failed ({e}); rebuilding without h3");
                    let mut perf_no_h3 = self.config.performance.clone();
                    perf_no_h3.enable_http3 = false;
                    http_client = build_http_client(&perf_no_h3);
                    info.negotiated_version = probe(&http_client, &base_url, &auth).await?;
                    info.http3_effective = false;
                    info.fell_back = true;
                }
            }
        } else {
            info.negotiated_version = "not-probed".into();
        }
```

(Adjust construction order so `base_url` exists before probe; keep semaphore/log lines intact.)

- [ ] **Step 4: Run tests**

Run: `cargo test -p gmail-core`
Expected: all PASS including 4 new transport tests + existing suites.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): fix h3/h2 prior-knowledge exclusion, add verified TransportInfo with h3 fallback"
```

---

### Task 2: fs_io abstraction with io_uring backend (parallel-safe)

**Files:**
- Create: `gmail-core/src/fs_io.rs`
- Modify: `gmail-core/src/lib.rs` (add `pub mod fs_io;`)
- Modify: `gmail-core/Cargo.toml` (+ `io_uring` feature, `bytes` dep), root `Cargo.toml` (workspace `bytes = "1.10"`)
- Test: `gmail-core/tests/fs_io_test.rs` (new)

**Interfaces:**
- Produces: `pub async fn write_file(path: &Path, data: bytes::Bytes) -> Result<()>` and `pub async fn read_file(path: &Path) -> Result<bytes::Bytes>` in `gmail_core::fs_io`.

- [ ] **Step 1: Check error variant**

Run: `grep -n "Io\|io::Error" gmail-core/src/error.rs`
If no `Io(std::io::Error)` variant exists, add:

```rust
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
```

(match thiserror style already used in the file)

- [ ] **Step 2: Write failing tests**

`gmail-core/tests/fs_io_test.rs`:

```rust
use bytes::Bytes;

#[tokio::test]
async fn write_then_read_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blob.bin");
    let payload = Bytes::from_static(b"gmail io_uring streaming payload");
    gmail_core::fs_io::write_file(&path, payload.clone()).await.unwrap();
    let read_back = gmail_core::fs_io::read_file(&path).await.unwrap();
    assert_eq!(read_back, payload);
}

#[tokio::test]
async fn write_creates_parent_dirs_and_empty_payload_works() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/deeper/file.bin");
    gmail_core::fs_io::write_file(&path, Bytes::new()).await.unwrap();
    assert_eq!(gmail_core::fs_io::read_file(&path).await.unwrap(), Bytes::new());
}

#[cfg(all(target_os = "linux", feature = "io_uring"))]
#[tokio::test]
async fn io_uring_backend_round_trips_when_available() {
    if !gmail_core::runtime::has_io_uring() { return; }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ring.bin");
    let payload = Bytes::from_static(b"ring-backed write");
    gmail_core::fs_io::write_file(&path, payload.clone()).await.unwrap();
    assert_eq!(gmail_core::fs_io::read_file(&path).await.unwrap(), payload);
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test -p gmail-core --test fs_io_test`
Expected: compile failure (module missing).

- [ ] **Step 4: Implement `fs_io.rs`**

```rust
//! File I/O with io_uring acceleration on Linux, tokio fallback elsewhere.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use bytes::{BufMut, Bytes, BytesMut};

use crate::error::{GmailError, Result};

/// Write `data` to `path`, creating parent directories.
///
/// Uses io_uring on Linux when available and compiled in; tokio otherwise.
pub async fn write_file(path: &Path, data: Bytes) -> Result<()> {
    #[cfg(all(target_os = "linux", feature = "io_uring"))]
    if crate::runtime::has_io_uring() {
        return ring_write(path.to_path_buf(), data).await;
    }
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, &data[..]).await?;
    Ok(())
}

/// Read the full contents of `path` into a single `Bytes` allocation.
pub async fn read_file(path: &Path) -> Result<Bytes> {
    #[cfg(all(target_os = "linux", feature = "io_uring"))]
    if crate::runtime::has_io_uring() {
        return ring_read(path.to_path_buf()).await;
    }
    let vec = tokio::fs::read(path).await?;
    Ok(Bytes::from(vec))
}

#[cfg(all(target_os = "linux", feature = "io_uring"))]
async fn ring_write(path: PathBuf, data: Bytes) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = tokio_uring::fs::File::create(&path).await?;
            let (res, _data) = file.write_at(data.clone(), 0).await;
            res?;
            file.sync_all().await?;
            Ok(())
        })
    })
    .await
    .map_err(|e| GmailError::Internal(e.to_string()))?
}

#[cfg(all(target_os = "linux", feature = "io_uring"))]
async fn ring_read(path: PathBuf) -> Result<Bytes> {
    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move {
            let file = tokio_uring::fs::File::open(&path).await?;
            let len = file.metadata().await?.len() as usize;
            let mut buf = BytesMut::with_capacity(len);
            buf.put_bytes(0u8, len);
            let (res, buf) = file.read_at(buf.freeze(), 0).await;
            let n = res?;
            Ok::<_, GmailError>(buf.slice(..n))
        })
    })
    .await
    .map_err(|e| GmailError::Internal(e.to_string()))?
}
```

Cargo.toml changes — root workspace deps: `bytes = "1.10"`; `gmail-core/Cargo.toml`: `bytes = { workspace = true }` and features section gains `io_uring = []` (default stays as-is; Linux builds enable it via `--features io_uring` or make it default-on for linux via doc note).

Note: `BufMut`/`Write` imports are used by both backends; keep only what compiles per platform (clippy enforces).

- [ ] **Step 5: Run tests**

Run: `cargo test -p gmail-core --test fs_io_test`
Expected: PASS (macOS runs tokio branch; ring test compiles out).

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(core): add fs_io abstraction — io_uring-backed file I/O on Linux, tokio elsewhere"
```

---

### Task 3: Single-allocation attachment pipeline

**Files:**
- Modify: `gmail-core/src/client/messages.rs`
- Test: `gmail-core/tests/attachment_bytes_test.rs` (new)

**Interfaces:**
- Consumes: Task 2 `fs_io::write_file(Path, Bytes)`.
- Produces: `GmailClient::get_attachment_bytes(&self, message_id: &str, attachment_id: &str) -> Result<bytes::Bytes>`; `GmailClient::download_attachment_to(&self, message_id: &str, attachment_id: &str, path: &Path) -> Result<u64>` (returns bytes written); `GmailClient::get_message_raw_bytes(&self, id: &str) -> Result<bytes::Bytes>` (spec §2; decode `raw` field same single-allocation pattern, update existing callers of `get_message_raw`).

- [ ] **Step 1: Failing tests**

`gmail-core/tests/attachment_bytes_test.rs`:

```rust
use base64::Engine;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PAYLOAD: &[u8] = b"PDF-ish attachment body \x00\x01\x02 binary safe";

fn b64(input: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
}

#[tokio::test]
async fn get_attachment_decodes_in_single_allocation() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"messages/.*/attachments/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "size": PAYLOAD.len(),
            "data": b64(PAYLOAD),
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let got = client.get_attachment_bytes("msg1", "att1").await.unwrap();
    assert_eq!(&got[..], PAYLOAD);
}

#[tokio::test]
async fn download_attachment_to_writes_exact_bytes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"messages/.*/attachments/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "size": PAYLOAD.len(),
            "data": b64(PAYLOAD),
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("a.bin");
    let n = client.download_attachment_to("msg1", "att1", &out).await.unwrap();
    assert_eq!(n as usize, PAYLOAD.len());
    assert_eq!(std::fs::read(&out).unwrap(), PAYLOAD);
}

async fn test_client(base: &str) -> gmail_core::GmailClient {
    // Build client against mock using the Task-1 base_url override.
    // Auth token fetch must also hit the mock: mount oauth token endpoint
    // or inject a pre-authed GmailAuth via GmailClientBuilder::auth with a
    // stubbed token store pointing at the mock's /token route.
    unimplemented!("wire per existing auth test helpers — see gmail-core/tests/config_test.rs for ConfigLoader patterns")
}
```

Implementer note: reuse however existing wiremock/mockito dev-tests construct a client (check `grep -rn "MockServer\|mockito" gmail-core/tests/ gmail-core/src/`). If none exist, mount a minimal token endpoint returning `{"access_token":"t","expires_in":3600}` and point OAuth config at the mock server.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p gmail-core --test attachment_bytes_test`
Expected: compile fail (methods missing).

- [ ] **Step 3: Implement in `client/messages.rs`**

```rust
    /// Get raw attachment bytes with a single output allocation.
    pub async fn get_attachment_bytes(
        &self,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<Bytes> {
        let response = self.execute_with_retry(
            self.http_client.get(self.api_url(&format!(
                "users/me/messages/{}/attachments/{}",
                message_id, attachment_id
            ))?),
        ).await?;

        let json: serde_json::Value = response.json().await?;
        let data = json
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GmailError::NotFound("Attachment data missing".into()))?;

        let estimated = data.len() * 3 / 4 + 3;
        let mut buf = bytes::BytesMut::zeroed(estimated);
        let written = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode_slice(data, &mut buf)
            .map_err(|e| GmailError::Internal(format!("attachment base64 decode: {e}")))?;
        buf.truncate(written);
        Ok(buf.freeze())
    }

    /// Stream an attachment straight to disk via fs_io (io_uring on Linux).
    pub async fn download_attachment_to(
        &self,
        message_id: &str,
        attachment_id: &str,
        path: &std::path::Path,
    ) -> Result<u64> {
        let bytes = self.get_attachment_bytes(message_id, attachment_id).await?;
        let len = bytes.len() as u64;
        crate::fs_io::write_file(path, bytes).await?;
        Ok(len)
    }
```

Add `use bytes::Bytes;` to imports. Keep existing `get_attachment` untouched.

- [ ] **Step 4: Wire CLI**

In `gmail-cli/src/commands/messages.rs` Attachment handler: when `-o/--output` given call `download_attachment_to` and print `"Saved N bytes to <path>"`; stdout-base64 branch switches to `get_attachment_bytes` → encode once for printing.

- [ ] **Step 5: Run tests**

Run: `cargo test --workspace`
Expected: all PASS.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(core): single-allocation attachment decode + streamed disk writes"
```

---

### Task 4: True streaming send/import via media-upload endpoints

**Files:**
- Modify: `gmail-core/src/client/messages.rs`, `gmail-core/src/client/mod.rs` (upload base URL const)
- Modify: `gmail-cli/src/commands/send.rs`, `gmail-cli/src/commands/import.rs`
- Test: `gmail-core/tests/streaming_upload_test.rs` (new)

**Interfaces:**
- Produces:
  - `GmailClient::send_mime_stream<S>(&self, stream: S, thread_id: Option<&str>) -> Result<Message>` where `S: futures::Stream<Item = std::io::Result<Bytes>> + Send + 'static` — POSTs `{upload_base}/users/me/messages/send?uploadType=media` with `Content-Type: message/rfc822`.
  - `GmailClient::import_stream<S>(&self, stream: S, deleted: bool) -> Result<Message>` — same shape, `/users/me/messages/import?uploadType=media&internalDateSource=dateHeader&deleted=<bool>`.
  - `pub fn mime_message_stream(to: &str, subject: &str, body: &str, attachments: Vec<StreamAttachment>, thread_id: Option<&str>) -> impl Stream<Item = io::Result<Bytes>>` where `pub struct StreamAttachment { pub path: PathBuf, pub filename: String, pub mime_type: String }`.
- Upload base: `Url::parse("https://gmail.googleapis.com/upload/gmail/v1/")`.

- [ ] **Step 1: Failing tests**

`gmail-core/tests/streaming_upload_test.rs` — mount capture endpoints and assert the assembled MIME arrives intact:

```rust
#[tokio::test]
async fn mime_stream_assembles_headers_body_and_attachments() {
    let server = MockServer::start().await;
    let expected_file = "streaming attachment content";
    Mock::given(method("POST"))
        .and(path_regex(r"users/me/messages/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id":"m1","threadId":"m1"})))
        .expect(1)
        .mount(&server)
        .await;

    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), expected_file).unwrap();

    let client = test_client(&server.uri()).await; // same helper pattern as Task 3
    let atts = vec![StreamAttachment {
        path: file.path().to_path_buf(),
        filename: "data.txt".into(),
        mime_type: "text/plain".into(),
    }];
    let msg = client.send_mime_stream(
        mime_message_stream("a@b.c", "Subj", "hello", atts, None),
        None,
    ).await.unwrap();
    assert_eq!(msg.id.as_deref(), Some("m1"));

    let recorded = &server.received_requests().await.unwrap()[0];
    let body = String::from_utf8(recorded.body.clone()).unwrap();
    assert!(body.starts_with("To: a@b.c\r\n"));
    assert!(body.contains("Content-Type: multipart/mixed"));
    assert!(body.contains("hello"));
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(extract_last_base64_segment(&body)).unwrap();
    assert_eq!(decoded, expected_file);
}

#[tokio::test]
async fn import_streams_rfc822_to_media_endpoint() {
    // Mount /upload/gmail/v1/users/me/messages/import, assert query params
    // contain uploadType=media & internalDateSource=dateHeader & deleted=true,
    // body equals file contents byte-for-byte (no MIME wrapping for import).
}
```

Helper `extract_last_base64_segment`: split body on `\r\n\r\n` boundaries, take last non-empty segment between final boundary markers. Implement inline in test file.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p gmail-core --test streaming_upload_test`
Expected: compile fail.

- [ ] **Step 3: Implement**

In `client/mod.rs`: add `upload_base_url: Url` field initialized from `https://gmail.googleapis.com/upload/gmail/v1/` (overridable alongside `base_url` for tests).

In `client/messages.rs` (or new `client/streaming.rs` impl block — prefer new file `gmail-core/src/client/streaming.rs`, registered in `client/mod.rs` as `mod streaming;` with `pub use streaming::*;`):

```rust
use std::io;
use std::path::PathBuf;

use bytes::Bytes;
use futures::stream::Stream;

pub struct StreamAttachment {
    pub path: PathBuf,
    pub filename: String,
    pub mime_type: String,
}

const CHUNK: usize = 192 * 1024; // multiple of 3 → clean base64 framing

/// Assemble a full RFC822 multipart message as a byte stream.
/// Files are read chunk-wise and base64-encoded incrementally.
pub fn mime_message_stream(
    to: &str,
    subject: &str,
    plain_body: &str,
    attachments: Vec<StreamAttachment>,
    thread_id: Option<&str>,
) -> impl Stream<Item = io::Result<Bytes>> {
    // Implementation: futures::stream::unfold over a state machine:
    //   State::Headers(Bytes queue) -> State::PerFile(idx, opened File)
    // Each frame yielded as Bytes. Per file: emit part headers frame, then loop:
    //   read CHUNK into Vec (AsyncReadExt::read), STANDARD.encode_slice into scratch,
    //   yield encoded frame; final partial chunk yields remaining padding chars.
    // Trailer frame: "--{boundary}--".
    // Boundary generated identically to existing send_with_attachments.
    todo!()
}

impl super::GmailClient {
    pub async fn send_mime_stream<S>(&self, stream: S, thread_id: Option<&str>) -> Result<Message>
    where S: Stream<Item = io::Result<Bytes>> + Send + 'static {
        let mut url = self.upload_base_url.join("users/me/messages/send")?;
        url.query_pairs_mut().append_pair("uploadType", "media");
        let response = self.execute_with_retry(
            self.http_client
                .post(url)
                .header(CONTENT_TYPE, HeaderValue::from_static("message/rfc822"))
                .body(reqwest::Body::wrap_stream(stream)),
        ).await?;
        Ok(response.json().await?)
    }

    pub async fn import_stream<S>(&self, stream: S, deleted: bool) -> Result<Message>
    where S: Stream<Item = io::Result<Bytes>> + Send + 'static {
        let mut url = self.upload_base_url.join("users/me/messages/import")?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("uploadType", "media");
            q.append_pair("internalDateSource", "dateHeader");
            q.append_pair("deleted", &deleted.to_string());
        }
        let response = self.execute_with_retry(
            self.http_client
                .post(url)
                .header(CONTENT_TYPE, HeaderValue::from_static("message/rfc822"))
                .body(reqwest::Body::wrap_stream(stream)),
        ).await?;
        Ok(response.json().await?)
    }
}
```

The `todo!()` above MUST be replaced with the real unfold state machine before commit — see test expectations for exact wire format. Required deps: `futures = { workspace = true }` (add `futures = "0.3"` to workspace + gmail-core).

CLI wiring:
- `commands/send.rs` SendAttach handler: build `Vec<StreamAttachment>` from arg paths (filename = basename, mime via existing `mime_guess`), call `mime_message_stream` + `send_mime_stream`. Remove in-memory `AttachmentData` reading.
- `commands/import.rs`: open `tokio::fs::File` → `tokio_util::io::ReaderStream` (add `tokio-util = "0.7"` workspace dep) → `import_stream(stream, args.deleted)`.

Keep legacy JSON-path methods for API/library users; CLI uses streaming paths exclusively.

- [ ] **Step 4: Run tests**

Run: `cargo test --workspace`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(core): true streaming send/import via media-upload endpoints; incremental base64 MIME assembly"
```

---

### Task 5: `gmail transport` command (after Task 1)

**Files:**
- Create: `gmail-cli/src/commands/transport.rs`
- Modify: `gmail-cli/src/main.rs` (enum variant + match arm)
- Test: extend `gmail-cli/tests/cli_test.rs`

**Interfaces:**
- Consumes: Task 1 `GmailClient::transport_info()`, `runtime_features()`.
- Produces: top-level subcommand `gmail transport`.

- [ ] **Step 1: Failing test** — add to `cli_test.rs` following existing `test_help_shows_all_commands` pattern:

```rust
#[tokio::test]
async fn help_lists_transport_command() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("--help").assert().success()
        .stdout(predicates::str::contains("transport").normalize());
}
```

- [ ] **Step 2: Verify failure**: `cargo test -p gmail-cli` → FAIL.

- [ ] **Step 3: Implement** — command prints, via `println!`:

```
negotiated_protocol: HTTP_3 | HTTP_2 | HTTP_11 | not-probed
http3_requested / http3_effective / fell_back: bool values
cpus / concurrency / pool_size from runtime_features + config
```

Match-arm calls the standard client-construction helper main.rs already uses, then `client.transport_info()`.

- [ ] **Step 4: Tests pass**: `cargo test --workspace`. Note: live invocation needs network — integration covers help/parsing only; protocol truth comes from Task 7.
- [ ] **Step 5: Commit**: `git commit -am "feat(cli): add transport introspection command"`

---

### Task 6: Honesty cleanup (fully independent — run parallel to Tasks 1–2)

**Files:**
- Modify: `Cargo.toml`, `gmail-core/Cargo.toml`, `gmail-cli/Cargo.toml`, `.gitignore`, `gmail-core/src/runtime.rs`, `gmail-core/src/client/mod.rs` (log line), `gmail-core/src/lib.rs` (doc comment), `gmail-skill-bindings/Cargo.toml` description if hype-y.

- [ ] **Step 1: Delete simd vapor** — remove feature entries (`simd = []` everywhere incl. default list), `RuntimeFeatures.simd` field + `has_simd()`, `simd={}` in client log line, any doc mentions. Build must pass: `cargo clippy --workspace --all-targets`.
- [ ] **Step 2: Truthful copy** — workspace description → `High-performance Gmail API client with verified HTTP/3 transport and streaming I/O`; keywords → `["gmail", "api", "http3", "streaming", "email"]`; gmail-cli description → `High-performance Gmail CLI with verified HTTP/3 transport and streaming I/O`.
- [ ] **Step 3: Commit Cargo.lock** — remove `Cargo.lock` line from `.gitignore`, `git add Cargo.lock`.
- [ ] **Step 4: Full check** — `cargo test --workspace && cargo clippy --workspace --all-targets` green.
- [ ] **Step 5: Commit**: `git commit -am "chore: truthful descriptions, drop simd vapor, track Cargo.lock"`

---

### Task 7: Live e2e verification (after Tasks 1–5)

**Files:**
- Create: `gmail-core/tests/live_e2e.rs` (new, fully `#[ignore]`-gated)

**Interfaces:** consumes everything above; requires valid `~/.gmail-opencode/token.json` + config.

- [ ] **Step 1: Implement** — single test `live_full_pipeline` gated on `std::env::var("LIVE_E2E") == Ok("1".into())` (early-return otherwise): load real config → build client (probe fires here) → assert `transport_info().fell_back == false || warn-only` → `search("in:inbox", 1)` → pick message with attachment via `get_message_metadata`, else skip attachment leg → `download_attachment_to(tmpfile)` asserts size > 0 → compose tiny RFC822 via `mime_message_stream` with no attachments → `import_stream` → trash imported message → report negotiated version via `println!`.
- [ ] **Step 2: Verify offline safety**: `cargo test --workspace` passes (ignored test skipped).
- [ ] **Step 3: Live run**: `LIVE_E2E=1 cargo test -p gmail-core --test live_e2e -- --ignored --nocapture` — record negotiated protocol output in task report. If h3 falls back, investigate quinn/Gmail compatibility and document finding in report (fallback working correctly is still acceptance-passing; silent claims are not).
- [ ] **Step 4: Commit**: `git commit -am "test: live e2e pipeline behind LIVE_E2E=1"`

---

### Task 8: Final verification sweep

- [ ] `cargo fmt --all -- --check` (add rustfmt defaults if repo lacks config)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace` (all green)
- [ ] `LIVE_E2E=1 cargo test -- --ignored` (if credentials available)
- [ ] Grep sweep: `grep -rn "zero-copy\|zero_copy\|simd" --include='*.rs' --include='*.toml' .` returns nothing inaccurate
- [ ] Update spec checkboxes/status note in `docs/superpowers/specs/2026-08-23-performance-claims-design.md` header → `Status: Implemented`
- [ ] Final commit + push-ready summary listing negotiated protocol observed live.
