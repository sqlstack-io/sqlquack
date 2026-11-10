//! `count_instructions`: one workload under callgrind, counted only inside its measured function.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::instructions::_helpers::own_instructions::own_instructions;
use crate::instructions::constants::{ANALYSIS_FUNCTION, PARSE_FUNCTION};
use crate::instructions::models::Workload;

/// The function whose instructions `workload` counts.
fn measured_function(workload: Workload) -> &'static str {
    match workload {
        Workload::Parse => PARSE_FUNCTION,
        Workload::Analysis => ANALYSIS_FUNCTION,
    }
}

/// The instructions `binary`'s own code runs in `workload`'s measured function, under callgrind.
pub fn count_instructions(binary: &Path, workload: Workload) -> Result<u64, String> {
    let profile: PathBuf = std::env::temp_dir().join(format!(
        "syntax-coverage-{}-{}.callgrind",
        std::process::id(),
        workload.name()
    ));
    let output: Output = Command::new("valgrind")
        .arg("--tool=callgrind")
        .arg("--collect-atstart=no")
        .arg(format!("--toggle-collect={}", measured_function(workload)))
        .arg(format!("--callgrind-out-file={}", profile.display()))
        .arg(binary)
        .args(["instruction-workload", "--workload", workload.name()])
        .output()
        .map_err(|error| {
            format!("cannot run valgrind ({error}); install it, as CI does with apt-get install valgrind")
        })?;
    let text: Result<String, std::io::Error> = std::fs::read_to_string(&profile);
    let _ = std::fs::remove_file(&profile);
    if !output.status.success() {
        return Err(format!(
            "{} workload failed under callgrind: {}",
            workload.name(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let name: &str = binary
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let count: u64 = own_instructions(&text.map_err(|error| error.to_string())?, name)?;
    match count {
        0 => Err(format!(
            "{} workload: callgrind counted nothing in {}; is it still a function of its own?",
            workload.name(),
            measured_function(workload)
        )),
        count => Ok(count),
    }
}
