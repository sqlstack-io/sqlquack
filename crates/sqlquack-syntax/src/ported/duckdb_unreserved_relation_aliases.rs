use crate::ported::compat::{parse_one, DialectType, Expression};

#[test]
fn quoted_keyword_relation_aliases_keep_their_spelling() {
    for name in ["Owned", "Match"] {
        for keyword in ["", "AS "] {
            let sql = format!("SELECT \"{name}\".id FROM samples {keyword}\"{name}\"");
            let Expression::Select(query) = parse_one(&sql, DialectType::DuckDB).unwrap() else {
                panic!("expected SELECT")
            };
            let Expression::Table(table) = &query.from.unwrap().expressions[0] else {
                panic!("expected table")
            };
            let alias = table.alias.as_ref().unwrap();
            assert_eq!(alias.name, name);
            assert!(alias.quoted);
        }
    }
}
