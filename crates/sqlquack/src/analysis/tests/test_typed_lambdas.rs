use crate::analysis::tests::helpers::shop_first_outcome;
use crate::analysis::tests::test_types::FirstValueTestCase;

#[test]
fn given_a_lambda_with_typed_parameters_when_converting_then_transform_converts_and_reduce_refuses()
{
    let test_cases = [
        FirstValueTestCase {
            description: "TRANSFORM with a typed VARCHAR parameter converts",
            sql: "SELECT ARRAY_TO_STRING(TRANSFORM(SPLIT('1&2&10', '&'), leg VARCHAR -> leg::INT), ',') AS v",
            expected_outcome: Ok("1,2,10"),
        },
        FirstValueTestCase {
            description: "TRANSFORM with a parenthesized typed VARCHAR parameter converts",
            sql: "SELECT ARRAY_TO_STRING(TRANSFORM(SPLIT('1&2&10', '&'), (leg VARCHAR) -> leg::INT), ',') AS v",
            expected_outcome: Ok("1,2,10"),
        },
        FirstValueTestCase {
            description: "REDUCE with typed parameters is refused",
            sql: "SELECT REDUCE([1, 2, 3], 0, (acc NUMBER(18, 2), i VARIANT) -> acc + i) AS v",
            expected_outcome: Err("REDUCE"),
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
