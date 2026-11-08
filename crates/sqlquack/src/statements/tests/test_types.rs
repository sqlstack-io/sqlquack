use crate::statements::models::Outcome;

pub(crate) struct SpellingsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_keys: &'static [&'static str],
}

pub(crate) struct RejectionTestCase {
    pub(crate) description: &'static str,
    /// The rejection under test, refused with this code on this key.
    pub(crate) rejection: (&'static str, &'static str),
    /// Another case: its reference, outcome, code and key.
    pub(crate) sibling: (&'static str, Outcome, &'static str, &'static str),
    /// The references that declare `compile_error`, besides the rejection's.
    pub(crate) declared: &'static [&'static str],
    pub(crate) expected_outcome: Outcome,
}
