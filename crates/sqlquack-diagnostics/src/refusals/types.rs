//! The rule catalogue seen from the refusal pipeline.

use crate::classification::models::Construct;
use crate::refusals::models::Disposition;

/// Decides what happens to each construct. Rule crates implement it.
pub trait ConstructPolicy {
    /// The catalogue's decision for `construct`.
    fn disposition(&self, construct: &Construct) -> Disposition;
}
