use sqlquack_syntax::Dialect;
use std::collections::BTreeSet;

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;

use crate::classification::main::classify::classify;
use crate::classification::models::SourceDialect;
use crate::classification::models::{ConstructFamily, Occurrence};
use sqlquack_parse::parsing::main::parse_source::parse_source;
use sqlquack_syntax::tokens::models::SourceLines;

pub(crate) fn parse_one(sql: &str) -> Expression {
    let mut statements = parse_source(sql, Dialect::Snowflake).expect("test SQL parses");
    assert_eq!(statements.len(), 1, "test SQL holds one statement");
    statements.remove(0)
}

fn occurrences(sql: &str) -> Vec<Occurrence> {
    classify(
        &parse_one(sql),
        &SourceLines::new(sql),
        SourceDialect::Snowflake,
    )
}

pub(crate) fn missing_keys(sql: &str, expected: &[&str]) -> Vec<String> {
    let found: BTreeSet<String> = occurrences(sql)
        .into_iter()
        .map(|occurrence| occurrence.construct.key())
        .collect();
    expected
        .iter()
        .filter(|key| !found.contains(**key))
        .map(|key| (*key).to_owned())
        .collect()
}

pub(crate) fn unclassified_nodes(sql: &str) -> usize {
    let identities = occurrences(sql)
        .iter()
        .filter(|occurrence| {
            [ConstructFamily::Node, ConstructFamily::Function]
                .contains(&occurrence.construct.family)
        })
        .count();
    parse_one(sql).dfs().count().abs_diff(identities)
}

pub(crate) fn unlocated_constructs(sql: &str) -> usize {
    occurrences(sql)
        .iter()
        .filter(|occurrence| occurrence.location.is_none())
        .count()
}

pub(crate) fn line_and_column(sql: &str, key: &str) -> Option<[usize; 2]> {
    occurrences(sql)
        .into_iter()
        .find(|occurrence| occurrence.construct.key() == key)
        .and_then(|occurrence| occurrence.location)
        .map(|location| [location.line, location.column])
}
