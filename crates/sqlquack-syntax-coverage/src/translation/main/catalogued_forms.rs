//! `catalogued_forms`: every catalogued form still in its source, by id with its family.

use std::collections::BTreeMap;

use crate::formats::models::SyntaxCoverage;
use crate::translation::_helpers::forms;

/// Every catalogued form still in its source, by id with its family; only `family` when given.
pub fn catalogued_forms<'a>(
    coverage: &'a SyntaxCoverage,
    family: Option<&str>,
) -> BTreeMap<&'a str, &'a str> {
    forms::catalogued_forms(coverage, family)
}
