//! Calls (parse.y's `idj LP ...` rules) and windows (`filter_over`, `window`, `frame_opt`).

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::queries::models::{
    Ordered, Over, WindowFrame, WindowFrameBound, WindowFrameExclude, WindowFrameKind,
};
use sqlquack_syntax::expressions::statements::models::Annotated;
use sqlquack_syntax::expressions::tree::constants::{
    TYPED_EXPRESSION_FUNCTION, TYPED_LIKE_EXPRESSION_FUNCTION,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_nm};
use crate::parse::_helpers::tree::nodes::{aggregate, function, over, star, window_function};
use crate::parse::constants::{
    DISTINCT_WINDOW, MAX_COLUMN, MAX_FUNCTION_ARG, ORDER_BY_TERMS, UNSUPPORTED_FRAME,
};
use crate::scan::main::scan::scan;
use crate::scan::models::TokenKind;

/// A type-assertion marker takes the expression and the type, or the expression it is typed like.
const MARKER_ARITY: usize = 2;

/// A frame bound's kind, as `sqlite3WindowAlloc` orders them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Preceding,
    Current,
    Following,
}

/// A SQLBuild type marker call as an `Annotated` node, as sqlquack-syntax lowers it; else the args.
fn typed_marker(name: &Identifier, args: Vec<Expression>) -> Result<Expression, Vec<Expression>> {
    let [this, second]: [Expression; MARKER_ARITY] = args.try_into()?;
    if name.quoted {
        return Err(vec![this, second]);
    }
    if name
        .name
        .eq_ignore_ascii_case(TYPED_LIKE_EXPRESSION_FUNCTION)
    {
        return Ok(annotated(this, None, Some(Box::new(second))));
    }
    if !name.name.eq_ignore_ascii_case(TYPED_EXPRESSION_FUNCTION) {
        return Err(vec![this, second]);
    }
    let Expression::Literal(literal) = &second else {
        return Err(vec![this, second]);
    };
    let Literal::String(text) = literal.as_ref() else {
        return Err(vec![this, second]);
    };
    match type_text(text) {
        Ok(data_type) => Ok(annotated(this, Some(data_type), None)),
        Err(_) => Err(vec![this, second]),
    }
}

/// The type `text` names when all of it is one non-empty `typetoken`, as a `CAST` reads it.
fn type_text(text: &str) -> Parsing<DataType> {
    let mut parser: Parser<'_> = Parser::new(text, scan(text));
    let data_type: DataType = parser.type_token()?;
    match parser.at > 0 && parser.peek().is_none() {
        true => Ok(data_type),
        false => Err(parser.error()),
    }
}

/// `this` annotated with an asserted type, or with the expression whose type it takes.
fn annotated(
    this: Expression,
    type_assertion: Option<DataType>,
    type_like: Option<Box<Expression>>,
) -> Expression {
    Expression::Annotated(Box::new(Annotated {
        this,
        trailing_comments: Vec::new(),
        type_assertion,
        type_like,
    }))
}

/// The side of a frame bound.
fn side(bound: &WindowFrameBound) -> Side {
    match bound {
        WindowFrameBound::CurrentRow => Side::Current,
        WindowFrameBound::UnboundedPreceding
        | WindowFrameBound::Preceding(_)
        | WindowFrameBound::BarePreceding => Side::Preceding,
        _ => Side::Following,
    }
}

