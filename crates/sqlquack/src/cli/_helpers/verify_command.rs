//! `sqlquack verify`.

use std::io::Write;
use std::process::ExitCode;

use sqlquack_evidence::verify::main::verify_cases::verify_cases;
use sqlquack_evidence::verify::models::{Verdict, VerifyReport};

use crate::analysis::models::PipelineConverter;
use crate::cli::_helpers::evidence::load_evidence;
use crate::cli::models::VerifyArguments;

pub(crate) fn verify_command(
    arguments: &VerifyArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(
        stderr,
        "sqlquack verify: loading cases and replaying recorded ones on DuckDB…"
    );
    let report = match verify(arguments) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack verify: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rendered = if arguments.json {
        serde_json::to_string_pretty(&report).unwrap_or_default() + "\n"
    } else {
        render(&report)
    };
    let _ = stdout.write_all(rendered.as_bytes());
    let failed = count(&report, |verdict| matches!(verdict, Verdict::Failed { .. }));
    let _ = writeln!(
        stderr,
        "sqlquack verify: {}, {} cases: {} passed, {} refused, {} pending, {failed} failed",
        if failed == 0 { "done" } else { "FAILED" },
        report.cases.len(),
        count(&report, |verdict| *verdict == Verdict::Passed),
        count(&report, |verdict| matches!(
            verdict,
            Verdict::Refused { .. }
        )),
        count(&report, |verdict| *verdict == Verdict::Pending),
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn verify(arguments: &VerifyArguments) -> Result<VerifyReport, String> {
    let evidence = load_evidence(&arguments.paths)?;
    verify_cases(
        &evidence.cases,
        &arguments.paths.results,
        &evidence.policy,
        &PipelineConverter,
    )
    .map_err(|error| error.to_string())
}

fn count(report: &VerifyReport, predicate: impl Fn(&Verdict) -> bool) -> usize {
    report
        .cases
        .iter()
        .filter(|case| predicate(&case.verdict))
        .count()
}

fn render(report: &VerifyReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    for case in &report.cases {
        let detail = match &case.verdict {
            Verdict::Pending => "pending: no recorded result yet".to_owned(),
            Verdict::Passed => "passed".to_owned(),
            Verdict::Refused { reasons } => format!(
                "refused: {} reasons, first: {}",
                reasons.len(),
                reasons.first().map_or("", String::as_str)
            ),
            Verdict::Failed { reason } => format!("FAILED: {reason}"),
        };
        lines.push(format!("{}  {detail}", case.reference));
    }
    for hash in &report.unreferenced_results {
        lines.push(format!(
            "note: result {hash} is no longer named by any case"
        ));
    }
    lines.join("\n") + "\n"
}
