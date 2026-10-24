//! Catalogue entries.

use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Disposition;
use sqlquack_core::refusals::types::ConstructPolicy;

/// A construct DuckDB cannot express, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotExpressibleDeclaration {
    /// The construct key, such as `fn:EXAMPLE`.
    pub construct: &'static str,
    /// Why DuckDB cannot express it.
    pub reason: &'static str,
}

/// The rules and declarations SQLQuack applies.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleCatalogue {
    /// Constructs declared impossible to express in DuckDB.
    pub not_expressible: Vec<NotExpressibleDeclaration>,
}

impl ConstructPolicy for RuleCatalogue {
    fn disposition(&self, construct: &Construct) -> Disposition {
        let key = construct.key();
        match self
            .not_expressible
            .iter()
            .find(|declaration| declaration.construct == key)
        {
            Some(declaration) => Disposition::NotExpressible {
                reason: declaration.reason.to_owned(),
            },
            None => Disposition::Missing,
        }
    }
}
