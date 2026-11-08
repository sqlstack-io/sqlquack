//! `FunctionExpression` and `SpecialFunctionExpression` (expression.gram), with their windows.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::functions::models::WithinGroup;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::{NamedArgSeparator, NamedArgument};
use sqlquack_syntax::expressions::queries::models::{
    Ordered, Over, WindowFrame, WindowFrameBound, WindowFrameExclude, WindowFrameKind,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier, is_type_func_name};
use crate::parse::_helpers::tree::nodes::{
    aggregate, call, function, ordered, over, string, window_function,
};
use crate::parse::constants::{COALESCE, IFNULL_ARGUMENTS};
use crate::scan::models::TokenKind;

/// The transformer's refusal of `IGNORE|RESPECT NULLS` in a call without `OVER`.
const DISTINCT_NO_ARGUMENTS: &str = "DISTINCT requires at least one argument";
const ALL_NO_ARGUMENTS: &str = "ALL requires at least one argument";
const EXPORT_STATE_OVER: &str = "EXPORT_STATE is not supported for window functions!";
const WINDOW_IN_WINDOW: &str = "window functions are not allowed in window definitions";
const WITHIN_DISTINCT: &str = "DISTINCT is not allowed in combination with WITHIN GROUP";
const WITHIN_ORDER_STATEMENTS: &str = "Cannot use multiple ORDER BY statements with WITHIN GROUP";
const WITHIN_ORDER_CLAUSES: &str = "Cannot use multiple ORDER BY clauses with WITHIN GROUP";
/// The calls the transformer rewrites into operators, and how it names them refusing a named argument.
const REWRITTEN_CONTEXTS: [(&str, &str); 4] = [
    ("unpack", "UNPACK operator"),
    ("try", "TRY expression"),
    ("construct_array", "array constructors"),
    ("ifnull", "IFNULL expressions"),
];
const NULLS_OUTSIDE_WINDOW: &str = "RESPECT/IGNORE NULLS is not supported for non-window functions";
/// The plain calls the transformer rewrites to operators or casts.
const REWRITTEN_CALLS: [&str; 5] = ["unpack", "try", "construct_array", "ifnull", "date"];

/// A call's parenthesized part: `[DISTINCT|ALL] args [ORDER BY ...] [IGNORE|RESPECT NULLS]`.
struct Arguments {
    /// `DISTINCT` or `ALL` with no argument, and where it began: refused in a plain call.
    bare_quantifier: Option<(&'static str, usize)>,
    distinct: bool,
    args: Vec<Expression>,
    order_by: Vec<Ordered>,
    ignore_nulls: Option<bool>,
}

impl Parser<'_> {
    /// `( FunctionExpressionArgumentList )`.
    fn arguments(&mut self) -> Parsing<Arguments> {
        self.expect_char('(')?;
        let start: usize = self.offset();
        let quantifier: Option<&'static str> = match self.word_at(0).as_deref() {
            Some("DISTINCT") => Some(DISTINCT_NO_ARGUMENTS),
            Some("ALL") => Some(ALL_NO_ARGUMENTS),
            _ => None,
        };
        let distinct: bool = quantifier == Some(DISTINCT_NO_ARGUMENTS);
        if quantifier.is_some() {
            self.at += 1;
        }
        let mut args: Vec<Expression> = Vec::new();
        if !(self.is_char(')') || self.order_by_at()) {
            args.push(self.function_argument()?);
            while self.eat_char(',') {
                if self.is_char(')') || self.order_by_at() {
                    break;
                }
                args.push(self.function_argument()?);
            }
        }
        let bare_quantifier: Option<(&'static str, usize)> = quantifier
            .filter(|_| args.is_empty())
            .map(|message| (message, start));
        let order_by: Vec<Ordered> = match self.order_by_at() {
            true => self.order_by_clause()?,
            false => Vec::new(),
        };
        let mut ignore_nulls: Option<bool> = None;
        if self.kw_in(0, &["IGNORE", "RESPECT"]) && self.kw_at(1, "NULLS") {
            ignore_nulls = Some(self.kw("IGNORE"));
            self.at += 2;
        }
        self.expect_char(')')?;
        Ok(Arguments {
            bare_quantifier,
            distinct,
            args,
            order_by,
            ignore_nulls,
        })
    }

