use crate::census::tests::helpers::{NOT_SQL, SHOP, census_summary};
use crate::census::tests::test_types::{CensusSummary, CensusTestCase};

#[test]
fn given_sql_directory_when_running_census_then_counts_files_constructs_and_coverage() {
    let test_cases = [
        CensusTestCase {
            description: "compiled shop models with one unparseable file and one non-SQL file",
            fixtures: SHOP,
            expected_summary: Ok(CensusSummary {
                files: 4,
                parsed: 3,
                unparsed: 1,
                missing_rule: 3,
                parse_fidelity: 1,
                convertible: 0,
                most_used_files: 3,
                select_files: 3,
                qualify_files: 1,
                all_rules_convertible: 3,
                text_mentions_sql: false,
            }),
        },
        CensusTestCase {
            description: "a directory with no SQL files",
            fixtures: NOT_SQL,
            expected_summary: Err("no .sql files found under the given paths".to_owned()),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            census_summary(test_case.fixtures),
            test_case.expected_summary,
            "{}",
            test_case.description
        );
    }
}
