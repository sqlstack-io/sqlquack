//! `checked_layout`: the layout of DuckDB SQL, checked to change only whitespace.

use crate::layout::_helpers::tokens::tokens;
use crate::layout::main::layout_sql::layout_sql;

/// `sql` laid out by [`layout_sql`], or why that changed more than whitespace or is not idempotent.
pub fn checked_layout(sql: &str) -> Result<String, String> {
    let laid_out = layout_sql(sql);
    let texts = |sql: &str| -> Vec<String> {
        tokens(sql)
            .iter()
            .map(|token| token.text.to_owned())
            .collect()
    };
    if texts(sql) != texts(&laid_out) {
        return Err("its tokens differ from the single-line SQL's".to_owned());
    }
    let stripped = |sql: &str| -> String { sql.chars().filter(|c| !c.is_whitespace()).collect() };
    if stripped(sql) != stripped(&laid_out) {
        return Err("its characters other than whitespace differ".to_owned());
    }
    if layout_sql(&laid_out) != laid_out {
        return Err("laying it out again changes it".to_owned());
    }
    Ok(laid_out)
}
