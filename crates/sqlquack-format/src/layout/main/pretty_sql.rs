//! `pretty_sql`: SQL laid out over lines, with exactly the tokens it had.

use sqlquack_syntax::{Dialect, tokenize};

use crate::layout::_helpers::words::{same_words, words};
use crate::layout::models::{Output, Printer};

/// `sql` laid out over lines with its tokens unchanged; returned as it is when that cannot be done.
pub fn pretty_sql(sql: &str, dialect: Dialect) -> String {
    let Ok(tokens) = tokenize(sql, dialect) else {
        return sql.to_string();
    };
    let words = words(sql, &tokens);
    let mut printer = Printer {
        words: &words,
        output: Output {
            line_start: true,
            ..Output::default()
        },
    };
    printer.statements();
    let text: String = printer.output.text;
    match same_words(&text, dialect, &words) {
        true => text,
        false => sql.to_string(),
    }
}
