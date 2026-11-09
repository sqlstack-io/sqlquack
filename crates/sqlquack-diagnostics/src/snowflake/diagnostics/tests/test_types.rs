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
