use crate::index::models::ConversionRecord;

pub(crate) struct BuildIndexTestCase {
    pub(crate) description: &'static str,
    pub(crate) records: Vec<ConversionRecord>,
    pub(crate) expected_lines: Vec<&'static str>,
}
