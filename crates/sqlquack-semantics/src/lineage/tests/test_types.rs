use std::collections::HashMap;

use sqlquack_syntax::Dialect;

use crate::dialects::types::DialectProfile;
use crate::lineage::models::{Nullability, NullabilityRule, SchemaColumn, TransformKind};

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
    pub(crate) expected_rows_resolved: bool,
}

/// A query and, per output, the SQL text its expression range covers, or none.
pub(crate) struct OutputRangeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) profile: &'static dyn DialectProfile,
    pub(crate) schema: fn() -> ShopSchema,
    pub(crate) expected_ranges: &'static [(&'static str, Option<&'static str>)],
}

/// A synthetic shop schema: relation name to its columns.
#[derive(Debug)]
pub(crate) struct ShopSchema(pub(crate) HashMap<String, Vec<SchemaColumn>>);

/// A synthetic typed shop schema: closed relations, typed columns of open ones, and functions.
#[derive(Debug, Default)]
pub(crate) struct TypedShopSchema {
    pub(crate) closed: HashMap<String, Vec<SchemaColumn>>,
    pub(crate) open: HashMap<String, Vec<SchemaColumn>>,
    pub(crate) functions: HashMap<String, String>,
    pub(crate) rules: HashMap<String, NullabilityRule>,
    /// Functions the project declares, whose calls the checks leave alone.
    pub(crate) declared: Vec<String>,
    /// The argument counts a catalog accepts for each function, by lower-case canonical name.
    pub(crate) arities: HashMap<String, Vec<(usize, Option<usize>)>>,
}

/// An output's expected name, type and nullability.
pub(crate) type ExpectedTypedOutput = (&'static str, Option<&'static str>, Nullability);

/// Engine-recorded result types: the schema the cases read and each case's bound type.
#[derive(serde::Deserialize)]
pub(crate) struct EngineTypeFixture {
    pub(crate) schema: EngineTypeSchema,
    pub(crate) cases: Vec<EngineTypeCase>,
}

#[derive(serde::Deserialize)]
pub(crate) struct EngineTypeSchema {
    pub(crate) tables: Vec<EngineTypeTable>,
}

#[derive(serde::Deserialize)]
pub(crate) struct EngineTypeTable {
    pub(crate) name: String,
    pub(crate) columns: Vec<EngineTypeColumn>,
}

#[derive(serde::Deserialize)]
pub(crate) struct EngineTypeColumn {
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) data_type: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct EngineTypeCase {
    pub(crate) id: String,
    pub(crate) category: String,
    pub(crate) sql: String,
    #[serde(rename = "type", default)]
    pub(crate) engine_type: Option<String>,
}

/// Per category: cases typed as the engine did, and cases left untyped.
#[derive(Debug, Default)]
pub(crate) struct EngineTypeTally {
    pub(crate) exact: usize,
    pub(crate) untyped: usize,
}

pub(crate) struct EngineTypeReplayTestCase {
    pub(crate) description: &'static str,
    pub(crate) fixture: &'static str,
    pub(crate) replayed: fn(&str) -> EngineTypeReplay,
    pub(crate) expected_wrong: &'static [String],
    pub(crate) expected_minimum_typed: usize,
    pub(crate) expected_maximum_untyped: usize,
}

/// A replay's wrong types, its exact count and the per-category tally.
#[derive(Debug, Default)]
pub(crate) struct EngineTypeReplay {
    pub(crate) wrong: Vec<String>,
    pub(crate) typed: usize,
    pub(crate) tally: std::collections::BTreeMap<String, EngineTypeTally>,
}

pub(crate) struct DialectTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) profile: &'static dyn DialectProfile,
    pub(crate) expected_outputs: &'static [ExpectedTypedOutput],
}

pub(crate) struct QueryFindingsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_findings: &'static [(&'static str, &'static str, usize)],
}