    /// `TransformParserFunctionRewrite`'s refusal of a named argument to a call it rewrites.
    fn check_rewritten_named(
        &self,
        name: &[Identifier],
        args: &[Expression],
        start: usize,
    ) -> Parsing<()> {
        let last: String = name
            .last()
            .map(|part| part.name.to_ascii_lowercase())
            .unwrap_or_default();
        let context: Option<&str> = REWRITTEN_CONTEXTS
            .iter()
            .find(|(rewritten, _)| *rewritten == last)
            .map(|(_, context)| *context);
        let named: bool = args
            .iter()
            .any(|arg| matches!(arg, Expression::NamedArgument(_)));
        match (context, named) {
            (Some(context), true) => {
                let message: String = format!("Named arguments are not supported in {context}");
                Err(self.error_with(&message, start, false))
            }
            _ => Ok(()),
        }
    }

    /// Whether `ORDER BY` comes next.
    pub(crate) fn order_by_at(&self) -> bool {
        self.kw("ORDER") && self.kw_at(1, "BY")
    }

    /// A method's arguments after `.name`: the arguments, `DISTINCT` and the argument `ORDER BY`.
    pub(crate) fn method_arguments(&mut self) -> Parsing<(Vec<Expression>, bool, Vec<Ordered>)> {
        let start: usize = self.offset();
        let arguments: Arguments = self.arguments()?;
        if arguments.ignore_nulls.is_some() {
            self.refuse_later(NULLS_OUTSIDE_WINDOW, start);
        }
        Ok((arguments.args, arguments.distinct, arguments.order_by))
    }

