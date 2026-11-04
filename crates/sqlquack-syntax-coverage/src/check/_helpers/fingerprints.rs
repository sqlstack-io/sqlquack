//! A form's fingerprint: what a review of it saw, so a parser change elsewhere leaves it current.

use std::collections::BTreeMap;

use crate::check::models::{CaseResult, LocalVerdict, ObligationResult};
use crate::formats::main::content_sha256::content_sha256;
use crate::formats::models::{Form, Obligation};

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

/// SHA-256 over one line per case citing the form, sorted by case id: its id, SQL hash and verdict.
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
                "{}\t{}\t{}\n",
                case.case_id,
                case.sql_sha256,
                verdict_text(&case.local)
            )
        })
        .collect();
    content_sha256(text.as_bytes())
}

/// SHA-256 of the form's catalogue entry and own obligations as compact JSON; empty if unserializable.
pub(crate) fn form_definition(form: &Form, obligations: &[&Obligation]) -> String {
    serde_json::to_vec(&(form, obligations))
        .map_or_else(|_| String::new(), |bytes| content_sha256(&bytes))
}
