use serde_json::Value;

use crate::parse::main::parse::parse;

/// Whether a recorded row's statement parses with a gap.
pub(crate) fn reaches_gap(row: &Value) -> bool {
    let sql: &str = row["sql"].as_str().expect("a sql field");
    parse(sql).is_ok_and(|parsed| !parsed.gaps.is_empty())
}

/// Whether the parser accepts as DuckDB did, refuses a syntax error as one, and else its message.
pub(crate) fn agrees_with_duckdb(row: &Value) -> bool {
    let sql: &str = row["sql"].as_str().expect("a sql field");
    let recorded: Option<&str> = row["duckdb"]["message"].as_str();
    let syntax: bool = recorded.is_some_and(|message| message.contains("syntax error"));
    let text: Option<&str> = recorded
        .and_then(|message| message.split_once(": "))
        .map(|(_, text)| text);
    let result = parse(sql);
    let refused_alike: bool = result.as_ref().err().is_some_and(|error| {
        (syntax && error.syntax) || (!syntax && text == Some(error.message.as_str()))
    });
    (recorded.is_none() && result.is_ok()) || refused_alike
}

/// A cast to `levels` STRUCT types, each wrapping the one inside and followed by `bounds` list bounds.
pub(crate) fn nested_struct_lists(levels: usize, bounds: usize) -> String {
    let mut data_type: String = "INTEGER".to_owned();
    for _ in 0..levels {
        data_type = format!("STRUCT(a {data_type}){}", "[]".repeat(bounds));
    }
    format!("SELECT CAST(NULL AS {data_type})")
}

/// `levels` parenthesised queries, each the first operand of a `links`-long `UNION ALL` chain.
pub(crate) fn nested_unions(levels: usize, links: usize) -> String {
    let mut query: String = "SELECT 1".to_owned();
    for _ in 0..levels {
        query = format!("({query}){}", " UNION ALL SELECT 1".repeat(links));
    }
    query
}
