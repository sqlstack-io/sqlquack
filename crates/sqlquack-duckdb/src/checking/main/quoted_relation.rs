//! `quoted_relation`: a relation name as the check creates it and converted SQL reads it.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::NAME_SEPARATOR;
use crate::blocks::main::quoted_identifier::quoted_identifier;

/// A relation named `TABLE`, `SCHEMA.TABLE` or `CATALOG.SCHEMA.TABLE`, each part double-quoted.
pub fn quoted_relation(name: &str, log: &ProvenanceLog) -> String {
    name.split(NAME_SEPARATOR)
        .map(|part| quoted_identifier(part, log))
        .collect::<Vec<_>>()
        .join(NAME_SEPARATOR)
}
