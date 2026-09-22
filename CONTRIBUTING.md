## Contributing to grr

Thank you for wanting to contribute! Please read through the following guidelines to make the process smooth.

### Development Setup

1. **Install Rust**: nightly is required and pinned by [`rust-toolchain.toml`](./rust-toolchain.toml) — `rustup default nightly`, then `rustup component add rust-src` (the build-std config in [`.cargo/config.toml`](./.cargo/config.toml) needs it and sets the `reqwest_unstable` cfg required by the `http3` feature). `grr-core` alone also builds on stable Rust.
2. **Clone the repo**: `git clone https://github.com/debanjanbasu/grr-cli.git`
3. **Config**: create `~/.grr/config.toml` with your OAuth client ID (see [`config.toml.example`](./config.toml.example); full Google Cloud walkthrough in [`docs/gcp-setup.md`](./docs/gcp-setup.md))
4. **Auth**: run `cargo run -p grr -- auth login` to complete Google consent
5. **Build**: `cargo build --workspace`
6. **Test**: `cargo test --workspace`

### Adding New Features

- Match the existing layout: CLI surface in `grr/src/commands/` (one module per command group), API client surface in `grr-core/src/client/`
- Keep the zero-config philosophy: new behavior should need no new config knobs unless there is no alternative
- Each change needs a clear brief, tests, and self-review
- Run `cargo clippy --workspace --all-targets -D warnings` before committing — must be clean
- Run `cargo fmt` to maintain formatting consistency
- Ensure `cargo test --workspace` passes (green) before committing

### Code Style

- **Clippy**: `cargo clippy --workspace --all-targets -D warnings` must pass
- **Fmt**: `cargo fmt --check` must pass
- **Doc comments**: Public API functions must have `///` doc comments
- **No `expect`/`unwrap`/`panic!`**: Workspace clippy bans these even in tests (use `unwrap_or_else` / `figment.extract::<T>().unwrap_or_else(|_| T::default())` pattern)

### Submitting Changes

1. Commit with a clear message: `git commit -m "feat: <descriptive title>"`
2. Push to your fork
3. Open a Pull Request against `main` branch
4. PR must pass `cargo clippy --workspace --all-targets -D warnings` and `cargo test --workspace`
5. Include test coverage for any new functionality

### Reporting Issues

- Use the GitHub issue tracker
- Include `cargo metadata --no-deps --format-version 1` output if build issues
- Specify OS, Rust version, and `grr` version
