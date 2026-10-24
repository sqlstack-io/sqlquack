use polyglot_sql::Expression;

pub(crate) struct SpellingTestCase {
    pub(crate) description: &'static str,
    pub(crate) field: &'static str,
    pub(crate) left: &'static str,
    pub(crate) right: &'static str,
    pub(crate) expected_raw_trees_equal: bool,
    pub(crate) expected_field_present: bool,
    pub(crate) expected_difference_path: Option<String>,
}

pub(crate) struct DeclarationTestCase {
    pub(crate) description: &'static str,
    pub(crate) tested_fields: &'static [&'static str],
    pub(crate) expected_untested_fields: Vec<&'static str>,
}

pub(crate) struct TreeTestCase {
    pub(crate) description: &'static str,
    pub(crate) left: Expression,
    pub(crate) right: Expression,
    pub(crate) expected_raw_trees_equal: bool,
    pub(crate) expected_difference_path: Option<String>,
}

pub(crate) struct DifferenceTestCase {
    pub(crate) description: &'static str,
    pub(crate) left: &'static str,
    pub(crate) right: &'static str,
    pub(crate) expected_difference_path: Option<String>,
}
