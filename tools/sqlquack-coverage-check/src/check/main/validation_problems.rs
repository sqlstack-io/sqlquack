//! `validation_problems`: what `validate` refuses once the files load.

use crate::check::_helpers::references::resolve_problems;
use crate::formats::models::SyntaxCoverage;

/// Every `resolves` entry that does not name a changes_requested review of the approval's binding.
pub fn validation_problems(coverage: &SyntaxCoverage) -> Vec<String> {
    resolve_problems(&coverage.reviews)
}
