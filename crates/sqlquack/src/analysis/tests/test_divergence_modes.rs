use sqlquack_core::refusals::models::RefusalCategory;

use crate::analysis::tests::helpers::{divergence_outcome, numeric_range, python_outcome};
use crate::analysis::tests::test_types::{
    DivergenceModeOutcome, DivergenceModeTestCase, PythonModeTestCase,
};

#[test]
fn given_a_known_divergence_when_converting_then_default_mode_warns_and_strict_mode_refuses() {
    let test_cases = [
        DivergenceModeTestCase {
            description: "comparing NUMBER(38,0) with NUMBER(10,2) relies on numeric_range by default",
            sql: "SELECT customer_id = amount AS same FROM orders",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: numeric_range("node:eq"),
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "the same widening comparison is refused in strict mode",
            sql: "SELECT customer_id = amount AS same FROM orders",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
        DivergenceModeTestCase {
            description: "NVL unifying NUMBER(10,2) with NUMBER(38,0) is refused in strict mode",
            sql: "SELECT NVL(amount, customer_id) AS v FROM orders",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
        DivergenceModeTestCase {
            description: "TO_DATE of text in any spelling is refused in strict mode",
            sql: "SELECT TO_DATE(note) AS d FROM customers",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
        DivergenceModeTestCase {
            description: "HASH is refused by default, as its value is never Snowflake's",
            sql: "SELECT HASH(note, customer_id) AS h FROM customers",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::TypeCondition],
            },
        },
        DivergenceModeTestCase {
            description: "HASH is refused in strict mode",
            sql: "SELECT HASH(note) AS h FROM customers",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
        DivergenceModeTestCase {
            description: "SUM of NUMBER(38,0) converts by default, naming the divergence and construct",
            sql: "SELECT SUM(customer_id) AS total FROM customers",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: numeric_range("fn:SUM"),
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "the same SUM is refused in strict mode",
            sql: "SELECT SUM(customer_id) AS total FROM customers",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::TypeCondition],
            },
        },
        DivergenceModeTestCase {
            description: "dividing by NUMBER(38,0) converts by default with a runtime guard",
            sql: "SELECT amount / customer_id AS share FROM orders",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: numeric_range("node:div"),
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "adding to NUMBER(38,0) converts by default, relying on numeric_range",
            sql: "SELECT customer_id + 1 AS next_id FROM customers",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: numeric_range("node:add"),
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "subtracting from NUMBER(38,0) is refused in strict mode",
            sql: "SELECT customer_id - 1 AS previous_id FROM customers",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::TypeCondition],
            },
        },
        DivergenceModeTestCase {
            description: "multiplying NUMBER(38,0) relies on numeric_range by default",
            sql: "SELECT customer_id * 2 AS doubled FROM customers",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: numeric_range("node:mul"),
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "parsing JSON text converts by default, TYPEOF guarding typed values",
            sql: "SELECT TYPEOF(PARSE_JSON(note)) AS kind FROM customers",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![
                    ("json_fidelity".to_owned(), vec!["fn:PARSE_JSON".to_owned()]),
                    (
                        "variant_typed_values".to_owned(),
                        vec!["fn:TYPEOF".to_owned()],
                    ),
                ],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "the same parse is refused in strict mode",
            sql: "SELECT TYPEOF(TRY_PARSE_JSON(note)) AS kind FROM customers",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
        DivergenceModeTestCase {
            description: "a VARIANT table column is guarded, and its path cast relies on json_fidelity",
            sql: "SELECT payload:order.id::VARCHAR AS order_id FROM events",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![(
                    "json_fidelity".to_owned(),
                    vec!["node:cast".to_owned(), "node:column".to_owned()],
                )],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "LATERAL FLATTEN converts by default, relying on json_fidelity",
            sql: "SELECT f.index AS position FROM events e, LATERAL FLATTEN(input => e.payload:items) f",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![(
                    "json_fidelity".to_owned(),
                    vec!["fn:FLATTEN".to_owned(), "node:column".to_owned()],
                )],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "LATERAL FLATTEN is refused in strict mode",
            sql: "SELECT f.index AS position FROM events e, LATERAL FLATTEN(input => e.payload:items) f",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
        DivergenceModeTestCase {
            description: "TRY_TO_DATE of text converts by default, relying on date_text_spelling",
            sql: "SELECT TRY_TO_DATE(note) AS noted_on FROM customers",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![(
                    "date_text_spelling".to_owned(),
                    vec!["fn:TRY_TO_DATE".to_owned()],
                )],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "TRY_CAST of text to DATE is refused in strict mode",
            sql: "SELECT TRY_CAST(note AS DATE) AS noted_on FROM customers",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::TypeCondition],
            },
        },
        DivergenceModeTestCase {
            description: "a VARIANT result column converts as JSON, relying on json_fidelity",
            sql: "SELECT payload:order AS order_object FROM events",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![("json_fidelity".to_owned(), vec!["node:column".to_owned()])],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "ordering by a VARIANT relies on array_sort_order's recorded kind order",
            sql: "SELECT note FROM customers ORDER BY PARSE_JSON(note)",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![
                    (
                        "array_sort_order".to_owned(),
                        vec!["node:ordered".to_owned()],
                    ),
                    ("json_fidelity".to_owned(), vec!["fn:PARSE_JSON".to_owned()]),
                ],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "a year-1 literal at precision 9 always raises in DuckDB, so it is refused by default",
            sql: "SELECT CAST('0001-01-01 00:00:00' AS TIMESTAMP_NTZ(9)) AS t",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::TypeCondition],
            },
        },
        DivergenceModeTestCase {
            description: "a far-future sentinel default is refused by default, not left to raise at run time",
            sql: "SELECT COALESCE(NULL::TIMESTAMP_NTZ, '9999-12-31'::TIMESTAMP_NTZ) AS t",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::TypeCondition],
            },
        },
        DivergenceModeTestCase {
            description: "the same literal is refused in strict mode",
            sql: "SELECT CAST('0001-01-01 00:00:00' AS TIMESTAMP_NTZ(9)) AS t",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::TypeCondition],
            },
        },
        DivergenceModeTestCase {
            description: "an in-range nanosecond literal needs no divergence, even in strict mode",
            sql: "SELECT CAST('2024-02-29 13:45:01.123456789' AS TIMESTAMP_NTZ(9)) AS t",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "a zoned nanosecond cast relies on timestamp_tz_submicro by default",
            sql: "SELECT CAST(CAST('2024-02-29 13:45:01.1' AS TIMESTAMP_NTZ(9)) AS TIMESTAMP_TZ(9)) AS t",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![
                    (
                        "session_timezone_utc".to_owned(),
                        vec!["node:cast".to_owned()],
                    ),
                    (
                        "timestamp_tz_submicro".to_owned(),
                        vec!["node:cast".to_owned()],
                    ),
                ],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "a cast keeping only microseconds needs no timestamp_tz_submicro, only \
                          the UTC session every naive-to-zoned cast relies on",
            sql: "SELECT CAST(CAST('2024-02-29 13:45:01.1' AS TIMESTAMP_NTZ(9)) AS TIMESTAMP_TZ(6)) AS t",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![(
                    "session_timezone_utc".to_owned(),
                    vec!["node:cast".to_owned()],
                )],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "SUM of NUMBER(10,2) never needs the divergence",
            sql: "SELECT SUM(amount) AS total FROM orders",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "a source VARIANT compared by text relies on source_variant_json by default",
            sql: "SELECT payload = PARSE_JSON('{\"k\":\"a\"}') AS same FROM events",
            strict: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![
                    (
                        "json_fidelity".to_owned(),
                        vec![
                            "fn:PARSE_JSON".to_owned(),
                            "node:column".to_owned(),
                            "node:eq".to_owned(),
                        ],
                    ),
                    ("source_variant_json".to_owned(), vec!["node:eq".to_owned()]),
                ],
                categories: vec![],
            },
        },
        DivergenceModeTestCase {
            description: "strict mode refuses a source VARIANT compared by text",
            sql: "SELECT payload = PARSE_JSON('{\"k\":\"a\"}') AS same FROM events",
            strict: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            divergence_outcome(test_case.sql, test_case.strict),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_python_function_call_when_converting_then_it_needs_a_confirmed_runtime_and_names_its_label()
 {
    let test_cases = [
        PythonModeTestCase {
            description: "without a Python runtime the call is refused",
            sql: "SELECT is_gift(note) AS gift FROM customers",
            strict: false,
            runtime: false,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
        PythonModeTestCase {
            description: "with a runtime the call relies on project_udf:<NAME> and python_value_crossing",
            sql: "SELECT is_gift(note) AS gift FROM customers",
            strict: false,
            runtime: true,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![
                    (
                        "project_udf:IS_GIFT".to_owned(),
                        vec!["fn:IS_GIFT".to_owned()],
                    ),
                    (
                        "python_value_crossing".to_owned(),
                        vec!["fn:IS_GIFT".to_owned()],
                    ),
                ],
                categories: vec![],
            },
        },
        PythonModeTestCase {
            description: "strict mode refuses the call even with a runtime, as its crossing can raise",
            sql: "SELECT is_gift(note) AS gift FROM customers",
            strict: true,
            runtime: true,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            python_outcome(test_case.sql, (test_case.strict, test_case.runtime)),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
