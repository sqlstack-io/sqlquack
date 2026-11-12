//! One function per subcommand; each prints its result and returns the exit code.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use clap::ValueEnum;
use serde_json::{Value, json};

use crate::check::main::check_coverage::check_coverage;
use crate::check::models::{
    CheckResult, FormResult, LocalFailure, LocalVerdict, ObligationResult, Reason,
};
use crate::cli::_helpers::local::{frozen_catalogue, local_verdict, parser_tree};
use crate::cli::_helpers::output::{print, say};
use crate::cli::_helpers::worker::Worker;
use crate::cli::models::{Common, TranslationArguments};
use crate::depth::main::analysis_verdict::analysis_verdict;
use crate::formats::constants::{DISAGREEMENTS_FILE, SNAPSHOT_FORMAT, SNOWFLAKE_DIALECT};
use crate::formats::main::current_dialect::current_dialect;
use crate::formats::main::load_coverage::load_coverage;
use crate::formats::main::load_focus::load_focus;
use crate::formats::main::obligation_form::obligation_form;
use crate::formats::models::{
    FocusFile, LocalParser, Obligation, ObligationKind, Review, SyntaxCoverage,
};
use crate::report::main::diff_fails::diff_fails;
use crate::report::main::diff_snapshots::diff_snapshots;
use crate::report::main::focus_report::focus_report;
use crate::report::main::snapshot::snapshot;
use crate::report::main::summarize::summarize;
use crate::report::main::tally_by::tally_by;
use crate::report::models::{
    FocusReport, FocusedSummary, ReceiptDisagreements, Snapshot, SnapshotDiff, Summary,
};
use crate::translation::constants::TRANSLATION_DIR;
use crate::translation::main::load_translation_files::load_translation_files;
use crate::translation::main::measure_translation::measure_translation;
use crate::translation::models::{TranslationInputs, TranslationReport, TranslationTally};

/// Kinds that test a limit; `boundaries` reports these.
const BOUNDARY_KINDS: [ObligationKind; 4] = [
    ObligationKind::BoundaryBelow,
    ObligationKind::BoundaryAt,
    ObligationKind::BoundaryAbove,
    ObligationKind::NestingDepth,
];

/// The time one local parse may take before it fails as a timeout.
const PARSE_BOUND: Duration = Duration::from_secs(10);
/// The exit code of a Rust process whose main thread panicked, which the worker reads as a panic.
const PANIC_EXIT: u8 = 101;
/// The worker's virtual memory bound in KiB, 4 GiB; exceeding it fails the case as a crash.
const PARSE_MEMORY_KIB: u64 = 4 * 1024 * 1024;

/// The worker command: this binary, memory-bounded by the shell, parsing on a `thread_stack` thread.
pub(crate) fn worker_command(
    binary: PathBuf,
    local_parser: LocalParser,
    (thread_stack, analyse): (Option<usize>, bool),
) -> Box<dyn Fn() -> Command> {
    let parser: String = local_parser
        .to_possible_value()
        .map_or_else(String::new, |value| value.get_name().to_owned());
    let stack: String =
        thread_stack.map_or_else(String::new, |bytes| format!(" --thread-stack {bytes}"));
    let analysis: &str = if analyse { " --analyse" } else { "" };
    let worker: String = format!("parse-worker --parser {parser}{stack}{analysis}");
    Box::new(move || {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg(format!(
                "ulimit -v {PARSE_MEMORY_KIB} && exec \"$0\" {worker}"
            ))
            .arg(&binary);
        command
    })
}

fn load(common: &Common) -> Result<SyntaxCoverage, ExitCode> {
    say(&format!("loading {}", common.root.display()));
    load_coverage(&common.root, common.evidence.as_deref()).map_err(|problems| {
        for problem in &problems {
            say(problem);
        }
        say(&format!("FAILED, {} problems", problems.len()));
        ExitCode::FAILURE
    })
}

