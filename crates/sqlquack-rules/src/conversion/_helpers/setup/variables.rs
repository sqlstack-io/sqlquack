//! SET and UNSET of session variables in a case's setup, which `$name` reads.

use std::collections::BTreeMap;

use serde_json::Value;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::statements::models::{SetStatement, UnsetStatement};
use sqlquack_syntax::expressions::tree::models::Expression;

use crate::conversion::_helpers::setup::tables::resolved;
use crate::conversion::_helpers::support::tree::strip_positions;
use crate::conversion::constants::{NONDETERMINISTIC_FUNCTIONS, TIME_DEPENDENT_PREFIXES};

/// `SET name = value` or `SET (name, …) = (value, …)`: each variable recorded for `$name` to read.
pub(crate) fn assigned(set: &SetStatement) -> Result<Vec<(String, Value)>, String> {
    let mut assignments: Vec<(String, Value)> = Vec::new();
    for item in &set.items {
        if item.kind.is_some() || item.no_equals {
            return Err("sets other than a session variable with =".to_owned());
        }
        let pairs: Vec<(&Expression, &Expression)> = match (&item.name, &item.value) {
            (Expression::Tuple(names), Expression::Tuple(values))
                if names.expressions.len() == values.expressions.len() =>
            {
                names.expressions.iter().zip(&values.expressions).collect()
            }
            (Expression::Tuple(_), _) => {
                return Err("sets a list of variables to other than as many values".to_owned());
            }
            (name, value) => vec![(name, value)],
        };
        for (name, value) in pairs {
            let name = match name {
                Expression::Identifier(identifier) => resolved(identifier)?,
                Expression::Column(column) if column.table.is_none() => resolved(&column.name)?,
                _ => return Err("sets a variable whose name is not a plain name".to_owned()),
            };
            assignments.push((name, variable_value(value)?));
        }
    }
    Ok(assignments)
}

/// `UNSET name` or `UNSET (name, …)`: the names, each a variable setup SET earlier.
pub(crate) fn unassigned(
    unset: &UnsetStatement,
    variables: &BTreeMap<String, Value>,
) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = Vec::with_capacity(unset.names.len());
    for identifier in &unset.names {
        let name = resolved(identifier)?;
        if !variables.contains_key(&name) || names.contains(&name) {
            return Err(format!("unsets {name}, which setup did not set"));
        }
        names.push(name);
    }
    Ok(names)
}

/// A literal, NULL or a scalar subquery whose value cannot change between the SET and its reads.
fn variable_value(value: &Expression) -> Result<Value, String> {
    let fixed = match value {
        Expression::Null(_) => true,
        Expression::Literal(literal) => {
            matches!(literal.as_ref(), Literal::Number(_) | Literal::String(_))
        }
        Expression::Subquery(_) => true,
        _ => false,
    };
    if !fixed {
        return Err(
            "sets a variable to other than a literal, NULL or a scalar subquery".to_owned(),
        );
    }
    let tree = serde_json::to_value(value)
        .map(|value| strip_positions(&value))
        .map_err(|error| format!("sets a value that does not serialise: {error}"))?;
    match unrepeatable(&tree) {
        Some(reason) => Err(format!("sets a variable to a subquery {reason}")),
        None => Ok(tree),
    }
}

/// Why a value read at each `$name` could differ from the value SET computed once, if it could.
fn unrepeatable(tree: &Value) -> Option<String> {
    match tree {
        Value::Object(fields) => {
            if fields.contains_key("parameter") {
                return Some("reading another variable or parameter".to_owned());
            }
            if let Some(Value::String(name)) = fields.get("name")
                && unrepeatable_name(name)
            {
                return Some(format!("calling {name}, whose result can change"));
            }
            if let Some(kind) = fields.keys().find(|kind| time_dependent(kind)) {
                return Some(format!("reading {kind}, whose result can change"));
            }
            fields.values().find_map(unrepeatable)
        }
        Value::Array(items) => items.iter().find_map(unrepeatable),
        _ => None,
    }
}

/// Whether a function of this name can give another result between the SET and a read.
fn unrepeatable_name(name: &str) -> bool {
    NONDETERMINISTIC_FUNCTIONS.contains(&name.to_ascii_uppercase().as_str()) || time_dependent(name)
}

/// Whether a function or node kind of this name reads the clock.
fn time_dependent(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    TIME_DEPENDENT_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}
