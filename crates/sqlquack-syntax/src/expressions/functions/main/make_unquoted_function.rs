//! `make_unquoted_function`.

use crate::expressions::functions::models::Function;
use crate::expressions::functions::models::SetQuantifier;
use crate::expressions::tree::models::Expression;
use crate::tokens::models::SourceSpan;

pub fn make_unquoted_function(name: &str, args: Vec<Expression>) -> Expression {
    Expression::Function(Box::new(Function {
        argument_span: Default::default(),
        name_span: Default::default(),
        on_error: None,
        qualified_name: Vec::new(),
        source_dialect: None,

        name: name.to_string(),
        args,
        set_quantifier: SetQuantifier::None,
        trailing_comments: Vec::new(),
        use_bracket_syntax: false,
        no_parens: false,
        quoted: false,
        span: SourceSpan::default(),
        inferred_type: None,
        export_state: false,
        from_separator: false,
        written_as: None,
    }))
}
