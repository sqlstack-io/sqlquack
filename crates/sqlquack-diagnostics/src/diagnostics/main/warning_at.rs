//! `warning_at`: a warning, without a code, placed in its source.

use sqlquack_syntax::tokens::models::SourceLocation;

use crate::diagnostics::_helpers::line_index::LineIndex;
use crate::diagnostics::models::{Diagnostic, Severity};

/// A warning saying `message`, labelled `label` at `location` in `source`, with `notes`.
pub fn warning_at(
    source: &str,
    location: Option<SourceLocation>,
    (message, label, notes): (String, String, Vec<String>),
) -> Diagnostic {
    Diagnostic {
        severity: Severity::Warning,
        code: "",
        name: "",
        alias: None,
        message,
        span: location.map(|location| LineIndex::new(source).span(location)),
        label,
        notes,
        help: None,
    }
}
