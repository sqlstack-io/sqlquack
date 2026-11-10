//! `instruction-counts`: the parse and analysis workloads' instruction counts against the baseline.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::cli::_helpers::output::say;
use crate::instructions::constants::BASELINE_HEADER;
use crate::instructions::main::compare_counts::compare_counts;
use crate::instructions::main::count_instructions::count_instructions;
use crate::instructions::main::run_workload::run_workload;
use crate::instructions::models::{InstructionBaseline, Workload};

fn failed(problem: &str) -> ExitCode {
    say(&format!("FAILED, {problem}"));
    ExitCode::FAILURE
}

fn read_baseline(path: &Path) -> Result<InstructionBaseline, ExitCode> {
    std::fs::read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|text| toml::from_str(&text).map_err(|error| error.to_string()))
        .map_err(|problem| failed(&format!("{}: {problem}", path.display())))
}

/// `baseline` with `measured` as its counts, written back with its explanatory header.
fn write_baseline(
    path: &Path,
    baseline: InstructionBaseline,
    measured: BTreeMap<String, u64>,
) -> Result<ExitCode, ExitCode> {
    let updated: InstructionBaseline = InstructionBaseline {
        arch: std::env::consts::ARCH.to_owned(),
        counts: measured,
        ..baseline
    };
    let counts: String = updated
        .counts
        .iter()
        .map(|(name, count)| format!("{name} = {count}\n"))
        .collect();
    let body: String = format!(
        "arch = \"{}\"\ntolerance_percent = {}\n\n[counts]\n{counts}",
        updated.arch, updated.tolerance_percent
    );
    std::fs::write(path, format!("{BASELINE_HEADER}{body}"))
        .map_err(|error| failed(&format!("{}: {error}", path.display())))?;
    say(&format!("recorded the counts in {}", path.display()));
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn instruction_counts(baseline_path: &Path, update: bool) -> Result<ExitCode, ExitCode> {
    let baseline: InstructionBaseline = read_baseline(baseline_path)?;
    let binary: PathBuf = std::env::current_exe().map_err(|error| {
        failed(&format!(
            "cannot find this binary for the workload: {error}"
        ))
    })?;
    let mut measured: BTreeMap<String, u64> = BTreeMap::new();
    for workload in Workload::ALL {
        let count: u64 =
            count_instructions(&binary, workload).map_err(|problem| failed(&problem))?;
        say(&format!(
            "{}: {count} instructions (baseline {})",
            workload.name(),
            baseline
                .counts
                .get(workload.name())
                .map_or_else(|| "none".to_owned(), u64::to_string)
        ));
        measured.insert(workload.name().to_owned(), count);
    }
    if update {
        return write_baseline(baseline_path, baseline, measured);
    }
    let problems: Vec<String> = compare_counts(&baseline, &measured, std::env::consts::ARCH);
    for problem in &problems {
        say(&format!("PROBLEM {problem}"));
    }
    match problems.is_empty() {
        true => Ok(ExitCode::SUCCESS),
        false => Err(ExitCode::FAILURE),
    }
}

pub(crate) fn instruction_workload(workload: Workload) -> Result<ExitCode, ExitCode> {
    run_workload(workload).map_err(|problem| failed(&problem))?;
    Ok(ExitCode::SUCCESS)
}
