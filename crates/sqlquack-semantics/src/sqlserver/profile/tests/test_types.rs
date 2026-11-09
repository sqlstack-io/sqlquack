/// SQL Server outputs: the query and each output's name and type.
pub(crate) struct SqlServerOutputsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outputs: &'static [(&'static str, Option<&'static str>)],
}
