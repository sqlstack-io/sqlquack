//! `syntax-coverage results`: the result-matching measure, checked against its baseline.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::cli::_helpers::commands::snowflake_only;
use crate::cli::_helpers::output::{print, say};
use crate::cli::models::Common;
use crate::formats::main::load_coverage::load_coverage;
use crate::results::constants::{BASELINE_FILE, PROBLEMS_PRINTED, WORK_PRINTED};
use crate::results::main::diff_results::diff_results;
use crate::results::main::load_results_files::load_results_files;
use crate::results::main::measure_results::measure_results;
use crate::results::main::results_baseline::results_baseline;
use crate::results::models::{
    FormTally, ResultsBaseline, ResultsDiff, ResultsReport, StatementsFile,
};
use crate::translation::constants::TRANSLATION_DIR;

/// Percentages print with one decimal place.
const PER_MILLE: usize = 1000;
/// Tenths of a percent in one percent.
const TENTHS: usize = 10;

/// Prints each problem, up to a bound, and fails.
fn fail(problems: &[String]) -> ExitCode {
    for problem in problems.iter().take(PROBLEMS_PRINTED) {
        say(problem);
    }
    if problems.len() > PROBLEMS_PRINTED {
        say(&format!("… {} more", problems.len() - PROBLEMS_PRINTED));
    }
    say(&format!("FAILED, {} problems", problems.len()));
    ExitCode::FAILURE
}

/// `part` of `whole` as a percentage with one decimal, rounded half up.
fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        return "no base".to_owned();
    }
    let tenths = (part * PER_MILLE + whole / 2) / whole;
    format!("{}.{}%", tenths / TENTHS, tenths % TENTHS)
}

fn tally_line(name: &str, tally: &FormTally) -> String {
    format!(
        "{name}: {} of {} translatable forms match ({}); {} syntax only, {} refused, {} with unmet \
         obligations, {} with none declared, {} differ, {} unmeasured; {} of {} catalogued forms \
         not translatable",
        tally.matching,
        tally.translatable,
        percent(tally.matching, tally.translatable),
        tally.syntax_only,
        tally.refused,
        tally.unmet_obligations,
        tally.no_obligations,
        tally.differs,
        tally.unmeasured,
        tally.not_translatable,
        tally.catalogued,
    )
}

