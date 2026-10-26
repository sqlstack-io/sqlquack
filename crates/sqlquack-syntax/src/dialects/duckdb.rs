//! The DuckDB dialect: LIST arrays, STRUCT dot access, EXCLUDE/REPLACE and DuckDB-style syntax.

use super::DialectImpl;
use crate::tokens::TokenizerConfig;

/// DuckDB dialect
pub(crate) struct DuckDBDialect;

impl DialectImpl for DuckDBDialect {
    fn tokenizer_config(&self) -> TokenizerConfig {
        let mut config = TokenizerConfig::default();
        // DuckDB uses double quotes for identifiers
        config.identifiers.insert('"', '"');
        // A leading escaped double quote belongs to the identifier, not a triple-quoted string.
        config.quotes.remove("\"\"\"");
        // DuckDB supports nested comments
        config.nested_comments = true;
        // DuckDB allows underscores as digit separators in numeric literals
        config.numbers_can_be_underscore_separated = true;
        // `//` is integer division (same tokenizer-level recognition as Vertica)
        config.double_slash_int_div = true;
        config
    }
}

#[cfg(test)]
mod tests {

    use crate::dialects::Dialect;
    use crate::dialects::DialectType;
    use crate::expressions::Expression;
    use crate::expressions::JoinKind;

    // Optional execution assertions complement the SQL assertions on machines
    // with DuckDB installed, without making it a dependency of the Rust suite.

    #[test]
    fn test_duckdb_keyword_relation_aliases_preserve_clause_boundaries() {
        let dialect = Dialect::get(DialectType::DuckDB);
        for alias in ["", " top"] {
            let sql = format!(
                "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items{alias} \
                 WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 \
                 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 \
                 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1"
            );
            let statements = dialect.parse(&sql).unwrap();
            let Expression::Select(select) = &statements[0] else {
                panic!("Expected SELECT for {sql}");
            };
            assert!(select.where_clause.is_some());
            assert!(select.group_by.is_some());
            assert!(select.having.is_some());
            assert!(select.windows.is_some());
            assert!(select.qualify.is_some());
            let ordering = &select.order_by.as_ref().unwrap().expressions;
            assert_eq!(ordering.len(), 2);
            assert_eq!(ordering[0].nulls_first, Some(true));
            assert_eq!(ordering[1].nulls_first, Some(false));
            assert!(select.limit.is_some());
            assert!(select.offset.is_some());
            let Expression::Table(table) = &select.from.as_ref().unwrap().expressions[0] else {
                panic!("Expected table source for {sql}");
            };
            assert_eq!(
                table.alias.as_ref().map(|a| a.name.as_str()),
                if alias.is_empty() { None } else { Some("top") }
            );

            for (join, kind) in [
                ("LEFT JOIN selected_items ON TRUE", JoinKind::Left),
                ("POSITIONAL JOIN selected_items", JoinKind::Positional),
            ] {
                let sql = format!("SELECT * FROM ranked_items{alias} {join}");
                let statements = dialect.parse(&sql).unwrap();
                let Expression::Select(select) = &statements[0] else {
                    panic!("Expected SELECT for {sql}");
                };
                assert_eq!(select.joins.len(), 1, "{sql}");
                assert_eq!(select.joins[0].kind, kind, "{sql}");
                let Expression::Table(table) = &select.from.as_ref().unwrap().expressions[0] else {
                    panic!("Expected table source for {sql}");
                };
                assert_eq!(
                    table.alias.as_ref().map(|a| a.name.as_str()),
                    if alias.is_empty() { None } else { Some("top") }
                );
            }
        }

        // Truly reserved DuckDB words must not become implicit relation aliases.
        for alias in ["end", "where", "qualify", "select", "join", "limit"] {
            let sql = format!("SELECT * FROM ranked_items {alias}");
            assert!(dialect.parse(&sql).is_err(), "Unexpectedly accepted {sql}");
        }
    }
}
