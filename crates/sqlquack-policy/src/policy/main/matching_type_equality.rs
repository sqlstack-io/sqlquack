//! `matching_type_equality`: the declared equality that admits a pair of result types.

use std::collections::BTreeMap;

use crate::policy::main::parse_type_pattern::parse_type_pattern;
use crate::policy::models::{PatternArgument, TypeEquality, TypePattern};

/// The first declared equality whose patterns match `snowflake` and `duckdb` with one binding.
pub fn matching_type_equality<'a>(
    equalities: &'a [TypeEquality],
    snowflake: &str,
    duckdb: &str,
) -> Option<&'a TypeEquality> {
    let snowflake = parse_type_pattern(snowflake).ok()?;
    let duckdb = parse_type_pattern(duckdb).ok()?;
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
    let mut bindings: BTreeMap<String, i64> = BTreeMap::new();
    bind(&snowflake_pattern, snowflake, &mut bindings)
        && bind(&duckdb_pattern, duckdb, &mut bindings)
}

fn bind(pattern: &TypePattern, actual: &TypePattern, bindings: &mut BTreeMap<String, i64>) -> bool {
    if pattern.base != actual.base || pattern.arguments.len() != actual.arguments.len() {
        return false;
    }
    pattern
        .arguments
        .iter()
        .zip(&actual.arguments)
        .all(|(expected, found)| match (expected, found) {
            (PatternArgument::Literal(expected), PatternArgument::Literal(found)) => {
                expected == found
            }
            (PatternArgument::Variable(name), PatternArgument::Literal(found)) => {
                *bindings.entry(name.clone()).or_insert(*found) == *found
            }
            _ => false,
        })
}
