//! `resolve_references`: sqlbuild reference calls in SQL replaced as census replaces them.

use crate::census::_helpers::placeholders::rewrite_with_references;

/// `sql` with `__ref(…)` and its kin as the placeholder relations a census schema declares.
pub fn resolve_references(sql: &str) -> String {
    rewrite_with_references(sql, None).sql
}
