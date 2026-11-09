use crate::postgres::profile::tests::helpers::window_only_answers;
use crate::postgres::profile::tests::test_types::WindowOnlyTestCase;

#[test]
fn given_a_called_name_when_checking_window_only_then_every_dialect_answers_as_postgres_catalogues_it()
 {
    let test_cases = [
        WindowOnlyTestCase {
            description: "a ranking function in upper case",
            name: "ROW_NUMBER",
            expected_window_only: [true, true, true],
        },
        WindowOnlyTestCase {
            description: "a ranking function in mixed case",
            name: "Rank",
            expected_window_only: [true, true, true],
        },
        WindowOnlyTestCase {
            description: "a navigation function qualified by its schema",
            name: "pg_catalog.lag",
            expected_window_only: [true, true, true],
        },
        WindowOnlyTestCase {
            description: "a padded name, trimmed only where the call name is canonicalised",
            name: " ntile ",
            expected_window_only: [false, true, true],
        },
        WindowOnlyTestCase {
            description: "a name lower-casing to a window function through the Kelvin sign",
            name: "RAN\u{212a}",
            expected_window_only: [true, true, true],
        },
        WindowOnlyTestCase {
            description: "a name upper-casing to a window function through a dotless i",
            name: "nt\u{131}le",
            expected_window_only: [false, true, true],
        },
        WindowOnlyTestCase {
            description: "an aggregate that also runs with OVER",
            name: "sum",
            expected_window_only: [false, false, false],
        },
        WindowOnlyTestCase {
            description: "an ordered-set aggregate BigQuery also lists among navigation functions",
            name: "percentile_cont",
            expected_window_only: [false, false, false],
        },
        WindowOnlyTestCase {
            description: "a name that only begins with a window function's",
            name: "first_value_x",
            expected_window_only: [false, false, false],
        },
    ];
    for test_case in test_cases {
        for (dialect, answers) in window_only_answers(test_case.name) {
            assert_eq!(
                answers, test_case.expected_window_only,
                "{}: {dialect}",
                test_case.description
            );
        }
    }
}
