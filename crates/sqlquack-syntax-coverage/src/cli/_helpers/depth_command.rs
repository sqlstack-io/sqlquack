//! `depth-probe`: every native parser's deep-nesting failures against the allowlist.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use serde_json::json;

use crate::cli::_helpers::commands::worker_command;
use crate::cli::_helpers::output::{print, say};
use crate::cli::_helpers::worker::Worker;
use crate::depth::constants::{PROBE_BOUND, PROBE_STACK_BYTES, PROBED_PARSERS};
use crate::depth::main::compare_allowlist::compare_allowlist;
use crate::depth::main::probe_construct::probe_construct;
use crate::depth::main::probed_constructs::probed_constructs;
use crate::depth::models::{Allowlist, Construct, ConstructResult, DepthReport};
use crate::formats::models::LocalParser;

fn read_allowlist(path: &Path) -> Result<Allowlist, ExitCode> {
    std::fs::read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|text| toml::from_str(&text).map_err(|error| error.to_string()))
        .map_err(|problem| {
            say(&format!("FAILED, {}: {problem}", path.display()));
            ExitCode::FAILURE
        })
}

/// Every construct of every probed parser, probed by `jobs` threads each with its own worker.
fn probe_all(binary: &Path, jobs: usize) -> Vec<ConstructResult> {
    let mut tasks: Vec<(LocalParser, Construct)> = Vec::new();
    for parser in PROBED_PARSERS {
        for construct in probed_constructs(parser) {
            tasks.push((parser, construct));
        }
    }
    let next: AtomicUsize = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<ConstructResult>>> = Mutex::new(vec![None; tasks.len()]);
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let index: usize = next.fetch_add(1, Ordering::SeqCst);
                    let Some(&(parser, construct)) = tasks.get(index) else {
                        break;
                    };
                    let mut worker: Worker = Worker::new(
                        worker_command(binary.to_path_buf(), parser, Some(PROBE_STACK_BYTES)),
                        PROBE_BOUND,
                    );
                    let result: ConstructResult =
                        probe_construct(parser, &construct, &mut |sql| worker.verdict(sql));
                    if let Ok(mut all) = results.lock()
                        && let Some(slot) = all.get_mut(index)
                    {
                        *slot = Some(result);
                    }
                }
            });
        }
    });
    results
        .into_inner()
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect()
}

fn result_line(result: &ConstructResult) -> String {
    let outcome: String = match result.failed_at {
        Some((depth, failure)) => format!("{failure:?} at depth {depth}").to_lowercase(),
        None => "clean at every depth".to_owned(),
    };
    format!(
        "{} {}: {outcome}",
        format!("{:?}", result.parser).to_lowercase(),
        result.construct
    )
}

pub(crate) fn depth_probe(
    profile: &str,
    allowlist: &Path,
    jobs: usize,
    json: bool,
) -> Result<ExitCode, ExitCode> {
    let known: Allowlist = read_allowlist(allowlist)?;
    let binary: PathBuf = std::env::current_exe().map_err(|problem| {
        say(&format!(
            "FAILED, cannot find this binary for the parse worker: {problem}"
        ));
        ExitCode::FAILURE
    })?;
    let started: Instant = Instant::now();
    let results: Vec<ConstructResult> = probe_all(&binary, jobs);
    let mut report: DepthReport = compare_allowlist(&results, &known, profile);
    let expected: usize = PROBED_PARSERS
        .iter()
        .map(|&parser| probed_constructs(parser).len())
        .sum();
    if results.len() != expected {
        report.problems.push(format!(
            "{} of {expected} constructs gave no result; a probe thread failed",
            expected - results.len().min(expected)
        ));
    }
    let failing: usize = results
        .iter()
        .filter(|result| result.failed_at.is_some())
        .count();
    let mut text: Vec<String> = results.iter().map(result_line).collect();
    text.extend(
        report
            .unlisted
            .iter()
            .map(|line| format!("UNLISTED {line}")),
    );
    text.extend(report.stale.iter().map(|line| format!("STALE {line}")));
    text.extend(report.problems.iter().map(|line| format!("PROBLEM {line}")));
    print(
        json,
        &json!({ "profile": profile, "results": results, "report": report }),
        &text,
    );
    say(&format!(
        "probed {} constructs in profile {profile} in {:.1} s: {failing} fail, {} unlisted, {} stale, {} problems",
        results.len(),
        started.elapsed().as_secs_f64(),
        report.unlisted.len(),
        report.stale.len(),
        report.problems.len()
    ));
    match report == DepthReport::default() {
        true => Ok(ExitCode::SUCCESS),
        false => Err(ExitCode::FAILURE),
    }
}
