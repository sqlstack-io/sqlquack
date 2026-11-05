//! `json_parse_url`: PARSE_URL of text in permissive mode, as a JSON object.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{
    JSON_PARSE_URL, PARSED_URL_NONE, PARSED_URL_PAIR_PATTERN, PARSED_URL_PATTERN,
    PARSED_URL_UNSCHEMED,
};
use crate::variants::_helpers::objects::{bound, fidelity_error};

/// The parts of the URL `text` as PARSE_URL(text, 1) writes them, as recorded: the error object
/// for text without a `:`, every part NULL for empty text, and a raise for other shapes.
pub fn json_parse_url(text: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_PARSE_URL);
    let part = |group: u8| format!("regexp_extract(u, '{PARSED_URL_PATTERN}', {group})");
    let marked = |group: u8| format!("nullif(substr({}, 2), '')", part(group));
    let json = |value: &str| format!("coalesce(CAST(to_json({value}) AS VARCHAR), 'null')");
    let query = format!("substr({}, 2)", part(5));
    let key = "split_part(p, '=', 1)";
    let parameters = format!(
        "CASE WHEN {query_part} = '' THEN 'null' ELSE list_transform([string_split({query}, '&')], \
         lambda ps: CASE WHEN NOT list_bool_and(list_transform(ps, lambda p: \
         regexp_full_match(p, '{PARSED_URL_PAIR_PATTERN}'))) THEN CAST({unread_query} AS VARCHAR) \
         ELSE '{{' || array_to_string(list_transform(list_sort(list_distinct(list_transform(ps, \
         lambda p: {key}))), lambda k: CAST(to_json(k) AS VARCHAR) || ':' || \
         coalesce(CAST(to_json(list_transform(list_filter(ps, lambda p: {key} = k), \
         lambda p: CASE WHEN contains(p, '=') THEN substr(p, strpos(p, '=') + 1) END)[-1]) \
         AS VARCHAR), 'null')), ',') || '}}' END)[1] END",
        query_part = part(5),
        unread_query = fidelity_error("a URL query no recording shows"),
    );
    let path = format!(
        "CASE WHEN {0} = '' THEN NULL ELSE substr({0}, 2) END",
        part(4)
    );
    let query_value = format!("CASE WHEN {} = '' THEN NULL ELSE {query} END", part(5));
    let body = format!(
        "CASE WHEN u IS NULL THEN NULL WHEN u = '' THEN '{PARSED_URL_NONE}' \
         WHEN NOT contains(u, ':') THEN '{PARSED_URL_UNSCHEMED}' \
         WHEN NOT regexp_full_match(u, '{PARSED_URL_PATTERN}') OR {query_part} = '?' \
         OR {fragment_part} = '#' THEN CAST({unread} AS VARCHAR) \
         ELSE '{{\"fragment\":' || {fragment} || ',\"host\":' || {host} || ',\"parameters\":' \
         || {parameters} || ',\"path\":' || {path} || ',\"port\":' || {port} || ',\"query\":' \
         || {query_json} || ',\"scheme\":' || {scheme} || '}}' END",
        query_part = part(5),
        fragment_part = part(6),
        unread = fidelity_error("a URL in a shape no recording shows"),
        fragment = json(&marked(6)),
        host = json(&part(2)),
        path = json(&path),
        port = json(&marked(3)),
        query_json = json(&query_value),
        scheme = json(&part(1)),
    );
    format!("CAST({} AS JSON)", bound(text, "u", &body))
}