fn checked(coverage: &SyntaxCoverage) -> Result<CheckResult, ExitCode> {
    let tree: Option<String> = parser_tree().map_err(|problem| {
        say(&format!("FAILED, cannot read the parser tree: {problem}"));
        ExitCode::FAILURE
    })?;
    say(&format!(
        "checking against parser tree {}",
        tree.as_deref()
            .unwrap_or("(uncommitted: every family review is stale)")
    ));
    let binary: PathBuf = std::env::current_exe().map_err(|problem| {
        say(&format!(
            "FAILED, cannot find this binary for the parse worker: {problem}"
        ));
        ExitCode::FAILURE
    })?;
    let local_parser: LocalParser = current_dialect().local_parser;
    let worker: RefCell<Worker> = RefCell::new(Worker::new(
        worker_command(binary, local_parser, (None, false)),
        PARSE_BOUND,
    ));
    let parses: bool = local_verdict(local_parser).is_some();
    let check: CheckResult = check_coverage(coverage, tree.as_deref(), &|sql| match parses {
        true => worker.borrow_mut().verdict(sql),
        false => LocalVerdict::Failed {
            failure: LocalFailure::NoParser,
        },
    });
    for problem in &check.problems {
        say(problem);
    }
    say(&format!(
        "checked {} forms, {} problems",
        check.forms.len(),
        check.problems.len()
    ));
    Ok(check)
}

/// Fails a Snowflake-to-DuckDB measure on any other dialect's root.
pub(crate) fn snowflake_only(command: &str) -> Result<(), ExitCode> {
    let dialect = current_dialect();
    match dialect.name == SNOWFLAKE_DIALECT {
        true => Ok(()),
        false => {
            say(&format!(
                "FAILED, {command} measures Snowflake conversion; this root is {}",
                dialect.name
            ));
            Err(ExitCode::FAILURE)
        }
    }
}

fn exit(check: &CheckResult) -> ExitCode {
    match check.problems.is_empty() {
        true => ExitCode::SUCCESS,
        false => ExitCode::FAILURE,
    }
}

fn in_family(form: &FormResult, family: Option<&str>) -> bool {
    family.is_none_or(|family| form.family == family)
}

fn unsatisfied(form: &FormResult) -> impl Iterator<Item = &ObligationResult> {
    form.obligations
        .iter()
        .filter(|obligation| obligation.required && obligation.reason.is_some())
}

