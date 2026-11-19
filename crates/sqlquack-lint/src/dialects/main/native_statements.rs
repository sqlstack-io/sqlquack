//! `native_statements`: the statements of SQL from a dialect's own strict parser.

use sqlquack_syntax::guard::main::enforce_ast::enforce_ast;
use sqlquack_syntax::{ComplexityGuardOptions, Expression, ParseOptions};

use crate::dialects::_helpers::parse_errors::{parse_error, uncovered};
use crate::dialects::types::StrictParse;

/// The statements `parse` reads from `sql`, refusing what that engine refuses and trees past `options`' AST depth.
pub(crate) fn native_statements(
    sql: &str,
    parse: StrictParse,
    options: &ParseOptions,
) -> Result<Vec<Expression>, String> {
    let statements: Vec<Expression> = parse(sql).map_err(|(position, message)| match message {
        Some(message) => parse_error(sql, position, &message),
        None => uncovered(sql, position),
    })?;
    let depth_only = ComplexityGuardOptions {
        max_ast_nodes: None,
        max_ast_depth: options
            .complexity_guard
            .as_ref()
            .and_then(|guard| guard.max_ast_depth),
        ..Default::default()
    };
    enforce_ast(&statements, &depth_only).map_err(|error| error.to_string())?;
    Ok(statements)
}
