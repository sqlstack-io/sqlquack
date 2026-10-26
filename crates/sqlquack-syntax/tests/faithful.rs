//! Faithful calls: Snowflake calls keep their written name and arguments.

use serde_json::Value;
use sqlquack_syntax::{parse_statements_with, CallMode, Dialect, ParseOptions};

fn first_expression(sql: &str, mode: CallMode) -> Value {
    let statements = parse_statements_with(sql, Dialect::Snowflake, &ParseOptions::default(), mode)
        .expect("the SQL parses");
    let statement = serde_json::to_value(&statements[0]).expect("trees serialise");
    statement["select"]["expressions"][0].clone()
}

fn kind(value: &Value) -> String {
    value
        .as_object()
        .and_then(|object| object.keys().next())
        .cloned()
        .unwrap_or_default()
}

#[test]
fn lowered_spellings_stay_calls_as_written() {
    for (sql, name, arity, lowered) in [
        ("SELECT LEN(s)", "LEN", 1, "length"),
        ("SELECT IFNULL(a, b)", "IFNULL", 2, "coalesce"),
        ("SELECT IFF(c, 1, 2)", "IFF", 3, "if_func"),
        ("SELECT TRY_TO_NUMBER(s)", "TRY_TO_NUMBER", 1, "to_number"),
        ("SELECT MOD(a, b)", "MOD", 2, "mod_func"),
        ("SELECT SUM(x)", "SUM", 1, "sum"),
        ("SELECT COUNT(*)", "COUNT", 1, "count"),
        ("SELECT ROW_NUMBER()", "ROW_NUMBER", 0, "row_number"),
    ] {
        let faithful = first_expression(sql, CallMode::Faithful);
        let payload = faithful
            .get("function")
            .or_else(|| faithful.get("aggregate_function"))
            .unwrap_or_else(|| panic!("{sql}: {faithful}"));
        assert_eq!(payload["name"], name, "{sql}");
        assert_eq!(
            payload["args"].as_array().map(Vec::len),
            Some(arity),
            "{sql}"
        );
        assert_eq!(
            kind(&first_expression(sql, CallMode::Lowered)),
            lowered,
            "{sql}"
        );
    }
}

#[test]
fn keyword_syntax_and_kept_calls_parse_as_polyglot_does() {
    for sql in [
        "SELECT EXTRACT(year FROM d)",
        "SELECT POSITION('a' IN s)",
        "SELECT TRIM(BOTH ' ' FROM s)",
        "SELECT DATEADD(day, 1, d)",
        "SELECT DATE_PART(hour, t)",
        "SELECT CONCAT(a, b)",
    ] {
        assert_eq!(
            first_expression(sql, CallMode::Faithful),
            first_expression(sql, CallMode::Lowered),
            "{sql}"
        );
    }
}

#[test]
fn duckdb_calls_are_not_changed() {
    let sql = "SELECT length(s), coalesce(a, b)";
    let options = ParseOptions::default();
    let faithful = parse_statements_with(sql, Dialect::DuckDB, &options, CallMode::Faithful);
    let lowered = parse_statements_with(sql, Dialect::DuckDB, &options, CallMode::Lowered);
    assert_eq!(
        serde_json::to_value(faithful.expect("parses")).expect("serialises"),
        serde_json::to_value(lowered.expect("parses")).expect("serialises")
    );
}

#[test]
fn quoted_path_keys_with_dots_stay_in_brackets() {
    for (sql, path) in [
        ("SELECT v:\"a.b\" FROM t", "[\"a.b\"]"),
        ("SELECT v:a.\"x.y\" FROM t", "a[\"x.y\"]"),
        ("SELECT v:a.\"x y\"[2] FROM t", "a[\"x y\"][2]"),
        ("SELECT v:a['b.c'] FROM t", "a[\"b.c\"]"),
        ("SELECT v:a.b FROM t", "a.b"),
        ("SELECT v:\"Q\".r FROM t", "Q.r"),
    ] {
        let expression = first_expression(sql, CallMode::Faithful);
        assert_eq!(
            expression["j_s_o_n_extract"]["expression"]["literal"]["value"], path,
            "{sql}: {expression}"
        );
    }
}
