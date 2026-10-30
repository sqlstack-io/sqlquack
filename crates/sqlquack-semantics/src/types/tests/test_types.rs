use crate::types::models::ColumnSpec;

pub(crate) struct SchemaColumnsTestCase {
    pub(crate) description: &'static str,
    pub(crate) column: ColumnSpec,
    pub(crate) expected_marks: (bool, bool),
    pub(crate) expected_refusal: bool,
}
