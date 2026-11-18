//! `nested_chains`: the join chains written inside one FROM item.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::queries::models::Join;

/// The join chains written inside `item`, outermost first, each with its joined table's left side.
pub(crate) fn nested_chains(item: &Expression) -> Vec<(Option<&Expression>, &[Join])> {
    match item {
        Expression::JoinedTable(joined) => {
            let mut chains: Vec<(Option<&Expression>, &[Join])> =
                vec![(Some(&joined.left), joined.joins.as_slice())];
            chains.extend(nested_chains(&joined.left));
            for join in &joined.joins {
                chains.extend(nested_chains(&join.this));
            }
            chains
        }
        Expression::Paren(paren) => nested_chains(&paren.this),
        _ => Vec::new(),
    }
}
