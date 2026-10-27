//! `make_unquoted_function`.

use crate::expressions::functions::models::Function;
use crate::expressions::tree::models::Expression;

pub(crate) fn make_unquoted_function(name: &str, args: Vec<Expression>) -> Expression {
    Expression::Function(Box::new(Function {
        on_error: None,
        qualified_name: Vec::new(),
        source_dialect: None,

        name: name.to_string(),
        args,
        distinct: false,
        trailing_comments: Vec::new(),
        use_bracket_syntax: false,
        no_parens: false,
        quoted: false,
        span: None,
        inferred_type: None,
    }))
}
