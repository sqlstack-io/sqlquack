//! `enforce_input`: enforce raw SQL input limits before tokenization.

use crate::guard::_helpers::codes::parse_guard_error;
use crate::guard::models::ComplexityGuardOptions;
use crate::tokens::types::Result;

/// Enforce raw SQL input limits before tokenization.
pub(crate) fn enforce_input(sql: &str, options: &ComplexityGuardOptions) -> Result<()> {
    if let Some(max) = options.max_input_bytes {
        let input_bytes = sql.len();
        if input_bytes > max {
            return Err(parse_guard_error(
                "E_GUARD_INPUT_TOO_LARGE",
                input_bytes,
                max,
                None,
            ));
        }
    }

    Ok(())
}
