//! `getFunctionLayer` and its special layers, windows and `ORDER BY` elements.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::functions::models::ParameterizedAgg;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, Null};
use sqlquack_syntax::expressions::operators::models::WithFill;
use sqlquack_syntax::expressions::queries::models::{
    Ordered, Over, WindowFrame, WindowFrameBound, WindowFrameKind,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    aggregate, alias, call, cast, exists, function, ordered, over, string, subquery, tuple, var,
    window_function,
};
use crate::parse::_helpers::values::primary::interval_unit;
use crate::parse::constants::TUPLE_FUNCTION;
use crate::scan::models::TokenKind;

/// What the arguments of a call that is no special layer were.
struct Arguments {
    items: Vec<Expression>,
    distinct: bool,
}

/// The special layer a call's name selects; `tuple` alone is matched with its case.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Tuple,
    Cast,
    Extract,
    Substring,
    Position,
    Exists,
    Trim(bool, bool),
    DateAdd,
    DateDiff,
    Function,
}

fn layer(name: &str) -> Layer {
    if name == TUPLE_FUNCTION {
        return Layer::Tuple;
    }
    match name.to_ascii_lowercase().as_str() {
        "cast" => Layer::Cast,
        "extract" => Layer::Extract,
        "substring" => Layer::Substring,
        "position" => Layer::Position,
        "exists" => Layer::Exists,
        "trim" => Layer::Trim(false, false),
        "ltrim" => Layer::Trim(true, false),
        "rtrim" => Layer::Trim(false, true),
        "dateadd" | "date_add" | "timestampadd" | "timestamp_add" | "datesub" | "date_sub"
        | "timestampsub" | "timestamp_sub" => Layer::DateAdd,
        "datediff" | "date_diff" | "timestampdiff" | "timestamp_diff" => Layer::DateDiff,
        _ => Layer::Function,
    }
}

