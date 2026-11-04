use std::path::Path;

use crate::translation::models::FormState;

/// What the fake `sqlquack convert` reports on `SELECT TOP 1 17`, and the state its form must take.
pub(crate) struct TranslationTestCase {
    pub(crate) description: &'static str,
    /// The shell lines the fake runs for that SQL, with `$schema` the census schema file or empty.
    pub(crate) convert: String,
    /// Census sidecar lines for the query family, or none.
    pub(crate) census: &'static str,
    /// Writes the by-design files under the translation directory.
    pub(crate) by_design: fn(&Path),
    pub(crate) expected_state: FormState,
    pub(crate) expected_refusals: &'static [&'static str],
    /// The measurable forms of the two in the fixture.
    pub(crate) expected_measurable: usize,
}
