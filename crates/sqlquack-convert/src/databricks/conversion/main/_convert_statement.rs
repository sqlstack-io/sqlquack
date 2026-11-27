//! `convert_statement`: DuckDB SQL for one Databricks statement, or the first refusal.

use sqlquack_blocks::checking::main::check_printed::check_printed;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_diagnostics::refusals::models::{Refusal, RefusalCategory};
use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::databricks::conversion::_helpers::query::statement;
use crate::databricks::conversion::_helpers::tree::refused;
use crate::databricks::conversion::constants::MAX_TREE_DEPTH;
use crate::databricks::conversion::models::Converted;

/// Converts a statement that defines every relation it reads; DuckDB must parse and bind the result.
pub(crate) fn convert_statement(tree: &Expression) -> Result<Converted, Refusal> {
    let depth = tree.tree_depth();
    if depth > MAX_TREE_DEPTH {
        return Err(Refusal {
            construct: None,
            location: None,
            category: RefusalCategory::ConversionBudget,
            reason: format!(
                "the statement nests {depth} levels deep, beyond the {MAX_TREE_DEPTH} conversion accepts"
            ),
        });
    }
    let node = serde_json::to_value(tree)
        .map_err(|error| refused("statement", format!("the tree does not serialise: {error}")))?;
    let log = ProvenanceLog::default();
    let (sql, columns) = statement(&node, &log)?;
    check_printed(&sql, (&|_| None, &|_| None)).map_err(|failure| {
        refused(
            "statement",
            format!("the emitted DuckDB SQL fails DuckDB's check: {failure}"),
        )
    })?;
    Ok(Converted {
        sql,
        columns,
        provenance: log.finish(),
    })
}
