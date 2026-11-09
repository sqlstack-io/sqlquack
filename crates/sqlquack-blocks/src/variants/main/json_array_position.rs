//! `json_array_position`: ARRAY_POSITION, the first index of a value in an array.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{JSON_ARRAY_POSITION, JSON_FRACTION_PATTERN};
use crate::variants::_helpers::objects::{bound, fidelity_error};

/// The needle's first index in the array; fractions, objects and arrays as needles raise.
pub fn json_array_position((needle, array): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_POSITION);
    bound(
        &format!("struct_pack(n := CAST({needle} AS JSON), a := CAST({array} AS JSON))"),
        "q",
        &format!(
            "(CASE WHEN q['n'] IS NULL OR q['a'] IS NULL THEN NULL \
             WHEN json_type(q['n']) IN ('DOUBLE', 'OBJECT', 'ARRAY') THEN CAST({} AS BIGINT) \
             WHEN json_type(q['a']) <> 'ARRAY' THEN NULL \
             WHEN json_type(q['n']) IN ('UBIGINT', 'BIGINT', 'HUGEINT') \
             AND list_bool_or(list_transform(json_extract(q['a'], '$[*]'), \
             lambda e: json_type(e) NOT IN ('UBIGINT', 'BIGINT', 'HUGEINT', 'VARCHAR', 'BOOLEAN', 'NULL', \
             'OBJECT', 'ARRAY') AND NOT (json_type(e) = 'DOUBLE' \
             AND regexp_full_match(CAST(e AS VARCHAR), '{JSON_FRACTION_PATTERN}')))) \
             THEN CAST({} AS BIGINT) \
             ELSE list_position(json_extract(q['a'], '$[*]'), q['n']) - 1 END)",
            fidelity_error("ARRAY_POSITION of a fraction, object or array"),
            fidelity_error(
                "ARRAY_POSITION of an integer in an array holding a fraction or exponent"
            )
        ),
    )
}
