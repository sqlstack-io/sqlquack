//! A form's fingerprint: what a review of it saw, so a parser change elsewhere leaves it current.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::check::models::{CaseResult, LocalVerdict, ObligationResult};
use crate::formats::main::content_sha256::content_sha256;
use crate::formats::models::{Form, Obligation, Reading};

/// The local verdict as fingerprint text: `accepted` with its tree hash, `rejected`, or the failure.
fn verdict_text(local: &LocalVerdict) -> String {
    match local {
        LocalVerdict::Accepted { tree_sha256, .. } => {
            format!("accepted\t{}", tree_sha256.as_deref().unwrap_or_default())
        }
        LocalVerdict::Rejected => "rejected\t".to_owned(),
        LocalVerdict::Failed { failure } => format!("failed:{failure:?}\t"),
    }
}

/// A classification field as its serde name, or `null`, so a renamed variant keeps its hash.
fn serde_name<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// SHA-256 over one line per citing case, by case id: id, SQL hash, verdict, expect, stage, phase.
pub(crate) fn form_fingerprint(obligations: &[ObligationResult]) -> String {
    let cases: BTreeMap<&str, &CaseResult> = obligations
        .iter()
        .flat_map(|obligation| &obligation.cases)
        .map(|case| (case.case_id.as_str(), case))
        .collect();
    let text: String = cases
        .values()
        .map(|case| {
            format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                case.case_id,
                case.sql_sha256,
                verdict_text(&case.local),
                serde_name(&case.expect),
                serde_name(&case.stage),
                serde_name(&case.phase)
            )
        })
        .collect();
    content_sha256(text.as_bytes())
}

/// SHA-256 of the form's catalogue entry, own obligations and any reading as JSON; empty on failure.
pub(crate) fn form_definition(
    form: &Form,
    obligations: &[&Obligation],
    reading: Option<&Reading>,
) -> String {
    let bytes = match reading {
        Some(reading) => serde_json::to_vec(&(form, obligations, reading)),
        None => serde_json::to_vec(&(form, obligations)),
    };
    bytes.map_or_else(|_| String::new(), |bytes| content_sha256(&bytes))
}
