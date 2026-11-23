use crate::tokens::models::DialectType;

pub(crate) struct ValidationLocationTestCase {
    pub(crate) description: &'static str,
    pub(crate) location: (usize, usize),
    pub(crate) span: (Option<usize>, Option<usize>),
    pub(crate) expected_position: (Option<usize>, Option<usize>, Option<usize>, Option<usize>),
}

pub(crate) struct DialectNameTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_dialect: Option<DialectType>,
}
