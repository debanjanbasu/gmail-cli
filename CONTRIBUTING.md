## Contributing to gmail-cli

Thank you for wanting to contribute! Please read through the following guidelines to make the process smooth.

### Development Setup

1. **Install Rust**: `rustup default nightly` (edition 2024; the `http3` feature requires nightly and the `--cfg reqwest_unstable` flags, which [`.cargo/config.toml`](./.cargo/config.toml) sets for you — also run `rustup component add rust-src`)
2. **Clone the repo**: `git clone https://github.com/debanjanbasu/gmail-cli.git`
3. **Config**: Create `~/.gmail-opencode/config.toml` with your OAuth credentials (see `config.toml.example`)
4. **Auth**: Run `cargo run -p gmail-cli -- auth login` to complete Google consent
5. **Build**: `cargo build --workspace`
6. **Test**: `cargo test --workspace`

### Adding New Features

- Follow the existing task plan in `.superpowers/sdd/` for structured implementation
- Each task should have a clear brief, tests, and self-review
- Run `cargo clippy --workspace --all-targets -D warnings` before committing — must be clean
- Run `cargo fmt` to maintain formatting consistency
- Ensure `cargo test --workspace` passes (green) before committing

### Code Style

- **Clippy**: `cargo clippy --workspace --all-targets -D warnings` must pass
- **Fmt**: `cargo fmt --check` must pass (no unresolved clues)
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
- Specify OS, Rust version, and `gmail-cli` version