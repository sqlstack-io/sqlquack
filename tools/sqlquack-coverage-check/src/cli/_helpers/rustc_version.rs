//! `rustc_version`: the compiler rust-toolchain.toml selects, as `rustc --version` names it.

use std::process::{Command, Output};

/// `rustc --version` where the guard runs, which rustup resolves through rust-toolchain.toml.
pub(crate) fn rustc_version() -> Result<String, String> {
    let output: Output = Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|error| format!("cannot run rustc --version: {error}"))?;
    match output.status.success() {
        true => Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned()),
        false => Err(format!(
            "rustc --version failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}
