//! `convert_setup`: a case's setup statements as DuckDB statements and the relations they create.

use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_syntax::expressions::tree::models::Expression;

use crate::conversion::_helpers::setup::tables::{created, inserted};
use crate::conversion::_helpers::setup::user_types::defined;
use crate::conversion::_helpers::setup::variables::{assigned, unassigned};
use crate::conversion::constants::SETUP_REPLAYED;
use crate::conversion::models::ConvertedSetup;

/// Each statement in order: CREATE TABLE, INSERT INTO it VALUES, CREATE TYPE, or SET and UNSET of variables.
pub fn convert_setup(statements: &[String]) -> Result<ConvertedSetup, String> {
    let log = ProvenanceLog::default();
    let mut converted = ConvertedSetup::default();
    for statement in statements {
        let refused = |reason: &str| format!("setup statement `{statement}` {reason}");
        let parsed = parse_source(statement, SourceDialect::Snowflake)
            .map_err(|_| refused("does not parse"))?;
        let duckdb = match parsed.as_slice() {
            [Expression::CreateTable(table)] => {
                let (sql, name, columns) =
                    created(table, &converted, &log).map_err(|reason| refused(&reason))?;
                converted.relations.insert(name, columns);
                sql
            }
            [Expression::Insert(insert)] => {
                inserted(insert, &converted, &log).map_err(|reason| refused(&reason))?
            }
            [Expression::CreateType(create)] => {
                let (name, base) =
                    defined(create, &converted).map_err(|reason| refused(&reason))?;
                converted.types.insert(name, base);
                continue;
            }
            [Expression::SetStatement(set)] => {
                let assignments = assigned(set).map_err(|reason| refused(&reason))?;
                converted.variables.extend(assignments);
                continue;
            }
            [Expression::UnsetStatement(unset)] => {
                let names =
                    unassigned(unset, &converted.variables).map_err(|reason| refused(&reason))?;
                names.iter().for_each(|name| {
                    converted.variables.remove(name);
                });
                continue;
            }
            _ => {
                return Err(refused(&format!(
                    "is not replayed on DuckDB: only {SETUP_REPLAYED} are"
                )));
            }
        };
        converted.statements.push(duckdb);
    }
    Ok(converted)
}
