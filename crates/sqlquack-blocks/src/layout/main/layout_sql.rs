//! `layout_sql`: converted SQL laid out over lines, or unchanged where the layout is refused.

use sqlquack_format::FormatDialect;

use crate::layout::main::checked_layout::checked_layout;

/// `sql` laid out by [`checked_layout`], or `sql` itself on one line when it is refused.
pub fn layout_sql(sql: &str, dialect: FormatDialect) -> String {
    checked_layout(sql, dialect).unwrap_or_else(|_| sql.to_owned())
}
