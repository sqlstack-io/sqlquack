use std::path::Path;

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

/// Alters the stand-in result named by a hash in a results directory.
pub(crate) type Tampering = fn(&Path, &str);

pub(crate) struct RoundTripTestCase {
    pub(crate) description: &'static str,
    pub(crate) tampering: Tampering,
    pub(crate) expected_round_trip: RoundTrip,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RoundTrip {
    pub(crate) pending_before: usize,
    pub(crate) pending_after: usize,
    pub(crate) verdicts: Vec<String>,
}
