//! `walk_in_scope`: the nodes of one query scope, depth first, without entering the scopes nested in it.

use sqlquack_syntax::Expression;

use crate::tree::_helpers::scope::{opens_scope, scope_children};

/// The nodes of the scope `expression` opens, depth first, without entering nested scopes.
pub(crate) fn walk_in_scope(expression: &Expression) -> impl Iterator<Item = &Expression> {
    let mut pending: Vec<&Expression> = vec![expression];
    let mut visited: Vec<&Expression> = Vec::new();
    while let Some(node) = pending.pop() {
        visited.push(node);
        let children: Vec<&Expression> = scope_children(node);
        pending.extend(
            children
                .into_iter()
                .rev()
                .filter(|child| !opens_scope(child)),
        );
    }
    visited.into_iter()
}
