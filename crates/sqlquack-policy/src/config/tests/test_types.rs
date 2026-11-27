pub(crate) struct ConfigTestCase {
    pub(crate) description: &'static str,
    pub(crate) text: &'static str,
    pub(crate) expected_error: Option<&'static str>,
}

pub(crate) struct CheckAllowTestCase {
    pub(crate) description: &'static str,
    pub(crate) allow: &'static [(&'static str, bool)],
    pub(crate) expected_error: Option<&'static str>,
}
