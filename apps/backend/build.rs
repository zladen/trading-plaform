//! Build script.
//!
//! Records the compiler version in the binary so that `/api/v1/info` can report
//! exactly which toolchain produced a given build.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RUSTC");

    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| String::from("rustc"));

    let Ok(output) = Command::new(rustc).arg("--version").output() else {
        return;
    };

    if !output.status.success() {
        return;
    }

    let rendered = String::from_utf8_lossy(&output.stdout);
    if let Some(version) = rendered.split_whitespace().nth(1) {
        println!("cargo:rustc-env=PLATFORM_RUST_VERSION={version}");
    }
}