impl Parser<'_> {
    /// A call after `name (`: its arguments, `)` and `FILTER` or `OVER`, with parse.y's checks.
    pub(crate) fn function_call(&mut self, name: Identifier) -> Parsing<Expression> {
        let start: usize = self.offset();
        let written: String = name.name.clone();
        let mut args: Vec<Expression> = Vec::new();
        let mut distinct: bool = false;
        let mut order_by: Vec<Ordered> = Vec::new();
        if self.op("STAR") {
            self.at += 1;
            args.push(Expression::Star(star(None, None)));
        } else {
            distinct = self.eat_kw("DISTINCT");
            if !distinct {
                self.eat_kw("ALL");
            }
            if !self.op("RP") && !self.kw("ORDER") {
                args = self.comma_list(|parser| parser.expression())?;
            }
            if self.kw("ORDER") {
                self.at += 1;
                self.expect_kw("BY")?;
                order_by = self.sort_list()?;
            }
        }
        self.expect_op("RP")?;
        let filter: Option<Expression> = match self.kw("FILTER") {
            true => {
                self.at += 1;
                self.expect_op("LP")?;
                self.expect_kw("WHERE")?;
                let condition: Expression = self.expression()?;
                self.expect_op("RP")?;
                Some(condition)
            }
            false => None,
        };
        let window: Option<Over> = match self.eat_kw("OVER") {
            true => Some(self.over_clause()?),
            false => None,
        };
        let mut check: Option<String> = None;
        if args.len() > MAX_FUNCTION_ARG {
            check = Some(format!("too many arguments on function {written}"));
        }
        if window.is_some() && distinct {
            check = Some(DISTINCT_WINDOW.to_owned());
        }
        if !order_by.is_empty() && !args.is_empty() {
            if window.is_some() {
                check = Some(format!(
                    "ORDER BY may not be used with non-aggregate {written}()"
                ));
            } else if order_by.len() > MAX_COLUMN {
                check = Some(ORDER_BY_TERMS.to_owned());
            }
        }
        if let Some(message) = check {
            self.defer(self.error_with(&message, start, false));
        }
        if args.is_empty() {
            order_by.clear();
        }
        let plain: bool = !distinct && filter.is_none() && order_by.is_empty() && window.is_none();
        let args: Vec<Expression> = match plain {
            true => match typed_marker(&name, args) {
                Ok(marker) => return Ok(marker),
                Err(args) => args,
            },
            false => args,
        };
        let call = function(vec![name], args);
        let node: Expression = match distinct || filter.is_some() || !order_by.is_empty() {
            true => {
                let mut node: Expression = aggregate(call, distinct, filter, None);
                if let Expression::AggregateFunction(aggregate) = &mut node {
                    aggregate.order_by = order_by;
                }
                node
            }
            false => Expression::Function(Box::new(call)),
        };
        Ok(match window {
            Some(window) => window_function(node, window),
            None => node,
        })
    }

    /// `over_clause` after `OVER`: `( window )` or a window's name.
    fn over_clause(&mut self) -> Parsing<Over> {
        if !self.op("LP") {
            let name: Identifier = self.name(is_nm)?;
            return Ok(over(Some(name), Vec::new(), Vec::new(), None));
        }
        self.at += 1;
        let (mut spec, base) = self.window()?;
        self.expect_op("RP")?;
        spec.window_name = base.map(Identifier::new);
        Ok(spec)
    }

    /// `window`: `[base] [PARTITION BY ...] [ORDER BY ...] frame_opt`, and the base as written.
    pub(crate) fn window(&mut self) -> Parsing<(Over, Option<String>)> {
        let clause: bool = matches!(
            self.kind_at(0),
            Some(TokenKind::Keyword(
                "PARTITION" | "ORDER" | "RANGE" | "ROWS" | "GROUPS"
            ))
        );
        let base: Option<String> = match !clause && self.is(is_nm) {
            true => {
                let token = self.advance()?;
                Some(token.value)
            }
            false => None,
        };
        let mut partition: Vec<Expression> = Vec::new();
        if self.eat_kw("PARTITION") {
            self.expect_kw("BY")?;
            partition = self.comma_list(|parser| parser.expression())?;
        }
        let order: Vec<Ordered> = match self.kw("ORDER") {
            true => {
                self.at += 1;
                self.expect_kw("BY")?;
                self.sort_list()?
            }
            false => Vec::new(),
        };
        let frame: Option<WindowFrame> = self.window_frame()?;
        Ok((over(None, partition, order, frame), base))
    }

    /// `frame_opt`: `RANGE|ROWS|GROUPS` a start bound, or `BETWEEN` two, and an `EXCLUDE`.
    fn window_frame(&mut self) -> Parsing<Option<WindowFrame>> {
        let kind: WindowFrameKind = match self.kind_at(0) {
            Some(TokenKind::Keyword("RANGE")) => WindowFrameKind::Range,
            Some(TokenKind::Keyword("ROWS")) => WindowFrameKind::Rows,
            Some(TokenKind::Keyword("GROUPS")) => WindowFrameKind::Groups,
            _ => return Ok(None),
        };
        let kind_text: String = self.advance()?.value.to_ascii_uppercase();
        let frame_start: usize = self.offset();
        let (start, end) = match self.eat_kw("BETWEEN") {
            true => {
                let start: WindowFrameBound = self.frame_bound(true)?;
                self.expect_kw("AND")?;
                (start, Some(self.frame_bound(false)?))
            }
            false => (self.frame_bound(true)?, None),
        };
        let end_side: Side = end.as_ref().map_or(Side::Current, side);
        let unsupported: bool = match side(&start) {
            Side::Current => end_side == Side::Preceding,
            Side::Following => end_side != Side::Following,
            Side::Preceding => false,
        };
        let exclude: Option<WindowFrameExclude> = match self.eat_kw("EXCLUDE") {
            true => Some(match self.kind_at(0) {
                Some(TokenKind::Keyword("NO")) => {
                    self.at += 1;
                    self.expect_kw("OTHERS")?;
                    WindowFrameExclude::NoOthers
                }
                Some(TokenKind::Keyword("CURRENT")) => {
                    self.at += 1;
                    self.expect_kw("ROW")?;
                    WindowFrameExclude::CurrentRow
                }
                Some(TokenKind::Keyword("GROUP")) => {
                    self.at += 1;
                    WindowFrameExclude::Group
                }
                Some(TokenKind::Keyword("TIES")) => {
                    self.at += 1;
                    WindowFrameExclude::Ties
                }
                _ => return Err(self.error()),
            }),
            false => None,
        };
        if unsupported {
            self.defer(self.error_with(UNSUPPORTED_FRAME, frame_start, false));
        }
        Ok(Some(WindowFrame {
            kind,
            start,
            end,
            exclude,
            kind_text: Some(kind_text),
            start_side_text: None,
            end_side_text: None,
        }))
    }

    /// `frame_bound_s` when `starting`, else `frame_bound_e`.
    fn frame_bound(&mut self, starting: bool) -> Parsing<WindowFrameBound> {
        if self.eat_kw("UNBOUNDED") {
            return match starting {
                true => self
                    .expect_kw("PRECEDING")
                    .map(|()| WindowFrameBound::UnboundedPreceding),
                false => self
                    .expect_kw("FOLLOWING")
                    .map(|()| WindowFrameBound::UnboundedFollowing),
            };
        }
        if self.eat_kw("CURRENT") {
            self.expect_kw("ROW")?;
            return Ok(WindowFrameBound::CurrentRow);
        }
        let offset: Expression = self.expression()?;
        if self.eat_kw("PRECEDING") {
            return Ok(WindowFrameBound::Preceding(Box::new(offset)));
        }
        self.expect_kw("FOLLOWING")?;
        Ok(WindowFrameBound::Following(Box::new(offset)))
    }
}
