//! SQLBuild's analysis-only type markers, lowered to `Annotated` as the other parsers lower them.

use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::statements::models::Annotated;
use sqlquack_syntax::expressions::tree::constants::{
    TYPED_EXPRESSION_FUNCTION, TYPED_LIKE_EXPRESSION_FUNCTION,
};
use sqlquack_syntax::{DataType, Expression};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::scan::main::scan::scan;

/// A marker takes the expression and its type, or the expression it is typed like.
const MARKER_ARITY: usize = 2;

impl Parser<'_> {
    /// The bare call `name` at its `(` as a marker, departing from SQL Server's 195; else `None`, cursor unmoved.
    pub(crate) fn typed_marker(&mut self, name: &str) -> Parsing<Option<Expression>> {
        let like: bool = name.eq_ignore_ascii_case(TYPED_LIKE_EXPRESSION_FUNCTION);
        if !like && !name.eq_ignore_ascii_case(TYPED_EXPRESSION_FUNCTION) {
            return Ok(None);
        }
        let open: usize = self.at;
        let read: Parsing<Vec<Expression>> = self.attempt(|parser| {
            parser.expect_char('(')?;
            let args: Vec<Expression> = parser.comma_list(Self::scalar)?;
            parser.expect_char(')')?;
            Ok(args)
        });
        let lowered: Option<Expression> = match read {
            Ok(args) => lowered(like, args),
            Err(error) if self.gapped() || !error.syntax => return Err(error),
            Err(_) => None,
        };
        if lowered.is_none() {
            self.at = open;
        }
        Ok(lowered)
    }
}

/// The marker's annotation, unless it has another arity or its type text is not one T-SQL type.
fn lowered(like: bool, args: Vec<Expression>) -> Option<Expression> {
    let [this, second]: [Expression; MARKER_ARITY] = match args.try_into() {
        Ok(pair) => pair,
        Err(_) => return None,
    };
    let (type_assertion, type_like) = match like {
        true => (None, Some(Box::new(second))),
        false => (Some(written_type(&second)?), None),
    };
    Some(Expression::Annotated(Box::new(Annotated {
        this,
        trailing_comments: Vec::new(),
        type_assertion,
        type_like,
    })))
}

/// The type a string literal spells, read whole by the T-SQL type grammar.
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
