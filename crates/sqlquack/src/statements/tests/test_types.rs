pub(crate) struct SpellingsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_keys: &'static [&'static str],
}
