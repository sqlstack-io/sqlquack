pub(crate) struct PropertyTestCase {
    pub(crate) description: &'static str,
    pub(crate) seed: u64,
    pub(crate) inputs: usize,
    pub(crate) expected_violations: Vec<String>,
}

pub(crate) struct ExtremeInputTestCase {
    pub(crate) description: &'static str,
    pub(crate) input: String,
    pub(crate) expected_violations: Vec<String>,
}