impl Parser<'_> {
    /// A call of `name`, its `(` consumed.
    pub(crate) fn function_layer(&mut self, name: Vec<Identifier>) -> Parsing<Expression> {
        let selected: Layer = match name.as_slice() {
            [single] if !single.quoted => layer(&single.name),
            _ => Layer::Function,
        };
        let label: String = name
            .iter()
            .map(|part| part.name.as_str())
            .collect::<Vec<&str>>()
            .join(".");
        match selected {
            Layer::Tuple => {
                let items: Vec<Expression> = self.layer_elements(TokenKind::ClosingRoundBracket)?;
                match self.is(TokenKind::Arrow) {
                    true => Err(self.error()),
                    false => Ok(tuple(items)),
                }
            }
            Layer::Cast => self.cast_layer(),
            Layer::Extract => self.extract_layer(),
            Layer::Substring => self.substring_layer(),
            Layer::Position => self.position_layer(),
            Layer::Exists => {
                let query: Expression = self.select_with_union()?;
                self.expect_kind(TokenKind::ClosingRoundBracket)?;
                Ok(exists(subquery(query, None, false)))
            }
            Layer::Trim(left, right) => self.trim_layer(&label, left, right),
            Layer::DateAdd | Layer::DateDiff => self.date_layer(&label, selected == Layer::DateAdd),
            Layer::Function => self.general_function(name),
        }
    }

    /// `CastLayer`: `CAST(x AS T)`, `CAST(x AS alias AS T)`, `CAST(x AS alias, T)`, `CAST(x, T)`.
    fn cast_layer(&mut self) -> Parsing<Expression> {
        let this: Expression = self.expression()?;
        if self.eat_kw("AS") {
            let named = self.attempt(|parser| {
                let name: Identifier = parser.ident()?;
                parser.expect_kw("AS")?;
                let to = parser.data_type()?;
                parser.expect_kind(TokenKind::ClosingRoundBracket)?;
                Ok((name, to))
            });
            if let Ok((name, to)) = named {
                return Ok(Expression::Cast(Box::new(cast(
                    alias(this, name, true),
                    to,
                    false,
                ))));
            }
            let aliased = self.attempt(|parser| {
                let name: Identifier = parser.ident()?;
                parser.expect_kind(TokenKind::Comma)?;
                Ok(name)
            });
            if let Ok(name) = aliased {
                return self.cast_by_name(alias(this, name, true));
            }
            let to = self.data_type()?;
            self.expect_kind(TokenKind::ClosingRoundBracket)?;
            return Ok(Expression::Cast(Box::new(cast(this, to, false))));
        }
        self.expect_kind(TokenKind::Comma)?;
        self.cast_by_name(this)
    }

    /// `CAST(x, T)` after its comma: a cast to the type a string names, else a `CAST` call.
    fn cast_by_name(&mut self, this: Expression) -> Parsing<Expression> {
        let to: Expression = self.element()?;
        self.expect_kind(TokenKind::ClosingRoundBracket)?;
        Ok(match to {
            Expression::Literal(literal) => match *literal {
                Literal::String(text) => Expression::Cast(Box::new(cast(
                    this,
                    sqlquack_syntax::DataType::Custom { name: text },
                    false,
                ))),
                other => call("CAST", vec![this, Expression::Literal(Box::new(other))]),
            },
            other => call("CAST", vec![this, other]),
        })
    }

    /// `ExtractLayer`: `EXTRACT(unit FROM x)`, else `extract(args)`.
    fn extract_layer(&mut self) -> Parsing<Expression> {
        let unit: Option<String> = self
            .peek()
            .filter(|token| {
                token.kind == TokenKind::BareWord && interval_unit(&token.value).is_some()
            })
            .map(|token| token.value.clone());
        if let Some(unit) = unit.filter(|_| self.kw_at(1, "FROM")) {
            self.at += 2;
            let this: Expression = self.element()?;
            self.expect_kind(TokenKind::ClosingRoundBracket)?;
            return Ok(call("EXTRACT", vec![var(&unit), this]));
        }
        let items: Vec<Expression> = self.layer_elements(TokenKind::ClosingRoundBracket)?;
        Ok(call("extract", items))
    }

    /// `SubstringLayer`: `SUBSTRING(x FROM|, a [FOR|, b])`.
    fn substring_layer(&mut self) -> Parsing<Expression> {
        let mut items: Vec<Expression> = vec![self.element()?];
        if !(self.eat(TokenKind::Comma) || self.eat_kw("FROM")) {
            return Err(self.error());
        }
        items.push(self.element()?);
        if self.eat(TokenKind::Comma) || self.eat_kw("FOR") {
            items.push(self.element()?);
        }
        self.expect_kind(TokenKind::ClosingRoundBracket)?;
        Ok(call("substring", items))
    }

    /// `PositionLayer`: `POSITION(needle IN haystack)` or `POSITION(haystack, needle[, start])`.
    fn position_layer(&mut self) -> Parsing<Expression> {
        let first: Expression = match self.element()? {
            Expression::In(mut node)
                if !node.not
                    && !node.global
                    && node.query.is_none()
                    && node.expressions.len() == 1 =>
            {
                self.expect_kind(TokenKind::ClosingRoundBracket)?;
                let needle: Expression = std::mem::replace(&mut node.this, Expression::Null(Null));
                let mut arguments: Vec<Expression> = std::mem::take(&mut node.expressions);
                arguments.push(needle);
                return Ok(call("position", arguments));
            }
            other => other,
        };
        let mut items: Vec<Expression> = vec![first];
        if self.eat_kw("IN") {
            items.insert(0, self.element()?);
        } else {
            self.expect_kind(TokenKind::Comma)?;
            items.push(self.element()?);
            while self.eat(TokenKind::Comma) {
                items.push(self.element()?);
            }
        }
        self.expect_kind(TokenKind::ClosingRoundBracket)?;
        Ok(call("position", items))
    }

    /// `TrimLayer`: `TRIM([BOTH|LEADING|TRAILING] chars FROM x)` or `TRIM(args)`.
    fn trim_layer(&mut self, name: &str, left: bool, right: bool) -> Parsing<Expression> {
        let side: Option<String> = match left || right {
            true => None,
            false => self
                .word_at(0)
                .filter(|word| ["BOTH", "LEADING", "TRAILING"].contains(&word.as_str())),
        };
        if let Some(side) = side {
            self.at += 1;
            let characters: Expression = self.element()?;
            self.expect_kw("FROM")?;
            let mut items: Vec<Expression> = self.comma_list(|parser| parser.element())?;
            self.expect_kind(TokenKind::ClosingRoundBracket)?;
            items.push(characters);
            return Ok(call(&format!("TRIM_{side}"), items));
        }
        let items: Vec<Expression> = self.comma_list(|parser| parser.element())?;
        self.expect_kind(TokenKind::ClosingRoundBracket)?;
        Ok(call(name, items))
    }

    /// `DateAddLayer` and `DateDiffLayer`: an interval kind and a comma may lead the arguments.
    fn date_layer(&mut self, name: &str, add: bool) -> Parsing<Expression> {
        let unit: Option<String> = self
            .peek()
            .filter(|token| {
                token.kind == TokenKind::BareWord && interval_unit(&token.value).is_some()
            })
            .map(|token| token.value.clone());
        let mut items: Vec<Expression> = Vec::new();
        if let Some(unit) = unit {
            self.at += 1;
            self.expect_kind(TokenKind::Comma)?;
            items.push(var(&unit));
        }
        let unit_given: bool = !items.is_empty();
        items.extend(self.layer_elements(TokenKind::ClosingRoundBracket)?);
        let (least, most): (usize, usize) = match (unit_given, add) {
            (true, true) => (3, usize::MAX),
            (true, false) => (3, 4),
            (false, _) => (0, usize::MAX),
        };
        if items.len() < least || items.len() > most {
            return Err(self.error_with("Syntax error: wrong number of arguments", true));
        }
        Ok(call(name, items))
    }

    /// `ALL` or `DISTINCT` leading arguments; either alone before `,` or `)` is a name.
    fn quantifier(&mut self) -> Parsing<bool> {
        let start: usize = self.at;
        let mut all: bool = self.eat_kw("ALL");
        let distinct: bool = self.eat_kw("DISTINCT");
        if !all {
            all = self.eat_kw("ALL");
        }
        if all && distinct {
            return Err(self.error());
        }
        if (all || distinct)
            && (self.is(TokenKind::Comma) || self.is(TokenKind::ClosingRoundBracket))
        {
            self.at = start;
            return Ok(false);
        }
        Ok(distinct)
    }

    /// One argument list after its `(`, up to and with its `)`.
    fn arguments(&mut self) -> Parsing<Arguments> {
        let distinct: bool = self.quantifier()?;
        let items: Vec<Expression> = self.layer_elements(TokenKind::ClosingRoundBracket)?;
        Ok(Arguments { items, distinct })
    }

    /// `FunctionLayer`: `f(args)`, `f(params)(args)`, then `FILTER`, null treatment and `OVER`.
    fn general_function(&mut self, name: Vec<Identifier>) -> Parsing<Expression> {
        let start: usize = self.offset();
        let toplevel: Arguments = self.arguments()?;
        let (parameters, arguments) = match self.eat(TokenKind::OpeningRoundBracket) {
            true if toplevel.distinct => return Err(self.error()),
            true => (Some(toplevel.items), self.arguments()?),
            false => (None, toplevel),
        };
        let filter: Option<Expression> = match self.eat_kw("FILTER") {
            true => {
                self.expect_kind(TokenKind::OpeningRoundBracket)?;
                self.expect_kw("WHERE")?;
                let condition: Expression = self.expression()?;
                self.expect_kind(TokenKind::ClosingRoundBracket)?;
                Some(condition)
            }
            false => None,
        };
        let mut ignore_nulls: Option<bool> = None;
        if self.kw_in(0, &["RESPECT", "IGNORE"]) && self.kw_at(1, "NULLS") {
            ignore_nulls = Some(self.kw("IGNORE"));
            self.at += 2;
        }
        let window: Option<Over> = match self.eat_kw("OVER") {
            true => Some(self.window_reference()?),
            false => None,
        };
        let arguments_distinct: bool = arguments.distinct;
        let filtered: bool = filter.is_some();
        let mut node: Expression = match parameters {
            Some(params) => {
                let label: Identifier = crate::parse::_helpers::tree::nodes::joined(name);
                Expression::ParameterizedAgg(Box::new(ParameterizedAgg {
                    this: Box::new(Expression::Identifier(label)),
                    expressions: arguments.items,
                    params,
                }))
            }
            None => {
                let single_not: bool = matches!(name.as_slice(), [single] if single.name.eq_ignore_ascii_case("not") && !single.quoted);
                if single_not && arguments.items.len() == 1 && !arguments.distinct {
                    let mut items: Vec<Expression> = arguments.items;
                    return Ok(Expression::Not(Box::new(
                        sqlquack_syntax::expressions::operators::models::UnaryOp::new(
                            items.remove(0),
                        ),
                    )));
                }
                let call = function(name, arguments.items);
                match arguments.distinct || filtered || ignore_nulls.is_some() {
                    true => aggregate(call, arguments.distinct, filter, ignore_nulls),
                    false => Expression::Function(Box::new(call)),
                }
            }
        };
        if matches!(node, Expression::ParameterizedAgg(_))
            && (arguments_distinct || filtered || ignore_nulls.is_some())
        {
            self.gap("parametric_aggregate_modifier", start);
        }
        if let Some(window) = window {
            node = window_function(node, window);
        }
        Ok(node)
    }

    /// `ParserWindowReference`: a window name, or a window definition in brackets.
    pub(crate) fn window_reference(&mut self) -> Parsing<Over> {
        if self.is_at(0, is_identifier) {
            let name: Identifier = self.ident()?;
            return Ok(Over {
                alias: Some(name.clone()),
                ..over(Some(name), Vec::new(), Vec::new(), None)
            });
        }
        self.window_definition()
    }

    /// `ParserWindowDefinition`: `( [parent] [PARTITION BY ...] [ORDER BY ...] [frame] )`.
    pub(crate) fn window_definition(&mut self) -> Parsing<Over> {
        self.expect_kind(TokenKind::OpeningRoundBracket)?;
        let plain = self.attempt(|parser| {
            let parts = parser.window_parts(None)?;
            parser.expect_kind(TokenKind::ClosingRoundBracket)?;
            Ok(parts)
        });
        if let Ok(parts) = plain {
            return Ok(parts);
        }
        let parent: Identifier = self.ident()?;
        let parts: Over = self.window_parts(Some(parent))?;
        self.expect_kind(TokenKind::ClosingRoundBracket)?;
        Ok(parts)
    }

    fn window_parts(&mut self, parent: Option<Identifier>) -> Parsing<Over> {
        let partition: Vec<Expression> = match self.eat_kws(&["PARTITION", "BY"]) {
            true => self.comma_list(|parser| parser.expression())?,
            false => Vec::new(),
        };
        let order: Vec<Ordered> = match self.eat_kws(&["ORDER", "BY"]) {
            true => self.comma_list(|parser| parser.order_element())?,
            false => Vec::new(),
        };
        let frame: Option<WindowFrame> = self.window_frame()?;
        Ok(over(parent, partition, order, frame))
    }

    /// `tryParseFrameDefinition`: `ROWS|GROUPS|RANGE [BETWEEN] bound [AND bound]`.
    fn window_frame(&mut self) -> Parsing<Option<WindowFrame>> {
        let kind: WindowFrameKind = match self.word_at(0).as_deref() {
            Some("ROWS") => WindowFrameKind::Rows,
            Some("GROUPS") => WindowFrameKind::Groups,
            Some("RANGE") => WindowFrameKind::Range,
            _ => return Ok(None),
        };
        let kind_text: String = self.advance()?.value.to_ascii_uppercase();
        let between: bool = self.eat_kw("BETWEEN");
        let start: WindowFrameBound = self.frame_bound(true)?;
        let end: Option<WindowFrameBound> = match between {
            true => {
                self.expect_kw("AND")?;
                Some(self.frame_bound(false)?)
            }
            false => None,
        };
        Ok(Some(WindowFrame {
            kind,
            start,
            end,
            exclude: None,
            kind_text: Some(kind_text),
            start_side_text: None,
            end_side_text: None,
        }))
    }

    /// A frame bound; `UNBOUNDED FOLLOWING` cannot start a frame or `UNBOUNDED PRECEDING` end it.
    fn frame_bound(&mut self, start: bool) -> Parsing<WindowFrameBound> {
        if self.eat_kws(&["CURRENT", "ROW"]) {
            return Ok(WindowFrameBound::CurrentRow);
        }
        let offset: Option<Expression> = match self.eat_kw("UNBOUNDED") {
            true => None,
            false => Some(self.expression()?),
        };
        let preceding: bool = match self.word_at(0).as_deref() {
            Some("PRECEDING") => true,
            Some("FOLLOWING") => false,
            _ => return Err(self.error()),
        };
        if offset.is_none() && preceding != start {
            return Err(self.error_with(
                "Frame start cannot be UNBOUNDED FOLLOWING, nor its end UNBOUNDED PRECEDING",
                false,
            ));
        }
        self.at += 1;
        Ok(match (offset, preceding) {
            (None, true) => WindowFrameBound::UnboundedPreceding,
            (None, false) => WindowFrameBound::UnboundedFollowing,
            (Some(offset), true) => WindowFrameBound::Preceding(Box::new(offset)),
            (Some(offset), false) => WindowFrameBound::Following(Box::new(offset)),
        })
    }

    /// `ParserOrderByElement`: `expr [ASC|DESC] [NULLS FIRST|LAST] [COLLATE 'c'] [WITH FILL ...]`.
    pub(crate) fn order_element(&mut self) -> Parsing<Ordered> {
        let this: Expression = self.element()?;
        let (mut desc, mut explicit_asc) = (false, false);
        match self.word_at(0).as_deref() {
            Some("ASC" | "ASCENDING") => {
                self.at += 1;
                explicit_asc = true;
            }
            Some("DESC" | "DESCENDING") => {
                self.at += 1;
                desc = true;
            }
            _ => {}
        }
        let mut nulls_first: Option<bool> = None;
        if self.kw("NULLS") && self.kw_in(1, &["FIRST", "LAST"]) {
            nulls_first = Some(self.kw_at(1, "FIRST"));
            self.at += 2;
        }
        if self.kw("COLLATE") {
            let start: usize = self.offset();
            self.at += 1;
            self.string_literal()?;
            self.gap("order_collate", start);
        }
        let mut node: Ordered = ordered(this, desc, explicit_asc, nulls_first);
        if self.eat_kws(&["WITH", "FILL"]) {
            node.with_fill = Some(Box::new(self.with_fill()?));
        }
        Ok(node)
    }

    /// `WITH FILL [FROM e] [TO e] [STEP e] [STALENESS e]`, its words consumed.
    fn with_fill(&mut self) -> Parsing<WithFill> {
        let mut fill: WithFill = WithFill {
            from_: None,
            to: None,
            step: None,
            staleness: None,
            interpolate: None,
        };
        if self.eat_kw("FROM") {
            fill.from_ = Some(Box::new(self.expression()?));
        }
        if self.eat_kw("TO") {
            fill.to = Some(Box::new(self.expression()?));
        }
        if self.eat_kw("STEP") {
            fill.step = Some(Box::new(self.expression()?));
        }
        if self.eat_kw("STALENESS") {
            fill.staleness = Some(Box::new(self.expression()?));
        }
        Ok(fill)
    }

    /// A string literal as an expression.
    pub(crate) fn string_expression(&mut self) -> Parsing<Expression> {
        Ok(string(&self.string_literal()?))
    }
}
