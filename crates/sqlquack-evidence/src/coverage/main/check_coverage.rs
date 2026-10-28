//! `check_coverage`: each rule's declared combinations and hazards without a passing case.

use std::collections::{BTreeMap, BTreeSet};

use crate::cases::models::LoadedCase;
use crate::coverage::_helpers::rules::{add, rule_coverage, rule_shapes, undeclared_rule};
use crate::coverage::models::{CoverageReport, HazardCatalogue, RuleDomain};
use crate::verify::models::CaseVerdict;

/// Coverage of `rules` by the verdicts of `cases`, against the declared domains and hazards.
pub fn check_coverage(
    rules: &[&str],
    (domains, catalogue): (&[RuleDomain], &HazardCatalogue),
    (cases, verdicts): (&[LoadedCase], &[CaseVerdict]),
) -> CoverageReport {
    let declared: BTreeMap<&str, &RuleDomain> = domains
        .iter()
        .map(|domain| (domain.id.as_str(), domain))
        .collect();
    let (covering, converted) = rule_shapes(cases, verdicts);
    let mut ids: BTreeSet<&str> = rules.iter().copied().collect();
    ids.extend(declared.keys().copied());
    let mut report = CoverageReport {
        catalogue_version: catalogue.version,
        ..CoverageReport::default()
    };
    for id in ids {
        let shapes = covering.get(id).map(Vec::as_slice).unwrap_or_default();
        let coverage = match declared.get(id) {
            None => undeclared_rule(id, converted.get(id)),
            Some(domain) => rule_coverage(domain, catalogue, shapes, converted.get(id)),
        };
        report.totals = add(std::mem::take(&mut report.totals), &coverage);
        report.rules.push(coverage);
    }
    report
}
