//! `func_expr` (gram.y): calls with their aggregate and window clauses, sorting, special calls.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::functions::models::WithinGroup;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::operators::models::{NamedArgSeparator, NamedArgument};
use sqlquack_syntax::expressions::queries::models::{
    Ordered, Over, WindowFrame, WindowFrameBound, WindowFrameExclude, WindowFrameKind,
};
use sqlquack_syntax::expressions::statements::models::{Annotated, Variadic};
use sqlquack_syntax::expressions::tree::constants::{
    TYPED_EXPRESSION_FUNCTION, TYPED_LIKE_EXPRESSION_FUNCTION,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id, is_type_function_name};
use crate::parse::_helpers::tree::nodes::{
    aggregate, function, literal, ordered, over, star, window_function,
};
use crate::parse::main::parse_type::parse_type;
use crate::scan::models::{Token, TokenKind};

/// A type-assertion marker takes the expression and the type, or the expression it is typed like.
const MARKER_ARITY: usize = 2;

/// Calls written with keywords inside their parentheses (`func_expr_common_subexpr`).
const SPECIAL_CALLS: [&str; 10] = [
    "coalesce",
    "extract",
    "greatest",
    "least",
    "normalize",
    "nullif",
    "overlay",
    "position",
    "substring",
    "trim",
];

/// Window frame words that never name an existing window (gram.y gives them `IDENT` precedence).
const FRAME_WORDS: [&str; 4] = ["partition", "range", "rows", "groups"];

fn call(name: &str, args: Vec<Expression>) -> Expression {
    Expression::Function(Box::new(function(vec![Identifier::new(name)], args)))
}

