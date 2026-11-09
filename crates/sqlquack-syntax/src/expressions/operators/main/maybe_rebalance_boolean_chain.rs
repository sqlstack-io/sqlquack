//! `maybe_rebalance_boolean_chain`: rebalances comment-free AND/OR chains into a balanced tree, keeping depth logarithmic.

use crate::expressions::operators::_helpers::boolean_chains::build_balanced_boolean_tree;
use crate::expressions::operators::_helpers::boolean_chains::flatten_boolean_terms_owned;
use crate::expressions::operators::_helpers::boolean_chains::should_rebalance_boolean_chain;
use crate::expressions::operators::constants::BALANCED_CHAIN_MAX_TERMS;
use crate::expressions::tree::models::Expression;

/// Rebalances comment-free AND/OR chains into a balanced tree, keeping depth logarithmic.
pub fn maybe_rebalance_boolean_chain(expr: Expression, is_and: bool) -> Expression {
    if !should_rebalance_boolean_chain(&expr, is_and) {
        return expr;
    }

    let terms = flatten_boolean_terms_owned(expr, is_and);
    if terms.len() <= BALANCED_CHAIN_MAX_TERMS {
        return build_balanced_boolean_tree(terms, is_and);
    }

    build_balanced_boolean_tree(terms, is_and)
}
