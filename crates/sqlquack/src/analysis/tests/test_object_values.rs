use crate::analysis::tests::helpers::shop_first_outcome;
use crate::analysis::tests::test_types::FirstValueTestCase;

#[test]
fn given_a_star_or_a_scaled_number_when_building_an_object_then_it_converts_as_recorded() {
    let test_cases = [
        FirstValueTestCase {
            description: "a star with EXCLUDE takes the other columns as keys named as resolved",
            sql: "SELECT OBJECT_CONSTRUCT_KEEP_NULL(o.* EXCLUDE (amount))::VARCHAR AS v \
                  FROM orders AS o ORDER BY o.customer_id",
            expected_outcome: Ok("{\"CUSTOMER_ID\":1}"),
        },
        FirstValueTestCase {
            description: "a quoted EXCLUDE name, whose quoting the tree drops, is refused",
            sql: "SELECT OBJECT_CONSTRUCT(o.* EXCLUDE (\"customer_id\"))::VARCHAR AS v \
                  FROM orders AS o",
            expected_outcome: Err("quotes a name"),
        },
        FirstValueTestCase {
            description: "a block comment before a quoted EXCLUDE list does not hide it",
            sql: "SELECT OBJECT_CONSTRUCT(o.* EXCLUDE /* c */ (\"customer_id\"))::VARCHAR AS v \
                  FROM orders AS o",
            expected_outcome: Err("quotes a name"),
        },
        FirstValueTestCase {
            description: "a line comment before a quoted EXCLUDE list does not hide it",
            sql: "SELECT OBJECT_CONSTRUCT(o.* EXCLUDE -- c\n (\"customer_id\"))::VARCHAR AS v \
                  FROM orders AS o",
            expected_outcome: Err("quotes a name"),
        },
        FirstValueTestCase {
            description: "a quote inside a comment after EXCLUDE is not a quoted name",
            sql: "SELECT OBJECT_CONSTRUCT(o.* EXCLUDE /* \" */ (customer_id))::VARCHAR AS v \
                  FROM orders AS o",
            expected_outcome: Ok("{\"AMOUNT\":5}"),
        },
        FirstValueTestCase {
            description: "a quoted upper-case EXCLUDE name is refused too",
            sql: "SELECT OBJECT_CONSTRUCT(o.* EXCLUDE \"CUSTOMER_ID\")::VARCHAR AS v FROM orders AS o",
            expected_outcome: Err("quotes a name"),
        },
        FirstValueTestCase {
            description: "an EXCLUDE entry the star does not read is refused",
            sql: "SELECT OBJECT_CONSTRUCT(o.* EXCLUDE (missing_column)) AS v FROM orders AS o",
            expected_outcome: Err("EXCLUDE names"),
        },
        FirstValueTestCase {
            description: "a whole NUMBER with a scale is written without a point",
            sql: "SELECT OBJECT_CONSTRUCT('a', 100.00::NUMBER(5,2))::VARCHAR AS v",
            expected_outcome: Ok("{\"a\":100}"),
        },
        FirstValueTestCase {
            description: "a NUMBER with a scale drops trailing fraction zeros",
            sql: "SELECT OBJECT_CONSTRUCT('a', (-87.50)::NUMBER(5,2))::VARCHAR AS v",
            expected_outcome: Ok("{\"a\":-87.5}"),
        },
        FirstValueTestCase {
            description: "TYPEOF where a NUMBER with a scale entered is refused",
            sql: "SELECT TYPEOF(OBJECT_CONSTRUCT('a', 1.50::NUMBER(5,2)):a) AS v",
            expected_outcome: Err("whole NUMBER with a scale"),
        },
        FirstValueTestCase {
            description: "a whole NUMBER with a scale equals the VARIANT integer, as recorded",
            sql: "SELECT OBJECT_CONSTRUCT('a', 100.00::NUMBER(5,2)):a = PARSE_JSON('100') AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "an integer is not found among fractions that are not whole, as recorded",
            sql: "SELECT ARRAY_CONTAINS(PARSE_JSON('-7'), ARRAY_CONSTRUCT(87.50::NUMBER(5,2))) AS v",
            expected_outcome: Ok("false"),
        },
        FirstValueTestCase {
            description: "ARRAY_POSITION finds a whole decimal among fractions, as recorded",
            sql: "SELECT ARRAY_POSITION(PARSE_JSON('0'), \
                  ARRAY_CONSTRUCT(87.50::NUMBER(5,2), 0.00::NUMBER(5,2))) AS v",
            expected_outcome: Ok("1"),
        },
        FirstValueTestCase {
            description: "a fraction looked for in an array still raises",
            sql: "SELECT ARRAY_CONTAINS(OBJECT_CONSTRUCT('a', 1.5::NUMBER(5,2)):a, \
                  ARRAY_CONSTRUCT(1)) AS v",
            expected_outcome: Err("json_fidelity"),
        },
        FirstValueTestCase {
            description: "a NUMBER with a scale beside a FLOAT in one VARIANT is refused",
            sql: "SELECT OBJECT_CONSTRUCT('a', 1.50::NUMBER(5,2), 'b', 2.5::FLOAT) AS v",
            expected_outcome: Err("beside a FLOAT"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            shop_first_outcome(test_case.sql, test_case.expected_outcome),
            test_case
                .expected_outcome
                .map(str::to_owned)
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
