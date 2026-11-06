use crate::checking::errors::CheckStage;
use crate::checking::main::bound_outside_lambdas::bound_outside_lambdas;
use crate::checking::tests::helpers::failed_stage;
use crate::checking::tests::test_types::BindingTestCase;

const LATEST: &str = "(SELECT max(\"PLACED\") FROM \"ORDERS\")";

#[test]
fn given_a_subquery_in_a_lambda_body_when_bound_outside_lambdas_then_duckdb_binds_it() {
    let test_cases = [
        BindingTestCase {
            description: "a subquery read in a lambda body is bound outside it",
            sql: "list_transform([1], lambda n: (SELECT max(\"PLACED\") FROM \"ORDERS\") \
                  + to_days(n))",
            value: LATEST,
            expected_binding: Ok(true),
            expected_stages: (Some(CheckStage::Bind), None),
        },
        BindingTestCase {
            description: "inner lambdas read the outer parameter, two levels deep",
            sql: "list_transform([1], lambda n: list_transform([n], lambda m: \
                  (SELECT max(\"PLACED\") FROM \"ORDERS\") + to_days(m))[1])[1]",
            value: LATEST,
            expected_binding: Ok(true),
            expected_stages: (Some(CheckStage::Bind), None),
        },
        BindingTestCase {
            description: "a subquery only in the list a lambda reads is left as it is",
            sql: "list_transform([(SELECT max(\"PLACED\") FROM \"ORDERS\")], lambda n: \
                  n)[1]",
            value: LATEST,
            expected_binding: Ok(false),
            expected_stages: (None, None),
        },
        BindingTestCase {
            description: "a comma at the lambda's depth ends its body, before the initial value",
            sql: "list_reduce([1, 2], lambda a, b: a + b, \
                  epoch_us((SELECT max(\"PLACED\") FROM \"ORDERS\")))",
            value: "epoch_us((SELECT max(\"PLACED\") FROM \"ORDERS\"))",
            expected_binding: Ok(false),
            expected_stages: (None, None),
        },
        BindingTestCase {
            description: "lambda text and brackets inside literals and quoted names do not count",
            sql: "'lambda x: (' AS \"lambda y: [\", (SELECT max(\"PLACED\") FROM \"ORDERS\") \
                 ",
            value: LATEST,
            expected_binding: Ok(false),
            expected_stages: (None, None),
        },
        BindingTestCase {
            description: "a subquery reading the parameter of the lambda around it cannot move",
            sql: "list_transform([1], lambda n: (SELECT n + 1))[1]",
            value: "(SELECT n + 1)",
            expected_binding: Err("n"),
            expected_stages: (Some(CheckStage::Bind), Some(CheckStage::Bind)),
        },
        BindingTestCase {
            description: "a lambda parameter list's comma does not end the body around it",
            sql: "list_transform([1], lambda n: list_reduce([n, 2], lambda a, b: a + b + \
                  epoch_us((SELECT max(\"PLACED\") FROM \"ORDERS\"))))[1]",
            value: "(SELECT max(\"PLACED\") FROM \"ORDERS\")",
            expected_binding: Ok(true),
            expected_stages: (Some(CheckStage::Bind), None),
        },
        BindingTestCase {
            description: "a parameter the value binds itself is not read from outside",
            sql: "list_transform([1], lambda n: (SELECT list_transform([1], lambda n: n)[1]))[1]",
            value: "(SELECT list_transform([1], lambda n: n)[1])",
            expected_binding: Ok(true),
            expected_stages: (Some(CheckStage::Bind), None),
        },
    ];
    for test_case in test_cases {
        let binding = bound_outside_lambdas(test_case.sql, test_case.value, "subquery_value_1");
        assert_eq!(
            binding.as_ref().map(Option::is_some).map_err(String::as_str),
            test_case.expected_binding,
            "{}",
            test_case.description
        );
        let printed = binding
            .ok()
            .flatten()
            .unwrap_or_else(|| test_case.sql.to_owned());
        let checked =
            |expression: &str| failed_stage(&format!("SELECT {expression} FROM \"ORDERS\""));
        assert_eq!(
            (checked(test_case.sql), checked(&printed)),
            test_case.expected_stages,
            "{}",
            test_case.description
        );
    }
}
