// The analysis-only type assertion `__polyglot_typed(expr, 'TYPE')`.

use crate::expressions::{Annotated, Select, StructField};
use crate::ported::compat::{parse_one, DataType, DialectType, Expression};

fn first_projection(sql: &str, dialect: DialectType) -> Expression {
    let Expression::Select(select) = parse_one(sql, dialect).unwrap() else {
        panic!("expected a SELECT: {sql}");
    };
    let Select { expressions, .. } = *select;
    expressions.into_iter().next().unwrap()
}

#[test]
fn malformed_typed_expressions_stay_ordinary_functions() {
    for sql in [
        "SELECT __polyglot_typed(amount) FROM orders",
        "SELECT __polyglot_typed(amount, 'BIGINT', 1) FROM orders",
        "SELECT __polyglot_typed(amount, 1) FROM orders",
        "SELECT __polyglot_typed(amount, note) FROM orders",
        "SELECT __polyglot_typed(amount, 'NOT A TYPE (') FROM orders",
        "SELECT __polyglot_typed(DISTINCT amount, 'BIGINT') FROM orders",
    ] {
        let item = first_projection(sql, DialectType::DuckDB);
        assert!(
            matches!(
                &item,
                Expression::Function(_) | Expression::AggregateFunction(_)
            ),
            "{sql}: {item:?}"
        );
    }
}

#[test]
fn comment_annotations_serialize_without_a_type_assertion() {
    let annotated = Annotated {
        this: Expression::Null(crate::expressions::Null),
        trailing_comments: vec!["c".into()],
        type_assertion: None,
        type_like: None,
    };
    let mut json = serde_json::to_value(&annotated).unwrap();
    assert!(json.get("type_assertion").is_none());
    json.as_object_mut().unwrap().remove("type_assertion");
    let restored: Annotated = serde_json::from_value(json).unwrap();
    assert_eq!(restored, annotated);
}

#[test]
fn type_assertions_apply_only_where_the_inferred_type_is_unknown() {
    use crate::expressions::type_assertion_result;
    let varchar = DataType::VarChar {
        length: None,
        parenthesized_length: false,
    };
    let bigint = DataType::BigInt { length: None };
    let unknown_array = DataType::Array {
        element_type: Box::new(DataType::Unknown),
        dimension: None,
    };
    assert_eq!(type_assertion_result(None, &varchar), &varchar);
    assert_eq!(
        type_assertion_result(Some(&DataType::Unknown), &varchar),
        &varchar
    );
    let bigint_array = DataType::Array {
        element_type: Box::new(bigint.clone()),
        dimension: None,
    };
    assert_eq!(
        type_assertion_result(Some(&unknown_array), &varchar),
        &unknown_array
    );
    assert_eq!(
        type_assertion_result(Some(&unknown_array), &bigint_array),
        &bigint_array
    );
    let partial_struct = DataType::Struct {
        fields: vec![
            StructField::new("a".into(), bigint.clone()),
            StructField::new("b".into(), DataType::Unknown),
        ],
        nested: true,
    };
    let full_struct = DataType::Struct {
        fields: vec![
            StructField::new("a".into(), bigint.clone()),
            StructField::new("b".into(), varchar.clone()),
        ],
        nested: true,
    };
    let other_struct = DataType::Struct {
        fields: vec![
            StructField::new("a".into(), varchar.clone()),
            StructField::new("b".into(), varchar.clone()),
        ],
        nested: true,
    };
    assert_eq!(
        type_assertion_result(Some(&partial_struct), &full_struct),
        &full_struct
    );
    assert_eq!(
        type_assertion_result(Some(&partial_struct), &other_struct),
        &partial_struct
    );
    assert_eq!(type_assertion_result(Some(&bigint), &varchar), &bigint);
    assert_eq!(
        type_assertion_result(Some(&DataType::Text), &varchar),
        &DataType::Text
    );
    assert!(unknown_array.contains_unknown());
    assert!(!bigint.contains_unknown());
}

mod analysis {}
