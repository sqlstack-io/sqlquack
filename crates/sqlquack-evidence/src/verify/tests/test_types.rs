use std::path::Path;

use sqlquack_core::provenance::models::Provenance;

use crate::cases::models::LoadedCase;
use crate::results::models::SessionDefaults;
use crate::verify::models::Conversion;
use crate::verify::types::CaseConverter;

/// Test-only converter: replays the source SQL unchanged, valid only for SQL both dialects share.
#[derive(Debug)]
pub(crate) struct UnchangedSqlForTests;

impl CaseConverter for UnchangedSqlForTests {
    fn convert(&self, case: &LoadedCase) -> Conversion {
        Conversion::Converted {
            sql: case.source_sql.clone(),
            setup: Vec::new(),
            provenance: Provenance::default(),
            raises: std::collections::BTreeMap::new(),
            python: None,
        }
    }
}

/// Alters the stand-in result named by a hash in a results directory.
pub(crate) type Tampering = fn(&Path, &str);

pub(crate) struct RoundTripTestCase {
    pub(crate) description: &'static str,
    pub(crate) tampered: &'static str,
    pub(crate) tampering: Tampering,
    pub(crate) expected_round_trip: RoundTrip,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RoundTrip {
    pub(crate) pending_before: usize,
    pub(crate) pending_after: usize,
    pub(crate) unproven_sessions: usize,
    pub(crate) verdicts: Vec<String>,
}

pub(crate) struct SessionTestCase {
    pub(crate) description: &'static str,
    pub(crate) tampering: Tampering,
    pub(crate) defaults: Option<SessionDefaults>,
    pub(crate) expected_round_trip: RoundTrip,
}

pub(crate) struct DivergenceErrorTestCase {
    pub(crate) description: &'static str,
    pub(crate) error: &'static str,
    pub(crate) expected_matched: Option<&'static str>,
}

pub(crate) struct MicrosecondEvidenceTestCase {
    pub(crate) description: &'static str,
    pub(crate) declared: &'static str,
    pub(crate) stored_unit: arrow::datatypes::TimeUnit,
    pub(crate) replayed_nanos: i64,
    pub(crate) expected_floored: Option<i64>,
}

pub(crate) struct CheckedMicrosecondTestCase {
    pub(crate) description: &'static str,
    pub(crate) checked: bool,
    pub(crate) replayed_nanos: i64,
    pub(crate) expected_verdict: &'static str,
}

pub(crate) struct SubMicrosecondMarkTestCase {
    pub(crate) description: &'static str,
    pub(crate) json: &'static str,
    pub(crate) expected_checked: bool,
}

pub(crate) struct CompileErrorTestCase {
    pub(crate) description: &'static str,
    pub(crate) sqlstate: &'static str,
    pub(crate) declared: bool,
    pub(crate) with_setup: bool,
    pub(crate) expected_failed: bool,
    /// Whether a refusal of the case would match Snowflake's: a declared compile-class error.
    pub(crate) expected_rejected: bool,
}
