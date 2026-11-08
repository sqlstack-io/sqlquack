use crate::lint::tests::helpers::diagnostics_for_dialect;
use crate::lint::tests::test_types::JoinPredicateTestCase;

#[test]
fn given_a_join_side_keyword_after_on_when_linting_natively_then_it_is_not_read_as_an_alias()
-> Result<(), String> {
    let test_cases = [
        JoinPredicateTestCase {
            description: "Databricks: a plain equality before LEFT JOIN",
            dialect: "databricks",
            predicate: "o.a = u.a",
            expected_findings: 0,
        },
        JoinPredicateTestCase {
            description: "Databricks: IS DISTINCT FROM before LEFT JOIN",
            dialect: "databricks",
            predicate: "o.a IS DISTINCT FROM u.a",
            expected_findings: 0,
        },
        JoinPredicateTestCase {
            description: "Databricks: a computed predicate before LEFT JOIN",
            dialect: "databricks",
            predicate: "o.a + 1 = u.a",
            expected_findings: 1,
        },
        JoinPredicateTestCase {
            description: "BigQuery: a plain equality before LEFT JOIN",
            dialect: "bigquery",
            predicate: "o.a = u.a",
            expected_findings: 0,
        },
        JoinPredicateTestCase {
            description: "BigQuery: a computed predicate before LEFT JOIN",
            dialect: "bigquery",
            predicate: "o.a + 1 = u.a",
            expected_findings: 1,
        },
    ];
    for test_case in &test_cases {
        let sql = format!(
            "SELECT o.a FROM o AS o JOIN u AS u ON {} LEFT JOIN w AS w ON w.a = o.a",
            test_case.predicate
        );
        let diagnostics = diagnostics_for_dialect(&sql, test_case.dialect, &["SQLCLEAN040"])?;
        assert_eq!(
            diagnostics.len(),
            test_case.expected_findings,
            "{}",
            test_case.description
        );
    }
    Ok(())
}
