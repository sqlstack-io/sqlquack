//! `classify`: every construct a statement uses, with its position.

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::types::models::DataType;

use crate::classification::_helpers::clauses::clause_constructs;
use crate::classification::_helpers::identity::node_constructs;
use crate::classification::_helpers::locations::resolve_locations;
use crate::classification::models::Occurrence;
use sqlquack_syntax::tokens::models::SourceLines;

/// Every construct in `statement` in source order, placed by `lines`, casts named by `type_name`.
pub fn classify(
    statement: &Expression,
    lines: &SourceLines,
    type_name: fn(&DataType) -> String,
) -> Vec<Occurrence> {
    let mut nodes: Vec<&Expression> = Vec::new();
    let mut parents: Vec<Option<usize>> = Vec::new();
    let mut pending: Vec<(&Expression, Option<usize>)> = vec![(statement, None)];
    while let Some((node, parent)) = pending.pop() {
        let position = nodes.len();
        nodes.push(node);
        parents.push(parent);
        pending.extend(
            node.children()
                .into_iter()
                .rev()
                .map(|child| (child, Some(position))),
        );
    }
    let locations = resolve_locations(&nodes, &parents, lines);
    let mut occurrences: Vec<Occurrence> = Vec::new();
    for (node, location) in nodes.iter().zip(locations) {
        let constructs = node_constructs(node, type_name)
            .into_iter()
            .chain(clause_constructs(node));
        occurrences.extend(constructs.map(|construct| Occurrence {
            construct,
            location,
        }));
    }
    occurrences
}
