//! `checked_layout`: the layout of DuckDB SQL, checked to change only whitespace.

use crate::layout::_helpers::breaks::lay_out;
use crate::layout::_helpers::tokens::{tokens, unmodelled};

/// `sql` laid out, or why its text is unmodelled or its layout changes more than whitespace.
pub fn checked_layout(sql: &str) -> Result<String, String> {
    if let Some(reason) = unmodelled(&tokens(sql)) {
        return Err(format!(
            "it holds {reason}, which the layout does not model"
        ));
    }
    let laid_out = lay_out(sql);
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
    if lay_out(&laid_out) != laid_out {
        return Err("laying it out again changes it".to_owned());
    }
    Ok(laid_out)
}
