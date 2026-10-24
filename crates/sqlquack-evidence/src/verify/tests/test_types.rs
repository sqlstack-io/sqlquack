use crate::cases::models::LoadedCase;
use crate::verify::models::Conversion;
use crate::verify::types::CaseConverter;

/// Test-only converter: replays the source SQL unchanged, valid only for SQL both dialects share.
pub(crate) struct UnchangedSqlForTests;

impl CaseConverter for UnchangedSqlForTests {
    fn convert(&self, case: &LoadedCase) -> Conversion {
        Conversion::Converted(case.source_sql.clone())
    }
}

/// How a stand-in result is tampered with before verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tampering {
    /// Left as recorded.
    None,
    /// A value changed and the metadata's Arrow hash updated to match.
    ValueChanged,
    /// A value changed without updating the metadata.
    ArrowEdited,
}

pub(crate) struct RoundTripTestCase {
    pub(crate) description: &'static str,
    pub(crate) tampering: Tampering,
    pub(crate) expected_pending_before: usize,
    pub(crate) expected_pending_after: usize,
    pub(crate) expected_verdicts: Vec<&'static str>,
}
