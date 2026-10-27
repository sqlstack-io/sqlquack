use serde_json::json;

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::queries::models::Select;
use crate::expressions::tree::models::Expression;
use crate::expressions::tree::tests::helpers::json_at;
use crate::expressions::tree::tests::helpers::positioned_column;
use crate::expressions::tree::tests::helpers::same_json;
use crate::expressions::tree::tests::test_types::BuilderTestCase;
use crate::expressions::tree::tests::test_types::ExpressionEqualityTestCase;
use crate::expressions::tree::tests::test_types::IdentifierEqualityTestCase;
use crate::tokens::models::Span;

#[test]
fn given_expressions_when_comparing_then_source_spans_are_metadata_not_identity() {
    let test_cases = [
        ExpressionEqualityTestCase {
            description: "a positioned column equals its synthetic twin but serializes its span",
            left: Expression::column("customer_id"),
            right: positioned_column("customer_id", Span::new(7, 18, 1, 19)),
            expected_equal: true,
            expected_same_json: false,
        },
        ExpressionEqualityTestCase {
            description: "columns with different names differ",
            left: Expression::column("customer_id"),
            right: Expression::column("other"),
            expected_equal: false,
            expected_same_json: false,
        },
        ExpressionEqualityTestCase {
            description: "a synthetic column serializes without spans",
            left: Expression::column("customer_id"),
            right: Expression::column("customer_id"),
            expected_equal: true,
            expected_same_json: true,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            (
                test_case.left == test_case.right,
                same_json(&test_case.left, &test_case.right)
            ),
            (test_case.expected_equal, test_case.expected_same_json),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_identifiers_when_comparing_then_quoting_matters_and_spans_do_not() {
    let test_cases = [
        IdentifierEqualityTestCase {
            description: "a span does not change identity",
            left: Identifier::new("a"),
            right: Identifier::new("a").with_span(Span::new(3, 4, 1, 5)),
            expected_equal: true,
        },
        IdentifierEqualityTestCase {
            description: "quoting changes identity",
            left: Identifier::new("a"),
            right: Identifier::quoted("a"),
            expected_equal: false,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            test_case.left == test_case.right,
            test_case.expected_equal,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_builders_when_building_then_nodes_carry_the_given_parts() {
    let test_cases = [
        BuilderTestCase {
            description: "a synthetic column has no span",
            built: Expression::column("customer_id"),
            pointer: "/column/span",
            expected_value: json!(null),
        },
        BuilderTestCase {
            description: "the select builder keeps its single projection",
            built: Expression::Select(Box::new(
                Select::new()
                    .column(Expression::star())
                    .from(Expression::Table(Box::new(TableRef::new("users")))),
            )),
            pointer: "/select/expressions",
            expected_value: json!([
                {"star": {"table": null, "except": null, "replace": null, "rename": null}}
            ]),
        },
        BuilderTestCase {
            description: "the select builder records its FROM table",
            built: Expression::Select(Box::new(
                Select::new()
                    .column(Expression::star())
                    .from(Expression::Table(Box::new(TableRef::new("users")))),
            )),
            pointer: "/select/from/expressions/0/table/name/name",
            expected_value: json!("users"),
        },
        BuilderTestCase {
            description: "alias wraps the expression under the given name",
            built: Expression::column("id").alias("user_id"),
            pointer: "/alias/alias/name",
            expected_value: json!("user_id"),
        },
        BuilderTestCase {
            description: "number builds a number literal",
            built: Expression::number(42),
            pointer: "/literal",
            expected_value: json!({"literal_type": "number", "value": "42"}),
        },
        BuilderTestCase {
            description: "string builds a string literal",
            built: Expression::string("hello"),
            pointer: "/literal",
            expected_value: json!({"literal_type": "string", "value": "hello"}),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            json_at(&test_case.built, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
