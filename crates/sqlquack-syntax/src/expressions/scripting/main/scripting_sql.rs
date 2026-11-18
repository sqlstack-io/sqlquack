//! `scripting_sql`.

use crate::expressions::scripting::models::{ScriptingSql, ScriptingStatement};
use crate::expressions::tree::models::Expression;
use crate::tokens::models::SourceSpan;

/// An ordinary statement as a scripting body member.
pub fn scripting_sql(statement: Expression) -> ScriptingStatement {
    ScriptingStatement::Sql(ScriptingSql {
        span: SourceSpan(None),
        statement,
    })
}
