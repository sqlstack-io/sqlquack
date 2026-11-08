//! SQLBuild's analysis-only type markers, lowered to `Annotated` as the other parsers lower them.

use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::statements::models::Annotated;
use sqlquack_syntax::expressions::tree::constants::{
    TYPED_EXPRESSION_FUNCTION, TYPED_LIKE_EXPRESSION_FUNCTION,
};
use sqlquack_syntax::{DataType, Expression};

use crate::parse::_helpers::tree::cursor::Parser;
use crate::scan::main::scan::scan;

/// A marker takes the expression and its type, or the expression it is typed like.
const MARKER_ARITY: usize = 2;

/// A plain call of `name` as a marker's annotation, or its arguments back when it is not one.
pub(crate) fn typed_marker(
    name: &[Identifier],
    args: Vec<Expression>,
) -> Result<Expression, Vec<Expression>> {
    let [called] = name else {
        return Err(args);
    };
    let like: bool = called
        .name
        .eq_ignore_ascii_case(TYPED_LIKE_EXPRESSION_FUNCTION);
    if called.quoted || !like && !called.name.eq_ignore_ascii_case(TYPED_EXPRESSION_FUNCTION) {
        return Err(args);
    }
    let [this, second]: [Expression; MARKER_ARITY] = args.try_into()?;
    let (type_assertion, type_like) = match (like, written_type(&second)) {
        (true, _) => (None, Some(Box::new(second))),
        (false, Some(data_type)) => (Some(data_type), None),
        (false, None) => return Err(vec![this, second]),
    };
    Ok(Expression::Annotated(Box::new(Annotated {
        this,
        trailing_comments: Vec::new(),
        type_assertion,
        type_like,
    })))
}

/// The type a string literal spells, read whole by DuckDB's type grammar.
fn written_type(literal: &Expression) -> Option<DataType> {
    let Expression::Literal(literal) = literal else {
        return None;
    };
    let Literal::String(text) = literal.as_ref() else {
        return None;
    };
    let mut parser: Parser<'_> = Parser::new(text, scan(text));
    match parser.data_type() {
        Ok(data_type) => parser.peek().is_none().then_some(data_type),
        Err(_) => None,
    }
}
