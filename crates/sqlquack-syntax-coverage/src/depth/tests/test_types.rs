use crate::check::models::{LocalFailure, LocalVerdict};
use crate::depth::models::KnownFailure;
use crate::formats::models::LocalParser;

/// A parser giving `deep` past `threshold` bytes of text and `shallow` below, and what the probe finds.
pub(crate) struct ProbeTestCase {
    pub(crate) description: &'static str,
    pub(crate) threshold: usize,
    pub(crate) deep: LocalVerdict,
    pub(crate) shallow: LocalVerdict,
    pub(crate) expected_failed_at: Option<(usize, LocalFailure)>,
    /// How many ladder depths the probe parses before it stops.
    pub(crate) expected_parses: usize,
}

/// A parser, a construct, and whether the probe runs that construct in that parser.
pub(crate) struct ConstructsTestCase {
    pub(crate) description: &'static str,
    pub(crate) parser: LocalParser,
    pub(crate) construct: &'static str,
    pub(crate) expected_probed: bool,
}

/// Allowlist entries, and the `parser construct` each report list must name.
pub(crate) struct AllowlistTestCase {
    pub(crate) description: &'static str,
    pub(crate) entries: Vec<KnownFailure>,
    pub(crate) expected_unlisted: Vec<&'static str>,
    pub(crate) expected_stale: Vec<&'static str>,
    pub(crate) expected_problems: Vec<&'static str>,
}
