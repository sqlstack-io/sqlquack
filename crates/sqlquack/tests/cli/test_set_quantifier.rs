use crate::helpers::convert_streams;
use crate::test_types::SetQuantifierTestCase;

#[test]
fn given_an_aggregate_with_an_explicit_all_when_converting_then_it_converts_as_if_none_was_written()
{
    let test_cases = [
        SetQuantifierTestCase {
            description: "MEDIAN with ALL converts as without; with DISTINCT it does not",
            sql: "SELECT MEDIAN(ALL a) AS m FROM (SELECT 1 AS a UNION ALL SELECT 1)",
            expected_same_as: "SELECT MEDIAN(a) AS m FROM (SELECT 1 AS a UNION ALL SELECT 1)",
            expected_unlike: "SELECT MEDIAN(DISTINCT a) AS m FROM (SELECT 1 AS a UNION ALL SELECT 1)",
        },
        SetQuantifierTestCase {
            description: "ARRAY_AGG with ALL converts as without; with DISTINCT it does not",
            sql: "SELECT ARRAY_AGG(ALL a) AS m FROM (SELECT 1 AS a UNION ALL SELECT 1)",
            expected_same_as: "SELECT ARRAY_AGG(a) AS m FROM (SELECT 1 AS a UNION ALL SELECT 1)",
            expected_unlike: "SELECT ARRAY_AGG(DISTINCT a) AS m FROM (SELECT 1 AS a UNION ALL SELECT 1)",
        },
    ];
    for test_case in test_cases {
        let converted = |sql: &str| convert_streams(sql, &["--no-config"]).0;
        let expected = converted(test_case.expected_same_as);
        assert!(!expected.is_empty(), "{}", test_case.description);
        assert_eq!(
            converted(test_case.sql),
            expected,
            "{}",
            test_case.description
        );
        assert_ne!(
            converted(test_case.expected_unlike),
            expected,
            "{}",
            test_case.description
        );
    }
}
