//! Typed, in-process lint of one SQL text.

use crate::lint::models::{LintRequest, LintResponse};

/// Lint `request.sql` with the selected built-in Rules and return findings in source order.
pub fn lint(request: LintRequest) -> Result<LintResponse, String> {
    crate::lint::_helpers::engine::lint(request)
}
