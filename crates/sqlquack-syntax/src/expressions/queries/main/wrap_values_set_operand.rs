//! `wrap_values_set_operand`: rewrites a VALUES set-operation branch as a SELECT from a derived table.

use crate::expressions::names::models::Identifier;
use crate::expressions::queries::models::From;
use crate::expressions::queries::models::Select;
use crate::expressions::tree::models::Expression;

/// Rewrites a VALUES set-operation branch as a SELECT from a derived table.
pub(crate) fn wrap_values_set_operand(expr: Expression) -> Expression {
    match expr {
        Expression::Values(mut values) => {
            if values.alias.is_none() {
                values.alias = Some(Identifier::new("_values"));
            }

            let mut select = Select::new();
            select.expressions = vec![Expression::star()];
            select.from = Some(From {
                expressions: vec![Expression::Values(values)],
            });
            Expression::Select(Box::new(select))
        }
        Expression::Annotated(mut annotated) => {
            annotated.this = wrap_values_set_operand(annotated.this);
            Expression::Annotated(annotated)
        }
        other => other,
    }
}
