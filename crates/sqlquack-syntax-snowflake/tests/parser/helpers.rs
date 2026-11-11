use serde_json::Value;
use sqlquack_syntax::TokenizeFailure;
use sqlquack_syntax::{CallMode, Dialect, ParseOptions};
use sqlquack_syntax_snowflake::parse_statements_with;
use sqlquack_syntax_snowflake::tokenize;

pub(crate) fn statements(sql: &str, dialect: Dialect, mode: CallMode) -> Value {
    serde_json::to_value(
        parse_statements_with(sql, dialect, &ParseOptions::default(), mode)
            .expect("the SQL parses"),
    )
    .expect("trees serialise")
}

pub(crate) fn first_projection(sql: &str, mode: CallMode) -> Value {
    statements(sql, Dialect::Snowflake, mode)[0]["select"]["expressions"][0].clone()
}

pub(crate) fn kind(value: &Value) -> String {
    value
        .as_object()
        .and_then(|object| object.keys().next())
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn faithful_call(sql: &str) -> (Value, Option<usize>) {
    let faithful = first_projection(sql, CallMode::Faithful);
    let payload = faithful
        .get("function")
        .or_else(|| faithful.get("aggregate_function"))
        .cloned()
        .unwrap_or_default();
    (
        payload["name"].clone(),
        payload["args"].as_array().map(Vec::len),
    )
}

pub(crate) fn projection_at(sql: &str, pointer: &str) -> Value {
    first_projection(sql, CallMode::Faithful)
        .pointer(pointer)
        .cloned()
        .unwrap_or_default()
}

/// The value at `pointer` in the faithful Snowflake parse of `sql`, or null.
pub(crate) fn statement_at(sql: &str, pointer: &str) -> Value {
    statements(sql, Dialect::Snowflake, CallMode::Faithful)
        .pointer(pointer)
        .cloned()
        .unwrap_or_default()
}

/// The source text a located parse records for the node at `pointer`, or why there is none.
pub(crate) fn located_text(sql: &str, pointer: &str) -> Result<String, String> {
    let tree = statements(sql, Dialect::Snowflake, CallMode::FaithfulLocated);
    let node = tree.pointer(pointer).ok_or("no node at the pointer")?;
    let span = node
        .as_object()
        .and_then(|object| object.values().next())
        .and_then(|payload| payload.get("span"))
        .or_else(|| node.get("span"))
        .ok_or("the node has no span")?;
    let start = span["start"].as_u64().ok_or("no start")? as usize;
    let end = span["end"].as_u64().ok_or("no end")? as usize;
    Ok(sql.chars().skip(start).take(end - start).collect())
}

/// Whether a located parse equals the faithful parse, spans aside.
pub(crate) fn located_matches_faithful(sql: &str, dialect: Dialect) -> bool {
    let parse = |mode| parse_statements_with(sql, dialect, &ParseOptions::default(), mode).ok();
    parse(CallMode::FaithfulLocated) == parse(CallMode::Faithful)
}

/// The cast target types of `SELECT x::<type>` and `SELECT CAST(x AS <type>)`.
pub(crate) fn cast_targets(type_text: &str) -> (Value, Value) {
    let target = |sql: String| first_projection(&sql, CallMode::Faithful)["cast"]["to"].clone();
    (
        target(format!("SELECT x::{type_text}")),
        target(format!("SELECT CAST(x AS {type_text})")),
    )
}

pub(crate) fn accepted(sql: &str, dialect: Dialect) -> bool {
    parse_statements_with(sql, dialect, &ParseOptions::default(), CallMode::Faithful).is_ok()
}

/// The node kind of the first statement when Snowflake SQL parses, and `None` when it is rejected.
pub(crate) fn snowflake_kind(sql: &str) -> Option<String> {
    accepted(sql, Dialect::Snowflake)
        .then(|| kind(&statements(sql, Dialect::Snowflake, CallMode::Faithful)[0]))
}

/// The string at `pointer` in each walked node that has one, sorted.
pub(crate) fn sorted_strings<'a>(values: &'a [Value], pointer: &str) -> Vec<&'a str> {
    let mut found: Vec<&str> = values
        .iter()
        .filter_map(|value| value.pointer(pointer).and_then(Value::as_str))
        .collect();
    found.sort_unstable();
    found
}

pub(crate) fn labelled_tokens(sql: &str, dialect: Dialect) -> Vec<(String, String)> {
    tokenize(sql, dialect)
        .expect("the SQL tokenizes")
        .iter()
        .map(|token| {
            let kind = format!("{:?}", token.kind);
            (
                kind.split('(').next().unwrap_or_default().to_owned(),
                token.text(sql).to_owned(),
            )
        })
        .collect()
}

pub(crate) fn trivia(sql: &str, dialect: Dialect) -> Vec<(String, String)> {
    labelled_tokens(sql, dialect)
        .into_iter()
        .filter(|(kind, _)| kind != "Significant")
        .collect()
}

pub(crate) fn missing_tokens(
    sql: &str,
    dialect: Dialect,
    wanted: &[(&str, &str)],
) -> Vec<(String, String)> {
    let tokens = labelled_tokens(sql, dialect);
    wanted
        .iter()
        .map(|(kind, text)| ((*kind).to_owned(), (*text).to_owned()))
        .filter(|token| !tokens.contains(token))
        .collect()
}

pub(crate) fn lossless_and_contiguous(sql: &str, dialect: Dialect) -> (bool, usize) {
    let tokens = tokenize(sql, dialect).expect("the SQL tokenizes");
    let joined: String = tokens.iter().map(|token| token.text(sql)).collect();
    (
        joined == sql && tokens.windows(2).all(|pair| pair[0].end == pair[1].start),
        tokens.len(),
    )
}

pub(crate) fn tokenizer_failed(sql: &str, dialect: Dialect) -> bool {
    matches!(tokenize(sql, dialect), Err(TokenizeFailure::Tokenizer(_)))
}
