pub(crate) struct BinaryTestCase {
    pub(crate) description: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) expected_outcome: BinaryOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BinaryOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) files: Option<u64>,
    pub(crate) unresolved_name: Option<u64>,
    pub(crate) completion_reported: bool,
    pub(crate) slowest_reported: bool,
    pub(crate) names_printed: bool,
}

pub(crate) struct SourceVariantTestCase {
    pub(crate) description: &'static str,
    pub(crate) payload: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_outcome: SourceVariantOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SourceVariantOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) convertible: Option<u64>,
    pub(crate) labelled: Option<u64>,
}

pub(crate) struct UpstreamTestCase {
    pub(crate) description: &'static str,
    pub(crate) typed: bool,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_outcome: UpstreamOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UpstreamOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) unit_testable: Option<u64>,
    pub(crate) strict: Option<u64>,
    pub(crate) labels: Vec<String>,
    pub(crate) text_counts: Option<String>,
    /// The plan's unit-testable and strict upper bounds.
    pub(crate) plan_bounds: Option<[u64; 2]>,
}

pub(crate) struct SessionTimeZoneTestCase {
    pub(crate) description: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_outcome: SourceVariantOutcome,
}

pub(crate) struct WarningsTestCase {
    pub(crate) description: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_warnings: Vec<String>,
}

pub(crate) struct CursorTestCase {
    pub(crate) description: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_convertible: Option<u64>,
}

pub(crate) struct DeclaredTestCase {
    pub(crate) description: &'static str,
    pub(crate) declare_model: bool,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_outcome: DeclaredOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DeclaredOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) assumptions: Vec<String>,
    pub(crate) divergences: Vec<String>,
    pub(crate) model_declaration_refused: bool,
}

pub(crate) struct ConvertedDirTestCase {
    pub(crate) description: &'static str,
    pub(crate) set_variable: bool,
    pub(crate) expected_files: Vec<ConvertedFile>,
}

/// One file census wrote under `SQLQUACK_CONVERTED_DIR`: its model and whether it holds a SELECT.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConvertedFile {
    pub(crate) model: String,
    pub(crate) holds_select: bool,
}

pub(crate) struct CompiledDbtTestCase {
    pub(crate) description: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_counts: (Option<u64>, Option<u64>),
}

pub(crate) struct DefaultOutputTestCase {
    pub(crate) description: &'static str,
    /// Whether the project's sources type the refused model, so its reader converts.
    pub(crate) typed: bool,
    /// How many of the converting file, the refused model and its reader the scan holds.
    pub(crate) files: usize,
    pub(crate) expected_outcome: DefaultOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DefaultOutcome {
    pub(crate) exit_code: Option<i32>,
    /// Text lines that start with `error[`, `warning:`, ` --> `, `Converted`, `Blocked` or `Converts`.
    pub(crate) text_lines: Vec<String>,
    /// From `--json`: ok, the codes of each blocked file's diagnostics, and the upstream readers.
    pub(crate) json_ok: Option<bool>,
    pub(crate) json_codes: Vec<String>,
    pub(crate) json_upstream: Vec<String>,
}
