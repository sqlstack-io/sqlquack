use crate::checking::errors::CheckStage;

pub(crate) struct CheckTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) expected_stage: Option<CheckStage>,
}

/// One check over the given relations, run after the previous case on the same engine.
pub(crate) struct RelationsTestCase {
    pub(crate) description: &'static str,
    pub(crate) relations: &'static [&'static str],
    pub(crate) expected_stage: Option<CheckStage>,
}

/// A printed expression and the subquery value in it, and how the value must be bound.
pub(crate) struct BindingTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) value: &'static str,
    /// Whether the value moves outside the lambdas, or the lambda parameter that keeps it in.
    pub(crate) expected_binding: Result<bool, &'static str>,
    /// DuckDB's verdict on the SQL as printed, and then as rewritten.
    pub(crate) expected_stages: (Option<CheckStage>, Option<CheckStage>),
}
