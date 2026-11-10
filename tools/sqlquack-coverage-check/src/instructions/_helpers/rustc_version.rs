//! `rustc_version`: the compiler the repository's toolchain file selects, as `rustc --version` names it.

use std::process::{Command, Output};

/// `rustc --version`, run where the guard runs, so rustup picks rust-toolchain.toml's compiler: the
/// one the gate build that runs the guard was built with.
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
