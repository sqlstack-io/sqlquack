//! The summary, per family and per page.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::check::models::{FormState, Reason};

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
    /// The commit whose catalogue the snapshot was taken on (decision D81).
    pub catalogue_commit: String,
    /// SHA-256 of `sources.toml`, `aliases.toml` and each catalogue file, by path.
    pub catalogue_files: BTreeMap<String, String>,
    /// The parser tree the claims were checked against; none when uncommitted.
    pub parser_tree: Option<String>,
    /// Every form's state, absent forms included.
    pub forms: BTreeMap<String, FormState>,
    /// Required obligations the checker found satisfied.
    pub satisfied_obligations: BTreeSet<String>,
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
}
