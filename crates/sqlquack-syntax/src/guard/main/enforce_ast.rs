//! `enforce_ast`: enforce AST size/depth limits and report parse-oriented errors.

use crate::expressions::tree::models::Expression;
use crate::guard::_helpers::ast::ast_stats;
use crate::guard::_helpers::ast::enforce_ast_depth_only;
use crate::guard::_helpers::codes::parse_guard_error;
use crate::guard::models::ComplexityGuardOptions;
use crate::tokens::types::Result;

/// Enforce AST size/depth limits and report parse-oriented errors.
pub(crate) fn enforce_ast(
    expressions: &[Expression],
    options: &ComplexityGuardOptions,
) -> Result<()> {
    let Some(max_nodes) = options.max_ast_nodes else {
        return enforce_ast_depth_only(expressions, options);
    };

    let stats = ast_stats(expressions, Some(max_nodes), options.max_ast_depth)?;
    if stats.node_count > max_nodes {
        return Err(parse_guard_error(
            "E_GUARD_AST_BUDGET_EXCEEDED",
            stats.node_count,
            max_nodes,
            None,
        ));
    }

    if let Some(max_depth) = options.max_ast_depth {
        if stats.max_depth > max_depth {
            return Err(parse_guard_error(
                "E_GUARD_AST_DEPTH_EXCEEDED",
                stats.max_depth,
                max_depth,
                None,
            ));
        }
    }

    Ok(())
}
