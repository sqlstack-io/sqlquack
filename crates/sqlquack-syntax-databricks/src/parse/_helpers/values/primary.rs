//! `primaryExpression` and `constant` (SqlBaseParser.g4): literals, names, calls and postfixes.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::functions::models::JSONExtract;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::operators::models::{
    CollationExpr, MethodCall, Subscript, UnaryOp,
};
use sqlquack_syntax::expressions::types::models::{
    Interval, IntervalSpan, IntervalUnit, IntervalUnitSpec,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    aggregate, binary, boolean, call, case, cast, column_from, dot, function, joined, lambda,
    literal, null, parameter, paren, star, string, subquery, tuple,
};
use crate::parse::constants::{
    CURRENT_LIKE, DATETIME_UNITS, DAY_SEGMENT, IDENTIFIER_CLAUSE, INTERVAL_UNITS, LITERAL_TYPES,
    MIN_LAMBDA_PARAMETERS, TIMESTAMP_ADD_NAMES, TIMESTAMP_DIFF_NAMES, TRUE, TRY_CAST,
    UNIT_TO_UNITS,
};
use crate::scan::models::{NumberKind, Token, TokenKind};

/// A numeric literal; a suffixed one is its digits cast to the type its suffix names.
fn number_literal(token: &Token, kind: NumberKind) -> Expression {
    let suffix: usize = match kind {
        NumberKind::BigInt
        | NumberKind::SmallInt
        | NumberKind::TinyInt
        | NumberKind::Double
        | NumberKind::Float => 1,
        NumberKind::BigDecimal => 2,
        NumberKind::Integer | NumberKind::Decimal | NumberKind::Exponent => 0,
    };
    let digits: String = token.value[..token.value.len() - suffix].to_owned();
    let number: Expression = literal(Literal::Number(digits));
    let to: DataType = match kind {
        NumberKind::BigInt => DataType::BigInt { length: None },
        NumberKind::SmallInt => DataType::SmallInt { length: None },
        NumberKind::TinyInt => DataType::TinyInt { length: None },
        NumberKind::Double => DataType::Double {
            precision: None,
            scale: None,
        },
        NumberKind::Float => DataType::Float {
            precision: None,
            scale: None,
            real_spelling: false,
        },
        NumberKind::BigDecimal => DataType::Decimal {
            precision: None,
            scale: None,
        },
        NumberKind::Integer | NumberKind::Decimal | NumberKind::Exponent => return number,
    };
    Expression::Cast(Box::new(cast(number, to, false)))
}

/// The shared-tree unit an interval unit word names, and whether it was plural.
fn interval_unit(word: &str) -> Option<(IntervalUnit, bool)> {
    INTERVAL_UNITS
        .iter()
        .find(|(name, _)| *name == word)
        .map(|(name, unit)| (*unit, name.ends_with('S')))
}

fn interval(value: Expression, unit: Option<IntervalUnitSpec>) -> Expression {
    Expression::Interval(Box::new(Interval {
        this: Some(value),
        unit,
    }))
}

/// Whether Spark's `stringToDate` reads `text`: `[+-]yyyy[y…][-[m]m[-[d]d[( |T)…]]]`, a real day.
fn is_date_text(text: &str) -> bool {
    let text: &str = text.trim();
    let body: &str = text.strip_prefix(['+', '-']).unwrap_or(text);
    let mut segments: [u32; 3] = [0, 1, 1];
    let mut segment: usize = 0;
    let mut digits: usize = 0;
    let valid = |segment: usize, digits: usize| match segment {
        0 => (4..=7).contains(&digits),
        _ => (1..=2).contains(&digits),
    };
    for character in body.chars() {
        if segment < DAY_SEGMENT && character == '-' {
            if !valid(segment, digits) {
                return false;
            }
            segment += 1;
            segments[segment] = 0;
            digits = 0;
        } else if segment == DAY_SEGMENT && (character == ' ' || character == 'T') {
            break;
        } else if let Some(digit) = character.to_digit(10) {
            segments[segment] = segments[segment].saturating_mul(10).saturating_add(digit);
            digits += 1;
        } else {
            return false;
        }
    }
    if !valid(segment, digits) {
        return false;
    }
    let (month, day) = (segments[1], segments[2]);
    let year: u32 = segments[0];
    let leap: bool =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days: u32 = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

/// Whether a typed literal's text can be its type's value, as far as its shape decides.
fn is_typed_text(word: &str, text: &str) -> bool {
    let trimmed: &str = text.trim();
    match word {
        "DATE" => is_date_text(text),
        "TIME" | "TIMESTAMP" | "TIMESTAMP_LTZ" | "TIMESTAMP_NTZ" => trimmed
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_digit() || matches!(first, '+' | '-' | 'T')),
        "X" => text.chars().all(|character| character.is_ascii_hexdigit()),
        "INTERVAL" => trimmed.chars().any(|character| character.is_ascii_digit()),
        _ => true,
    }
}

