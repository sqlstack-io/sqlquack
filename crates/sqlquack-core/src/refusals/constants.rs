//! The wording refusals share with the code that reads them back.

/// The words before the divergence id in a refused divergence's reason.
pub const NEEDS_DIVERGENCE: &str = " needs known divergence `";
/// The words after the divergence id in a refused divergence's reason.
pub const REFUSED_IN_MODE: &str = "`, which the policy refuses in this mode";
