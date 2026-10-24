//! `check_source_fidelity`: whether the parsed tree survives printing back to its own dialect.

use polyglot_sql::Expression;

use crate::comparison::main::compare_trees::compare_trees;
use crate::parsing::main::parse_source::parse_source;
use crate::parsing::models::SourceDialect;
use crate::reparse::_helpers::single::single_statement;
use crate::reparse::models::FidelityOutcome;

/// Prints `statement` in its own dialect, re-parses it and classifies any tree change.
pub fn check_source_fidelity(statement: &Expression, dialect: SourceDialect) -> FidelityOutcome {
    let first = match print_and_parse(statement, dialect) {
        Ok(first) => first,
        Err(outcome) => return outcome,
    };
    let Some(change) = compare_trees(statement, &first) else {
        return FidelityOutcome::Identical;
    };
    let second = match print_and_parse(&first, dialect) {
        Ok(second) => second,
        Err(outcome) => return outcome,
    };
    match compare_trees(&first, &second) {
        None => FidelityOutcome::Canonicalised { difference: change },
        Some(difference) => FidelityOutcome::Unstable { difference },
    }
}

fn print_and_parse(
    statement: &Expression,
    dialect: SourceDialect,
) -> Result<Expression, FidelityOutcome> {
    let sql = polyglot_sql::generate(statement, dialect.polyglot()).map_err(|error| {
        FidelityOutcome::PrintFailed {
            message: error.to_string(),
        }
    })?;
    single_statement(parse_source(&sql, dialect))
        .map_err(|message| FidelityOutcome::ReparseFailed { message })
}
