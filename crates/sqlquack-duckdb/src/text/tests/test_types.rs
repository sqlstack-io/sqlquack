pub(crate) struct EditDistanceTestCase {
    pub(crate) description: &'static str,
    pub(crate) left: Option<&'static str>,
    pub(crate) right: Option<&'static str>,
    pub(crate) expected_distance: Option<i64>,
}
