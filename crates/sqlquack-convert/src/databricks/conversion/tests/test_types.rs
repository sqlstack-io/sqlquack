use sqlquack_diagnostics::refusals::models::RefusalCategory;

use crate::databricks::conversion::models::DatabricksType;

/// A query over the fixture, text its DuckDB SQL must hold, and the result columns' types.
pub(crate) struct ConvertTestCase {
    pub(crate) description: &'static str,
    pub(crate) query: &'static str,
    pub(crate) expected_sql: &'static [&'static str],
    pub(crate) expected_types: &'static [DatabricksType],
}

/// A query over the fixture and text its refusal's reason must hold.
pub(crate) struct RefuseTestCase {
    pub(crate) description: &'static str,
    pub(crate) query: &'static str,
    pub(crate) expected_reason: &'static str,
}

/// A refused statement and the category its first refusal has.
pub(crate) struct CategoryTestCase {
    pub(crate) description: &'static str,
    pub(crate) query: &'static str,
    pub(crate) expected_category: RefusalCategory,
}
