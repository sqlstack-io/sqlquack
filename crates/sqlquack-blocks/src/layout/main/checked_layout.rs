//! `checked_layout`: the layout of converted SQL, checked to change only whitespace.

use sqlquack_format::{FormatDialect, FormatOptions, format_sql};

use crate::layout::constants::{INDENT, LINE_WIDTH};

/// `sql` in `dialect` laid out with its tokens and their case kept, or why it was refused.
pub fn checked_layout(sql: &str, dialect: FormatDialect) -> Result<String, String> {
    let options = FormatOptions {
        line_width: Some(LINE_WIDTH),
        indent: INDENT,
        keep_case: true,
        ..FormatOptions::new(dialect)
    };
    let laid_out = format_sql(sql, &options)
        .map_err(|refusal| format!("the formatter refused it: {}", refusal.reason))?
        .sql;
    let stripped = |sql: &str| -> String { sql.chars().filter(|c| !c.is_whitespace()).collect() };
    if stripped(sql) != stripped(&laid_out) {
        return Err("its characters other than whitespace differ".to_owned());
    }
    Ok(laid_out)
}
