/// A text read in a literal timestamp format, and the timestamp text expected, or None for NULL.
pub(crate) struct FormattedTimestampTestCase {
    pub(crate) description: &'static str,
    pub(crate) format: &'static str,
    pub(crate) text: &'static str,
    pub(crate) expected_timestamp: Result<Option<&'static str>, ()>,
}

/// A format the recordings do not cover.
pub(crate) struct RejectedFormatTestCase {
    pub(crate) description: &'static str,
    pub(crate) format: &'static str,
    pub(crate) expected_rejected: bool,
}
