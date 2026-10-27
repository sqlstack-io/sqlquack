pub(crate) struct SortedTextTestCase {
    pub(crate) description: &'static str,
    pub(crate) document: String,
    pub(crate) expected_text: Option<String>,
    pub(crate) expected_raise: bool,
}

pub(crate) struct GrowthTestCase {
    pub(crate) description: &'static str,
    pub(crate) length: fn(usize) -> usize,
    pub(crate) expected_writer_only: bool,
}