fn report_lines(report: &ResultsReport) -> Vec<String> {
    let statements = &report.statements;
    let mut lines: Vec<String> = vec![
        format!(
            "forms: {} of {} translatable catalogue forms translate to DuckDB with results matching \
             Snowflake ({}); {} syntax only, {} refused, {} differ, {} with no recorded statement",
            report.forms.matching,
            report.forms.translatable,
            percent(report.forms.matching, report.forms.translatable),
            report.forms.syntax_only,
            report.forms.refused,
            report.forms.differs,
            report.forms.unmeasured,
        ),
        format!(
            "statements (our recorded Snowflake statements, not a sample of real-world SQL): {} of \
             {} return rows matching Snowflake ({}); {} more raise the same kind of error, or are \
             refused by both sides ({}); {} link to no form",
            statements.matching_rows,
            statements.recorded,
            percent(statements.matching_rows, statements.recorded),
            statements.matching_errors,
            percent(statements.matching_errors, statements.recorded),
            statements.unlinked,
        ),
        format!(
            "statements by outcome: {}",
            statements
                .by_outcome
                .iter()
                .map(|(outcome, count)| format!("{outcome:?} {count}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        format!(
            "obligations: {} declared, {} met, {} unmet, {} held, {} unmeasured; a form matches only \
             when every required obligation is met; {} recorded translation cases cite none",
            report.obligations.declared,
            report.obligations.met,
            report.obligations.unmet,
            report.obligations.held,
            report.obligations.unmeasured,
            report.uncited_cases.len(),
        ),
        tally_line("total", &report.forms),
    ];
    lines.extend(
        report
            .families
            .iter()
            .map(|(name, tally)| tally_line(name, tally)),
    );
    lines.push(format!("signed off: {}", report.signed_off.join(", ")));
    lines.push(format!("not signed off: {}", report.unsigned.join(", ")));
    lines.push(format!(
        "capture queue: {} function, expression and query forms need a recorded statement with \
         values, {} statements need a recorded result",
        report.capture_forms.len(),
        report.capture_statements.len()
    ));
    lines.push(format!(
        "effect receipt gap: {} DDL, DML, session and utility forms need a receipt of a \
         statement's effect, which is not built",
        report.effect_receipt_forms.len()
    ));

    lines.extend(report.worklist.iter().take(WORK_PRINTED).map(|item| {
        format!(
            "  {} {}: {} forms, {} statements, such as {}",
            item.kind,
            item.subject,
            item.forms,
            item.statements,
            item.examples.join(", ")
        )
    }));
    lines
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

/// The diff's lines: every regression, then the first changes.
fn diff_lines(label: &str, diff: &ResultsDiff) -> Vec<String> {
    let mut lines: Vec<String> = diff
        .regressions
        .iter()
        .map(|line| format!("{label}: REGRESSION {line}"))
        .collect();
    lines.extend(
        diff.changes
            .iter()
            .take(PROBLEMS_PRINTED)
            .map(|line| format!("{label}: {line}")),
    );
    if diff.changes.len() > PROBLEMS_PRINTED {
        lines.push(format!(
            "{label}: … {} more changes",
            diff.changes.len() - PROBLEMS_PRINTED
        ));
    }
    lines
}

/// The problems of `head` against the target's baseline: its regressions.
fn against_target(path: &Path, head: &ResultsBaseline) -> Result<Vec<String>, String> {
    let target: ResultsBaseline = read_json(path)?;
    let diff = diff_results(&target, head);
    for line in diff_lines("against the target", &diff) {
        say(&line);
    }
    say(&format!(
        "against the target: {} regressions, {} other changes",
        diff.regressions.len(),
        diff.changes.len()
    ));
    Ok(match diff.regressions.is_empty() {
        true => Vec::new(),
        false => vec![format!(
            "{} regressions against the target's baseline",
            diff.regressions.len()
        )],
    })
}

pub(crate) fn results(
    statements: &Path,
    (write_baseline, report_path, against): (bool, Option<&Path>, Option<&Path>),
    common: &Common,
) -> Result<ExitCode, ExitCode> {
    say(&format!(
        "loading {} and {}",
        common.root.display(),
        statements.display()
    ));
    let coverage = load_coverage(&common.root, common.evidence.as_deref())
        .map_err(|problems| fail(&problems))?;
    snowflake_only("results")?;
    let coverage_root: &Path = common.root.parent().unwrap_or(Path::new("."));
    let directory: PathBuf = coverage_root.join(TRANSLATION_DIR);
    let repository: &Path = coverage_root.parent().unwrap_or(Path::new("."));
    let files = load_results_files((&directory, repository), &coverage)
        .map_err(|problems| fail(&problems))?;
    let recorded: StatementsFile = read_json(statements).map_err(|problem| fail(&[problem]))?;
    let report =
        measure_results(&coverage, &files, &recorded).map_err(|problems| fail(&problems))?;
    if let Some(path) = report_path {
        let text = serde_json::to_string_pretty(&report).unwrap_or_default() + "\n";
        std::fs::write(path, text)
            .map_err(|error| fail(&[format!("{}: {error}", path.display())]))?;
    }
    print(common.json, &report, &report_lines(&report));
    let head: ResultsBaseline = results_baseline(&report);
    let mut problems: Vec<String> = match against {
        Some(path) => against_target(path, &head).map_err(|problem| fail(&[problem]))?,
        None => Vec::new(),
    };
    let baseline_path = directory.join(BASELINE_FILE);
    if write_baseline {
        let text = serde_json::to_string_pretty(&head).unwrap_or_default() + "\n";
        std::fs::write(&baseline_path, text)
            .map_err(|error| fail(&[format!("{}: {error}", baseline_path.display())]))?;
        say(&format!("wrote {}", baseline_path.display()));
    } else {
        let base: ResultsBaseline =
            read_json(&baseline_path).map_err(|problem| fail(&[problem]))?;
        let diff = diff_results(&base, &head);
        for line in diff_lines("against the committed baseline", &diff) {
            say(&line);
        }
        if !diff.regressions.is_empty() || !diff.changes.is_empty() {
            problems.push(format!(
                "the measure differs from {} in {} entries; re-record it with --write-baseline",
                baseline_path.display(),
                diff.regressions.len() + diff.changes.len()
            ));
        }
    }
    match problems.is_empty() {
        true => {
            say("done: the baseline holds the measure, and nothing regressed");
            Ok(ExitCode::SUCCESS)
        }
        false => Err(fail(&problems)),
    }
}
