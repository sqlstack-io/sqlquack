//! `classify`: every construct a statement uses, with its position.

use polyglot_sql::Expression;
use polyglot_sql::traversal::ExpressionWalk;

use crate::classification::_helpers::clauses::clause_constructs;
use crate::classification::_helpers::identity::node_constructs;
use crate::classification::_helpers::locations::{LineIndex, resolve_locations};
use crate::classification::models::Occurrence;
use crate::parsing::models::SourceDialect;

/// Every construct in `statement`, in source order, positioned against the `source` it came from.
pub fn classify(statement: &Expression, source: &str, dialect: SourceDialect) -> Vec<Occurrence> {
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
    let locations = resolve_locations(&nodes, &parents, &LineIndex::new(source));
    let mut occurrences: Vec<Occurrence> = Vec::new();
    for (node, location) in nodes.iter().zip(locations) {
        let constructs = node_constructs(node, dialect.polyglot())
            .into_iter()
            .chain(clause_constructs(node));
        occurrences.extend(constructs.map(|construct| Occurrence {
            construct,
            location,
        }));
    }
    occurrences
}
