use crate::analysis::tests::helpers::shop_row_count;
use crate::analysis::tests::test_types::StructuredTypeTestCase;

#[test]
fn given_a_cast_to_a_structured_type_when_converting_then_it_is_refused_as_not_supported_yet() {
    let test_cases = [
        StructuredTypeTestCase {
            description: "a cast to ARRAY of a type",
            sql: "SELECT note::ARRAY(VARCHAR) AS a FROM customers",
            expected_refusal: Err(vec![
                "casting to the structured type ARRAY(VARCHAR) is not supported yet".to_owned()
            ]),
        },
        StructuredTypeTestCase {
            description: "a CAST to OBJECT with fields",
            sql: "SELECT CAST(payload AS OBJECT(a VARCHAR NOT NULL, b NUMBER(10,2))) AS o FROM events",
            expected_refusal: Err(vec![
                "casting to the structured type OBJECT(a VARCHAR NOT NULL, b NUMBER(10, 2)) is not supported yet".to_owned()
            ]),
        },
        StructuredTypeTestCase {
            description: "a TRY_CAST to MAP",
            sql: "SELECT TRY_CAST(payload AS MAP(VARCHAR, NUMBER)) AS m FROM events",
            expected_refusal: Err(vec![
                "casting to the structured type MAP(VARCHAR, NUMBER) is not supported yet".to_owned()
            ]),
        },
        StructuredTypeTestCase {
            description: "a nested structured type",
            sql: "SELECT payload::ARRAY(OBJECT(k ARRAY(INT))) AS n FROM events",
            expected_refusal: Err(vec![
                "casting to the structured type ARRAY(OBJECT(k ARRAY(INT))) is not supported yet".to_owned()
            ]),
        },
        StructuredTypeTestCase {
            description: "a plain ARRAY stays the semi-structured type",
            sql: "SELECT payload::ARRAY AS a FROM events",
            expected_refusal: Ok(2),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            shop_row_count(test_case.sql),
            test_case.expected_refusal,
            "{}",
            test_case.description
        );
    }
}
