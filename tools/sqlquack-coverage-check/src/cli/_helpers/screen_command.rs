//! The `screen` subcommand: same-tree variants of the accepted cases, against the root's allowlist.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

use serde_json::Value;

use crate::cli::_helpers::commands::load;
use crate::cli::_helpers::local::{local_statements, local_tokens};
use crate::cli::_helpers::output::{print, say};
use crate::cli::models::Common;
use crate::formats::main::current_dialect::current_dialect;
use crate::formats::main::obligation_form::obligation_form;
use crate::formats::models::SyntaxCoverage;
use crate::screen::constants::ALLOWLIST_FILE;
use crate::screen::main::screen_cases::screen_cases;
use crate::screen::main::written_tree::written_tree;
use crate::screen::models::{SameTree, ScreenAllowlist, ScreenReport, ScreenedCase};

/// The root's allowlist; empty when the root has none.
fn read_allowlist(root: &Path) -> Result<ScreenAllowlist, ExitCode> {
    let path = root.join(ALLOWLIST_FILE);
    if !path.exists() {
        return Ok(ScreenAllowlist::default());
    }
    std::fs::read_to_string(&path)
        .map_err(|error| error.to_string())
        .and_then(|text| toml::from_str(&text).map_err(|error| error.to_string()))
        .map_err(|problem| {
            say(&format!("FAILED, {}: {problem}", path.display()));
            ExitCode::FAILURE
        })
}

/// Whether `form` is screened: every form when none is chosen.
fn chosen(forms: Option<&[String]>, form: &str) -> bool {
    forms.is_none_or(|forms| forms.iter().any(|wanted| wanted == form))
}

/// Each case of the chosen family and forms, once per form its obligations belong to.
fn screened_cases(
    coverage: &SyntaxCoverage,
    forms: Option<&[String]>,
    family: Option<&str>,
) -> Vec<ScreenedCase> {
    let mut out: Vec<ScreenedCase> = Vec::new();
    for (name, cases) in &coverage.cases {
        if family.is_some_and(|wanted| wanted != name) {
            continue;
        }
        for case in cases {
            let owners: BTreeSet<&str> = case
                .obligations
                .iter()
                .filter_map(|obligation| obligation_form(obligation, name))
                .filter(|form| chosen(forms, form))
                .collect();
            out.extend(owners.into_iter().map(|form| ScreenedCase {
                form: form.to_owned(),
                case_id: case.case_id.clone(),
                sql: case.sql.clone(),
            }));
        }
    }
    out
}

fn line(found: &SameTree) -> String {
    format!(
        "    {} [{:?} {}@{}] {}  ==  {}",
        found.case_id,
        found.mutation.kind,
        found.mutation.token,
        found.mutation.position,
        found.original,
        found.mutation.sql
    )
}

/// The report's count line, then each flagged form with its variants, then the failing entries.
pub(crate) fn text(report: &ScreenReport) -> Vec<String> {
    let mut out: Vec<String> = vec![format!(
        "{} accepted cases, {} variants: {} same-tree variants flagged, {} allowed, {} stale entries, {} unpinned entries",
        report.cases,
        report.variants,
        report.flagged.len(),
        report.allowed.len(),
        report.stale.len(),
        report.unpinned.len()
    )];
    let forms: BTreeSet<&str> = report
        .flagged
        .iter()
        .map(|found| found.form.as_str())
        .collect();
    for form in forms {
        let flagged: Vec<&SameTree> = report
            .flagged
            .iter()
            .filter(|found| found.form == form)
            .collect();
        out.push(format!("- {form} ({})", flagged.len()));
        out.extend(flagged.into_iter().map(line));
    }
    out.extend(report.stale.iter().map(|entry| format!("stale: {entry}")));
    out.extend(
        report
            .unpinned
            .iter()
            .map(|entry| format!("unpinned: {entry}")),
    );
    out
}

/// Whether the report has no flagged variant, stale entry or unpinned entry.
pub(crate) fn passes(report: &ScreenReport) -> bool {
    report.flagged.is_empty() && report.stale.is_empty() && report.unpinned.is_empty()
}

/// The chosen forms' (every form's) screen; unfiltered, an entry for an unscreened form is stale.
pub(crate) fn screen_report(
    coverage: &SyntaxCoverage,
    root: &Path,
    forms: Option<&[String]>,
    family: Option<&str>,
) -> Result<ScreenReport, ExitCode> {
    let allowlist: ScreenAllowlist = read_allowlist(root)?;
    let Some(parse) = local_statements(current_dialect().local_parser) else {
        say("FAILED, this dialect has no local parser to screen with");
        return Err(ExitCode::FAILURE);
    };
    let cases: Vec<ScreenedCase> = screened_cases(coverage, forms, family);
    say(&format!("screening {} cases", cases.len()));
    let tree = move |sql: &str| -> Option<Value> { parse(sql).as_deref().map(written_tree) };
    let tokens = local_tokens(current_dialect().local_parser);
    let lexed = move |sql: &str| -> Vec<(usize, usize)> {
        tokens.map_or_else(Vec::new, |ranges| ranges(sql))
    };
    let full: bool = forms.is_none() && family.is_none();
    Ok(screen_cases(&cases, (&tree, &lexed), &allowlist, full))
}

/// Fails when a same-tree variant has no allowing entry, or an entry is stale or unpinned.
pub(crate) fn screen(
    forms: &[String],
    family: Option<&str>,
    common: &Common,
) -> Result<ExitCode, ExitCode> {
    let coverage: SyntaxCoverage = load(common)?;
    let chosen: Option<&[String]> = (!forms.is_empty()).then_some(forms);
    let report: ScreenReport = screen_report(&coverage, &common.root, chosen, family)?;
    print(common.json, &report, &text(&report));
    Ok(match passes(&report) {
        true => ExitCode::SUCCESS,
        false => ExitCode::FAILURE,
    })
}
