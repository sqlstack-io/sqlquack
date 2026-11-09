//! A diagnostic for a registered code, before it is placed in the source.

use crate::diagnostics::_helpers::code_entry::code_entry;
use crate::diagnostics::constants::{CODES, PARSE_FIDELITY};
use crate::diagnostics::models::{Diagnostic, Severity};

/// A diagnostic for a code the registry is known to hold.
pub(crate) fn coded(code: &str, label: String) -> Diagnostic {
    let entry = code_entry(code)
        .or_else(|| code_entry(PARSE_FIDELITY))
        .unwrap_or(&CODES[0]);
    Diagnostic {
        severity: Severity::Error,
        code: entry.code,
        name: entry.name,
        alias: entry.alias,
        message: entry.title.to_owned(),
        span: None,
        label,
        notes: Vec::new(),
        help: entry.help.map(str::to_owned),
    }
}
