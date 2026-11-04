//! Receipts a later receipt supersedes or a run report withdraws, which are no longer evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::formats::_helpers::files::read_text;
use crate::formats::_helpers::hashes::is_sha256;
use crate::formats::constants::{REPORT_RECEIPT_SHA256, REPORT_WITHDRAWN};
use crate::formats::models::Receipt;

/// The receipt line hashes the run reports withdraw, and every problem reading them.
pub(crate) fn withdrawn_receipts(reports: &[PathBuf]) -> (BTreeSet<String>, Vec<String>) {
    let mut withdrawn: BTreeSet<String> = BTreeSet::new();
    let mut problems: Vec<String> = Vec::new();
    for path in reports {
        let report: Value = match read_text(path).and_then(|(text, _)| {
            serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
        }) {
            Ok(report) => report,
            Err(problem) => {
                problems.push(problem);
                continue;
            }
        };
        let entries: &[Value] = report[REPORT_WITHDRAWN]
            .as_array()
            .map_or(&[], Vec::as_slice);
        for entry in entries {
            match entry[REPORT_RECEIPT_SHA256]
                .as_str()
                .filter(|hash| is_sha256(hash))
            {
                Some(hash) => {
                    withdrawn.insert(hash.to_owned());
                }
                None => problems.push(withdrawn_problem(path)),
            }
        }
    }
    (withdrawn, problems)
}

fn withdrawn_problem(path: &Path) -> String {
    format!(
        "{}: a withdrawn entry has no {REPORT_RECEIPT_SHA256}",
        path.display()
    )
}

/// The current receipts, the retired line hashes, and retirements naming no receipt of the case.
pub(crate) fn retire(
    receipts: Vec<(Receipt, String)>,
    withdrawn: &BTreeSet<String>,
) -> (Vec<Receipt>, Vec<String>, Vec<String>) {
    let cases: BTreeMap<&str, &str> = receipts
        .iter()
        .map(|(receipt, hash)| (hash.as_str(), receipt.case_id.as_str()))
        .collect();
    let mut problems: Vec<String> = Vec::new();
    let mut retired: BTreeSet<String> = withdrawn.clone();
    for (receipt, _) in &receipts {
        let Some(old) = &receipt.supersedes_receipt_sha256 else {
            continue;
        };
        if cases.get(old.as_str()) != Some(&receipt.case_id.as_str()) {
            problems.push(format!(
                "receipt {}: supersedes {old}, which is no published receipt of its case",
                receipt.case_id
            ));
        }
        retired.insert(old.clone());
    }
    problems.extend(
        withdrawn
            .iter()
            .filter(|hash| !cases.contains_key(hash.as_str()))
            .map(|hash| format!("withdrawn receipt {hash} is not published")),
    );
    let kept: Vec<Receipt> = receipts
        .into_iter()
        .filter(|(_, hash)| !retired.contains(hash))
        .map(|(receipt, _)| receipt)
        .collect();
    (kept, retired.into_iter().collect(), problems)
}
