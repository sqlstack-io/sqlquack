//! `analyze`: parse Databricks SQL, then convert its one statement or refuse.

use sqlquack_diagnostics::classification::models::{Construct, ConstructFamily};
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_parse::databricks::parse::main::parse::parse;
use sqlquack_syntax::tokens::errors::ParseFailure;
use sqlquack_syntax::tokens::models::SourceLines;

use crate::databricks::conversion::main::_convert_statement::convert_statement;
use crate::databricks::conversion::models::Converted;

/// The DuckDB conversion of `sql`, which must be one Databricks statement the parser reads whole.
pub fn analyze(sql: &str) -> Result<Converted, Refusal> {
    let parsed = parse(sql).map_err(|error| {
        refuse_unparsed(
            &ParseFailure {
                message: error.message,
                location: None,
            },
            &SourceLines::new(sql),
        )
    })?;
    if !parsed.gaps.is_empty() {
        return Err(missing(
            statement_construct(),
            "the parser kept part of the statement outside the tree".to_owned(),
        ));
    }
    match parsed.statements.as_slice() {
        [statement] => convert_statement(statement),
        statements => Err(missing(
            statement_construct(),
            format!("a conversion reads one statement, not {}", statements.len()),
        )),
    }
}

fn statement_construct() -> Construct {
    Construct::new(ConstructFamily::Node, "statement")
}
