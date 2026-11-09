//! The constructs a single node stands for, apart from its clauses.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::operators::models::Cast;
use sqlquack_syntax::expressions::types::models::DataType;

use crate::classification::constants::{PROJECT_FUNCTION_CONSTRUCT, PROJECT_FUNCTION_PLACEHOLDER};
use crate::classification::models::{Construct, ConstructFamily};

pub(crate) fn node_constructs(
    node: &Expression,
    type_name: fn(&DataType) -> String,
) -> Vec<Construct> {
    match node {
        Expression::Function(function) => vec![function_construct(&function.name)],
        Expression::AggregateFunction(function) => vec![function_construct(&function.name)],
        Expression::Cast(cast) | Expression::TryCast(cast) | Expression::SafeCast(cast) => {
            cast_constructs(node.variant_name(), cast, type_name)
        }
        other => vec![Construct::new(ConstructFamily::Node, other.variant_name())],
    }
}

fn function_construct(name: &str) -> Construct {
    if name
        .to_ascii_lowercase()
        .starts_with(PROJECT_FUNCTION_PLACEHOLDER)
    {
        return Construct::new(ConstructFamily::Function, PROJECT_FUNCTION_CONSTRUCT);
    }
    Construct::new(ConstructFamily::Function, name.to_ascii_uppercase())
}

fn cast_constructs(
    variant: &str,
    cast: &Cast,
    type_name: fn(&DataType) -> String,
) -> Vec<Construct> {
    let mut constructs = vec![
        Construct::new(ConstructFamily::Node, variant),
        Construct::new(ConstructFamily::Cast, type_name(&cast.to)),
    ];
    if cast.double_colon_syntax {
        constructs.push(Construct::new(ConstructFamily::Syntax, "double_colon_cast"));
    }
    constructs
}
