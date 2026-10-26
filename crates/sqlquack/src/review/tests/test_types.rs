use sqlquack_compare::values::models::CanonicalValue;

pub(crate) struct RenderValueTestCase {
    pub(crate) description: &'static str,
    pub(crate) value: CanonicalValue,
    pub(crate) declared_scale: Option<u8>,
    pub(crate) expected_text: &'static str,
}
