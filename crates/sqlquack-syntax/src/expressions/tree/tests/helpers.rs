use serde::Serialize;
use serde_json::Value;

use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::statements::models::Annotated;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::StructField;
use crate::tokens::models::Span;

pub(crate) fn json_at(value: &impl Serialize, pointer: &str) -> Value {
    serde_json::to_value(value)
        .expect("syntax nodes serialize")
        .pointer(pointer)
        .cloned()
        .unwrap_or(Value::Null)
}

pub(crate) fn same_json(left: &impl Serialize, right: &impl Serialize) -> bool {
    serde_json::to_value(left).expect("syntax nodes serialize")
        == serde_json::to_value(right).expect("syntax nodes serialize")
}

pub(crate) fn positioned_column(name: &str, span: Span) -> Expression {
    Expression::Column(Box::new(Column {
        name: Identifier::new(name).with_span(span),
        table: None,
        join_mark: false,
        trailing_comments: Vec::new(),
        span: Some(span),
        inferred_type: None,
    }))
}

pub(crate) fn annotation_round_trip(annotated: &Annotated) -> (bool, Option<Annotated>) {
    let json = serde_json::to_value(annotated).expect("annotations serialize");
    (
        json.get("type_assertion").is_some(),
        serde_json::from_value(json).ok(),
    )
}

pub(crate) fn bigint() -> DataType {
    DataType::BigInt { length: None }
}

pub(crate) fn varchar() -> DataType {
    DataType::VarChar {
        length: None,
        parenthesized_length: false,
    }
}

pub(crate) fn array_of(element: DataType) -> DataType {
    DataType::Array {
        element_type: Box::new(element),
        dimension: None,
    }
}

pub(crate) fn struct_of(a: DataType, b: DataType) -> DataType {
    DataType::Struct {
        fields: vec![
            StructField::new("a".into(), a),
            StructField::new("b".into(), b),
        ],
        nested: true,
    }
}
