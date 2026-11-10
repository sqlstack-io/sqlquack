use crate::macros::main::all_helper_statements::all_helper_statements;
use crate::macros::main::explain_helper::explain_helper;
use crate::macros::main::helper_statements::helper_statements;
use crate::macros::tests::test_types::{
    CreationTestCase, ExplainTestCase, HelperListTestCase, PinnedDefinitionTestCase,
    UnsitedRaiseTestCase,
};

#[test]
fn given_sql_when_listing_helpers_then_only_called_ones_come_dependencies_first() {
    let test_cases = [
        HelperListTestCase {
            description: "SQL calling no helper needs no statement",
            sql: r#"SELECT CAST("P" AS VARCHAR)"#,
            expected_heads: vec![],
        },
        HelperListTestCase {
            description: "variant_text_v1 calls json_escaped_v1, so its definition comes first",
            sql: r#"SELECT sqlquack.variant_text_v1('#1', "P")"#,
            expected_heads: vec![
                "CREATE SCHEMA IF NOT EXISTS sqlquack",
                "CREATE OR REPLACE MACRO sqlquack.json_escaped_v1",
                "CREATE OR REPLACE MACRO sqlquack.variant_text_v1",
            ],
        },
        HelperListTestCase {
            description: "a line break between a helper's name and its parenthesis is still a call",
            sql: "SELECT sqlquack.json_escaped_v1\n    (\"P\")",
            expected_heads: vec![
                "CREATE SCHEMA IF NOT EXISTS sqlquack",
                "CREATE OR REPLACE MACRO sqlquack.json_escaped_v1",
            ],
        },
        HelperListTestCase {
            description: "a longer name beginning with a helper's name is not a call of it",
            sql: r#"SELECT sqlquack.json_escaped_v10("P")"#,
            expected_heads: vec![],
        },
    ];
    for test_case in test_cases {
        let statements = helper_statements(&[test_case.sql]);
        let heads: Vec<&str> = statements
            .iter()
            .map(|statement| statement.split('(').next().unwrap_or_default())
            .collect();
        assert_eq!(heads, test_case.expected_heads, "{}", test_case.description);
    }
}

