use crate::lint::tests::helpers::diagnostics_for_rules;
use crate::lint::tests::test_types::KeywordAliasTestCase;

#[test]
fn given_a_first_or_last_join_alias_when_linting_then_it_qualifies_like_an_ordinary_alias()
-> Result<(), String> {
    let test_cases = [
        KeywordAliasTestCase {
            description: "joined relation aliased FIRST contributes a selected value",
            sql: "SELECT o.brand_code, first.brand_name AS b FROM stg_brands AS o INNER JOIN stg_brands first ON first.brand_code = o.brand_code",
            rule: "SQLCLEAN032",
            expected_anchor: None,
        },
        KeywordAliasTestCase {
            description: "joined relation aliased LAST contributes a selected value",
            sql: "SELECT o.brand_code, last.country_code AS c FROM stg_brands AS o LEFT JOIN stg_brands last USING (brand_code)",
            rule: "SQLCLEAN032",
            expected_anchor: None,
        },
        KeywordAliasTestCase {
            description: "cross-joined relation aliased FIRST contributes a selected value",
            sql: "SELECT o.order_id, first.customer_id FROM orders AS o CROSS JOIN customers first",
            rule: "SQLCLEAN032",
            expected_anchor: None,
        },
        KeywordAliasTestCase {
            description: "column qualified by a FIRST alias has a stable output name",
            sql: "SELECT o.order_id, first.customer_id FROM orders AS o CROSS JOIN customers first",
            rule: "SQLCLEAN022",
            expected_anchor: None,
        },
        KeywordAliasTestCase {
            description: "star qualified by a FIRST alias with EXCLUDE has a stable output shape",
            sql: "SELECT first.* EXCLUDE (customer_id) FROM orders AS o CROSS JOIN customers first",
            rule: "SQLCLEAN022",
            expected_anchor: None,
        },
        KeywordAliasTestCase {
            description: "ordinary joined alias contributes a selected value",
            sql: "SELECT o.order_id, f.customer_id FROM orders AS o CROSS JOIN customers f",
            rule: "SQLCLEAN032",
            expected_anchor: None,
        },
        KeywordAliasTestCase {
            description: "joined relation aliased LAST that contributes nothing",
            sql: "SELECT o.order_id FROM orders AS o LEFT JOIN customers last ON last.customer_id = o.customer_id",
            rule: "SQLCLEAN032",
            expected_anchor: Some("JOIN"),
        },
    ];
    for test_case in &test_cases {
        let diagnostics = diagnostics_for_rules(test_case.sql, &[test_case.rule])?;
        let anchors: Vec<String> = diagnostics
            .iter()
            .map(|diagnostic| {
                let start = diagnostic["start"].as_u64().unwrap_or_default() as usize;
                let end = diagnostic["end"].as_u64().unwrap_or_default() as usize;
                test_case
                    .sql
                    .chars()
                    .skip(start)
                    .take(end - start)
                    .collect()
            })
            .collect();
        assert_eq!(
            anchors,
            test_case.expected_anchor.into_iter().collect::<Vec<_>>(),
            "{}",
            test_case.description
        );
    }
    Ok(())
}
