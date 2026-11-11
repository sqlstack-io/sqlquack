//! `snowflake_rejections`: a refusal for each construct Snowflake parses but then rejects.

use std::cell::OnceCell;

use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::classification::models::{Construct, ConstructFamily};
use crate::refusals::models::{Refusal, RefusalCategory};
use crate::snowflake::refusals::_helpers::clause_location::clause_location;
use crate::snowflake::refusals::_helpers::rejections::{DOUBLE_EQUALS, double_equals, rejection};
use sqlquack_syntax::tokens::main::node_location::node_location;
use sqlquack_syntax::tokens::models::SourceLines;

/// Refuses, in tree order, every node of the statement in `source` that Snowflake would reject.
pub fn snowflake_rejections(
    statement: &Expression,
    (source, lines): (&str, &SourceLines),
) -> Vec<Refusal> {
    let characters: OnceCell<Vec<char>> = OnceCell::new();
    let windowed: Vec<&Expression> = statement
        .dfs()
        .filter_map(|node| match node {
            Expression::WindowFunction(window) => Some(&window.this),
            _ => None,
        })
        .collect();
    let written_double = source.contains(DOUBLE_EQUALS);
    statement
        .dfs()
        .filter_map(|node| {
            if written_double
                && let Some((start, end)) = double_equals(
                    node,
                    characters.get_or_init(|| source.chars().collect()),
                    lines,
                )
            {
                return Some(Refusal {
                    construct: Some(Construct::new(ConstructFamily::Syntax, "double_equals")),
                    location: Some(lines.source_location(start, end)),
                    category: RefusalCategory::SnowflakeRejects,
                    reason: "Snowflake reads == as a call to an unknown function".to_owned(),
                });
            }
            let (name, reason) = rejection(node, &windowed)?;
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
