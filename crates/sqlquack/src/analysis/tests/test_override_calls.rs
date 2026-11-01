use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::models::Provenance;
use sqlquack_rules::conversion::models::{ConversionOptions, Converted};

use crate::analysis::_helpers::override_calls::overrides_applied;
use crate::analysis::_helpers::override_definition::load_override;
use crate::analysis::tests::test_types::OverrideCallsTestCase;

const UPPER_OVERRIDE: &str = "FUNCTION (\n  arguments (x VARCHAR),\n  returns VARCHAR,\n  replaces UPPER,\n  dialect duckdb,\n);\nupper(x)";

#[test]
fn given_an_override_when_converted_sql_skips_a_call_then_the_statement_is_refused() {
    let test_cases = [
        OverrideCallsTestCase {
            description: "each call reaches the override's macro",
            sql: "SELECT UPPER(s) AS a, UPPER(s) AS b FROM t",
            emitted: "SELECT \"__sqlquack_override__SHOUT\"(s), \"__sqlquack_override__SHOUT\"(s)",
            expected_refused: false,
        },
        OverrideCallsTestCase {
            description: "one call converted past the override by a built-in rule",
            sql: "SELECT UPPER(s) AS a, UPPER(s) AS b FROM t",
            emitted: "SELECT \"__sqlquack_override__SHOUT\"(s), upper(s)",
            expected_refused: true,
        },
        OverrideCallsTestCase {
            description: "a literal spelling the name is not a call",
            sql: "SELECT 'UPPER(s)' AS a FROM t",
            emitted: "SELECT 'UPPER(s)'",
            expected_refused: false,
        },
    ];
    let (replaces, replacement) = load_override("shout", UPPER_OVERRIDE).expect("loads");
    let mut options = ConversionOptions::default();
    options.overrides.insert(replaces, replacement);
    for test_case in test_cases {
        let statements =
            parse_source(test_case.sql, SourceDialect::Snowflake).expect("the statement parses");
        let converted = Converted {
            sql: test_case.emitted.to_owned(),
            columns: Vec::new(),
            provenance: Provenance::default(),
            raises: Default::default(),
        };
        let outcome = overrides_applied(converted, &statements[0], &options);
        assert_eq!(
            outcome
                .err()
                .is_some_and(|refusal| refusal.reason.contains("does not reach here")),
            test_case.expected_refused,
            "{}",
            test_case.description
        );
    }
}
