use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rerun-if-changed=build.rs");

    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let Ok(output) = Command::new(rustc).arg("--version").output() else {
        return;
    };
    if !output.status.success() {
        return;
    }

    let version = String::from_utf8_lossy(&output.stdout);
    let Some(toolchain) = version.split_whitespace().nth(1) else {
        return;
    };
    if !toolchain.contains("-nightly") {
        panic!(
            "grr requires Rust nightly: HTTP/3 (rustls + quinn) needs reqwest's unstable `http3` feature, which requires `-Z`-era nightly APIs. Install it with `rustup toolchain install nightly`."
        );
    }
}
