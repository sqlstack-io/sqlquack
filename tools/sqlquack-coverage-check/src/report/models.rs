//! The summary, per family and per page.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::check::models::{FormState, Reason};
use crate::report::_helpers::obligation_order::in_obligation_order;

/// Forms counted in one family or on one page.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Tally {
    /// Catalogued forms; absent ones are never counted.
    pub forms: usize,
    pub verified: usize,
    pub excluded: usize,
    pub required_obligations: usize,
    pub satisfied_obligations: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Summary {
    pub parser_tree: Option<String>,
    pub total: Tally,
    /// verified / catalogued forms, in percent.
    pub verified_percent: f64,
    pub absent_forms: usize,
    /// Unverified forms by each reason they carry; a form can carry several.
    pub forms_by_reason: BTreeMap<Reason, usize>,
    /// Satisfied obligations whose receipt stopped after parsing: semantic, not syntax, gaps.
    pub semantic_only_obligations: usize,
    pub unexcluded_pages: usize,
    pub problems: usize,
    pub families: BTreeMap<String, FamilySummary>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FamilySummary {
    pub tally: Tally,
    /// Family forms / catalogued forms, in percent.
    pub scope_percent: f64,
}

/// What `baseline.json` records: the frozen catalogue and every claim the checker made on it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Always `SNAPSHOT_FORMAT`.
    pub format: String,
    /// The commit whose catalogue the snapshot was taken on.
    pub catalogue_commit: String,
    /// SHA-256 of `sources.toml`, `aliases.toml` and each catalogue file, by path.
    pub catalogue_files: BTreeMap<String, String>,
    /// The parser tree the claims were checked against; none when uncommitted.
    pub parser_tree: Option<String>,
    /// Every form's state, absent forms included.
    pub forms: BTreeMap<String, FormState>,
    /// Required obligations the checker found satisfied, written in obligation order.
    #[serde(serialize_with = "in_obligation_order")]
    pub satisfied_obligations: BTreeSet<String>,
    /// Receipted cases Snowflake parsed that the parser accepts, opaque or not.
    #[serde(default)]
    pub parsed_cases: BTreeSet<String>,
    /// Receipted cases Snowflake parsed that the parser rejects.
    #[serde(default)]
    pub refused_cases: BTreeSet<String>,
    /// Receipted cases Snowflake refused as syntax that the parser rejects too.
    #[serde(default)]
    pub syntax_error_cases_rejected: BTreeSet<String>,
    /// Receipted cases Snowflake refused as syntax that the parser accepts, opaque or not.
    #[serde(default)]
    pub syntax_error_cases_accepted: BTreeSet<String>,
    /// Cases the catalogue marks documented: their acceptance against a syntax error is intended.
    #[serde(default)]
    pub documented_cases: BTreeSet<String>,
    /// Every receipt case id whose statement the parser contradicts, cited or not; none before receipt contradictions were tracked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_contradictions: Option<BTreeSet<String>>,
    /// Receipt case ids whose SQL no case or run report gives; none before receipt contradictions were tracked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unjudged_receipts: Option<BTreeSet<String>>,
}

/// `receipt-disagreements.toml`: receipted cases the parser contradicts today, case keys by form.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptDisagreements {
    /// Cases Snowflake refused as syntax that the parser accepts, documented or not.
    #[serde(default)]
    pub syntax_errors_accepted: BTreeMap<String, BTreeSet<String>>,
    /// Cases Snowflake parsed that the parser rejects.
    #[serde(default)]
    pub parsed_cases_refused: BTreeMap<String, BTreeSet<String>>,
    /// Other receipt case ids the parser contradicts: retired cases, run reports, unjudged cases.
    #[serde(default)]
    pub other_receipts_contradicted: BTreeSet<String>,
}

/// One snapshot against a later one; coverage and catalogue changes kept apart.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SnapshotDiff {
    /// Catalogue files whose hash changed, appeared or disappeared.
    pub catalogue_files_changed: Vec<String>,
    pub forms_added: Vec<String>,
    pub forms_removed: Vec<String>,
    /// Forms verified or excluded in the base that no longer are: regressions.
    pub forms_lost: Vec<String>,
    /// Forms verified or excluded now that were not in the base.
    pub forms_gained: Vec<String>,
    /// Obligations satisfied in the base that no longer are: regressions.
    pub obligations_lost: Vec<String>,
    pub obligations_gained: Vec<String>,
    /// Cases Snowflake parsed that the base parser accepted, opaque or not, and the head rejects.
    pub parsed_cases_refused: Vec<String>,
    /// Cases Snowflake refused as syntax that the base parser rejected and the head accepts, less documented ones.
    pub syntax_error_cases_accepted: Vec<String>,
    /// The same flips on cases marked documented: listed for review, not failing.
    pub documented_cases_accepted: Vec<String>,
    /// Receipts the head contradicts and the base did not, less documented ones: regressions.
    pub receipts_contradicted: Vec<String>,
    /// The same on receipts of cases marked documented: listed, not failing.
    pub documented_receipts_contradicted: Vec<String>,
    /// Receipts both contradict, or every head contradiction when the base has no receipt verdicts.
    pub receipts_still_contradicted: Vec<String>,
    /// Receipts the head cannot judge because no case or run report gives their SQL.
    pub unjudged_receipts: Vec<String>,
    /// Syntax-error cases the head accepts, documented or not, that the file does not list.
    pub unlisted_syntax_errors_accepted: Vec<String>,
    /// Cases Snowflake parsed that the head rejects and the file does not list.
    pub unlisted_parsed_cases_refused: Vec<String>,
    /// Other receipts the head contradicts that the file does not list.
    pub unlisted_receipts_contradicted: Vec<String>,
    /// Listed cases the head no longer contradicts, or that have no such receipt: delete their entries.
    pub disagreements_resolved: Vec<String>,
    /// Listed cases the target branch's disagreements file did not list: the list only shrinks.
    pub disagreements_added: Vec<String>,
}

/// One focus form: its uses and how far it is.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FocusLine {
    pub form: String,
    pub uses: Vec<String>,
    pub state: FormState,
    pub required_obligations: usize,
    pub satisfied_obligations: usize,
    pub reasons: Vec<Reason>,
}

/// Focus forms counted together.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct FocusTally {
    pub forms: usize,
    pub verified: usize,
    pub required_obligations: usize,
    pub satisfied_obligations: usize,
}

/// `summary --focus`: the focus file's forms in work order, in total and by SQLBuild use.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FocusReport {
    pub dialect: String,
    pub sqlbuild_commit: String,
    pub total: FocusTally,
    pub by_use: BTreeMap<String, FocusTally>,
    pub forms: Vec<FocusLine>,
}

/// The summary with its focus report, as `summary --focus --json` prints it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FocusedSummary {
    #[serde(flatten)]
    pub summary: Summary,
    pub focus: FocusReport,
}
