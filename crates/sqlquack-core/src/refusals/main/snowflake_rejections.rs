//! `snowflake_rejections`: a refusal for each construct Snowflake parses but then rejects.

use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::classification::models::{Construct, ConstructFamily};
use crate::parsing::main::node_location::node_location;
use crate::parsing::models::SourceLines;
use crate::refusals::_helpers::rejections::rejection;
use crate::refusals::models::{Refusal, RefusalCategory};

/// Refuses, in tree order, every node of the statement that Snowflake would reject.
pub fn snowflake_rejections(statement: &Expression, lines: &SourceLines) -> Vec<Refusal> {
    statement
        .dfs()
        .filter_map(|node| rejection_refusal(node, lines))
        .collect()
}

/// The refusal of one node Snowflake would reject, placed at the node.
fn rejection_refusal(node: &Expression, lines: &SourceLines) -> Option<Refusal> {
    let (name, reason) = rejection(node)?;
    Some(Refusal {
        construct: Some(Construct::new(ConstructFamily::Syntax, name)),
        location: node_location(node, lines),
        category: RefusalCategory::SnowflakeRejects,
        reason,
    })
}
