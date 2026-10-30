use crate::dialects::types::DialectProfile;

pub(crate) struct IdentifierTestCase {
    pub(crate) description: &'static str,
    pub(crate) profile: &'static dyn DialectProfile,
    pub(crate) written: (&'static str, bool),
    pub(crate) compared_with: &'static str,
    pub(crate) expected_resolved: &'static str,
    pub(crate) expected_same: bool,
}
