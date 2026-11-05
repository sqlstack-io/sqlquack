//! What lineage asks of a schema.

use std::collections::BTreeSet;
use std::rc::Rc;

use crate::lineage::models::{NullabilityRule, SchemaColumn, SourceColumn};

/// The known relations: each one's columns in order, by the relation's resolved name.
pub trait LineageSchema: std::fmt::Debug + Sync {
    /// The columns of `relation` (its parts resolved and joined by dots), when known.
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]>;

    /// Known columns of `relation` whose full column list is not known, to type the ones read.
    fn open_columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        let _ = relation;
        None
    }

    /// The type the function `name` returns whatever its arguments, when the catalog knows it.
    fn function_type(&self, name: &str) -> Option<&str> {
        let _ = name;
        None
    }

    /// How the function `name`'s nullability follows from its arguments', when the catalog knows.
    fn function_nullability(&self, name: &str) -> Option<NullabilityRule> {
        let _ = name;
        None
    }

    /// Whether the project declares the function `name`, whose calls the type checks leave alone.
    fn known_function(&self, name: &str) -> bool {
        let _ = name;
        false
    }

    /// The `(least, most)` argument counts the catalog accepts for `name`, when it knows them.
    fn function_arities(&self, name: &str, canonical: &str) -> Option<Vec<(usize, Option<usize>)>> {
        let _ = (name, canonical);
        None
    }

    /// Whether the schema declares every relation and function the SQL may name, so an unknown one is wrong.
    fn closed(&self) -> bool {
        false
    }

    /// Whether the project declares the type `name`, which a cast may name.
    fn known_type(&self, name: &str) -> bool {
        let _ = name;
        false
    }
}

/// A set of source columns that copies of a value's lineage share until one changes it.
pub(crate) type SharedColumns = Rc<BTreeSet<SourceColumn>>;
