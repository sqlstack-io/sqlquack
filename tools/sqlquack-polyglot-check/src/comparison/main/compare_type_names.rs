//! `compare_type_names`: SQLQuack's cast type names against polyglot's printed types.

use polyglot_sql::DialectType;
use sqlquack_diagnostics::snowflake::classification::main::cast_type_name::cast_type_name;
use sqlquack_syntax::{DataType, Dialect, Expression, ExpressionWalk, ParseOptions};
use sqlquack_syntax_snowflake::parse_statements;

use crate::comparison::_helpers::outcomes::polyglot_base_type_name;

/// Each cast in Snowflake `sql` whose classified name differs from polyglot's, as text.
pub fn compare_type_names(sql: &str, dialect: Dialect) -> Vec<String> {
    if dialect != Dialect::Snowflake {
        return Vec::new();
    }
    let Ok(statements) = parse_statements(sql, dialect, &ParseOptions::default()) else {
        return Vec::new();
    };
    let mut mismatches: Vec<String> = Vec::new();
    for statement in &statements {
        for node in statement.dfs() {
            let target: &DataType = match node {
                Expression::Cast(cast) | Expression::TryCast(cast) | Expression::SafeCast(cast) => {
                    &cast.to
                }
                _ => continue,
            };
            let ours = cast_type_name(target);
            let theirs = polyglot_base_type_name(target, DialectType::Snowflake);
            if ours != theirs {
                mismatches.push(format!("{target:?}: sqlquack {ours}, polyglot {theirs}"));
            }
        }
    }
    mismatches
}
