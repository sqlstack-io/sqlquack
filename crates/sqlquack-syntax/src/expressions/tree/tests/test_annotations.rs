use crate::expressions::names::models::Null;
use crate::expressions::statements::models::Annotated;
use crate::expressions::tree::main::_type_assertion_result::type_assertion_result;
use crate::expressions::tree::models::Expression;
use crate::expressions::tree::tests::helpers::annotation_round_trip;
use crate::expressions::tree::tests::helpers::array_of;
use crate::expressions::tree::tests::helpers::bigint;
use crate::expressions::tree::tests::helpers::struct_of;
use crate::expressions::tree::tests::helpers::varchar;
use crate::expressions::tree::tests::test_types::AnnotationTestCase;
use crate::expressions::tree::tests::test_types::TypeAssertionTestCase;
use crate::expressions::tree::tests::test_types::UnknownTypeTestCase;
use crate::expressions::types::models::DataType;

#[test]
fn given_a_comment_annotation_when_serializing_then_it_has_no_type_assertion_and_round_trips() {
    let comment_only = Annotated {
        this: Expression::Null(Null),
        trailing_comments: vec!["c".into()],
        type_assertion: None,
        type_like: None,
    };
    let test_cases = [AnnotationTestCase {
        description: "a comment-only annotation omits the assertion and reads back equal",
        annotated: comment_only.clone(),
        expected_round_trip: (false, Some(comment_only)),
    }];

    for test_case in test_cases {
        assert_eq!(
            annotation_round_trip(&test_case.annotated),
            test_case.expected_round_trip,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_type_assertion_when_applying_it_then_only_unknown_parts_are_replaced() {
    let test_cases = [
        TypeAssertionTestCase {
            description: "no inferred type takes the assertion",
            inferred: None,
            asserted: varchar(),
            expected_result: varchar(),
        },
        TypeAssertionTestCase {
            description: "an unknown type takes the assertion",
            inferred: Some(DataType::Unknown),
            asserted: varchar(),
            expected_result: varchar(),
        },
        TypeAssertionTestCase {
            description: "a scalar assertion does not replace an array",
            inferred: Some(array_of(DataType::Unknown)),
            asserted: varchar(),
            expected_result: array_of(DataType::Unknown),
        },
        TypeAssertionTestCase {
            description: "an array assertion fills an unknown element",
            inferred: Some(array_of(DataType::Unknown)),
            asserted: array_of(bigint()),
            expected_result: array_of(bigint()),
        },
        TypeAssertionTestCase {
            description: "a compatible struct fills its unknown field",
            inferred: Some(struct_of(bigint(), DataType::Unknown)),
            asserted: struct_of(bigint(), varchar()),
            expected_result: struct_of(bigint(), varchar()),
        },
        TypeAssertionTestCase {
            description: "a conflicting struct is not applied",
            inferred: Some(struct_of(bigint(), DataType::Unknown)),
            asserted: struct_of(varchar(), varchar()),
            expected_result: struct_of(bigint(), DataType::Unknown),
        },
        TypeAssertionTestCase {
            description: "a known type keeps itself",
            inferred: Some(bigint()),
            asserted: varchar(),
            expected_result: bigint(),
        },
        TypeAssertionTestCase {
            description: "a known text type keeps itself",
            inferred: Some(DataType::Text),
            asserted: varchar(),
            expected_result: DataType::Text,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            type_assertion_result(test_case.inferred.as_ref(), &test_case.asserted),
            &test_case.expected_result,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_data_type_when_checking_for_unknown_parts_then_nested_unknowns_count() {
    let test_cases = [
        UnknownTypeTestCase {
            description: "an array of unknown contains an unknown",
            data_type: array_of(DataType::Unknown),
            expected_contains_unknown: true,
        },
        UnknownTypeTestCase {
            description: "a known scalar does not",
            data_type: bigint(),
            expected_contains_unknown: false,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            test_case.data_type.contains_unknown(),
            test_case.expected_contains_unknown,
            "{}",
            test_case.description
        );
    }
}