pub(crate) fn validate(common: &Common) -> Result<ExitCode, ExitCode> {
    let coverage: SyntaxCoverage = load(common)?;
    let forms: usize = coverage.catalogue.values().map(Vec::len).sum();
    let retired: usize = coverage.retired_receipts.len();
    let counts: Value = json!({
        "forms": forms,
        "receipts": coverage.receipts.len(),
        "retired_receipts": retired,
    });
    let line: String = format!(
        "valid, {forms} forms, {} receipts ({retired} superseded or withdrawn)",
        coverage.receipts.len()
    );
    print(common.json, &counts, &[line]);
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn summary(focus: Option<&Path>, common: &Common) -> Result<ExitCode, ExitCode> {
    let coverage: SyntaxCoverage = load(common)?;
    let focus: Option<FocusFile> = match focus {
        None => None,
        Some(path) => Some(load_focus(path, &coverage).map_err(|problems| {
            for problem in &problems {
                say(problem);
            }
            say(&format!("FAILED, {} focus problems", problems.len()));
            ExitCode::FAILURE
        })?),
    };
    let check: CheckResult = checked(&coverage)?;
    let summary: Summary = summarize(&check);
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
    let Some(focus) = focus else {
        print(common.json, &summary, &text);
        return Ok(exit(&check));
    };
    let report: FocusReport = focus_report(&focus, &check);
    text.push(format!(
        "focus {}: {} of {} forms verified; obligations {}/{}",
        report.dialect,
        report.total.verified,
        report.total.forms,
        report.total.satisfied_obligations,
        report.total.required_obligations
    ));
    text.extend(report.by_use.iter().map(|(letter, tally)| {
        format!(
            "  use {letter}: {} of {} forms verified",
            tally.verified, tally.forms
        )
    }));
    text.extend(report.forms.iter().map(|line| {
        format!(
            "  {} [{}] {:?} {}/{} {:?}",
            line.form,
            line.uses.join(""),
            line.state,
            line.satisfied_obligations,
            line.required_obligations,
            line.reasons
        )
    }));
    print(
        common.json,
        &FocusedSummary {
            summary,
            focus: report,
        },
        &text,
    );
    Ok(exit(&check))
}

pub(crate) fn family(family: &str, common: &Common) -> Result<ExitCode, ExitCode> {
    let check: CheckResult = checked(&load(common)?)?;
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

pub(crate) fn missing(family: Option<&str>, common: &Common) -> Result<ExitCode, ExitCode> {
    let check: CheckResult = checked(&load(common)?)?;
    let missing: Vec<Value> = check
        .forms
        .iter()
        .filter(|form| in_family(form, family))
        .flat_map(unsatisfied)
        .map(|obligation| json!({"obligation": obligation.id, "reason": obligation.reason}))
        .collect();
    let text: Vec<String> = missing.iter().map(Value::to_string).collect();
    print(common.json, &missing, &text);
    Ok(exit(&check))
}

/// Whether `review` names `form`: of its family, in its `forms` list or with none.
fn names_form(review: &Review, form: &FormResult) -> bool {
    review.family == form.family
        && review
            .forms
            .as_ref()
            .is_none_or(|forms| forms.contains(&form.id))
}

/// A stale form, with its current fingerprint and what each review naming it recorded.
fn stale_entry(form: &FormResult, reviews: &[Review]) -> Value {
    let mut recorded: Vec<Value> = Vec::new();
    for review in reviews.iter().filter(|review| names_form(review, form)) {
        recorded.push(json!({
            "id": review.id,
            "fingerprint": review.fingerprints.get(&form.id),
            "definition_sha256": review.definitions.get(&form.id),
            "parser_tree": review.parser_tree,
        }));
    }
    json!({
        "form": form.id,
        "fingerprint": form.fingerprint,
        "definition_sha256": form.definition_sha256,
        "reviews": recorded,
    })
}

pub(crate) fn stale(family: Option<&str>, common: &Common) -> Result<ExitCode, ExitCode> {
    let coverage: SyntaxCoverage = load(common)?;
    let check: CheckResult = checked(&coverage)?;
    let stale: Vec<Value> = check
        .forms
        .iter()
        .filter(|form| in_family(form, family) && form.reasons.contains(&Reason::StaleReview))
        .map(|form| stale_entry(form, &coverage.reviews))
        .collect();
    let text: Vec<String> = stale.iter().map(Value::to_string).collect();
    print(common.json, &stale, &text);
    Ok(exit(&check))
}

pub(crate) fn pages(family: Option<&str>, common: &Common) -> Result<ExitCode, ExitCode> {
    let check: CheckResult = checked(&load(common)?)?;
    let filtered: CheckResult = CheckResult {
        forms: check
            .forms
            .iter()
            .filter(|form| in_family(form, family))
            .cloned()
            .collect(),
        ..check.clone()
    };
    let pages = tally_by(&filtered, |form| &form.source);
    let text: Vec<String> = pages
        .iter()
        .map(|(page, tally)| format!("{page} {}/{} forms verified", tally.verified, tally.forms))
        .collect();
    print(common.json, &pages, &text);
    Ok(exit(&check))
}

/// One boundary obligation as declared, with its result.
fn boundary_row(obligation: &Obligation, form: &FormResult) -> Value {
    let result: Option<&ObligationResult> = form
        .obligations
        .iter()
        .find(|found| found.id == obligation.id);
    json!({
        "obligation": obligation.id,
        "kind": obligation.kind,
        "required": obligation.required,
        "target": obligation.target,
        "depths": obligation.depths,
        "anchor": obligation.anchor,
        "note": obligation.note,
        "reason": result.and_then(|found| found.reason),
        "cases": result.map(|found| &found.cases),
    })
}

/// The boundary and nesting obligations of one form, each with its cases and verdicts.
pub(crate) fn boundaries(form_id: &str, common: &Common) -> Result<ExitCode, ExitCode> {
    let coverage: SyntaxCoverage = load(common)?;
    let check: CheckResult = checked(&coverage)?;
    let Some(form) = check.forms.iter().find(|form| form.id == form_id) else {
        say(&format!("FAILED, no form {form_id} is catalogued"));
        return Err(ExitCode::FAILURE);
    };
    let declared = coverage.obligations.get(&form.family).into_iter().flatten();
    let rows: Vec<Value> = declared
        .filter(|obligation| obligation_form(&obligation.id, &form.family) == Some(form_id))
        .filter(|obligation| BOUNDARY_KINDS.contains(&obligation.kind))
        .map(|obligation| boundary_row(obligation, form))
        .collect();
    let mut text: Vec<String> = vec![format!(
        "{form_id} {:?}: {} boundary obligations",
        form.state,
        rows.len()
    )];
    text.extend(rows.iter().map(Value::to_string));
    print(common.json, &rows, &text);
    Ok(exit(&check))
}

/// The current claims as a snapshot, bound to `catalogue_commit`; always JSON.
pub(crate) fn snapshot_command(
    catalogue_commit: &str,
    common: &Common,
) -> Result<ExitCode, ExitCode> {
    let root: String = common.root.display().to_string();
    let commit: String = frozen_catalogue(catalogue_commit, &root).map_err(|problem| {
        say(&format!("FAILED, {problem}"));
        ExitCode::FAILURE
    })?;
    let coverage: SyntaxCoverage = load(common)?;
    let check: CheckResult = checked(&coverage)?;
    if !check.problems.is_empty() {
        say("FAILED, a snapshot of a tree with problems claims nothing");
        return Err(ExitCode::FAILURE);
    }
    print(true, &snapshot(&check, &coverage, &commit), &[]);
    Ok(ExitCode::SUCCESS)
}

fn read_snapshot(path: &Path) -> Result<Snapshot, ExitCode> {
    let text: String = std::fs::read_to_string(path).map_err(|error| {
        say(&format!("FAILED, {}: {error}", path.display()));
        ExitCode::FAILURE
    })?;
    let parsed: Snapshot = serde_json::from_str(&text).map_err(|error| {
        say(&format!("FAILED, {}: {error}", path.display()));
        ExitCode::FAILURE
    })?;
    if parsed.format != SNAPSHOT_FORMAT {
        say(&format!(
            "FAILED, {}: not {SNAPSHOT_FORMAT}",
            path.display()
        ));
        return Err(ExitCode::FAILURE);
    }
    Ok(parsed)
}

/// The disagreements file at `path`; none listed when it does not exist.
fn read_disagreements(path: &Path) -> Result<ReceiptDisagreements, ExitCode> {
    match std::fs::read_to_string(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(ReceiptDisagreements::default())
        }
        read => read
            .map_err(|error| error.to_string())
            .and_then(|text| toml::from_str(&text).map_err(|error| error.to_string()))
            .map_err(|problem| {
                say(&format!("FAILED, {}: {problem}", path.display()));
                ExitCode::FAILURE
            }),
    }
}

/// Lines naming each non-empty list of the diff.
fn diff_lines(diff: &SnapshotDiff) -> Vec<String> {
    let lists: [(&str, &Vec<String>); 17] = [
        (
            "REGRESSION Snowflake syntax errors the parser accepts, documented or not, that receipt-disagreements.toml does not list",
            &diff.unlisted_syntax_errors_accepted,
        ),
        (
            "REGRESSION Snowflake-parsed cases the parser rejects that receipt-disagreements.toml does not list",
            &diff.unlisted_parsed_cases_refused,
        ),
        (
            "REGRESSION other receipts the parser contradicts that receipt-disagreements.toml does not list",
            &diff.unlisted_receipts_contradicted,
        ),
        (
            "RESOLVED disagreements the parser now agrees with; delete their entries from receipt-disagreements.toml",
            &diff.disagreements_resolved,
        ),
        (
            "ADDED disagreements the target branch did not list; receipt-disagreements.toml only shrinks",
            &diff.disagreements_added,
        ),
        ("REGRESSION forms lost", &diff.forms_lost),
        (
            "REGRESSION receipts the parser now contradicts, cited by a form or not",
            &diff.receipts_contradicted,
        ),
        (
            "DOCUMENTED receipts the parser now contradicts, for review",
            &diff.documented_receipts_contradicted,
        ),
        ("REGRESSION obligations lost", &diff.obligations_lost),
        (
            "REGRESSION Snowflake-parsed cases the parser now rejects",
            &diff.parsed_cases_refused,
        ),
        (
            "REGRESSION Snowflake syntax errors the parser now accepts",
            &diff.syntax_error_cases_accepted,
        ),
        (
            "DOCUMENTED Snowflake syntax errors the parser now accepts, for review",
            &diff.documented_cases_accepted,
        ),
        ("CATALOGUE files changed", &diff.catalogue_files_changed),
        ("CATALOGUE forms added", &diff.forms_added),
        ("CATALOGUE forms removed", &diff.forms_removed),
        ("forms gained", &diff.forms_gained),
        ("obligations gained", &diff.obligations_gained),
    ];
    let mut lines: Vec<String> = lists
        .iter()
        .filter(|(_, ids)| !ids.is_empty())
        .map(|(label, ids)| format!("{label} ({}): {}", ids.len(), ids.join(" ")))
        .collect();
    let counted: [(&str, usize); 2] = [
        (
            "receipts the parser already contradicted, each listed or failing above",
            diff.receipts_still_contradicted.len(),
        ),
        (
            "receipts without SQL to judge, not failing",
            diff.unjudged_receipts.len(),
        ),
    ];
    lines.extend(
        counted
            .iter()
            .filter(|(_, count)| *count > 0)
            .map(|(label, count)| format!("{label}: {count} (listed in --json)")),
    );
    lines
}

/// BASE against HEAD, or against the current tree; fails on a regression or catalogue change.
pub(crate) fn diff(
    (base, head): (&Path, Option<&Path>),
    target_disagreements: Option<&Path>,
    common: &Common,
) -> Result<ExitCode, ExitCode> {
    let base: Snapshot = read_snapshot(base)?;
    let disagreements: ReceiptDisagreements =
        read_disagreements(&common.root.join(DISAGREEMENTS_FILE))?;
    let target_disagreements: Option<ReceiptDisagreements> =
        target_disagreements.map(read_disagreements).transpose()?;
    let head: Snapshot = match head {
        Some(path) => read_snapshot(path)?,
        None => {
            let coverage: SyntaxCoverage = load(common)?;
            let check: CheckResult = checked(&coverage)?;
            if !check.problems.is_empty() {
                return Err(ExitCode::FAILURE);
            }
            snapshot(&check, &coverage, &base.catalogue_commit)
        }
    };
    let diff: SnapshotDiff =
        diff_snapshots(&base, &head, &disagreements, target_disagreements.as_ref());
    let failing: bool = diff_fails(&diff);
    let mut text: Vec<String> = diff_lines(&diff);
    if text.is_empty() {
        text.push(format!(
            "no change against catalogue {}",
            base.catalogue_commit
        ));
    }
    print(common.json, &diff, &text);
    Ok(match failing {
        true => ExitCode::FAILURE,
        false => ExitCode::SUCCESS,
    })
}

/// `verdict_of(sql)` on a thread with `bytes` of stack; a panic there exits as a panicking worker.
fn on_thread(
    bytes: usize,
    verdict_of: fn(&str) -> LocalVerdict,
    sql: &str,
) -> Result<LocalVerdict, ExitCode> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(bytes)
            .spawn_scoped(scope, || verdict_of(sql))
            .map_err(|_| ExitCode::FAILURE)?
            .join()
            .map_err(|_| ExitCode::from(PANIC_EXIT))
    })
}

