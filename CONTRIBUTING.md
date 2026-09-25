## Contributing to grr

Thank you for wanting to contribute! Please read through the following guidelines to make the process smooth.

### Development Setup

1. **Install Rust**: the default CLI build requires nightly. [`rust-toolchain.toml`](./rust-toolchain.toml) pins it and supplies the components the build needs. The default `http3` feature needs the `--cfg reqwest_unstable` flag, which [`.cargo/config.toml`](./.cargo/config.toml) sets for in-repo builds. The library can build on stable Rust with `default-features = false` (HTTP/2); the CLI enables HTTP/3, so it needs nightly.
2. **Clone the repo**: `git clone https://github.com/debanjanbasu/grr-cli.git`
3. **Config**: create `~/.grr/config.toml` with your OAuth client ID (see [`config.toml.example`](./config.toml.example); full Google Cloud walkthrough in [`docs/gcp-setup.md`](./docs/gcp-setup.md))
4. **Auth**: run `cargo run -- auth login` to complete Google consent
5. **Build**: `cargo build`
6. **Test**: `cargo test --locked`

### Adding New Features

- This is one package, `grr-cli`, with the library target `grr_cli` and the `grr` binary. Match the existing layout: CLI surface in `src/commands/` (one module per command group); service clients and models in `src/<service>/` (for example, `src/gmail/client/`); shared transport, auth, and config in `src/core/` (`http.rs`, `auth/`, `config.rs`).
- Add new service code under `src/<service>/` and gate the service module with its matching Cargo feature (`gmail`, `calendar`, `drive`, `people`, `chat`, or `forms`) when the service is optional. The `cli` feature enables the complete command surface and the binary.
- Keep the zero-config philosophy: new behavior should need no new config knobs unless there is no alternative
- Each change needs a clear brief, tests, and self-review
- Run `cargo clippy --all-targets -- -D warnings` before committing — must be clean
- Run `cargo fmt --all --check` to maintain formatting consistency
- Ensure `cargo test --locked` passes (green) before committing

### Code Style

- **Clippy**: `cargo clippy --all-targets -- -D warnings` must pass; `-D warnings` is the active lint enforcement (there is no workspace lints table to inherit)
- **Fmt**: `cargo fmt --all --check` must pass
- **Doc comments**: Public API functions must have `///` doc comments
- **No `expect`/`unwrap`/`panic!`**: avoid these in library and CLI code — degrade gracefully instead (tests use the `unwrap_or_else` pattern, e.g. `figment.extract::<T>().unwrap_or_else(|_| T::default())`). The root package has no workspace lints table; `-D warnings` in the Clippy command is the enforcement.

### Submitting Changes

1. Commit with a clear message: `git commit -m "feat: <descriptive title>"`
2. Push to your fork
3. Open a Pull Request against `main` branch
4. PR must pass `cargo clippy --all-targets -- -D warnings` and `cargo test --locked`
5. Include test coverage for any new functionality

### Reporting Issues

- Use the GitHub issue tracker
- Include `cargo metadata --no-deps --format-version 1` output if build issues
- Specify OS, Rust version, and `grr` version
