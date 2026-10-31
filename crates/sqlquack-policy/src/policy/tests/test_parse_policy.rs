use crate::policy::tests::helpers::observed_error;
use crate::policy::tests::test_types::PolicyTestCase;

#[test]
fn given_policy_text_when_parsing_then_schema_and_rules_are_enforced() {
    let test_cases = [
        PolicyTestCase {
            description: "the base policy is valid",
            replace: "",
            with: "",
            expected_error: None,
        },
        PolicyTestCase {
            description: "an unknown top-level key is rejected",
            replace: "schema_version = 1",
            with: "schema_version = 1\noverrides = true",
            expected_error: Some("unknown field `overrides`"),
        },
        PolicyTestCase {
            description: "an unknown comparator key is rejected",
            replace: "timestamp_tz = \"instant\"",
            with: "timestamp_tz = \"instant\"\nfloat_tolerance = 0.5",
            expected_error: Some("unknown field `float_tolerance`"),
        },
        PolicyTestCase {
            description: "an unknown conversion budget key is rejected",
            replace: "size_ratio = 2000",
            with: "size_ratio = 2000\nseconds = 120",
            expected_error: Some("unknown field `seconds`"),
        },
        PolicyTestCase {
            description: "an unknown enumeration value is rejected",
            replace: "column_names = \"case_insensitive\"",
            with: "column_names = \"loose\"",
            expected_error: Some("unknown variant `loose`"),
        },
        PolicyTestCase {
            description: "another schema version is rejected",
            replace: "schema_version = 1",
            with: "schema_version = 2",
            expected_error: Some(
                "the policy file is invalid: schema_version is 2, but this build reads version 1",
            ),
        },
        PolicyTestCase {
            description: "a DuckDB pattern may not use unbound parameters",
            replace: "duckdb = \"VARCHAR\"",
            with: "duckdb = \"VARCHAR(m)\"",
            expected_error: Some(
                "the policy file is invalid: type_equality `VARCHAR(n)` = `VARCHAR(m)` uses \
                 parameters the Snowflake side does not bind: m",
            ),
        },
        PolicyTestCase {
            description: "strict mode may not be more lenient than the default",
            replace: "default = \"warn\"\nstrict = \"refuse\"",
            with: "default = \"refuse\"\nstrict = \"warn\"",
            expected_error: Some(
                "the policy file is invalid: divergence `hash` is treated more leniently in strict \
                 mode",
            ),
        },
        PolicyTestCase {
            description: "a divergence needs construct keys",
            replace: "constructs = [\"fn:HASH\"]",
            with: "constructs = [\"HASH\"]",
            expected_error: Some(
                "the policy file is invalid: divergence `hash` has an invalid construct key `HASH`",
            ),
        },
        PolicyTestCase {
            description: "a session profile must run DuckDB in UTC",
            replace: "duckdb_time_zone = \"UTC\"",
            with: "duckdb_time_zone = \"Europe/London\"",
            expected_error: Some(
                "the policy file is invalid: session profile `utc-v1` needs duckdb_time_zone = \"UTC\"",
            ),
        },
        PolicyTestCase {
            description: "a session profile must run Snowflake in UTC, spelled canonically",
            replace: "TIMEZONE = \"UTC\"",
            with: "TIMEZONE = \"utc\"",
            expected_error: Some(
                "the policy file is invalid: session profile `utc-v1` needs TIMEZONE = \"UTC\"",
            ),
        },
        PolicyTestCase {
            description: "an edge-case category must require something",
            replace: "required = [\"null\", \"zero\"]",
            with: "required = []",
            expected_error: Some(
                "the policy file is invalid: edge-case category `numeric` requires no input",
            ),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            observed_error(test_case.replace, test_case.with, test_case.expected_error).as_deref(),
            test_case.expected_error,
            "{}",
            test_case.description
        );
    }
}
