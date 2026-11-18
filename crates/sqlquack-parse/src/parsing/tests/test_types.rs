pub(crate) struct CallDepthTestCase {
    pub(crate) description: &'static str,
    /// The caller's limit, or `None` for `parse_source`'s default.
    pub(crate) limit: Option<usize>,
    pub(crate) levels: usize,
    /// How many statements parse, or how the parse error ends.
    pub(crate) expected_outcome: Result<usize, &'static str>,
}

pub(crate) struct CheckedDepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) depth: usize,
    pub(crate) expected_result: Result<usize, &'static str>,
}
