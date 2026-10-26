//! Parses SQLQuack accepts where polyglot rejects valid Snowflake; each is listed as a fix.

use serde_json::Value;
use sqlquack_syntax::{parse_statements_with, CallMode, Dialect, ParseOptions};

fn first_expression(sql: &str) -> Value {
    let statements = parse_statements_with(
        sql,
        Dialect::Snowflake,
        &ParseOptions::default(),
        CallMode::Faithful,
    )
    .expect("the SQL parses");
    let statement = serde_json::to_value(&statements[0]).expect("trees serialise");
    statement["select"]["expressions"][0].clone()
}

#[test]
fn an_index_followed_by_a_key_is_a_path_on_the_element() {
    for (sql, path) in [
        ("SELECT v[3]:a FROM t", "a"),
        ("SELECT v[0]:a.b FROM t", "a.b"),
    ] {
        let expression = first_expression(sql);
        let extract = &expression["j_s_o_n_extract"];
        assert!(
            extract["this"].get("subscript").is_some(),
            "{sql}: {expression}"
        );
        assert_eq!(
            extract["expression"]["literal"]["value"], path,
            "{sql}: {expression}"
        );
    }
}
