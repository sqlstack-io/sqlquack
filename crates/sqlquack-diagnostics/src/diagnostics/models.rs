//! Diagnostics: a code, a span in the source, a message and what to do about it.

use serde::Serialize;

/// How serious a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The input is refused.
    Error,
    /// The input converts, with something the reader should know.
    Warning,
}

impl Severity {
    /// The severity as rendered before the code.
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

/// The escape sequences that style diagnostics and SQL; empty strings render plain text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// An error's label, code, underline and summary.
    pub error: &'static str,
    /// A warning's label and underline.
    pub warning: &'static str,
    /// The headline message.
    pub message: &'static str,
    /// The `-->` arrow, the gutter and its line numbers.
    pub gutter: &'static str,
    /// The `note` label.
    pub note: &'static str,
    /// The `help` label.
    pub help: &'static str,
    /// A summary that reports success.
    pub success: &'static str,
    /// A SQL keyword or type name.
    pub keyword: &'static str,
    /// A SQL string literal.
    pub string: &'static str,
    /// A SQL number.
    pub number: &'static str,
    /// A SQL comment.
    pub comment: &'static str,
    /// A SQL identifier, quoted or not, including function names.
    pub identifier: &'static str,
    /// Ends any of the styles above.
    pub reset: &'static str,
}

impl Palette {
    /// The style of a diagnostic of `severity`.
    pub fn severity(&self, severity: Severity) -> &'static str {
        match severity {
            Severity::Error => self.error,
            Severity::Warning => self.warning,
        }
    }
}

/// One registered diagnostic code; codes are never reused or renumbered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CodeEntry {
    /// The code, `SQ` and four digits.
    pub code: &'static str,
    /// A stable snake_case name for the code.
    pub name: &'static str,
    /// A former name the code is also known by, such as a guard's `E_GUARD_*` name.
    pub alias: Option<&'static str>,
    /// The headline every diagnostic with this code starts with.
    pub title: &'static str,
    /// The likely fix, when one is known for every occurrence.
    pub help: Option<&'static str>,
    /// One paragraph explaining what the code means and what to do.
    pub explanation: &'static str,
    /// A command that draws the code, or what input does when no short command can.
    pub example: &'static str,
}

/// Where a diagnostic points, in characters of the source; lines and columns are one-based.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct DiagnosticSpan {
    /// Character offset of the first character.
    pub start: usize,
    /// Character offset one past the last character.
    pub end: usize,
    /// Line of the first character.
    pub line: usize,
    /// Column of the first character.
    pub column: usize,
    /// Line of the last character, or of `start` when the span is empty.
    pub end_line: usize,
    /// Column one past the last character on `end_line`.
    pub end_column: usize,
}

/// One diagnostic, the same for the command line and the Python API; built only from the registry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Diagnostic {
    /// How serious it is.
    pub severity: Severity,
    /// The registered code.
    pub code: &'static str,
    /// The code's stable name.
    pub name: &'static str,
    /// The code's former name, kept for readers of older output.
    pub alias: Option<&'static str>,
    /// The headline.
    pub message: String,
    /// Where it points, when it has a place in the source.
    pub span: Option<DiagnosticSpan>,
    /// What is wrong at the span.
    pub label: String,
    /// Further facts about it.
    pub notes: Vec<String>,
    /// The likely fix, when one is known.
    pub help: Option<String>,
}
