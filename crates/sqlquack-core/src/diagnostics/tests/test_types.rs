use crate::refusals::models::Refusal;

pub(crate) struct RenderTestCase {
    pub(crate) description: &'static str,
    pub(crate) source: String,
    pub(crate) refusal: Refusal,
    pub(crate) expected_text: String,
}

pub(crate) struct GuardTestCase {
    pub(crate) description: &'static str,
    pub(crate) source: String,
    pub(crate) expected_code: (&'static str, Option<&'static str>),
    pub(crate) expected_label: &'static str,
}

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
