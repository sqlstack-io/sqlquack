//! The converter seen from verification.

use crate::cases::models::LoadedCase;
use crate::verify::models::Conversion;

/// Turns a case's source SQL into DuckDB SQL, or refuses.
pub trait CaseConverter {
    /// The conversion of `case`.
    fn convert(&self, case: &LoadedCase) -> Conversion;
}
