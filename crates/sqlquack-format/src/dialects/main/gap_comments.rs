//! `gap_comments`: the comments in the blanks a dialect's own scanner skipped.

use crate::dialects::constants::{
    BLOCK_COMMENT_END, BLOCK_COMMENT_START, HASH_COMMENT_START, LINE_COMMENT_START,
};
use crate::dialects::main::comment::comment;
use crate::dialects::models::{Comment, CommentSyntax};

/// The comments in the blanks of `sql` from `start` to `end`, which the scanner skipped; only blanks and comments may lie there.
pub(crate) fn gap_comments(
    sql: &str,
    start: usize,
    end: usize,
    syntax: CommentSyntax,
) -> Vec<Comment> {
    let mut comments: Vec<Comment> = Vec::new();
    let mut at = start;
    while at < end {
        let rest = &sql[at..end];
        let length = if (syntax.hash && rest.starts_with(HASH_COMMENT_START))
            || rest.starts_with(LINE_COMMENT_START)
        {
            rest.find(['\r', '\n']).unwrap_or(rest.len())
        } else if rest.starts_with(BLOCK_COMMENT_START) {
            block_comment_length(rest, syntax.nested)
        } else {
            at += rest.chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let line = !rest.starts_with(BLOCK_COMMENT_START);
        comments.push(comment(sql, at, at + length, line));
        at += length;
    }
    comments
}

/// The length of the block comment `rest` starts with, through the `*/` that closes it.
fn block_comment_length(rest: &str, nested: bool) -> usize {
    let mut depth: usize = 0;
    let mut at = 0;
    while at < rest.len() {
        if rest[at..].starts_with(BLOCK_COMMENT_START) && (nested || depth == 0) {
            depth += 1;
            at += BLOCK_COMMENT_START.len();
        } else if rest[at..].starts_with(BLOCK_COMMENT_END) {
            depth -= 1;
            at += BLOCK_COMMENT_END.len();
            if depth == 0 {
                return at;
            }
        } else {
            at += rest[at..].chars().next().map_or(1, char::len_utf8);
        }
    }
    rest.len()
}
