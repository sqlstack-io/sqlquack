use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::check::main::check_coverage::check_coverage;
use crate::check::models::{FormState, LocalVerdict};
use crate::check::tests::helpers::{
    TREE, case_line, replace, rewrite_select_17, select_17, with_fields,
};
use crate::formats::main::load_coverage::load_coverage;
use crate::report::main::snapshot::snapshot;

use crate::formats::constants::SNAPSHOT_FORMAT;
use crate::report::models::Snapshot;

pub(crate) fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|id| (*id).to_owned()).collect()
}

/// One verified, one excluded and one unverified form, with one satisfied obligation.
pub(crate) fn base() -> Snapshot {
    Snapshot {
        format: SNAPSHOT_FORMAT.to_owned(),
        catalogue_commit: "e863d7de60ef88df1bce25a018f6592647b5729c".to_owned(),
        catalogue_files: BTreeMap::from([("catalogue/query.toml".to_owned(), "a".repeat(64))]),
        parser_tree: Some("0123456789abcdef0123456789abcdef01234567".to_owned()),
        forms: BTreeMap::from([
            ("sf.query.select".to_owned(), FormState::Verified),
            ("sf.query.select.top".to_owned(), FormState::Excluded),
            ("sf.query.with".to_owned(), FormState::Unverified),
        ]),
        satisfied_obligations: BTreeSet::from(["sf.query.select.o001".to_owned()]),
        parsed_cases: BTreeSet::from(["sf.query.select.k1".to_owned()]),
        refused_cases: BTreeSet::from(["sf.query.with.k2".to_owned()]),
        syntax_error_cases_rejected: BTreeSet::from(["sf.query.select.k3".to_owned()]),
        syntax_error_cases_accepted: BTreeSet::new(),
        documented_cases: BTreeSet::new(),
    }
}

/// Makes `select-17` an observe case whose receipt records `phase` and `errno`.
pub(crate) fn observed_select_17(root: &Path, phase: &str, errno: i64) {
    rewrite_select_17(
        root,
        &with_fields(&select_17(), serde_json::json!({"expect": "observe"})),
    );
    replace(
        root,
        "receipts/run-1.jsonl",
        "\"phase\":\"accepted\"",
        &format!("\"errno\":{errno},\"phase\":\"{phase}\""),
    );
}

/// Makes `select-from` an observe case whose receipt is a syntax error with `errno`.
pub(crate) fn observed_select_from(root: &Path, errno: i64) {
    let case: String = case_line("select-from", "sf.query.select.o002", "SELECT FROM", &[]);
    replace(
        root,
        "cases/query.jsonl",
        &case,
        &with_fields(&case, serde_json::json!({"expect": "observe"})),
    );
    replace(
        root,
        "receipts/run-1.jsonl",
        "\"phase\":\"syntax_error\"",
        &format!("\"errno\":{errno},\"phase\":\"syntax_error\""),
    );
}

/// Marks `select-from` with `mark` in `marks/query.jsonl`.
pub(crate) fn mark_select_from(root: &Path, mark: &str) {
    let coverage = load_coverage(root).expect("load");
    let case = coverage.cases["query"]
        .iter()
        .find(|case| case.case_id == "select-from")
        .expect("select-from");
    let line: String = serde_json::json!({
        "case_id": case.case_id,
        "sql_sha256": case.sql_sha256,
        "obligations": case.obligations,
        "mark": mark,
    })
    .to_string();
    std::fs::create_dir_all(root.join("marks")).expect("marks directory");
    std::fs::write(root.join("marks/query.jsonl"), format!("{line}\n")).expect("write marks");
}

/// The snapshot of the tree at `root` under `parse`.
pub(crate) fn snapshot_with(root: &Path, parse: fn(&str) -> LocalVerdict) -> Snapshot {
    let coverage = load_coverage(root).expect("load");
    snapshot(
        &check_coverage(&coverage, Some(TREE), &parse),
        &coverage,
        "catalogue",
    )
}