/// A SQLBuild type marker call as an `Annotated` node, as sqlquack-syntax lowers it; else the args.
fn typed_marker(name: &[Identifier], args: Vec<Expression>) -> Result<Expression, Vec<Expression>> {
    let [called] = name else {
        return Err(args);
    };
    let [this, second]: [Expression; MARKER_ARITY] = args.try_into()?;
    if called.quoted {
        return Err(vec![this, second]);
    }
    if called.name == TYPED_LIKE_EXPRESSION_FUNCTION {
        return Ok(annotated(this, None, Some(Box::new(second))));
    }
    if called.name != TYPED_EXPRESSION_FUNCTION {
        return Err(vec![this, second]);
    }
    let Expression::Literal(literal) = &second else {
        return Err(vec![this, second]);
    };
    let Literal::String(text) = literal.as_ref() else {
        return Err(vec![this, second]);
    };
    match parse_type(text) {
        Ok(data_type) => Ok(annotated(this, Some(data_type), None)),
        Err(_) => Err(vec![this, second]),
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

impl Parser<'_> {
    /// Whether `name` followed by `(` is one of the special calls.
    pub(crate) fn is_special_call(&self, name: &str) -> bool {
        SPECIAL_CALLS.contains(&name)
    }

    /// `func_application` and, unless `windowless`, `WITHIN GROUP`, `FILTER` and `OVER`.
    pub(crate) fn function_call(
        &mut self,
        name: Vec<Identifier>,
        windowless: bool,
    ) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_char('(')?;
        let (mut distinct, mut all_star, mut variadic) = (false, false, false);
        let mut args: Vec<Expression> = Vec::new();
        let mut order: Vec<Ordered> = Vec::new();
        if self.is_char('*') && self.char_at(1, ')') {
            self.at += 1;
            all_star = true;
        } else if !self.is_char(')') {
            let quantified: bool = if self.eat_kw("distinct") {
                distinct = true;
                true
            } else {
                self.eat_kw("all")
            };
            loop {
                if !quantified && self.eat_kw("variadic") {
                    variadic = true;
                    let argument: Expression = self.function_argument()?;
                    args.push(Expression::Variadic(Box::new(Variadic {
                        this: Box::new(argument),
                    })));
                    break;
                }
                args.push(self.function_argument()?);
                if !self.eat_char(',') {
                    break;
                }
            }
            if self.kw("order") && self.kw_at(1, "by") {
                order = self.sort_clause()?;
            }
        }
        self.expect_char(')')?;
        let (mut within, mut filter, mut window) = (None, None, None);
        if !windowless {
            if self.kw("within") && self.kw_at(1, "group") {
                self.at += 2;
                self.expect_char('(')?;
                let sort: Vec<Ordered> = self.sort_clause()?;
                self.expect_char(')')?;
                let refusal: Option<&str> = if !order.is_empty() {
                    Some("cannot use multiple ORDER BY clauses with WITHIN GROUP")
                } else if distinct {
                    Some("cannot use DISTINCT with WITHIN GROUP")
                } else if variadic {
                    Some("cannot use VARIADIC with WITHIN GROUP")
                } else {
                    None
                };
                if let Some(message) = refusal {
                    return Err(self.error_with(message, start));
                }
                within = Some(sort);
            }
            (filter, window) = self.filter_and_window()?;
        }
        let plain: bool = !distinct
            && !all_star
            && !variadic
            && order.is_empty()
            && filter.is_none()
            && within.is_none()
            && window.is_none();
        let args: Vec<Expression> = match plain {
            true => match typed_marker(&name, args) {
                Ok(marker) => return Ok(marker),
                Err(args) => args,
            },
            false => args,
        };
        let mut call = function(name, args);
        if all_star {
            call.args = vec![star(None)];
        }
        let aggregate_form: bool =
            distinct || all_star || !order.is_empty() || filter.is_some() || within.is_some();
        let mut node: Expression = match aggregate_form {
            true => aggregate(call, distinct, filter, order),
            false => Expression::Function(Box::new(call)),
        };
        if let Some(order_by) = within {
            node = Expression::WithinGroup(Box::new(WithinGroup {
                this: node,
                order_by,
            }));
        }
        Ok(match window {
            Some(window) => window_function(node, window),
            None => node,
        })
    }

    /// `filter_clause over_clause`: `FILTER (WHERE ...)` and `OVER window`, when present.
    pub(crate) fn filter_and_window(&mut self) -> Parsing<(Option<Expression>, Option<Over>)> {
        let mut filter: Option<Expression> = None;
        if self.kw("filter") && self.char_at(1, '(') {
            self.at += 2;
            self.expect_kw("where")?;
            filter = Some(self.a_expr()?);
            self.expect_char(')')?;
        }
        if !(self.kw("over") && (self.char_at(1, '(') || self.peek_at(1).is_some_and(is_col_id))) {
            return Ok((filter, None));
        }
        self.at += 1;
        let window: Over = match self.is_char('(') {
            true => self.window_specification()?,
            false => over(Some(self.name(is_col_id)?), Vec::new(), Vec::new(), None),
        };
        Ok((filter, Some(window)))
    }

    /// `func_arg_expr`: `a_expr`, or `name := a_expr` / `name => a_expr`.
    pub(crate) fn function_argument(&mut self) -> Parsing<Expression> {
        let separator: Option<NamedArgSeparator> = match self.peek_at(1).map(|token| token.kind) {
            Some(TokenKind::ColonEquals) => Some(NamedArgSeparator::ColonEq),
            Some(TokenKind::EqualsGreater) => Some(NamedArgSeparator::DArrow),
            _ => None,
        };
        match separator {
            Some(separator) if self.is(is_type_function_name) => {
                let name: Identifier = self.name(is_type_function_name)?;
                self.at += 1;
                let value: Expression = self.a_expr()?;
                Ok(Expression::NamedArgument(Box::new(NamedArgument {
                    name,
                    value,
                    separator,
                })))
            }
            _ => self.a_expr(),
        }
    }

    /// `window_specification`: `( [name] [PARTITION BY ...] [ORDER BY ...] [frame] )`.
    pub(crate) fn window_specification(&mut self) -> Parsing<Over> {
        self.expect_char('(')?;
        let existing: Option<Identifier> = match self.peek() {
            Some(token)
                if is_col_id(token)
                    && !self
                        .keyword_at(0)
                        .is_some_and(|word| FRAME_WORDS.contains(&word)) =>
            {
                Some(self.name(is_col_id)?)
            }
            _ => None,
        };
        let partition: Vec<Expression> = match self.kw("partition") && self.kw_at(1, "by") {
            true => {
                self.at += 2;
                self.expr_list()?
            }
            false => Vec::new(),
        };
        let order: Vec<Ordered> = match self.kw("order") && self.kw_at(1, "by") {
            true => self.sort_clause()?,
            false => Vec::new(),
        };
        let frame: Option<WindowFrame> = self.frame()?;
        self.expect_char(')')?;
        Ok(over(existing, partition, order, frame))
    }

    /// `opt_frame_clause`: `RANGE|ROWS|GROUPS` bounds and an exclusion.
    fn frame(&mut self) -> Parsing<Option<WindowFrame>> {
        let kind: WindowFrameKind = match self.keyword_at(0) {
            Some("range") => WindowFrameKind::Range,
            Some("rows") => WindowFrameKind::Rows,
            Some("groups") => WindowFrameKind::Groups,
            _ => return Ok(None),
        };
        let kind_text: String = self.advance()?.value.to_uppercase();
        let (start, end) = match self.eat_kw("between") {
            true => {
                let start: WindowFrameBound = self.frame_bound()?;
                self.expect_kw("and")?;
                (start, Some(self.frame_bound()?))
            }
            false => (self.frame_bound()?, None),
        };
        let exclude: Option<WindowFrameExclude> = match self.eat_kw("exclude") {
            false => None,
            true => Some(match self.advance()?.value.as_str() {
                "current" => {
                    self.expect_kw("row")?;
                    WindowFrameExclude::CurrentRow
                }
                "group" => WindowFrameExclude::Group,
                "ties" => WindowFrameExclude::Ties,
                "no" => {
                    self.expect_kw("others")?;
                    WindowFrameExclude::NoOthers
                }
                _ => {
                    self.at -= 1;
                    return Err(self.error());
                }
            }),
        };
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

    /// `frame_bound`: `UNBOUNDED PRECEDING|FOLLOWING`, `CURRENT ROW`, `a_expr PRECEDING|FOLLOWING`.
    fn frame_bound(&mut self) -> Parsing<WindowFrameBound> {
        if self.kw("unbounded") && (self.kw_at(1, "preceding") || self.kw_at(1, "following")) {
            self.at += 2;
            return Ok(match self.tokens[self.at - 1].value.as_str() {
                "preceding" => WindowFrameBound::UnboundedPreceding,
                _ => WindowFrameBound::UnboundedFollowing,
            });
        }
        if self.kw("current") && self.kw_at(1, "row") {
            self.at += 2;
            return Ok(WindowFrameBound::CurrentRow);
        }
        let offset: Expression = self.a_expr()?;
        if self.eat_kw("preceding") {
            return Ok(WindowFrameBound::Preceding(Box::new(offset)));
        }
        self.expect_kw("following")?;
        Ok(WindowFrameBound::Following(Box::new(offset)))
    }

    /// `sort_clause`: `ORDER BY sortby, ...`.
    pub(crate) fn sort_clause(&mut self) -> Parsing<Vec<Ordered>> {
        self.expect_kw("order")?;
        self.expect_kw("by")?;
        self.comma_list(|parser| parser.sort_by())
    }

    /// `sortby`: `a_expr [ASC|DESC | USING op] [NULLS FIRST|LAST]`.
    fn sort_by(&mut self) -> Parsing<Ordered> {
        let this: Expression = self.a_expr()?;
        let (mut desc, mut explicit_asc) = (false, false);
        if self.eat_kw("asc") {
            explicit_asc = true;
        } else if self.eat_kw("desc") {
            desc = true;
        } else if self.kw("using") {
            let start: usize = self.offset();
            self.at += 1;
            let operator: String = match self.kw("operator") && self.char_at(1, '(') {
                true => {
                    self.at += 1;
                    self.operator_name()?
                }
                false => {
                    let token: Token = self.expect_token(
                        crate::parse::_helpers::values::expressions::is_operator_token,
                    )?;
                    token.value
                }
            };
            match operator.as_str() {
                "<" => explicit_asc = true,
                ">" => desc = true,
                _ => self.gap("order_by_using_operator", start),
            }
        }
        let nulls_first: Option<bool> = self.nulls_order();
        Ok(ordered(this, desc, explicit_asc, nulls_first))
    }

    /// A special call whose arguments use keywords, as the function PostgreSQL turns it into.
    pub(crate) fn special_call(&mut self, name: &'static str) -> Parsing<Expression> {
        self.at += 1;
        self.expect_char('(')?;
        let node: Expression = match name {
            "extract" => {
                let field: String = match self.peek().map(|token| token.kind) {
                    Some(TokenKind::Ident) => self.advance()?.value,
                    Some(TokenKind::String | TokenKind::EscapeString | TokenKind::DollarString) => {
                        self.advance()?.value
                    }
                    Some(TokenKind::Keyword(keyword))
                        if ["year", "month", "day", "hour", "minute", "second"]
                            .contains(&keyword.name) =>
                    {
                        self.advance()?.value
                    }
                    _ => return Err(self.error()),
                };
                self.expect_kw("from")?;
                let source: Expression = self.a_expr()?;
                call("extract", vec![literal(Literal::String(field)), source])
            }
            "normalize" => {
                let mut args: Vec<Expression> = vec![self.a_expr()?];
                if self.eat_char(',') {
                    let form: String = match self.keyword_at(0) {
                        Some(form @ ("nfc" | "nfd" | "nfkc" | "nfkd")) => form.to_uppercase(),
                        _ => return Err(self.error()),
                    };
                    self.at += 1;
                    args.push(literal(Literal::String(form)));
                }
                call("normalize", args)
            }
            "position" => {
                let needle: Expression = self.b_expr()?;
                self.expect_kw("in")?;
                let haystack: Expression = self.b_expr()?;
                call("position", vec![haystack, needle])
            }
            "overlay" | "substring" => self.keyword_arguments(name)?,
            "trim" => {
                let function_name: &str = if self.eat_kw("leading") {
                    "ltrim"
                } else if self.eat_kw("trailing") {
                    "rtrim"
                } else {
                    self.eat_kw("both");
                    "btrim"
                };
                let args: Vec<Expression> = if self.eat_kw("from") {
                    self.expr_list()?
                } else {
                    let first: Expression = self.a_expr()?;
                    if self.eat_kw("from") {
                        let mut list: Vec<Expression> = self.expr_list()?;
                        list.push(first);
                        list
                    } else {
                        let mut list: Vec<Expression> = vec![first];
                        if self.eat_char(',') {
                            list.extend(self.expr_list()?);
                        }
                        list
                    }
                };
                call(function_name, args)
            }
            "nullif" => {
                let left: Expression = self.a_expr()?;
                self.expect_char(',')?;
                call("nullif", vec![left, self.a_expr()?])
            }
            _ => call(name, self.expr_list()?),
        };
        self.expect_char(')')?;
        Ok(node)
    }

    /// `OVERLAY` and `SUBSTRING`: their keyword forms, or an ordinary argument list.
    fn keyword_arguments(&mut self, name: &str) -> Parsing<Expression> {
        if self.is_char(')') {
            return Ok(call(name, Vec::new()));
        }
        let first: Expression = self.function_argument()?;
        let args: Vec<Expression> = match (name, self.keyword_at(0)) {
            ("overlay", Some("placing")) => {
                self.at += 1;
                let replacement: Expression = self.a_expr()?;
                self.expect_kw("from")?;
                let mut args: Vec<Expression> = vec![first, replacement, self.a_expr()?];
                if self.eat_kw("for") {
                    args.push(self.a_expr()?);
                }
                args
            }
            ("substring", Some("from")) => {
                self.at += 1;
                let mut args: Vec<Expression> = vec![first, self.a_expr()?];
                if self.eat_kw("for") {
                    args.push(self.a_expr()?);
                }
                args
            }
            ("substring", Some("for")) => {
                self.at += 1;
                let count: Expression = self.a_expr()?;
                let from: Expression = match self.eat_kw("from") {
                    true => self.a_expr()?,
                    false => literal(Literal::Number("1".to_owned())),
                };
                vec![first, from, count]
            }
            ("substring", Some("similar")) => {
                self.at += 1;
                let pattern: Expression = self.a_expr()?;
                self.expect_kw("escape")?;
                vec![first, pattern, self.a_expr()?]
            }
            _ => {
                let mut args: Vec<Expression> = vec![first];
                while self.eat_char(',') {
                    args.push(self.function_argument()?);
                }
                args
            }
        };
        Ok(call(name, args))
    }
}
