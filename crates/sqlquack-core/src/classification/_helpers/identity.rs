//! The constructs a single node stands for, apart from its clauses.

use polyglot_sql::expressions::{Cast, DataType};
use polyglot_sql::{DialectType, Expression};

use crate::classification::_helpers::names::base_type_name;
use crate::classification::constants::{PROJECT_FUNCTION_CONSTRUCT, PROJECT_FUNCTION_PLACEHOLDER};
use crate::classification::models::{Construct, ConstructFamily};

pub(crate) fn node_constructs(node: &Expression, dialect: DialectType) -> Vec<Construct> {
    match node {
        Expression::Function(function) => vec![function_construct(&function.name)],
        Expression::AggregateFunction(function) => vec![function_construct(&function.name)],
        Expression::Cast(cast) | Expression::TryCast(cast) | Expression::SafeCast(cast) => {
            cast_constructs(node.variant_name(), cast, dialect)
        }
        other => vec![Construct::new(ConstructFamily::Node, other.variant_name())],
    }
}

fn function_construct(name: &str) -> Construct {
    if name.eq_ignore_ascii_case(PROJECT_FUNCTION_PLACEHOLDER) {
        return Construct::new(ConstructFamily::Function, PROJECT_FUNCTION_CONSTRUCT);
    }
    Construct::new(ConstructFamily::Function, name.to_ascii_uppercase())
}

fn cast_constructs(variant: &str, cast: &Cast, dialect: DialectType) -> Vec<Construct> {
    let mut constructs = vec![
        Construct::new(ConstructFamily::Node, variant),
        Construct::new(ConstructFamily::Cast, type_name(&cast.to, dialect)),
    ];
    if cast.double_colon_syntax {
        constructs.push(Construct::new(ConstructFamily::Syntax, "double_colon_cast"));
    }
    constructs
}

fn type_name(data_type: &DataType, dialect: DialectType) -> String {
    match polyglot_sql::generate_data_type(data_type, dialect) {
        Ok(rendered) => base_type_name(&rendered),
        Err(_) => "<unprintable>".to_owned(),
    }
}
