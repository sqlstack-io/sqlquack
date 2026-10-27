use serde_json::Value;

use crate::expressions::names::models::Identifier;
use crate::expressions::statements::models::Annotated;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;

pub(crate) struct ExpressionEqualityTestCase {
    pub(crate) description: &'static str,
    pub(crate) left: Expression,
    pub(crate) right: Expression,
    pub(crate) expected_equal: bool,
    pub(crate) expected_same_json: bool,
}

pub(crate) struct IdentifierEqualityTestCase {
    pub(crate) description: &'static str,
    pub(crate) left: Identifier,
    pub(crate) right: Identifier,
    pub(crate) expected_equal: bool,
}

pub(crate) struct BuilderTestCase {
    pub(crate) description: &'static str,
    pub(crate) built: Expression,
    pub(crate) pointer: &'static str,
    pub(crate) expected_value: Value,
}

pub(crate) struct AnnotationTestCase {
    pub(crate) description: &'static str,
    pub(crate) annotated: Annotated,
    pub(crate) expected_round_trip: (bool, Option<Annotated>),
}

pub(crate) struct TypeAssertionTestCase {
    pub(crate) description: &'static str,
    pub(crate) inferred: Option<DataType>,
    pub(crate) asserted: DataType,
    pub(crate) expected_result: DataType,
}

pub(crate) struct UnknownTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) data_type: DataType,
    pub(crate) expected_contains_unknown: bool,
}
