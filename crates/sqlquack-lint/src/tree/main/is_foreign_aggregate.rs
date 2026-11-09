//! `is_foreign_aggregate`: whether a call name is another dialect's aggregate the parsed dialect lacks.

use crate::dialects::main::profile::profile;
use crate::lint::models::LintDialect;

/// Whether `name` is another dialect's aggregate that `dialect` has none of, so its kind is unknown.
pub(crate) fn is_foreign_aggregate(name: &str, dialect: LintDialect) -> bool {
    profile(dialect)
        .foreign_aggregates
        .iter()
        .any(|aggregate| aggregate.eq_ignore_ascii_case(name))
}
