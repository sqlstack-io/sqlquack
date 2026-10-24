//! `matching_type_equality`: the declared equality that admits a pair of result types.

use std::collections::BTreeMap;

use crate::policy::_helpers::type_patterns::parse_type_pattern;
use crate::policy::models::{PatternArgument, TypeEquality, TypePattern};

/// The first declared equality whose patterns match `snowflake` and `duckdb` with one binding.
pub fn matching_type_equality<'a>(
    equalities: &'a [TypeEquality],
    snowflake: &str,
    duckdb: &str,
) -> Option<&'a TypeEquality> {
    let (Ok(snowflake), Ok(duckdb)) = (parse_type_pattern(snowflake), parse_type_pattern(duckdb))
    else {
        return None;
    };
    equalities
        .iter()
        .find(|equality| admits(equality, &snowflake, &duckdb))
}

fn admits(equality: &TypeEquality, snowflake: &TypePattern, duckdb: &TypePattern) -> bool {
    let (Ok(snowflake_pattern), Ok(duckdb_pattern)) = (
        parse_type_pattern(&equality.snowflake),
        parse_type_pattern(&equality.duckdb),
    ) else {
        return false;
    };
    let Some(bindings) = bind(&snowflake_pattern, snowflake, BTreeMap::new()) else {
        return false;
    };
    bind(&duckdb_pattern, duckdb, bindings).is_some()
}

fn bind(
    pattern: &TypePattern,
    actual: &TypePattern,
    bindings: BTreeMap<String, i64>,
) -> Option<BTreeMap<String, i64>> {
    if pattern.base != actual.base || pattern.arguments.len() != actual.arguments.len() {
        return None;
    }
    let mut bound = bindings;
    for (expected, found) in pattern.arguments.iter().zip(&actual.arguments) {
        let consistent = match (expected, found) {
            (PatternArgument::Literal(expected), PatternArgument::Literal(found)) => {
                expected == found
            }
            (PatternArgument::Variable(name), PatternArgument::Literal(found)) => {
                *bound.entry(name.clone()).or_insert(*found) == *found
            }
            _ => false,
        };
        if !consistent {
            return None;
        }
    }
    Some(bound)
}
