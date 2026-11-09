pub(crate) struct EditDistanceTestCase {
    pub(crate) description: &'static str,
    pub(crate) left: Option<&'static str>,
    pub(crate) right: Option<&'static str>,
    pub(crate) expected_distance: Option<i64>,
}

pub(crate) struct SplitRowsTestCase {
    pub(crate) description: &'static str,
    pub(crate) text: Option<&'static str>,
    pub(crate) delimiter: &'static str,
    pub(crate) expected_rows: &'static [(i64, &'static str)],
}
