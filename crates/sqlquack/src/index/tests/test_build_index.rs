use crate::index::main::build_index::build_index;
use crate::index::main::render_index_text::render_index_text;
use crate::index::models::RecordSource;
use crate::index::tests::helpers::{converted, refused};
use crate::index::tests::test_types::BuildIndexTestCase;

#[test]
fn given_conversion_records_when_building_the_index_then_each_identifier_lists_its_dependents() {
    let test_cases = [BuildIndexTestCase {
        description: "a building block is attributed to the rules calling it and their records",
        records: vec![
            converted(
                RecordSource::File("models/orders.sql".to_owned()),
                &[
                    ("rule.cast", &["duckdb.cast", "snowflake.cast_target"]),
                    ("rule.column", &[]),
                ],
            ),
            converted(
                RecordSource::Case("casts.toml#text".to_owned()),
                &[(
                    "rule.cast_text",
                    &["duckdb.text_to_boolean", "snowflake.cast_target"],
                )],
            ),
            converted(
                RecordSource::Case("casts.toml#number".to_owned()),
                &[("rule.cast", &["duckdb.cast"])],
            ),
            refused(
                RecordSource::File("models/events.sql".to_owned()),
                &["node:parse_json"],
            ),
        ],
        expected_lines: vec![
            "2 cases, 2 files: 3 converted, 1 refused",
            "",
            "building block duckdb.cast: 1 rules, 1 cases, 1 files",
            "  rule rule.cast",
            "  case casts.toml#number",
            "  file models/orders.sql",
            "",
            "building block duckdb.text_to_boolean: 1 rules, 1 cases, 0 files",
            "  rule rule.cast_text",
            "  case casts.toml#text",
            "",
            "typing function snowflake.cast_target: 2 rules, 1 cases, 1 files",
            "  rule rule.cast",
            "  rule rule.cast_text",
            "  case casts.toml#text",
            "  file models/orders.sql",
            "",
            "rule rule.cast: 1 cases, 1 files",
            "  uses duckdb.cast",
            "  uses snowflake.cast_target",
            "  case casts.toml#number",
            "  file models/orders.sql",
            "",
            "rule rule.cast_text: 1 cases, 0 files",
            "  uses duckdb.text_to_boolean",
            "  uses snowflake.cast_target",
            "  case casts.toml#text",
            "",
            "rule rule.column: 0 cases, 1 files",
            "  file models/orders.sql",
            "",
            "refused construct node:parse_json: 0 cases, 1 files",
            "  file models/events.sql",
        ],
    }];
    for test_case in test_cases {
        let mut reversed = test_case.records.clone();
        reversed.reverse();
        let rendered = render_index_text(&build_index(&test_case.records));
        let lines: Vec<&str> = rendered.lines().collect();
        assert_eq!(lines, test_case.expected_lines, "{}", test_case.description);
        assert_eq!(
            build_index(&reversed),
            build_index(&test_case.records),
            "{}: record order does not change the index",
            test_case.description
        );
    }
}
