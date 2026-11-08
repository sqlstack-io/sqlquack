//! `layout_sql`: DuckDB SQL laid out over lines, or unchanged where the layout cannot be checked.

use crate::layout::main::checked_layout::checked_layout;

/// `sql` laid out by [`checked_layout`], or `sql` itself on one line when the check fails.
pub fn layout_sql(sql: &str) -> String {
    checked_layout(sql).unwrap_or_else(|_| sql.to_owned())
}
