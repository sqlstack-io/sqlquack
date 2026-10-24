use polyglot_sql::Expression;
use polyglot_sql::expressions::DataType;
use serde_json::Value;

use crate::comparison::constants::SPELLING_ONLY_FIELDS;
use crate::comparison::main::compare_trees::compare_trees;
use crate::parsing::main::parse_source::parse_source;
use crate::parsing::models::SourceDialect;

pub(crate) fn parse_one(sql: &str) -> Expression {
    let mut statements = parse_source(sql, SourceDialect::Snowflake).expect("test SQL parses");
    assert_eq!(statements.len(), 1, "test SQL holds one statement");
    statements.remove(0)
}

fn raw(tree: &Expression) -> Value {
    serde_json::to_value(tree).expect("trees serialise")
}

fn contains_key(value: &Value, wanted: &str) -> bool {
    let in_object = value.as_object().is_some_and(|object| {
        object
            .iter()
            .any(|(key, inner)| key.contains(wanted) || contains_key(inner, wanted))
    });
    let in_array = value
        .as_array()
        .is_some_and(|items| items.iter().any(|item| contains_key(item, wanted)));
    in_object || in_array
}

pub(crate) fn raw_trees_equal(left: &Expression, right: &Expression) -> bool {
    raw(left) == raw(right)
}

pub(crate) fn field_present(left: &Expression, right: &Expression, field: &str) -> bool {
    contains_key(&raw(left), field) || contains_key(&raw(right), field)
}

pub(crate) fn difference_path(left: &Expression, right: &Expression) -> Option<String> {
    compare_trees(left, right).map(|difference| difference.path)
}

pub(crate) fn sql_difference_path(left: &str, right: &str) -> Option<String> {
    difference_path(&parse_one(left), &parse_one(right))
}

pub(crate) fn untested_fields(tested: &[&str]) -> Vec<&'static str> {
    SPELLING_ONLY_FIELDS
        .iter()
        .map(|field| field.name)
        .filter(|name| !tested.contains(name))
        .collect()
}

pub(crate) fn typed_column(name: &str) -> Expression {
    let mut column = Expression::column(name);
    column.set_inferred_type(DataType::Boolean);
    column
}