impl Parser<'_> {
    /// `INVALID_TYPED_LITERAL`, Spark's refusal of a typed literal whose text is no such value.
    fn invalid_typed_literal(
        &self,
        word: &str,
        text: &str,
        at: usize,
    ) -> crate::parse::errors::ParseError {
        self.error_with(
            &format!(
                "[INVALID_TYPED_LITERAL] The value of the typed literal \"{word}\" is invalid: '{text}'."
            ),
            at,
            true,
        )
    }

    /// `primaryExpression` with its postfixes: subscripts, fields, casts, paths and `COLLATE`.
    pub(crate) fn primary_expression(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let base: Expression = self.primary_base()?;
        let node: Expression = self.postfixes(base)?;
        Ok(self.spanned(first, node))
    }

    /// `stringLit`: one or more adjacent string literals, joined.
    pub(crate) fn string_lit(&mut self) -> Parsing<String> {
        if !self.string_at(0) {
            return Err(self.error());
        }
        let mut text: String = String::new();
        while self.string_at(0) {
            text.push_str(&self.advance()?.value);
        }
        Ok(text)
    }

    /// `stringLit` as a string literal.
    pub(crate) fn string_literal(&mut self) -> Parsing<Expression> {
        Ok(string(&self.string_lit()?))
    }

    fn primary_base(&mut self) -> Parsing<Expression> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        match token.kind {
            TokenKind::Number(kind) => {
                self.at += 1;
                Ok(number_literal(&token, kind))
            }
            TokenKind::String { raw: true } if !self.string_at(1) => {
                self.at += 1;
                Ok(literal(Literal::RawString(token.value)))
            }
            TokenKind::String { .. } => self.string_literal(),
            TokenKind::Char('(') => self.parenthesized_primary(),
            TokenKind::Char('*') => {
                self.at += 1;
                let except: Option<Vec<Identifier>> = self.star_except()?;
                Ok(Expression::Star(Box::new(star(None, except))))
            }
            TokenKind::Char('?') => {
                self.at += 1;
                Ok(parameter(None))
            }
            TokenKind::Char(':') if self.adjacent(0) && self.is_at(1, is_identifier) => {
                self.at += 1;
                let name: Token = self.advance()?;
                Ok(parameter(Some(name.value)))
            }
            TokenKind::Char('@') if self.adjacent(0) && self.word_at(1).is_some() => {
                self.macro_call()
            }
            TokenKind::Word => self.word_expression(&token),
            TokenKind::Quoted => self.name_expression(),
            _ => Err(self.error()),
        }
    }

    /// `@name[.field...].method(args)`, a templating macro call left in the text.
    fn macro_call(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let name: Token = self.advance()?;
        let mut parts: Vec<Identifier> = vec![Identifier::new(format!("@{}", name.value))];
        while self.is_char('.') && self.is_at(1, is_identifier) && !self.char_at(2, '(') {
            self.at += 1;
            parts.push(self.name(is_identifier)?);
        }
        let mut this: Expression = column_from(parts);
        while self.is_char('.') && self.is_at(1, is_identifier) && self.char_at(2, '(') {
            self.at += 1;
            let method: Identifier = self.name(is_identifier)?;
            self.expect_char('(')?;
            let mut args: Vec<Expression> = Vec::new();
            if !self.is_char(')') {
                args = self.comma_list(|parser| parser.function_argument())?;
            }
            self.expect_char(')')?;
            this = Expression::MethodCall(Box::new(MethodCall::new(this, method, args)));
        }
        Ok(this)
    }

    /// `exceptClause` after a star: `EXCEPT (a, b.c)`.
    fn star_except(&mut self) -> Parsing<Option<Vec<Identifier>>> {
        if !(self.kw("EXCEPT") && self.char_at(1, '(') && !self.starts_query_at(1)) {
            return Ok(None);
        }
        self.at += 1;
        let names: Vec<Vec<Identifier>> =
            self.parenthesized(|parser| parser.multipart_identifier())?;
        Ok(Some(names.into_iter().map(joined).collect()))
    }

    /// A primary starting with a word: a keyword form, a call, a lambda or a column reference.
    fn word_expression(&mut self, token: &Token) -> Parsing<Expression> {
        if self.op_at(1, "->") {
            let name: Identifier = identifier(token);
            self.at += 2;
            let body: Expression = self.expression()?;
            return Ok(lambda(vec![name], body));
        }
        let word: String = token.value.to_ascii_uppercase();
        let call_follows: bool = self.char_at(1, '(');
        match word.as_str() {
            "NULL" => {
                self.at += 1;
                return Ok(null());
            }
            "TRUE" | "FALSE" => {
                self.at += 1;
                return Ok(boolean(word == TRUE));
            }
            "CASE" => return self.case_expression(),
            "CAST" | "TRY_CAST" if call_follows => return self.cast_call(word == TRY_CAST),
            "STRUCT" if call_follows => {
                self.at += 1;
                return self.struct_call();
            }
            "INTERVAL" if self.interval_value_at(1) => return self.interval(),
            literal_type if LITERAL_TYPES.contains(&literal_type) && self.string_at(1) => {
                return self.typed_literal(&word);
            }
            current
                if CURRENT_LIKE.contains(&current) && !call_follows && !self.char_at(1, '.') =>
            {
                self.at += 1;
                let mut node = function(vec![identifier(token)], Vec::new());
                node.no_parens = true;
                return Ok(Expression::Function(Box::new(node)));
            }
            _ => {}
        }
        if word == IDENTIFIER_CLAUSE && call_follows && self.char_at(2, ')') {
            self.at += 2;
            return Err(self.error());
        }
        if call_follows && let Some(special) = self.special_call(&word)? {
            return Ok(special);
        }
        if self.string_at(1) {
            return Err(self.error_with(
                &format!("unsupported typed literal: {}", token.value),
                token.start,
                false,
            ));
        }
        self.name_expression()
    }

    /// A dotted name: a column reference, `t.*`, or a (qualified) function call.
    pub(crate) fn name_expression(&mut self) -> Parsing<Expression> {
        let mut parts: Vec<Identifier> = vec![self.name(is_identifier)?];
        while self.is_char('.') {
            if self.char_at(1, '*') {
                self.at += 2;
                let except: Option<Vec<Identifier>> = self.star_except()?;
                return Ok(Expression::Star(Box::new(star(parts.pop(), except))));
            }
            if !self.is_at(1, is_identifier) {
                break;
            }
            self.at += 1;
            parts.push(self.name(is_identifier)?);
        }
        if self.is_char('(') {
            return self.function_call(parts);
        }
        Ok(column_from(parts))
    }

    /// The forms that start with `(`: a lambda's parameters, a subquery, a row or parentheses.
    fn parenthesized_primary(&mut self) -> Parsing<Expression> {
        if let Some(count) = self.lambda_parameters_at(0) {
            self.at += 1;
            let mut parameters: Vec<Identifier> = Vec::with_capacity(count);
            for _ in 0..count {
                parameters.push(self.name(is_identifier)?);
                self.eat_char(',');
            }
            self.expect_char(')')?;
            self.at += 1;
            let body: Expression = self.expression()?;
            return Ok(lambda(parameters, body));
        }
        if self.starts_query_at(1) {
            let attempt = self.attempt(|parser| {
                parser.at += 1;
                let query: Expression = parser.query()?;
                parser.expect_char(')')?;
                Ok(query)
            });
            if let Ok(query) = attempt {
                return Ok(subquery(query, None, false));
            }
        }
        self.at += 1;
        let first: Expression = self.named_expression()?;
        if self.eat_char(',') {
            let mut items: Vec<Expression> = vec![first];
            items.extend(self.comma_list(|parser| parser.named_expression())?);
            self.expect_char(')')?;
            return Ok(tuple(items));
        }
        if matches!(first, Expression::Alias(_)) {
            self.at -= 1;
            return Err(self.error());
        }
        self.expect_char(')')?;
        Ok(paren(first))
    }

    /// The number of parameters of `(a, b, ...) ->` at `offset`, when a lambda starts there.
    fn lambda_parameters_at(&self, offset: usize) -> Option<usize> {
        if !self.char_at(offset, '(') {
            return None;
        }
        let mut at: usize = offset + 1;
        let mut count: usize = 0;
        loop {
            if !self.is_at(at, is_identifier) {
                return None;
            }
            count += 1;
            at += 1;
            if self.char_at(at, ',') {
                at += 1;
                continue;
            }
            break;
        }
        (count >= MIN_LAMBDA_PARAMETERS && self.char_at(at, ')') && self.op_at(at + 1, "->"))
            .then_some(count)
    }

    /// `CASE [value] WHEN ... THEN ... [ELSE ...] END`.
    fn case_expression(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let operand: Option<Expression> = match self.kw("WHEN") {
            true => None,
            false => Some(self.expression()?),
        };
        let mut whens: Vec<(Expression, Expression)> = Vec::new();
        while self.eat_kw("WHEN") {
            let condition: Expression = self.expression()?;
            self.expect_kw("THEN")?;
            whens.push((condition, self.expression()?));
        }
        if whens.is_empty() {
            return Err(self.error());
        }
        let else_: Option<Expression> = match self.eat_kw("ELSE") {
            true => Some(self.expression()?),
            false => None,
        };
        self.expect_kw("END")?;
        Ok(case(operand, whens, else_))
    }

    /// `CAST (expression AS dataType)` or `TRY_CAST (...)`.
    fn cast_call(&mut self, try_cast: bool) -> Parsing<Expression> {
        self.at += 2;
        let this: Expression = self.expression()?;
        self.expect_kw("AS")?;
        let to: DataType = self.data_type()?;
        self.expect_char(')')?;
        let node = Box::new(cast(this, to, false));
        Ok(match try_cast {
            true => Expression::TryCast(node),
            false => Expression::Cast(node),
        })
    }

    /// `STRUCT (namedExpression, ...)`, as the call `struct` with its aliased arguments.
    fn struct_call(&mut self) -> Parsing<Expression> {
        self.expect_char('(')?;
        let args: Vec<Expression> = match self.is_char(')') {
            true => Vec::new(),
            false => self.comma_list(|parser| parser.named_expression())?,
        };
        self.expect_char(')')?;
        Ok(call("struct", args))
    }

    /// Whether an `intervalValue` (a signed number or a string) starts at `offset`.
    fn interval_value_at(&self, offset: usize) -> bool {
        let signed: bool = self.char_at(offset, '+') || self.char_at(offset, '-');
        let at: usize = offset + usize::from(signed);
        self.peek_at(at).is_some_and(|token| {
            matches!(
                token.kind,
                TokenKind::Number(NumberKind::Integer | NumberKind::Decimal)
                    | TokenKind::String { .. }
            )
        })
    }

    /// The tokens an `intervalValue` at `offset` spans.
    fn interval_value_width(&self, offset: usize) -> usize {
        1 + usize::from(self.char_at(offset, '+') || self.char_at(offset, '-'))
    }

    fn interval_value(&mut self) -> Parsing<Expression> {
        let negative: bool = self.is_char('-');
        if negative || self.is_char('+') {
            self.at += 1;
        }
        let token: Token = self.advance()?;
        let value: Expression = match token.kind {
            TokenKind::Number(_) if negative => {
                literal(Literal::Number(format!("-{}", token.value)))
            }
            TokenKind::Number(_) => literal(Literal::Number(token.value)),
            _ if negative => Expression::Neg(Box::new(UnaryOp::new(string(&token.value)))),
            _ => string(&token.value),
        };
        Ok(value)
    }

    /// `INTERVAL` with units, a unit range, or a string alone (a typed literal).
    fn interval(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let width: usize = self.interval_value_width(0);
        let string_only: bool = width == 1 && self.string_at(0);
        let range: bool = self
            .word_at(width)
            .is_some_and(|word| UNIT_TO_UNITS.contains(&word.as_str()))
            && self.kw_at(width + 1, "TO");
        if range {
            let value: Expression = self.interval_value()?;
            let from: IntervalUnit = self.range_unit()?;
            self.expect_kw("TO")?;
            let to: IntervalUnit = self.range_unit()?;
            let span = IntervalUnitSpec::Span(IntervalSpan {
                this: from,
                expression: to,
            });
            return Ok(interval(value, Some(span)));
        }
        let has_unit = |parser: &Self, offset: usize| {
            parser
                .word_at(offset)
                .is_some_and(|word| interval_unit(&word).is_some())
        };
        if !has_unit(self, width) {
            if string_only {
                let at: usize = self.offset();
                let text: String = self
                    .peek()
                    .map(|token| token.value.clone())
                    .unwrap_or_default();
                if !is_typed_text("INTERVAL", &text) {
                    return Err(self.invalid_typed_literal("INTERVAL", &text, at));
                }
                let value: Expression = self.interval_value()?;
                return Ok(Expression::Interval(Box::new(Interval {
                    this: Some(value),
                    unit: None,
                })));
            }
            self.at += width;
            return Err(self.error());
        }
        let mut node: Option<Expression> = None;
        while self.interval_value_at(0) && has_unit(self, self.interval_value_width(0)) {
            let value: Expression = self.interval_value()?;
            let word: String = self.word_at(0).unwrap_or_default();
            let (unit, use_plural) = interval_unit(&word).ok_or_else(|| self.error())?;
            self.at += 1;
            let part: Expression =
                interval(value, Some(IntervalUnitSpec::Simple { unit, use_plural }));
            node = Some(match node {
                Some(left) => binary(Expression::Add, left, part),
                None => part,
            });
        }
        node.ok_or_else(|| self.error())
    }

    /// `unitInUnitToUnit`.
    fn range_unit(&mut self) -> Parsing<IntervalUnit> {
        let unit: Option<IntervalUnit> = self
            .word_at(0)
            .filter(|word| UNIT_TO_UNITS.contains(&word.as_str()))
            .and_then(|word| interval_unit(&word))
            .map(|(unit, _)| unit);
        match unit {
            Some(unit) => {
                self.at += 1;
                Ok(unit)
            }
            None => Err(self.error()),
        }
    }

    /// `literalType singleStringLitWithoutMarker`: `DATE '...'`, `TIMESTAMP '...'`, `X'...'`.
    fn typed_literal(&mut self, word: &str) -> Parsing<Expression> {
        let at: usize = self.offset();
        self.at += 1;
        let value: String = self.advance()?.value;
        if !is_typed_text(word, &value) {
            return Err(self.invalid_typed_literal(word, &value, at));
        }
        Ok(match word {
            "DATE" => literal(Literal::Date(value)),
            "TIME" => literal(Literal::Time(value)),
            "TIMESTAMP" => literal(Literal::Timestamp(value)),
            "X" => literal(Literal::HexString(value)),
            _ => Expression::Cast(Box::new(cast(
                string(&value),
                DataType::Custom {
                    name: word.to_owned(),
                },
                false,
            ))),
        })
    }

    /// The calls whose arguments use keywords; `None` when the call is an ordinary one.
    fn special_call(&mut self, word: &str) -> Parsing<Option<Expression>> {
        let name: String = self
            .peek()
            .map(|token| token.value.clone())
            .unwrap_or_default();
        let lower: String = name.to_ascii_lowercase();
        let attempt = match word {
            "POSITION" => self.attempt(|parser| {
                parser.at += 2;
                let substring: Expression = parser.value_expression(0)?;
                parser.expect_kw("IN")?;
                let text: Expression = parser.value_expression(0)?;
                parser.expect_char(')')?;
                Ok(call("position", vec![substring, text]))
            }),
            "EXTRACT" => self.attempt(|parser| {
                parser.at += 2;
                let field: Token = parser.expect_token(is_identifier)?;
                parser.expect_kw("FROM")?;
                let source: Expression = parser.value_expression(0)?;
                parser.expect_char(')')?;
                Ok(call("extract", vec![string(&field.value), source]))
            }),
            "SUBSTR" | "SUBSTRING" => self.attempt(|parser| {
                parser.at += 2;
                let text: Expression = parser.value_expression(0)?;
                if !(parser.eat_kw("FROM") || parser.eat_char(',')) {
                    return Err(parser.error());
                }
                let mut args: Vec<Expression> = vec![text, parser.value_expression(0)?];
                if parser.eat_kw("FOR") || parser.eat_char(',') {
                    args.push(parser.value_expression(0)?);
                }
                parser.expect_char(')')?;
                Ok(call(&lower, args))
            }),
            "TRIM" => self.attempt(|parser| {
                parser.at += 2;
                let side: &str = match parser.word_at(0).as_deref() {
                    Some("LEADING") => "ltrim",
                    Some("TRAILING") => "rtrim",
                    Some("BOTH") => "trim",
                    _ => "",
                };
                if !side.is_empty() {
                    parser.at += 1;
                }
                let characters: Option<Expression> = match parser.kw("FROM") {
                    true => None,
                    false => Some(parser.value_expression(0)?),
                };
                parser.expect_kw("FROM")?;
                let source: Expression = parser.value_expression(0)?;
                parser.expect_char(')')?;
                let name: &str = match side {
                    "" => "trim",
                    other => other,
                };
                Ok(call(
                    name,
                    std::iter::once(source).chain(characters).collect(),
                ))
            }),
            "OVERLAY" => self.attempt(|parser| {
                parser.at += 2;
                let input: Expression = parser.value_expression(0)?;
                parser.expect_kw("PLACING")?;
                let replacement: Expression = parser.value_expression(0)?;
                parser.expect_kw("FROM")?;
                let mut args: Vec<Expression> =
                    vec![input, replacement, parser.value_expression(0)?];
                if parser.eat_kw("FOR") {
                    args.push(parser.value_expression(0)?);
                }
                parser.expect_char(')')?;
                Ok(call("overlay", args))
            }),
            unit_call
                if TIMESTAMP_ADD_NAMES.contains(&unit_call)
                    || TIMESTAMP_DIFF_NAMES.contains(&unit_call) =>
            {
                let unit_at: usize = self.at + 2;
                let unit_is_string: bool = self.string_at(2);
                let result = self.attempt(|parser| {
                    parser.at += 2;
                    let unit: Expression = match parser.word_at(0) {
                        Some(unit) if DATETIME_UNITS.contains(&unit.as_str()) => {
                            parser.at += 1;
                            string(&unit)
                        }
                        _ => parser.string_literal()?,
                    };
                    parser.expect_char(',')?;
                    let first: Expression = parser.value_expression(0)?;
                    parser.expect_char(',')?;
                    let second: Expression = parser.value_expression(0)?;
                    parser.expect_char(')')?;
                    Ok(call(&lower, vec![unit, first, second]))
                });
                match result {
                    Ok(_) if unit_is_string => {
                        let position: usize =
                            self.tokens.get(unit_at).map_or(0, |token| token.start);
                        return Err(self.error_with("invalid datetime unit", position, false));
                    }
                    result => result,
                }
            }
            "FIRST" | "LAST" | "ANY_VALUE" => self.attempt(|parser| {
                parser.at += 2;
                let value: Expression = parser.expression()?;
                parser.expect_kw("IGNORE")?;
                parser.expect_kw("NULLS")?;
                parser.expect_char(')')?;
                Ok(aggregate(
                    function(vec![Identifier::new(name.clone())], vec![value]),
                    false,
                    None,
                    Some(true),
                ))
            }),
            _ => return Ok(None),
        };
        match attempt {
            Ok(special) => Ok(Some(special)),
            Err(_ordinary_call) => Ok(None),
        }
    }

    /// The postfixes of a primary, left to right.
    fn postfixes(&mut self, mut this: Expression) -> Parsing<Expression> {
        loop {
            if self.eat_char('[') {
                let index: Expression = self.deeper(|parser| parser.value_expression(0))?;
                self.expect_char(']')?;
                this = Expression::Subscript(Box::new(Subscript { this, index }));
            } else if self.is_char('.') && self.is_at(1, is_identifier) {
                self.at += 1;
                let field: Identifier = self.name(is_identifier)?;
                this = dot(this, field);
            } else if self.eat_op("::") {
                let to: DataType = self.data_type()?;
                this = Expression::Cast(Box::new(cast(this, to, true)));
            } else if self.eat_op("?::") {
                let to: DataType = self.data_type()?;
                this = Expression::TryCast(Box::new(cast(this, to, true)));
            } else if self.is_char(':') && (self.is_at(1, is_identifier) || self.char_at(1, '[')) {
                this = self.semi_structured(this)?;
            } else if self.kw("COLLATE") && self.is_at(1, is_identifier) {
                self.at += 1;
                let name: Identifier = joined(self.multipart_identifier()?);
                this = Expression::Collation(Box::new(CollationExpr {
                    this,
                    collation: name.name,
                    quoted: false,
                    double_quoted: false,
                }));
            } else {
                return Ok(this);
            }
        }
    }

    /// `col : path`, the path written as text, as the shared tree's variant extraction holds it.
    fn semi_structured(&mut self, this: Expression) -> Parsing<Expression> {
        self.at += 1;
        let mut path: String = String::new();
        match self.is_char('[') {
            true => path.push_str(&self.path_bracket(false)?),
            false => path.push_str(&self.advance()?.value),
        }
        loop {
            if self.is_char('.') && self.is_at(1, is_identifier) {
                self.at += 1;
                path.push('.');
                path.push_str(&self.advance()?.value);
            } else if self.is_char('[') {
                path.push_str(&self.path_bracket(true)?);
            } else {
                break;
            }
        }
        Ok(Expression::JSONExtract(Box::new(JSONExtract {
            this: Box::new(this),
            expression: Box::new(string(&path)),
            only_json_types: None,
            expressions: Vec::new(),
            variant_extract: Some(Box::new(boolean(true))),
            json_query: None,
            option: None,
            quote: None,
            on_condition: None,
            requires_json: None,
        })))
    }

    /// `['key']`, `[n]` or, after the first part, `[name]` in a path.
    fn path_bracket(&mut self, names: bool) -> Parsing<String> {
        self.expect_char('[')?;
        let text: String = match self.peek().map(|token| token.kind) {
            Some(TokenKind::String { .. }) => {
                let key: Token = self.advance()?;
                format!("['{}']", key.value)
            }
            Some(TokenKind::Number(NumberKind::Integer)) => {
                let index: Token = self.advance()?;
                format!("[{}]", index.value)
            }
            Some(TokenKind::Char('*')) => {
                self.advance()?;
                "[*]".to_owned()
            }
            _ if names && self.is(is_identifier) => {
                let name: Token = self.advance()?;
                format!("[{}]", name.value)
            }
            _ => return Err(self.error()),
        };
        self.expect_char(']')?;
        Ok(text)
    }

    /// `namedExpression`: `expression [AS] alias` or `expression AS (a, b)`.
    pub(crate) fn named_expression(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let this: Expression = self.expression()?;
        let node: Expression = if self.kw("AS") && self.char_at(1, '(') {
            self.at += 1;
            let names: Vec<Identifier> = self.identifier_list()?;
            let mut node =
                sqlquack_syntax::expressions::operators::models::Alias::with_columns(this, names);
            node.alias_explicit_as = true;
            Expression::Alias(Box::new(node))
        } else if self.eat_kw("AS") {
            let name: Identifier = self.ident()?;
            crate::parse::_helpers::tree::nodes::alias(this, name, true)
        } else if self.bare_alias_at(0, false) {
            let name: Identifier = self.ident()?;
            crate::parse::_helpers::tree::nodes::alias(this, name, false)
        } else {
            return Ok(this);
        };
        Ok(self.spanned(first, node))
    }
}
