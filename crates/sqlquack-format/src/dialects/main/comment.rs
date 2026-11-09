//! `comment`: a comment read from the SQL it is written in.

use crate::dialects::models::Comment;

/// The comment of `sql` from `start` to `end`; `line` when it runs to the end of its line.
pub(crate) fn comment(sql: &str, start: usize, end: usize, line: bool) -> Comment {
    let line_start = sql[..start].rfind('\n').map_or(0, |at| at + 1);
    Comment {
        start,
        end,
        text: sql[start..end].trim_end_matches(['\r', '\n']).to_string(),
        line,
        leading: sql[line_start..start].trim().is_empty(),
    }
}
