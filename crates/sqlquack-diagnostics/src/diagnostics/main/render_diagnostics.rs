//! `render_diagnostics`: diagnostics as rustc-style text, the source line underlined at the span.

use crate::diagnostics::constants::PLAIN_PALETTE;
use crate::diagnostics::main::render_styled_diagnostics::render_styled_diagnostics;
use crate::diagnostics::models::Diagnostic;

/// Every diagnostic of `source`, read from `path`, as plain text separated by blank lines.
pub fn render_diagnostics(diagnostics: &[Diagnostic], source: &str, path: &str) -> String {
    render_styled_diagnostics(diagnostics, source, path, &PLAIN_PALETTE)
}
