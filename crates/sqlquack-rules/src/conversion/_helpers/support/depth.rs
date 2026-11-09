//! The bound on how deep a converted tree may nest, checked before conversion recurses into it.

use serde_json::Value;
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::models::{Refusal, RefusalCategory};
use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::conversion::_helpers::support::tree::node_construct;
use crate::conversion::constants::MAX_TREE_DEPTH;

/// Nothing, or a budget refusal when `tree` nests deeper than conversion can recurse safely.
pub(crate) fn within_depth(tree: &Expression) -> Result<(), Refusal> {
    let depth = tree.tree_depth();
    match depth > MAX_TREE_DEPTH {
        false => Ok(()),
        true => Err(Refusal {
            construct: None,
            location: None,
            category: RefusalCategory::ConversionBudget,
            reason: format!(
                "the statement nests {depth} levels deep, beyond the {MAX_TREE_DEPTH} conversion \
                 accepts; each operand of a chain such as a + b + c counts as one level"
            ),
        }),
    }
}

/// `tree` as the JSON conversion walks, once its depth is within the bound.
pub(crate) fn positioned(tree: &Expression) -> Result<Value, Refusal> {
    within_depth(tree)?;
    serde_json::to_value(tree).map_err(|error| {
        missing(
            node_construct("statement"),
            format!("the tree does not serialise: {error}"),
        )
    })
}
