//! The values Databricks SQL's profile and lineage rules read.

/// Databricks' fixed-point type, whose precision and scale default when omitted.
pub(crate) const DATABRICKS_DECIMAL: &str = "DECIMAL";
/// The precision and scale of a Databricks DECIMAL declared without them.
pub(crate) const DATABRICKS_DEFAULT_DECIMAL: (u32, u32) = (10, 0);
/// The arguments of a two-argument function.
pub(crate) const TWO_ARGUMENTS: usize = 2;
