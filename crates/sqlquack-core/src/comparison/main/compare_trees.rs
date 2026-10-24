//! `compare_trees`: whether two statements mean the same, up to spelling.

use polyglot_sql::Expression;

use crate::comparison::_helpers::difference::first_difference;
use crate::comparison::_helpers::normalise::normalised_tree;
use crate::comparison::models::TreeDifference;

/// The first difference after normalisation, or `None`; an unserialisable tree never matches.
pub fn compare_trees(expected: &Expression, actual: &Expression) -> Option<TreeDifference> {
    match (normalised_tree(expected), normalised_tree(actual)) {
        (Ok(expected), Ok(actual)) => first_difference(&expected, &actual),
        (expected, actual) => Some(TreeDifference {
            path: ".".to_owned(),
            expected: describe(expected.err()),
            actual: describe(actual.err()),
        }),
    }
}

fn describe(error: Option<serde_json::Error>) -> String {
    match error {
        Some(error) => format!("<unserialisable tree: {error}>"),
        None => "<serialisable tree>".to_owned(),
    }
}
