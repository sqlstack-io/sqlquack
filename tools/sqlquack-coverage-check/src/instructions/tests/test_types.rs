/// Measured counts against a baseline of 1,000 parse and 2,000 analysis instructions at 2%.
pub(crate) struct CompareTestCase {
    pub(crate) description: &'static str,
    pub(crate) arch: &'static str,
    pub(crate) measured: Vec<(&'static str, u64)>,
    /// A fragment each expected problem holds, in order.
    pub(crate) expected_problems: Vec<&'static str>,
}

/// A callgrind profile and the instructions it counts in the program's own code.
pub(crate) struct ProfileTestCase {
    pub(crate) description: &'static str,
    pub(crate) profile: &'static str,
    pub(crate) expected_count: u64,
}
