//! `checked_layout`: the layout of converted SQL, checked to change only whitespace and the case of words.

use sqlquack_format::{FormatDialect, FormatOptions, format_sql};

use crate::layout::constants::{INDENT, LINE_WIDTH, QUOTES};

/// `sql` in `dialect` laid out with keywords and called names upper-cased, or why it was refused.
pub fn checked_layout(sql: &str, dialect: FormatDialect) -> Result<String, String> {
    let options = FormatOptions {
        line_width: Some(LINE_WIDTH),
        indent: INDENT,
        ..FormatOptions::new(dialect)
    };
    let laid_out = format_sql(sql, &options)
        .map_err(|refusal| format!("the formatter refused it: {}", refusal.reason))?
        .sql;
    if !same_apart_from_layout(sql, &laid_out) {
        return Err(
            "its characters differ beyond whitespace and upper-cased letters outside quotes"
                .to_owned(),
        );
    }
    Ok(laid_out)
}

/// Whether `laid_out` has the characters of `sql` apart from whitespace and lower-case letters outside quotes printed in upper case.
fn same_apart_from_layout(sql: &str, laid_out: &str) -> bool {
    let kept = |text: &str| -> Vec<char> { text.chars().filter(|c| !c.is_whitespace()).collect() };
    let (before, after) = (kept(sql), kept(laid_out));
    let mut quote: Option<char> = None;
    before.len() == after.len()
        && before.iter().zip(&after).all(|(&was, &now)| {
            let same = was == now
                || (quote.is_none() && was.is_ascii_lowercase() && now == was.to_ascii_uppercase());
            if QUOTES.contains(&was) {
                quote = match quote {
                    None => Some(was),
                    Some(open) if open == was => None,
                    open => open,
                };
            }
            same
        })
}
