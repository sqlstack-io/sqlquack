//! The summary, per family and per page.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::check::models::Reason;

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