    /// A call of `name` with its arguments and trailing clauses.
    pub(crate) fn function_call(&mut self, name: Vec<Identifier>) -> Parsing<Expression> {
        let start: usize = self.offset();
        let Arguments {
            bare_quantifier,
            distinct,
            args,
            order_by,
            ignore_nulls,
        } = self.arguments()?;
        if let Some((message, position)) = bare_quantifier {
            self.refuse_later(message, position);
        }
        let mut within: Option<Vec<Ordered>> = None;
        if self.kw("WITHIN") && self.kw_at(1, "GROUP") && self.char_at(2, '(') {
            self.at += 3;
            within = Some(self.order_by_clause()?);
            self.expect_char(')')?;
        }
        let nested: bool = self.window_definitions > 0;
        let mut filter: Option<Expression> = None;
        if self.kw("FILTER") && self.char_at(1, '(') {
            self.at += 2;
            self.eat_kw("WHERE");
            filter = Some(self.expression()?);
            self.expect_char(')')?;
        }
        if let Some(within) = &within {
            let message: Option<&str> = match (distinct, order_by.is_empty(), within.len()) {
                (true, _, _) => Some(WITHIN_DISTINCT),
                (_, false, _) => Some(WITHIN_ORDER_STATEMENTS),
                (_, _, 1) => None,
                _ => Some(WITHIN_ORDER_CLAUSES),
            };
            if let Some(message) = message {
                return Err(self.error_with(message, start, false));
            }
        }
        self.check_arity(&name, args.len(), within.is_some())?;
        self.check_rewritten_named(&name, &args, start)?;
        let export_state: bool = self.eat_kw("EXPORT_STATE");
        let window: Option<Over> = match self.eat_kw("OVER") {
            true => Some(self.over_clause()?),
            false => None,
        };
        if nested && window.is_some() {
            self.refuse_later(WINDOW_IN_WINDOW, start);
        }
        if export_state && window.is_some() {
            self.refuse_later(EXPORT_STATE_OVER, start);
        }
        if window.is_none() && ignore_nulls.is_some() && !Self::rewritten_call(&name) {
            self.refuse_later(NULLS_OUTSIDE_WINDOW, start);
        }
        let mut call = function(name, args);
        call.export_state = export_state;
        let aggregated: bool =
            distinct || filter.is_some() || ignore_nulls.is_some() || !order_by.is_empty();
        let mut node: Expression = match aggregated {
            true => {
                let mut node = aggregate(call, distinct, filter, ignore_nulls);
                if let Expression::AggregateFunction(aggregate) = &mut node {
                    aggregate.order_by = order_by;
                }
                node
            }
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

    /// Whether the transformer rewrites a plain call of `name` before it checks `IGNORE NULLS`.
    fn rewritten_call(name: &[Identifier]) -> bool {
        name.last().is_some_and(|part| {
            REWRITTEN_CALLS
                .iter()
                .any(|call| part.name.eq_ignore_ascii_case(call))
        })
    }

    /// The argument counts the transformer checks for the calls it rewrites.
    fn check_arity(&self, name: &[Identifier], count: usize, within: bool) -> Parsing<()> {
        let written: String = name
            .last()
            .map(|part| part.name.clone())
            .unwrap_or_default();
        let lower: String = written.to_ascii_lowercase();
        let message: Option<String> = match (lower.as_str(), within) {
            ("percentile_cont", true) if count != 1 => {
                Some("Wrong number of arguments for PERCENTILE_CONT".to_owned())
            }
            ("percentile_disc", true) if count != 1 => {
                Some("Wrong number of arguments for PERCENTILE_DISC".to_owned())
            }
            ("mode", true) if count != 0 => Some("Wrong number of arguments for MODE".to_owned()),
            ("percentile_cont" | "percentile_disc" | "mode", true) => None,
            (_, true) => Some(format!(
                "Unknown ordered aggregate \"{}\".",
                written.replace('"', "\"\"")
            )),
            ("try", _) if count != 1 => {
                Some("Wrong number of arguments provided to TRY expression".to_owned())
            }
            ("unpack", _) if count != 1 => {
                Some("Wrong number of arguments to the UNPACK operator".to_owned())
            }
            ("ifnull", _) if count != IFNULL_ARGUMENTS => {
                Some("Wrong number of arguments to IFNULL.".to_owned())
            }
            ("date", _) if count != 1 => {
                Some("Wrong number of arguments provided to DATE function".to_owned())
            }
            _ => None,
        };
        match message {
            Some(message) => Err(self.error_with(&message, self.offset(), false)),
            None => Ok(()),
        }
    }

    /// `FunctionArgument`: `name := value`, `name => value`, or an expression.
    fn function_argument(&mut self) -> Parsing<Expression> {
        let named: bool =
            self.is(is_type_func_name) && (self.op_at(1, ":=") || self.op_at(1, "=>"));
        if named {
            let name: Identifier = self.name(is_type_func_name)?;
            let separator: NamedArgSeparator = match self.is_op(":=") {
                true => NamedArgSeparator::ColonEq,
                false => NamedArgSeparator::DArrow,
            };
            self.at += 1;
            let value: Expression = self.expression()?;
            return Ok(Expression::NamedArgument(Box::new(NamedArgument {
                name,
                value,
                separator,
            })));
        }
        self.expression()
    }

    /// The `SpecialFunctionExpression` named `word`; `None` when its form does not fit.
    pub(crate) fn special_function(&mut self, word: &str) -> Parsing<Option<Expression>> {
        let attempt = match word {
            "EXTRACT" => self.attempt(|parser| {
                parser.at += 2;
                let part: String = match parser.string_at(0) {
                    true => parser.string_lit()?,
                    false => parser.name(is_identifier)?.name,
                };
                parser.expect_kw("FROM")?;
                let source: Expression = parser.expression()?;
                parser.expect_char(')')?;
                Ok(call("extract", vec![string(&part), source]))
            }),
            "POSITION" => self.attempt(|parser| {
                parser.at += 2;
                let needle: Expression = parser.value_expression(0)?;
                parser.expect_kw("IN")?;
                let haystack: Expression = parser.expression()?;
                parser.expect_char(')')?;
                Ok(call("position", vec![haystack, needle]))
            }),
            "SUBSTRING" => self.attempt(|parser| {
                parser.at += 2;
                let text: Expression = parser.expression()?;
                let mut args: Vec<Expression> = vec![text];
                if parser.eat_kw("FROM") {
                    args.push(parser.expression()?);
                    if parser.eat_kw("FOR") {
                        args.push(parser.expression()?);
                    }
                } else if parser.eat_kw("FOR") {
                    args.push(literal_one());
                    args.push(parser.expression()?);
                } else {
                    while parser.eat_char(',') {
                        if parser.is_char(')') {
                            break;
                        }
                        args.push(parser.expression()?);
                    }
                }
                parser.expect_char(')')?;
                Ok(call("substring", args))
            }),
            "TRIM" => self.attempt(|parser| {
                parser.at += 2;
                let name: &str = match parser.word_at(0).as_deref() {
                    Some("LEADING") => "ltrim",
                    Some("TRAILING") => "rtrim",
                    Some("BOTH") => "trim",
                    _ => "trim",
                };
                if parser.kw_in(0, &["LEADING", "TRAILING", "BOTH"]) {
                    parser.at += 1;
                }
                let characters: Option<Expression> = match parser.eat_kw("FROM") {
                    true => None,
                    false => parser.trim_characters(),
                };
                let source: Vec<Expression> = parser.expression_list(')')?;
                parser.expect_char(')')?;
                Ok(call(name, source.into_iter().chain(characters).collect()))
            }),
            "COALESCE" | "ROW" | "GROUPING" | "GROUPING_ID" => self.attempt(|parser| {
                parser.at += 2;
                let args: Vec<Expression> = match parser.is_char(')') && word != COALESCE {
                    true => Vec::new(),
                    false => parser.expression_list(')')?,
                };
                parser.expect_char(')')?;
                Ok(call(&word.to_ascii_lowercase(), args))
            }),
            "TRY" | "UNPACK" => self.attempt(|parser| {
                parser.at += 2;
                let this: Expression = parser.expression()?;
                parser.expect_char(')')?;
                Ok(call(&word.to_ascii_lowercase(), vec![this]))
            }),
            "NULLIF" => self.attempt(|parser| {
                parser.at += 2;
                let first: Expression = parser.expression()?;
                parser.expect_char(',')?;
                let second: Expression = parser.expression()?;
                parser.expect_char(')')?;
                Ok(call("nullif", vec![first, second]))
            }),
            "OVERLAY" => self.attempt(|parser| {
                parser.at += 2;
                let input: Expression = parser.expression()?;
                if !parser.kw("PLACING") {
                    let mut args: Vec<Expression> = vec![input];
                    if parser.eat_char(',') && !parser.is_char(')') {
                        args.extend(parser.expression_list(')')?);
                    }
                    parser.expect_char(')')?;
                    return Ok(call("overlay", args));
                }
                parser.expect_kw("PLACING")?;
                let replacement: Expression = parser.expression()?;
                parser.expect_kw("FROM")?;
                let mut args: Vec<Expression> = vec![input, replacement, parser.expression()?];
                if parser.eat_kw("FOR") {
                    args.push(parser.expression()?);
                }
                parser.expect_char(')')?;
                Ok(call("overlay", args))
            }),
            _ => return Ok(None),
        };
        let Ok(special) = attempt else {
            return Ok(None);
        };
        Ok(Some(special))
    }

    /// `TRIM`'s `characters FROM`, when it is there; the cursor stays put when it is not.
    fn trim_characters(&mut self) -> Option<Expression> {
        if !self.has_from_before_close() {
            return None;
        }
        let attempt = self.attempt(|parser| {
            let characters: Expression = parser.expression()?;
            parser.expect_kw("FROM")?;
            Ok(characters)
        });
        let Ok(characters) = attempt else {
            return None;
        };
        Some(characters)
    }

    /// Whether `FROM` comes before the `)` closing this call, at the call's own nesting.
    fn has_from_before_close(&self) -> bool {
        let mut depth: usize = 0;
        let mut offset: usize = 0;
        while let Some(token) = self.peek_at(offset) {
            match token.kind {
                TokenKind::Char('(' | '[' | '{') => depth += 1,
                TokenKind::Char(')' | ']' | '}') if depth == 0 => return false,
                TokenKind::Char(')' | ']' | '}') => depth -= 1,
                TokenKind::Word if depth == 0 && token.value.eq_ignore_ascii_case("FROM") => {
                    return true;
                }
                _ => {}
            }
            offset += 1;
        }
        false
    }

    /// `OverClause`'s window: `name`, `(name)`, or a parenthesized specification.
    fn over_clause(&mut self) -> Parsing<Over> {
        if self.is(is_identifier) {
            let name: Identifier = self.name(is_identifier)?;
            return Ok(Over {
                alias: Some(name.clone()),
                ..over(Some(name), Vec::new(), Vec::new(), None)
            });
        }
        self.window_spec()
    }

    /// `WindowFrameDefinition`: `( [base] [PARTITION BY ...] [ORDER BY ...] [frame] )`.
    pub(crate) fn window_spec(&mut self) -> Parsing<Over> {
        self.window_definitions += 1;
        let spec: Parsing<Over> = self.window_spec_body();
        self.window_definitions -= 1;
        spec
    }

    fn window_spec_body(&mut self) -> Parsing<Over> {
        self.expect_char('(')?;
        let clause: bool =
            self.frame_kind_at() || (self.kw_in(0, &["PARTITION", "ORDER"]) && self.kw_at(1, "BY"));
        let base: Option<Identifier> = match self.is(is_identifier) && !clause {
            true => Some(self.name(is_identifier)?),
            false => None,
        };
        let mut partition: Vec<Expression> = Vec::new();
        if self.kw("PARTITION") && self.kw_at(1, "BY") {
            self.at += 2;
            partition = self.comma_list(|parser| parser.expression())?;
        }
        let order: Vec<Ordered> = match self.order_by_at() {
            true => self.order_by_clause()?,
            false => Vec::new(),
        };
        let all: bool = order
            .iter()
            .any(|item| matches!(&item.this, Expression::Var(var) if var.this == "ALL"));
        if all {
            return Err(self.error_with(
                "Cannot ORDER BY ALL in a window expression",
                self.offset(),
                false,
            ));
        }
        let frame: Option<WindowFrame> = self.window_frame()?;
        self.expect_char(')')?;
        Ok(over(base, partition, order, frame))
    }

    fn frame_kind_at(&self) -> bool {
        self.kw_in(0, &["ROWS", "RANGE", "GROUPS"])
    }

    /// `FrameClause`: `ROWS|RANGE|GROUPS` a bound or `BETWEEN` two, and an `EXCLUDE`.
    fn window_frame(&mut self) -> Parsing<Option<WindowFrame>> {
        let kind: WindowFrameKind = match self.word_at(0).as_deref() {
            Some("RANGE") => WindowFrameKind::Range,
            Some("ROWS") => WindowFrameKind::Rows,
            Some("GROUPS") => WindowFrameKind::Groups,
            _ => return Ok(None),
        };
        let kind_text: String = self.advance()?.value.to_ascii_uppercase();
        let frame_start: usize = self.offset();
        let (start, end) = match self.eat_kw("BETWEEN") {
            true => {
                let start: WindowFrameBound = self.frame_bound()?;
                self.expect_kw("AND")?;
                (start, Some(self.frame_bound()?))
            }
            false => (self.frame_bound()?, None),
        };
        if let Some(message) = frame_refusal(&start, end.as_ref()) {
            self.refuse_later(message, frame_start);
        }
        let exclude: Option<WindowFrameExclude> = match self.eat_kw("EXCLUDE") {
            true => Some(match self.word_at(0).as_deref() {
                Some("CURRENT") if self.kw_at(1, "ROW") => {
                    self.at += 2;
                    WindowFrameExclude::CurrentRow
                }
                Some("GROUP") => {
                    self.at += 1;
                    WindowFrameExclude::Group
                }
                Some("TIES") => {
                    self.at += 1;
                    WindowFrameExclude::Ties
                }
                Some("NO") if self.kw_at(1, "OTHERS") => {
                    self.at += 2;
                    WindowFrameExclude::NoOthers
                }
                _ => return Err(self.error()),
            }),
            false => None,
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

    /// `FrameBound`.
    fn frame_bound(&mut self) -> Parsing<WindowFrameBound> {
        if self.kw("UNBOUNDED") && self.kw_in(1, &["PRECEDING", "FOLLOWING"]) {
            let preceding: bool = self.kw_at(1, "PRECEDING");
            self.at += 2;
            return Ok(match preceding {
                true => WindowFrameBound::UnboundedPreceding,
                false => WindowFrameBound::UnboundedFollowing,
            });
        }
        if self.kw("CURRENT") && self.kw_at(1, "ROW") {
            self.at += 2;
            return Ok(WindowFrameBound::CurrentRow);
        }
        let offset: Expression = self.expression()?;
        if self.eat_kw("PRECEDING") {
            return Ok(WindowFrameBound::Preceding(Box::new(offset)));
        }
        self.expect_kw("FOLLOWING")?;
        Ok(WindowFrameBound::Following(Box::new(offset)))
    }

    /// `OrderByClause`'s items, `ORDER BY` included; `OrderByAll` is one `ALL` item, alone.
    pub(crate) fn order_by_clause(&mut self) -> Parsing<Vec<Ordered>> {
        self.expect_kw("ORDER")?;
        self.expect_kw("BY")?;
        if self.eat_kw("ALL") {
            let all: Expression = crate::parse::_helpers::tree::nodes::var("ALL");
            return Ok(vec![self.sort_order(all)]);
        }
        self.comma_list(|parser| parser.sort_item())
    }

    /// `OrderByExpression`: `expression [ASC|DESC] [NULLS FIRST|LAST]`.
    pub(crate) fn sort_item(&mut self) -> Parsing<Ordered> {
        let this: Expression = self.expression()?;
        Ok(self.sort_order(this))
    }

    /// `DescOrAsc? NullsFirstOrLast?` after `this`.
    fn sort_order(&mut self, this: Expression) -> Ordered {
        let (mut desc, mut explicit_asc) = (false, false);
        if self.eat_kw("ASC") || self.eat_kw("ASCENDING") {
            explicit_asc = true;
        } else if self.eat_kw("DESC") || self.eat_kw("DESCENDING") {
            desc = true;
        }
        let mut nulls_first: Option<bool> = None;
        if self.kw("NULLS") && self.kw_in(1, &["FIRST", "LAST"]) {
            nulls_first = Some(self.kw_at(1, "FIRST"));
            self.at += 2;
        }
        ordered(this, desc, explicit_asc, nulls_first)
    }
}

/// `1`, the start `SUBSTRING (x FOR n)` implies.
fn literal_one() -> Expression {
    crate::parse::_helpers::tree::nodes::literal(
        sqlquack_syntax::expressions::names::models::Literal::Number("1".to_owned()),
    )
}

/// `TransformFrameClause`'s refusals of a frame's bounds, in its order; a lone bound ends at the current row.
fn frame_refusal(start: &WindowFrameBound, end: Option<&WindowFrameBound>) -> Option<&'static str> {
    use WindowFrameBound::{
        CurrentRow, Following, Preceding, UnboundedFollowing, UnboundedPreceding,
    };
    match (start, end) {
        (Following(_), None) => {
            Some("Frame starting from following row cannot end with current row")
        }
        (CurrentRow, Some(Preceding(_))) => {
            Some("Frame starting from current row cannot have preceding rows")
        }
        (Following(_), Some(CurrentRow | Preceding(_))) => {
            Some("Frame starting from following row cannot have preceding rows")
        }
        (UnboundedFollowing, _) => Some("Frame start cannot be UNBOUNDED FOLLOWING"),
        (_, Some(UnboundedPreceding)) => Some("Frame end cannot be UNBOUNDED PRECEDING"),
        _ => None,
    }
}
