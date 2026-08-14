# Cleanup & Optimization Design

Date: 2026-08-15
Status: Approved (with model-set and build.rs decisions)

## Goal

Make the gmail-opencode-rust workspace warning-free, dependency-lean, and structurally maintainable, and get the NAPI bindings actually working. No API behavior changes — the public `GmailClient` interface and CLI subcommand surface stay identical; all 26 existing tests must pass after each phase.

## Current state (verified)

- Workspace builds with 0 errors; ~15 warnings across gmail-core, gmail-cli, gmail-skill-bindings
- 26 tests pass (11 CLI + 9 core + 6 doc? — verified: 11 CLI + 11 core)
- `client.rs` = 918 lines, 8 API domains in one file
- `models.rs` = 492 lines, two parallel model sets (raw API models + Info types)
- `auth.rs` = 387 lines (token storage + OAuth client + callback server + builder)
- `gmail-skill-bindings/src/lib.rs` = 490 lines with **zero** `#[napi]` attributes — plain Rust async functions
- Dead deps declared but never referenced in source: `simd-json`, `smallvec`, `ahash`, `dashmap`, `parking_lot`, `bytes`, `tokio-util`, `oauth2`, `metrics`, `metrics-exporter-prometheus`, `rkyv` (only a dead error variant), `quinn`, `h3`, `tokio-rustls`, `axum`, `hyper`, `tower`
- `zstd`/`brotli` direct crates referenced **only** by `build.rs` (dictionary training); payload compression is handled by reqwest's own `zstd`/`brotli` features (`.zstd()`/`.brotli()` builder calls in client.rs are reqwest methods for `Accept-Encoding` decompression)

## Phases

### Phase 1 — Warning-free workspace

- Run `cargo clippy --fix` + manual fixes: 5 redundant closures (auth.rs), unused imports (CLI command files), unused var (watch.rs), empty doc line, "bound defined in more than one place"
- Fix Cargo.toml warnings: unrecognized lint tool `lints.unsafe_code`, `panic` setting ignored for test/bench profiles
- Gate: `cargo clippy --workspace` → zero warnings; `cargo test --workspace` → all pass

### Phase 2 — Dependency trim

Remove from workspace + crate Cargo.tomls (verified unreferenced in source):
- gmail-core: `simd-json`, `smallvec`, `ahash`, `dashmap`, `parking_lot`, `bytes`, `tokio-util`, `oauth2`, `metrics`, `metrics-exporter-prometheus`, `quinn`, `h3`, `tokio-rustls`, `axum`, `hyper`, `tower`, `rkyv` (+ `Rkyv` error variant in error.rs, + "zero-copy" doc claims in lib.rs)
- Remove `zstd`/`brotli` **direct crates** + workspace deps (including `fat-lto`/`zdict_builder`/`zstdmt` features — all dictionary-training-only). Payload zstd/brotli compression is preserved via reqwest's `zstd`/`brotli` features, which handle request/response decompression transparently. Config flags `enable_zstd`/`enable_brotli` remain and keep calling reqwest's `.zstd()`/`.brotli()`.
- Keep: `reqwest` (http3/zstd/brotli features), `tokio`, `serde`, `serde_json`, `url`, `ring`, `rustls`, `webpki-roots`, `clap`, `dirs`, `toml`, `figment`, `tracing`, `futures`, `anyhow`, `thiserror`
- Remove `gmail-core/build.rs` (zstd dictionary training — trains from zero samples, emits 4 warnings every build) and the `assets/dict/` directory it writes
- Trim `reqwest` features to what source actually uses
- Gate: build + clippy clean, tests pass

### Phase 3 — Structural refactor

Split `client.rs` → `gmail-core/src/client/` module:
- `mod.rs` — GmailClient struct, builder, new(), api_url(), execute_with_retry(), search()
- `messages.rs` — message CRUD, trash/untrash/delete, batch, attachments, send, import
- `threads.rs` — thread CRUD, modify/trash/untrash/delete
- `drafts.rs` — draft CRUD + send
- `labels.rs` — label CRUD + modify + batch
- `settings.rs` — send-as CRUD
- `history.rs` — history, profile
- `watch.rs` — watch/stop-watch
Each file is an `impl GmailClient` block — public interface unchanged.

Split `auth.rs` → `gmail-core/src/auth/` module:
- `mod.rs` — GmailAuth struct, re-exports
- `oauth.rs` — OAuth flow, token exchange, PKCE
- `token.rs` — TokenStorage, token persistence
- `server.rs` — local callback server

Resolve model duplication (approved: delete Info types):
- Delete `EmailMessage`, `ThreadMessage`, `AttachmentInfo`, `DraftInfo`, `LabelInfo`, `HistoryRecord`, `SendAsInfo`
- Make CLI consume raw models (`Message`, `Thread`, etc.) directly — adjust CLI output code
- Delete `Rkyv` error variant, "zero-copy" claims in lib.rs/models.rs docs

Gate: build + clippy clean, tests pass, CLI help output identical

### Phase 4 — NAPI bindings

- Rewire `gmail-skill-bindings/src/lib.rs` with proper `#[napi]` macros (napi 3.x):
  - `#[napi]` on async fns (napi supports async via `#[napi]` on async functions)
  - NAPI-serializable structs via `#[napi(object)]`
  - Keep the existing plain-fn conversion layer where it helps, but expose real napi surface
- Verify: `cargo build -p gmail-skill-bindings` succeeds; `cargo clippy -p gmail-skill-bindings` clean

## Non-goals

- No new features, no API renames
- No change to config format or env-var mapping
- No change to OAuth flow

## Risks

- CLI output rendering differs slightly after Info-type deletion → mitigate by keeping field names identical in raw models (they already carry the same fields)
- napi 3.x async signature differences → mitigate by checking napi docs during Phase 4
