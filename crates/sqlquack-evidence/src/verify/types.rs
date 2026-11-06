//! The converter seen from verification.

use std::collections::BTreeMap;

use sqlquack_core::failures::models::FailureMode;

use crate::cases::models::LoadedCase;
use crate::verify::models::Conversion;

/// Turns a case's source SQL into DuckDB SQL, or refuses.
pub trait CaseConverter: std::fmt::Debug {
    /// The conversion of `case`.
    fn convert(&self, case: &LoadedCase) -> Conversion;

    /// The converted `sql` as users see it, or why laying it out changed more than whitespace.
    fn laid_out(&self, sql: &str) -> Result<String, String> {
        Ok(sql.to_owned())
    }
}

/// For each rule id, the failure modes it declares.
pub type FailureModes = BTreeMap<String, Vec<FailureMode>>;
