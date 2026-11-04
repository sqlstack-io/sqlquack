//! `run_cli`: the whole command.

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use serde_json::json;

use crate::check::main::check_coverage::check_coverage;
use crate::check::models::{CheckResult, FormResult};
use crate::cli::_helpers::local::{parser_tree, snowflake_verdict};
use crate::cli::_helpers::output::print;
use crate::cli::models::{Arguments, Command, Common};
use crate::formats::main::load_coverage::load_coverage;
use crate::formats::models::SyntaxCoverage;
use crate::report::main::summarize::{summarize, tally_by};

fn load(root: &Path) -> Result<SyntaxCoverage, ExitCode> {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "syntax-coverage: loading {}", root.display());
    load_coverage(root).map_err(|problems| {
        for problem in &problems {
            let _ = writeln!(stderr, "syntax-coverage: {problem}");
        }
        let _ = writeln!(
            stderr,
            "syntax-coverage: FAILED, {} problems",
            problems.len()
        );
        ExitCode::FAILURE
    })
}

fn checked(root: &Path) -> Result<CheckResult, ExitCode> {
    let coverage: SyntaxCoverage = load(root)?;
    let mut stderr = std::io::stderr().lock();
    let tree: Option<String> = parser_tree();
    let _ = writeln!(
        stderr,
        "syntax-coverage: checking against parser tree {}",
        tree.as_deref()
            .unwrap_or("(uncommitted: every review is stale)")
    );
    let check: CheckResult = check_coverage(&coverage, tree.as_deref(), &snowflake_verdict);
    for problem in &check.problems {
        let _ = writeln!(stderr, "syntax-coverage: {problem}");
    }
    let _ = writeln!(
        stderr,
        "syntax-coverage: checked {} forms, {} problems",
        check.forms.len(),
        check.problems.len()
    );
    Ok(check)
}

fn exit(check: &CheckResult) -> ExitCode {
    match check.problems.is_empty() {
        true => ExitCode::SUCCESS,
        false => ExitCode::FAILURE,
    }
}

fn in_family(form: &FormResult, family: &Option<String>) -> bool {
    family.as_ref().is_none_or(|family| &form.family == family)
}

fn run(command: Command) -> Result<ExitCode, ExitCode> {
    match command {
        Command::Validate { common } => {
            let coverage: SyntaxCoverage = load(&common.root)?;
            let forms: usize = coverage.catalogue.values().map(Vec::len).sum();
            let counts = json!({"forms": forms, "receipts": coverage.receipts.len()});
            print(
                common.json,
                &counts,
                &[format!(
                    "valid, {forms} forms, {} receipts",
                    coverage.receipts.len()
                )],
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Summary { common } => summary(&common),
        Command::Family { family, common } => {
            let check: CheckResult = checked(&common.root)?;
            let forms: Vec<&FormResult> = check
                .forms
                .iter()
                .filter(|form| form.family == family)
                .collect();
            let text: Vec<String> = forms
                .iter()
                .map(|form| {
                    format!(
                        "{} {:?} {}/{} {:?}",
                        form.id,
                        form.state,
                        form.satisfied_obligations,
                        form.required_obligations,
                        form.reasons
                    )
                })
                .collect();
            print(common.json, &forms, &text);
            Ok(exit(&check))
        }
        Command::Missing { family, common } => {
            let check: CheckResult = checked(&common.root)?;
            let missing: Vec<_> = check
                .forms
                .iter()
                .filter(|form| in_family(form, &family))
                .flat_map(|form| {
                    form.obligations
                        .iter()
                        .filter(|o| o.required && o.reason.is_some())
                })
                .map(|obligation| json!({"obligation": obligation.id, "reason": obligation.reason}))
                .collect();
            let text: Vec<String> = missing.iter().map(|row| row.to_string()).collect();
            print(common.json, &missing, &text);
            Ok(exit(&check))
        }
        Command::Pages { family, common } => {
            let check: CheckResult = checked(&common.root)?;
            let filtered = CheckResult {
                forms: check
                    .forms
                    .iter()
                    .filter(|form| in_family(form, &family))
                    .cloned()
                    .collect(),
                ..check.clone()
            };
            let pages = tally_by(&filtered, |form| &form.source);
            let text: Vec<String> = pages
                .iter()
                .map(|(page, tally)| {
                    format!("{page} {}/{} forms verified", tally.verified, tally.forms)
                })
                .collect();
            print(common.json, &pages, &text);
            Ok(exit(&check))
        }
    }
}

fn summary(common: &Common) -> Result<ExitCode, ExitCode> {
    let check: CheckResult = checked(&common.root)?;
    let summary = summarize(&check);
    let total = &summary.total;
    let mut text: Vec<String> = vec![format!(
        "{} of {} forms verified ({}%), {} excluded; obligations {}/{}",
        total.verified,
        total.forms,
        summary.verified_percent,
        total.excluded,
        total.satisfied_obligations,
        total.required_obligations
    )];
    text.extend(
        summary
            .forms_by_reason
            .iter()
            .map(|(reason, count)| format!("  {reason:?}: {count} forms")),
    );
    print(common.json, &summary, &text);
    Ok(exit(&check))
}

/// Runs one subcommand; progress and problems go to stderr, results to stdout.
pub fn run_cli() -> ExitCode {
    let arguments: Arguments = Arguments::parse();
    run(arguments.command).unwrap_or_else(|code| code)
}
