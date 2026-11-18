//! `join_chains`: a SELECT's joins, each chain with the relation it starts from.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::queries::models::{Join, Select};

use crate::tree::main::nested_chains::nested_chains;

/// Every join chain of `select` with the relation it starts from, its own first, then nested ones.
pub(crate) fn join_chains(select: &Select) -> Vec<(Option<&Expression>, &[Join])> {
    let items: &[Expression] = select
        .from
        .as_ref()
        .map_or(&[], |from| from.expressions.as_slice());
    let lone: Option<&Expression> = match items {
        [item] => Some(item),
        _ => None,
    };
    let mut chains: Vec<(Option<&Expression>, &[Join])> = vec![(lone, select.joins.as_slice())];
    for item in items
        .iter()
        .chain(select.joins.iter().map(|join| &join.this))
    {
        chains.extend(nested_chains(item));
    }
    chains
}
