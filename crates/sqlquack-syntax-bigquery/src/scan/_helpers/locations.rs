//! Token locations: each token's range in characters, as the shared tree records source ranges.

use sqlquack_syntax::tokens::models::Span;

use crate::scan::models::Token;

/// Records each token's range in characters and the line and column after its last character.
pub(crate) fn locate(sql: &str, tokens: &mut [Token]) {
    let mut character: Vec<usize> = vec![0; sql.len() + 1];
    let mut line: Vec<usize> = vec![1; sql.len() + 1];
    let mut line_start: Vec<usize> = vec![0; sql.len() + 1];
    let (mut count, mut lines, mut first): (usize, usize, usize) = (0, 1, 0);
    for (offset, text) in sql.char_indices() {
        for byte in offset..offset + text.len_utf8() {
            character[byte] = count;
            line[byte] = lines;
            line_start[byte] = first;
        }
        count += 1;
        if text == '\n' {
            lines += 1;
            first = count;
        }
    }
    character[sql.len()] = count;
    line[sql.len()] = lines;
    line_start[sql.len()] = first;
    for token in tokens {
        let start: usize = token.start.min(sql.len());
        let end: usize = token.end.clamp(start, sql.len());
        let last: usize = end.saturating_sub(1).max(start);
        token.span = Span::new(
            character[start],
            character[end],
            line[last],
            character[end] - line_start[last] + 1,
        );
    }
}
