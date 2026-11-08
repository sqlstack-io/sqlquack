use crate::lint::tests::helpers::diagnostics_for_dialect;
use crate::lint::tests::test_types::UnusedCteTestCase;

#[test]
fn given_postgres_values_bodies_when_linting_then_unused_ctes_are_reported() -> Result<(), String> {
    let test_cases = [
        UnusedCteTestCase {
            description: "an unused CTE before SELECT * from an unaliased VALUES subquery",
            sql: "WITH a AS (SELECT 1) SELECT * FROM (VALUES (1))",
            expected_unused: &["a"],
        },
        UnusedCteTestCase {
            description: "an unused CTE before a bare VALUES body",
            sql: "WITH a AS (SELECT 1) VALUES (1)",
            expected_unused: &["a"],
        },
        UnusedCteTestCase {
            description: "a CTE a bare VALUES body reads is used",
            sql: "WITH a(x) AS (VALUES (42)) VALUES ((SELECT x FROM a))",
            expected_unused: &[],
        },
    ];

    for test_case in &test_cases {
        let diagnostics = diagnostics_for_dialect(test_case.sql, "postgres", &["SQLCLEAN005"])?;
        let characters: Vec<char> = test_case.sql.chars().collect();
        let unused: Vec<String> = diagnostics
            .iter()
            .map(|diagnostic| {
                let start = diagnostic["start"].as_u64().unwrap_or_default() as usize;
                let end = diagnostic["end"].as_u64().unwrap_or_default() as usize;
                characters[start..end].iter().collect()
            })
            .collect();
        assert_eq!(
            unused, test_case.expected_unused,
            "{}",
            test_case.description
        );
    }
    Ok(())
}
