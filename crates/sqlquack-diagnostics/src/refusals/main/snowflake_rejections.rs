//! `snowflake_rejections`: a refusal for each construct Snowflake parses but then rejects.

use std::cell::OnceCell;

use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::classification::models::{Construct, ConstructFamily};
use crate::refusals::_helpers::clause_location::clause_location;
use crate::refusals::_helpers::rejections::rejection;
use crate::refusals::models::{Refusal, RefusalCategory};
use sqlquack_syntax::tokens::main::node_location::node_location;
use sqlquack_syntax::tokens::models::SourceLines;

/// Refuses, in tree order, every node of the statement in `source` that Snowflake would reject.
pub fn snowflake_rejections(
    statement: &Expression,
    (source, lines): (&str, &SourceLines),
) -> Vec<Refusal> {
    let characters: OnceCell<Vec<char>> = OnceCell::new();
    statement
        .dfs()
        .filter_map(|node| {
            let (name, reason) = rejection(node)?;
            let text = characters.get_or_init(|| source.chars().collect());
            Some(Refusal {
                construct: Some(Construct::new(ConstructFamily::Syntax, name)),
                location: clause_location(node, text, lines).or_else(|| node_location(node, lines)),
                category: RefusalCategory::SnowflakeRejects,
                reason,
            })
        })
        .collect()
}
