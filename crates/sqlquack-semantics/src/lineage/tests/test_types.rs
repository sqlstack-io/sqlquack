use std::collections::HashMap;

use sqlquack_syntax::Dialect;

use crate::dialects::types::DialectProfile;
use crate::lineage::models::{Nullability, NullabilityRule, SchemaColumn, TransformKind};
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
    pub(crate) expected_rows_resolved: bool,
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

pub(crate) struct OutputTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outputs: &'static [ExpectedTypedOutput],
}

pub(crate) struct ProjectsStarTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_projects_star: bool,
}

pub(crate) struct ResolvedReadTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_sources: &'static [&'static str],
    pub(crate) expected_resolved: bool,
}

pub(crate) struct ClickhouseJoinTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_output: (Option<&'static str>, Nullability),
    pub(crate) expected_codes: &'static [&'static str],
}

pub(crate) struct ClickhouseTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_output: ExpectedTypedOutput,
}

pub(crate) struct DuckdbTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_type: Option<&'static str>,
}

pub(crate) struct BigqueryTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_type: Option<&'static str>,
}

pub(crate) struct PostgresTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_type: Option<&'static str>,
}

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

/// SQLite outputs: the query and each output's name, type and nullability.
pub(crate) struct SqliteOutputsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outputs: &'static [(&'static str, Option<&'static str>, Nullability)],
}

/// A deeply nested SQLite query: its tree depth, outputs and findings, or the parser's refusal.
pub(crate) struct DeepSqliteTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) expected_result: Result<(usize, usize, usize), &'static str>,
}

/// A nesting analysed at two depths, and how many times the shallow work the deep may take.
pub(crate) struct NestingWorkTestCase {
    pub(crate) description: &'static str,
    pub(crate) shape: fn(usize) -> String,
    pub(crate) expected_growth_at_most: usize,
}