/// Reads one JSON string of SQL per line and prints the local verdict on each, flushed.
pub(crate) fn parse_worker(
    parser: LocalParser,
    thread_stack: Option<usize>,
    analyse: bool,
) -> Result<ExitCode, ExitCode> {
    let verdict_of: fn(&str) -> LocalVerdict = match analyse {
        true => analysis_verdict(parser),
        false => local_verdict(parser),
    }
    .ok_or(ExitCode::FAILURE)?;
    let stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines() {
        let line: String = line.map_err(|_| ExitCode::FAILURE)?;
        let sql: String = serde_json::from_str(&line).map_err(|_| ExitCode::FAILURE)?;
        let verdict: LocalVerdict = match thread_stack {
            None => verdict_of(&sql),
            Some(bytes) => on_thread(bytes, verdict_of, &sql)?,
        };
        let verdict: String = serde_json::to_string(&verdict).map_err(|_| ExitCode::FAILURE)?;
        let mut out = stdout.lock();
        writeln!(out, "{verdict}")
            .and_then(|()| out.flush())
            .map_err(|_| ExitCode::FAILURE)?;
    }
    Ok(ExitCode::SUCCESS)
}

/// One tally as a text line: the measurable forms that translate, and those needing a schema.
fn tally_line(name: &str, tally: &TranslationTally) -> String {
    format!(
        "{name}: {} of {} measurable forms translate; {} need a schema, {} have no relation in scope, {} not measurable; {} by design, {} declared value divergence, {} not yet; cases {}/{} translated, {} did not finish",
        tally.translated_forms,
        tally.measurable_forms,
        tally.needs_schema_forms,
        tally.no_relation_in_scope_forms,
        tally.not_measurable_forms,
        tally.by_design_forms,
        tally.declared_value_divergence_forms,
        tally.not_yet_translated_forms,
        tally.translated_cases,
        tally.counted_cases,
        tally.did_not_finish_cases,
    )
}

