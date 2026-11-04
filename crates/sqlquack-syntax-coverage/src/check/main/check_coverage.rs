//! `check_coverage`: every form's verdict, failing closed with a named reason.

use std::collections::BTreeSet;

use crate::check::_helpers::forms::form_result;
use crate::check::_helpers::join::join;
use crate::check::_helpers::references::reference_problems;
use crate::check::_helpers::reviews::{current_exclusions, review_reason, unexcluded_pages};
use crate::check::models::{CheckResult, FamilyContext, FormResult, Joined, LocalVerdict};
use crate::formats::models::SyntaxCoverage;

/// Checks every catalogued form against `parse`, the current parser, built from `parser_tree`.
pub fn check_coverage(
    coverage: &SyntaxCoverage,
    parser_tree: Option<&str>,
    parse: &dyn Fn(&str) -> LocalVerdict,
) -> CheckResult {
    let joined: Joined = join(coverage, parse);
    let (excluded_forms, excluded_sources): (BTreeSet<String>, BTreeSet<String>) =
        current_exclusions(&coverage.reviews, &coverage.file_hashes, parser_tree);
    let mut forms: Vec<FormResult> = Vec::new();
    for (family, family_forms) in &coverage.catalogue {
        let context: FamilyContext = FamilyContext {
            joined: &joined,
            review: review_reason(
                family,
                &coverage.reviews,
                &coverage.file_hashes,
                parser_tree,
            ),
            excluded: &excluded_forms,
        };
        for form in family_forms {
            forms.push(form_result(form, family, &context));
        }
    }
    CheckResult {
        parser_tree: parser_tree.map(str::to_owned),
        forms,
        unexcluded_pages: unexcluded_pages(&coverage.sources, &excluded_sources),
        problems: reference_problems(coverage, &joined.forms),
    }
}
