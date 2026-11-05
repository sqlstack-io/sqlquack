//! One function per subcommand; each prints its result and returns the exit code.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use serde_json::{Value, json};

use crate::check::main::check_coverage::check_coverage;
use crate::check::models::{CheckResult, FormResult, ObligationResult, Reason};
use crate::cli::_helpers::local::{frozen_catalogue, parser_tree, snowflake_verdict};
use crate::cli::_helpers::output::{print, say};
use crate::cli::_helpers::worker::Worker;
use crate::cli::models::{Common, TranslationArguments};
use crate::formats::constants::SNAPSHOT_FORMAT;
use crate::formats::main::load_coverage::load_coverage;
use crate::formats::main::obligation_form::obligation_form;
use crate::formats::models::{Obligation, ObligationKind, Review, SyntaxCoverage};
use crate::report::main::diff_snapshots::diff_snapshots;
use crate::report::main::snapshot::snapshot;
use crate::report::main::summarize::summarize;
use crate::report::main::tally_by::tally_by;
use crate::report::models::{Snapshot, SnapshotDiff, Summary};
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

/// The time one local parse may take before it fails as a timeout (decision D92).
const PARSE_BOUND: Duration = Duration::from_secs(10);
/// The worker's virtual memory bound in KiB, 4 GiB; exceeding it fails the case as a crash.
const PARSE_MEMORY_KIB: u64 = 4 * 1024 * 1024;

/// The worker command: this binary, its memory bounded by the shell before it starts.
fn worker_command(binary: PathBuf) -> Box<dyn Fn() -> Command> {
    Box::new(move || {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg(format!(
                "ulimit -v {PARSE_MEMORY_KIB} && exec \"$0\" parse-worker"
            ))
            .arg(&binary);
        command
    })
}

fn load(root: &Path) -> Result<SyntaxCoverage, ExitCode> {
    say(&format!("loading {}", root.display()));
    load_coverage(root).map_err(|problems| {
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
    let worker: RefCell<Worker> = RefCell::new(Worker::new(worker_command(binary), PARSE_BOUND));
    let check: CheckResult = check_coverage(coverage, tree.as_deref(), &|sql| {
        worker.borrow_mut().verdict(sql)
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
    let coverage: SyntaxCoverage = load(&common.root)?;
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

pub(crate) fn summary(common: &Common) -> Result<ExitCode, ExitCode> {
    let check: CheckResult = checked(&load(&common.root)?)?;
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
    print(common.json, &summary, &text);
    Ok(exit(&check))
}

pub(crate) fn family(family: &str, common: &Common) -> Result<ExitCode, ExitCode> {
    let check: CheckResult = checked(&load(&common.root)?)?;
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
    let check: CheckResult = checked(&load(&common.root)?)?;
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
    let coverage: SyntaxCoverage = load(&common.root)?;
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
    let check: CheckResult = checked(&load(&common.root)?)?;
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
    let coverage: SyntaxCoverage = load(&common.root)?;
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
    let coverage: SyntaxCoverage = load(&common.root)?;
    let check: CheckResult = checked(&coverage)?;
    if !check.problems.is_empty() {
        say("FAILED, a snapshot of a tree with problems claims nothing");
        return Err(ExitCode::FAILURE);
    }
    print(true, &snapshot(&check, &coverage.file_hashes, &commit), &[]);
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

/// Lines naming each non-empty list of the diff.
fn diff_lines(diff: &SnapshotDiff) -> Vec<String> {
    let lists: [(&str, &Vec<String>); 9] = [
        ("REGRESSION forms lost", &diff.forms_lost),
        ("REGRESSION obligations lost", &diff.obligations_lost),
        (
            "REGRESSION Snowflake-parsed cases the parser now rejects",
            &diff.parsed_cases_refused,
        ),
        (
            "REGRESSION Snowflake syntax errors the parser now accepts",
            &diff.syntax_error_cases_accepted,
        ),
        ("CATALOGUE files changed", &diff.catalogue_files_changed),
        ("CATALOGUE forms added", &diff.forms_added),
        ("CATALOGUE forms removed", &diff.forms_removed),
        ("forms gained", &diff.forms_gained),
        ("obligations gained", &diff.obligations_gained),
    ];
    lists
        .iter()
        .filter(|(_, ids)| !ids.is_empty())
        .map(|(label, ids)| format!("{label} ({}): {}", ids.len(), ids.join(" ")))
        .collect()
}

/// BASE against HEAD, or against the current tree; fails on a regression or catalogue change.
pub(crate) fn diff(
    base: &Path,
    head: Option<&Path>,
    common: &Common,
) -> Result<ExitCode, ExitCode> {
    let base: Snapshot = read_snapshot(base)?;
    let head: Snapshot = match head {
        Some(path) => read_snapshot(path)?,
        None => {
            let coverage: SyntaxCoverage = load(&common.root)?;
            let check: CheckResult = checked(&coverage)?;
            if !check.problems.is_empty() {
                return Err(ExitCode::FAILURE);
            }
            snapshot(&check, &coverage.file_hashes, &base.catalogue_commit)
        }
    };
    let diff: SnapshotDiff = diff_snapshots(&base, &head);
    let failing: bool = [
        &diff.forms_lost,
        &diff.obligations_lost,
        &diff.parsed_cases_refused,
        &diff.syntax_error_cases_accepted,
        &diff.catalogue_files_changed,
        &diff.forms_added,
        &diff.forms_removed,
    ]
    .iter()
    .any(|ids| !ids.is_empty());
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

/// Reads one JSON string of SQL per line and prints the local verdict on each, flushed.
pub(crate) fn parse_worker() -> Result<ExitCode, ExitCode> {
    let stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines() {
        let line: String = line.map_err(|_| ExitCode::FAILURE)?;
        let sql: String = serde_json::from_str(&line).map_err(|_| ExitCode::FAILURE)?;
        let verdict: String =
            serde_json::to_string(&snowflake_verdict(&sql)).map_err(|_| ExitCode::FAILURE)?;
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
    let coverage: SyntaxCoverage = load(&common.root)?;
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
        policy: &arguments.policy,
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