#[test]
fn given_every_helper_when_created_in_duckdb_then_each_run_succeeds() {
    let test_cases = [
        CreationTestCase {
            description: "every helper is created on a fresh connection",
            runs: 1,
            expected_failures: vec![],
        },
        CreationTestCase {
            description: "creating every helper again replaces it",
            runs: 2,
            expected_failures: vec![],
        },
    ];
    for test_case in test_cases {
        let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
        let failures: Vec<String> = std::iter::repeat_n(all_helper_statements(), test_case.runs)
            .flatten()
            .filter_map(|statement| {
                connection
                    .execute_batch(&statement)
                    .err()
                    .map(|error| format!("{statement}: {error}"))
            })
            .collect();
        assert_eq!(
            failures, test_case.expected_failures,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_name_when_explaining_then_only_helper_macros_explain_with_or_without_schema() {
    let test_cases = [
        ExplainTestCase {
            description: "a helper macro's bare name",
            key: "variant_number_v1",
            expected_first_line: Some(
                "sqlquack.variant_number_v1(quack_site, quack_value, quack_type, quack_largest): \
                 helper macro",
            ),
        },
        ExplainTestCase {
            description: "a helper macro's name with its schema",
            key: "sqlquack.variant_number_v1",
            expected_first_line: Some(
                "sqlquack.variant_number_v1(quack_site, quack_value, quack_type, quack_largest): \
                 helper macro",
            ),
        },
        ExplainTestCase {
            description: "a version no helper macro has",
            key: "variant_number_v2",
            expected_first_line: None,
        },
        ExplainTestCase {
            description: "a diagnostic code",
            key: "SQ0203",
            expected_first_line: None,
        },
    ];
    for test_case in test_cases {
        let explained = explain_helper(test_case.key);
        assert_eq!(
            explained.as_deref().and_then(|text| text.lines().next()),
            test_case.expected_first_line,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_every_helper_when_defined_then_each_raise_takes_the_call_site() {
    let test_cases = [UnsitedRaiseTestCase {
        description: "no raise in a helper body stays unidentified",
        marker: "#?: ",
        expected_unsited: vec![],
    }];
    for test_case in test_cases {
        let unsited: Vec<String> = all_helper_statements()
            .into_iter()
            .filter(|statement| statement.contains(test_case.marker))
            .collect();
        assert_eq!(
            unsited, test_case.expected_unsited,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_released_helper_when_defined_then_its_definition_is_unchanged() {
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    let test_cases = [
        PinnedDefinitionTestCase {
            description: "json_read_v1's released definition is unchanged; a change takes a new version",
            name: "json_read_v1",
            expected_md5: Some("837f045c25d607dedc3101514c403292"),
        },
        PinnedDefinitionTestCase {
            description: "json_checked_v1's released definition is unchanged; a change takes a new version",
            name: "json_checked_v1",
            expected_md5: Some("5cc2d500d81eb99fc4836b29432df627"),
        },
        PinnedDefinitionTestCase {
            description: "json_escaped_v1's released definition is unchanged; a change takes a new version",
            name: "json_escaped_v1",
            expected_md5: Some("bddb8bb9d55a3c90b994bdda90d47a60"),
        },
        PinnedDefinitionTestCase {
            description: "variant_text_v1's released definition is unchanged; a change takes a new version",
            name: "variant_text_v1",
            expected_md5: Some("b7a6c56e64c916ca02d326a84d76bc2f"),
        },
        PinnedDefinitionTestCase {
            description: "variant_json_text_v1's released definition is unchanged; a change takes a new version",
            name: "variant_json_text_v1",
            expected_md5: Some("7c2319469ba632f57dfa5dbc4e09577a"),
        },
        PinnedDefinitionTestCase {
            description: "variant_number_v1's released definition is unchanged; a change takes a new version",
            name: "variant_number_v1",
            expected_md5: Some("3b486bbca37aa67bdefbc7505ab7ebe6"),
        },
        PinnedDefinitionTestCase {
            description: "variant_float_v1's released definition is unchanged; a change takes a new version",
            name: "variant_float_v1",
            expected_md5: Some("7f02e2cea9c53bdd9ff1c54dd2e57f68"),
        },
        PinnedDefinitionTestCase {
            description: "variant_boolean_v1's released definition is unchanged; a change takes a new version",
            name: "variant_boolean_v1",
            expected_md5: Some("f69ab50e516d7421bff1fe0d1a6a142f"),
        },
        PinnedDefinitionTestCase {
            description: "variant_date_v1's released definition is unchanged; a change takes a new version",
            name: "variant_date_v1",
            expected_md5: Some("19aa95c1fa174d9fffca63fbd7f80cd5"),
        },
        PinnedDefinitionTestCase {
            description: "variant_timestamp_ntz_v1's released definition is unchanged; a change takes a new version",
            name: "variant_timestamp_ntz_v1",
            expected_md5: Some("5d0504a84287d99b6725bb1b917d8466"),
        },
        PinnedDefinitionTestCase {
            description: "timestamp_text_v1's released definition is unchanged; a change takes a new version",
            name: "timestamp_text_v1",
            expected_md5: Some("100a2b398e7c1e131969da9297a62b24"),
        },
        PinnedDefinitionTestCase {
            description: "try_timestamp_text_v1's released definition is unchanged; a change takes a new version",
            name: "try_timestamp_text_v1",
            expected_md5: Some("7640845d87be228faf66e9ed84d4326a"),
        },
        PinnedDefinitionTestCase {
            description: "date_text_v1's released definition is unchanged; a change takes a new version",
            name: "date_text_v1",
            expected_md5: Some("ce2bac08a72d1b71c56359974a3eea5d"),
        },
        PinnedDefinitionTestCase {
            description: "try_date_text_v1's released definition is unchanged; a change takes a new version",
            name: "try_date_text_v1",
            expected_md5: Some("35603478cb0081d0a0a4ec7f575b1c6a"),
        },
        PinnedDefinitionTestCase {
            description: "zoned_text_v1's released definition is unchanged; a change takes a new version",
            name: "zoned_text_v1",
            expected_md5: Some("87f3973edab23f28d6d5e3f38c8f7068"),
        },
        PinnedDefinitionTestCase {
            description: "try_zoned_text_v1's released definition is unchanged; a change takes a new version",
            name: "try_zoned_text_v1",
            expected_md5: Some("cc16ca7e985fb7afb4ad4da149dc79cd"),
        },
        PinnedDefinitionTestCase {
            description: "utc_zoned_text_v1's released definition is unchanged; a change takes a new version",
            name: "utc_zoned_text_v1",
            expected_md5: Some("b1bcbedc46f2c7b91defff1be6f82d78"),
        },
        PinnedDefinitionTestCase {
            description: "try_utc_zoned_text_v1's released definition is unchanged; a change takes a new version",
            name: "try_utc_zoned_text_v1",
            expected_md5: Some("0a15e68699dbf4a31224f060a59bb5ac"),
        },
        PinnedDefinitionTestCase {
            description: "time_zone_v1's released definition is unchanged; a change takes a new version",
            name: "time_zone_v1",
            expected_md5: Some("a41a0270b8028db02b1b191d968a1d80"),
        },
        PinnedDefinitionTestCase {
            description: "decimal_quotient_v1's released definition is unchanged; a change takes a new version",
            name: "decimal_quotient_v1",
            expected_md5: Some("d063fab5b1be26806d77b2accdd21569"),
        },
    ];
    for test_case in test_cases {
        let opening = format!("CREATE OR REPLACE MACRO sqlquack.{}(", test_case.name);
        let md5 = all_helper_statements()
            .into_iter()
            .find(|statement| statement.starts_with(&opening))
            .map(|statement| {
                connection
                    .query_row("SELECT md5(?)", [statement], |row| row.get::<_, String>(0))
                    .expect("DuckDB hashes the definition")
            });
        assert_eq!(
            md5.as_deref(),
            test_case.expected_md5,
            "{}",
            test_case.description
        );
    }
}
