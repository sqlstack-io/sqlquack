//! What an expression can see: the columns of the relation it reads, and whether it may aggregate.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::databricks::conversion::models::{DatabricksType, Emitted};

/// A relation's columns: the name as defined, and the Databricks type.
pub(crate) type Columns = Vec<(String, DatabricksType)>;

/// The relation a query block reads.
#[derive(Clone, Debug, Default)]
pub(crate) struct Scope {
    /// The name the block calls the relation by.
    pub(crate) relation: Option<String>,
    /// Its columns, in order.
    pub(crate) columns: Columns,
    /// The block's own output names and expressions, which `ORDER BY` may name.
    pub(crate) outputs: Vec<(String, Emitted)>,
}

impl Scope {
    /// The relation's one column called `name`, compared without case as Databricks does.
    pub(crate) fn column(&self, name: &str) -> Option<&(String, DatabricksType)> {
        let mut found = self
            .columns
            .iter()
            .filter(|(column, _)| column.eq_ignore_ascii_case(name));
        match (found.next(), found.next()) {
            (Some(column), None) => Some(column),
            _ => None,
        }
    }

    /// The block's one output called `name`, compared without case.
    pub(crate) fn output(&self, name: &str) -> Option<&(String, Emitted)> {
        let mut found = self
            .outputs
            .iter()
            .filter(|(output, _)| output.eq_ignore_ascii_case(name));
        match (found.next(), found.next()) {
            (Some(output), None) => Some(output),
            _ => None,
        }
    }
}

/// Where an expression is converted.
#[derive(Clone, Copy)]
pub(crate) struct Site<'a> {
    /// The record of rules applied.
    pub(crate) log: &'a ProvenanceLog,
    /// The relation the expression reads.
    pub(crate) scope: &'a Scope,
    /// Whether an aggregate may appear here.
    pub(crate) aggregates: bool,
    /// Whether a bare name may be one of the block's output names, as in `ORDER BY`.
    pub(crate) outputs: bool,
    /// Whether a window function may appear: in the projection of a block without GROUP BY or HAVING.
    pub(crate) windows: bool,
}

impl Site<'_> {
    /// The same site, where aggregates and windows may not appear: inside an aggregate's arguments.
    pub(crate) fn without_aggregates(self) -> Self {
        Site {
            aggregates: false,
            windows: false,
            ..self
        }
    }
}
