//! `layout_sql`: DuckDB SQL laid out over lines, with only the whitespace between tokens changed.

use crate::layout::_helpers::breaks::start;
use crate::layout::_helpers::plan::Plan;
use crate::layout::_helpers::tokens::{Token, tokens};
use crate::layout::constants::INLINE_WIDTH;

/// `sql` one clause and select item per line, nested queries and long `CASE`s indented by depth.
pub fn layout_sql(sql: &str) -> String {
    let tokens: Vec<Token<'_>> = tokens(sql);
    let plan = Plan::new(&tokens);
    let mut walk = start(plan.width(0, tokens.len().saturating_sub(1)) > INLINE_WIDTH);
    for index in 0..tokens.len() {
        let level = walk.line_break(&tokens, index);
        walk.push_token(&tokens, index, level);
        walk.forced = walk.after_token(&tokens, index, &plan);
    }
    walk.laid_out
}
