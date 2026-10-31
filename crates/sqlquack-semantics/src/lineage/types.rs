//! What lineage asks of a schema.

use std::collections::BTreeSet;
use std::rc::Rc;

use crate::lineage::models::{SchemaColumn, SourceColumn};

/// The known relations: each one's columns in order, by the relation's resolved name.
pub trait LineageSchema: std::fmt::Debug + Sync {
    /// The columns of `relation` (its parts resolved and joined by dots), when known.
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]>;
}

/// A set of source columns that copies of a value's lineage share until one changes it.
pub(crate) type SharedColumns = Rc<BTreeSet<SourceColumn>>;
