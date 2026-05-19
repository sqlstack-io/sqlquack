use crate::helpers;
use crate::test_types;

const TEST_CASES: &[test_types::AnalyzeTestCase] = &[test_types::AnalyzeTestCase {
    description: "simple select returns no diagnostics in placeholder API",
    sql: "select a from raw.orders",
    expected_diagnostic_count: 0,
}];

#[test]
fn given_sql_unit_when_analyzing_then_returns_expected_diagnostics() {
    for test_case in TEST_CASES {
        let result = helpers::analyze_sql(test_case);

        assert_eq!(
            result.diagnostics.len(),
            test_case.expected_diagnostic_count,
            "{}",
            test_case.description
        );
    }
}
