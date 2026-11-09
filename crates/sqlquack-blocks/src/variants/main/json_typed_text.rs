//! `json_typed_text`: a DATE or timestamp put in a VARIANT, as the JSON string Snowflake writes.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::semi_structured::constants::{JSON_TYPED_TEXT, VARIANT_TYPED_VALUES};
use crate::semi_structured::models::TypedSource;
use crate::variants::_helpers::objects::bound;

/// The value's Snowflake text as a JSON string; parts that text drops raise instead.
pub fn json_typed_text(sql: &str, source: TypedSource, log: &ProvenanceLog) -> String {
    log.building_block(JSON_TYPED_TEXT);
    let day = "CAST(CAST(t AS DATE) AS VARCHAR)";
    let (lost, text) = match source {
        TypedSource::Date => ("false".to_owned(), "CAST(t AS VARCHAR)".to_owned()),
        TypedSource::Timestamp => (
            "right(strftime(t, '%f'), 3) <> '000'".to_owned(),
            format!("{day} || strftime(t, ' %H:%M:%S.%g')"),
        ),
        TypedSource::TimestampNanos => (
            "right(strftime(t, '%n'), 6) <> '000000'".to_owned(),
            format!("{day} || strftime(t, ' %H:%M:%S.%g')"),
        ),
        TypedSource::TimestampLocal => (
            "right(strftime(t, '%f'), 3) <> '000' OR date_part('timezone', t) <> 0".to_owned(),
            format!("{day} || strftime(t, ' %H:%M:%S.%g') || ' Z'"),
        ),
    };
    bound(
        sql,
        "t",
        &format!(
            "(CASE WHEN t IS NULL THEN NULL WHEN {lost} THEN CAST({} AS JSON) \
             ELSE CAST(to_json({text}) AS JSON) END)",
            raised(&format!(
                "{VARIANT_TYPED_VALUES}: a value whose VARIANT text Snowflake writes with fewer \
                 digits or another offset"
            ))
        ),
    )
}
