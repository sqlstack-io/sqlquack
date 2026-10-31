//! What lineage asks of a schema.

use crate::lineage::models::SchemaColumn;

/// The known relations: each one's columns in order, by the relation's resolved name.
pub trait LineageSchema: std::fmt::Debug + Sync {
    /// The columns of `relation` (its parts resolved and joined by dots), when known.
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]>;
}