pub(crate) fn translation(
    arguments: &TranslationArguments,
    common: &Common,
) -> Result<ExitCode, ExitCode> {
    let (sqlquack, family) = (&arguments.sqlquack, arguments.family.as_deref());
    let coverage: SyntaxCoverage = load(common)?;
    snowflake_only("translation")?;
    if family.is_some_and(|family| !coverage.catalogue.contains_key(family)) {
        say(&format!("FAILED, no family {}", family.unwrap_or_default()));
        return Err(ExitCode::FAILURE);
    }
    let directory: PathBuf = common
        .root
        .parent()
        .unwrap_or(Path::new("."))
        .join(TRANSLATION_DIR);
    let fail = |problems: Vec<String>| {
        for problem in &problems {
            say(problem);
        }
        say(&format!("FAILED, {} problems", problems.len()));
        ExitCode::FAILURE
    };
    let by_design: BTreeSet<String> = load_translation_files(&directory).map_err(fail)?;
    let work: PathBuf =
        std::env::temp_dir().join(format!("syntax-translation-{}", std::process::id()));
    std::fs::create_dir_all(&work).map_err(|error| {
        say(&format!("FAILED, no scratch directory: {error}"));
        ExitCode::FAILURE
    })?;
    say(&format!("converting with {}", sqlquack.display()));
    let report = measure_translation(&TranslationInputs {
        coverage: &coverage,
        root: &common.root,
        by_design: &by_design,
        binary: sqlquack,
        config: &arguments.config,
        work: &work,
        family,
        jobs: arguments.jobs,
    });
    let _ = std::fs::remove_dir_all(&work);
    let report: TranslationReport = report.map_err(fail)?;
    let mut text: Vec<String> = vec![tally_line("total", &report.total)];
    text.extend(
        report
            .families
            .iter()
            .map(|(name, tally)| tally_line(name, tally)),
    );
    text.push(format!("results match: {}", report.results_match));
    text.extend(
        report
            .refusals
            .iter()
            .take(20)
            .map(|(refusal, forms)| format!("  blocks {forms} forms: {refusal}")),
    );
    if family.is_some() {
        text.extend(report.forms.iter().map(|form| {
            format!(
                "{} {:?} {}/{} cases{}",
                form.id,
                form.state,
                form.translated_cases,
                form.measured_cases,
                match form.refusals.is_empty() {
                    true => String::new(),
                    false => format!(": {}", form.refusals.join("; ")),
                }
            )
        }));
    }
    print(common.json, &report, &text);
    Ok(ExitCode::SUCCESS)
}
