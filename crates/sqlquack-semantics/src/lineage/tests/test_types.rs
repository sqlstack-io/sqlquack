use std::collections::HashMap;

use sqlquack_syntax::Dialect;

use crate::dialects::types::DialectProfile;
use crate::lineage::models::{SchemaColumn, TransformKind};
use crate::types::models::Holding;

pub(crate) struct StoredColumnsTestCase {
    pub(crate) description: &'static str,
    pub(crate) written: Holding,
    pub(crate) expected_read: Holding,
}

pub(crate) struct QueryLineageTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) profile: &'static dyn DialectProfile,
    pub(crate) expected_outputs:
        &'static [(&'static str, TransformKind, &'static [&'static str])],
    pub(crate) expected_open_star: bool,
}

/// An output's expected name, transform, sources (quoted columns in quotes) and whether resolved.
pub(crate) type ExpectedOutput = (&'static str, TransformKind, &'static [&'static str], bool);

pub(crate) struct LineageDetailTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) profile: &'static dyn DialectProfile,
    pub(crate) expected_outputs: &'static [ExpectedOutput],
    pub(crate) expected_row_sources: &'static [&'static str],
}

/// A synthetic shop schema: relation name to its columns.
#[derive(Debug)]
pub(crate) struct ShopSchema(pub(crate) HashMap<String, Vec<SchemaColumn>>);
