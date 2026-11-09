pub(crate) struct RegistryTestCase {
    pub(crate) description: &'static str,
    pub(crate) lock: &'static str,
    pub(crate) whole_registry: bool,
    pub(crate) expected_mismatches: Vec<String>,
}

pub(crate) struct GuardNameTestCase {
    pub(crate) description: &'static str,
    pub(crate) roots: &'static [&'static str],
    pub(crate) expected_unregistered: Vec<String>,
}
