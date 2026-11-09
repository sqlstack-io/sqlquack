//! `convert_statement`: DuckDB SQL for one canonical Snowflake query, or the first refusal.

use crate::snowflake::conversion::_helpers::support::typed_leaves::checked;
use sqlquack_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_blocks::checking::main::check_printed::check_printed;
use sqlquack_blocks::held::main::variant_written::variant_written;
use sqlquack_blocks::semi_structured::main::json_output::json_output;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::tokens::models::SourceLines;

use crate::snowflake::conversion::_helpers::queries::function_body::{
    column_types, function_stub, override_stub,
};
use crate::snowflake::conversion::_helpers::queries::query::convert_query;
use crate::snowflake::conversion::_helpers::support::depth::positioned;
use crate::snowflake::conversion::_helpers::support::locations::Locator;
use crate::snowflake::conversion::_helpers::support::scope::{Context, StatementTracks};
use crate::snowflake::conversion::_helpers::support::tree::{node_construct, session_tree};
use crate::snowflake::conversion::constants::{NULL_BRANCHES, OUTPUT_ALIAS};
use crate::snowflake::conversion::models::{ConversionOptions, Converted};
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_semantics::schemas::models::Holding;
use sqlquack_semantics::snowflake::types::models::TypedLeaves;
use sqlquack_semantics::snowflake::typing::types::{ColumnSpec, Schema};

/// Converts a statement over the schema's relations, placing refusals in `lines`; DuckDB must bind it.
pub fn convert_statement(
    statement: &Expression,
    (schema, options, lines): (&Schema, &ConversionOptions, &SourceLines),
    log: ProvenanceLog,
) -> Result<Converted, Refusal> {
    let positioned = positioned(statement)?;
    let tree = session_tree(&positioned, options.quoted_identifiers_ignore_case)
        .map_err(|reason| outside_conditions(node_construct("statement"), reason))?;
    let locator = Locator::new(&tree, &positioned, lines);
    let typed = TypedLeaves::default();
    let tracks = StatementTracks::default();
    let context = Context {
        log: &log,
        schema,
        options,
        ctes: Vec::new(),
        locator: &locator,
        typed: &typed,
        order: &tracks.order,
        subqueries: &tracks.subqueries,
        qualifiers: &tracks.qualifiers,
        free_projections: false,
        fixture_body: false,
    };
    let (sql, mut columns) = checked(&typed, convert_query(&tree, None, &context, true))
        .map_err(|refusal| locator.placed(refusal, &tree))?;
    let (sql, raises) = locator.finished(&output_variants(sql, &columns, &log), &tree);
    for column in &mut columns {
        if column.data_type == SnowflakeType::Null {
            column.data_type = SnowflakeType::Varchar;
        }
    }
    let relation = |name: &str| -> Option<Vec<(String, SnowflakeType)>> {
        let columns = schema.relations.get(name)?;
        Some(column_types(columns))
    };
    let function =
        |name: &str| function_stub(schema, name).or_else(|| override_stub(options, name));
    check_printed(&sql, (&relation, &function)).map_err(|failure| {
        let refusal = missing(
            node_construct("statement"),
            format!("the emitted DuckDB SQL fails DuckDB's check: {failure}"),
        );
        locator.placed(refusal, &tree)
    })?;
    Ok(Converted {
        raises,
        sql,
        columns,
        provenance: log.finish(),
    })
}

/// The query with VARIANT result columns written as Snowflake returns them, NULL-only ones VARCHAR.
fn output_variants(sql: String, columns: &[ColumnSpec], log: &ProvenanceLog) -> String {
    let replaced: Vec<String> = columns
        .iter()
        .filter_map(|column| {
            let name = quoted_identifier(&column.name, log);
            match (column.data_type, column.holding) {
                (SnowflakeType::SemiStructured, Holding::Variant) => Some(format!(
                    "CAST({} AS JSON) AS {name}",
                    variant_written(&name, log)
                )),
                (SnowflakeType::SemiStructured, Holding::Json | Holding::Chained) => {
                    Some(format!("{} AS {name}", json_output(&name, log)))
                }
                (SnowflakeType::Null, _) => {
                    let _rule = log.rule(NULL_BRANCHES);
                    Some(format!("CAST({name} AS VARCHAR) AS {name}"))
                }
                _ => None,
            }
        })
        .collect();
    match replaced.is_empty() {
        true => sql,
        false => format!(
            "SELECT * REPLACE ({}) FROM ({sql}) AS {OUTPUT_ALIAS}",
            replaced.join(", ")
        ),
    }
}
