//! `unplaced_diagnostic`: a coded diagnostic that has no place in the source.

use crate::diagnostics::_helpers::code_entry::code_entry;
use crate::diagnostics::_helpers::coded::coded;
use crate::diagnostics::models::Diagnostic;

/// A diagnostic for registered `code` with `label`, or `None` when the code is not registered.
pub fn unplaced_diagnostic(code: &str, label: String) -> Option<Diagnostic> {
    code_entry(code).map(|entry| coded(entry.code, label))
}
