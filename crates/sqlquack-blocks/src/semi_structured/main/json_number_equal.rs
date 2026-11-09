//! `json_number_equal`: a VARIANT compared for equality with an integer NUMBER.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::semi_structured::constants::{JSON_FIDELITY, JSON_NUMBER_EQUAL};

/// TRUE for an equal integer, FALSE for other values and JSON null; fractions raise.
pub fn json_number_equal((variant, number): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_NUMBER_EQUAL);
    let fraction = raised(&format!(
        "{JSON_FIDELITY}: a VARIANT fraction compared with a NUMBER"
    ));
    format!(
        "list_transform([struct_pack(v := CAST({variant} AS JSON), n := {number})], lambda q: \
         CASE WHEN q['v'] IS NULL OR q['n'] IS NULL THEN NULL \
         WHEN json_type(q['v']) IN ('UBIGINT', 'BIGINT') THEN CAST(q['v'] AS HUGEINT) = q['n'] \
         WHEN json_type(q['v']) = 'DOUBLE' THEN CAST({fraction} AS BOOLEAN) ELSE FALSE END)[1]"
    )
}
