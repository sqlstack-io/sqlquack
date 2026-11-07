//! `primaryExpression` and `constant` (SqlBaseParser.g4): literals, names, calls and postfixes.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::operators::models::{MethodCall, Subscript};
use sqlquack_syntax::expressions::types::models::{
    Interval, IntervalSpan, IntervalUnit, IntervalUnitSpec,
};

use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, identifier, is_identifier, is_path_word,
};
use crate::parse::_helpers::tree::nodes::{
    aggregate, boolean, call, case, cast, column_from, dot, function, joined, lambda, literal,
    null, parameter, paren, star, string, subquery, tuple,
};
use crate::parse::constants::{
    CURRENT_LIKE, DATETIME_UNITS, INTERVAL_UNITS, LITERAL_TYPES, MIN_LAMBDA_LIST_PARAMETERS,
    RESERVED_CALLS, SAFE_CAST_CALL, SYSTEM_VARIABLE_PREFIX, TIMESTAMP_ADD_NAMES,
    TIMESTAMP_DIFF_NAMES, TRUE_LITERAL,
};
use crate::scan::models::{NumberKind, Token, TokenKind};

/// `this.method(args)`, a chained call; a parenthesized path method keeps its dotted name.
fn method_call(this: Expression, method: Identifier, args: Vec<Expression>) -> Expression {
    Expression::MethodCall(Box::new(MethodCall { this, method, args }))
}

/// A numeric literal: GoogleSQL's numbers carry no type suffix.
fn number_literal(token: &Token, _kind: NumberKind) -> Expression {
    literal(Literal::Number(token.value.clone()))
}

/// A value whose type its syntax fixes: a cast whose `inferred_type` is its own target.
fn typed(value: Expression, to: DataType) -> Expression {
    let mut node = cast(value, to.clone(), false);
    node.inferred_type = Some(to);
    Expression::Cast(Box::new(node))
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

impl Parser<'_> {
    /// `primaryExpression` with its postfixes: subscripts, fields, casts, paths and `COLLATE`.
    pub(crate) fn primary_expression(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let base: Expression = self.primary_base()?;
        let node: Expression = self.postfixes(base)?;
        Ok(self.spanned(first, node))
    }

    /// `string_literal` or `bytes_literal`: its chunks of one kind joined, with whether bytes.
    fn literal_chunks(&mut self) -> Parsing<(String, bool)> {
        let bytes: bool = match self.peek().map(|token| token.kind) {
            Some(TokenKind::String { bytes, .. }) => bytes,
            _ => return Err(self.error()),
        };
        let mut text: String = String::new();
        loop {
            text.push_str(&self.advance()?.value);
            let Some(TokenKind::String { bytes: next, .. }) = self.peek().map(|token| token.kind)
            else {
                return Ok((text, bytes));
            };
            if next != bytes {
                return Err(self.error_with(
                    "Syntax error: string and bytes literals cannot be concatenated.",
                    self.offset(),
                    true,
                ));
            }
            if self.at > 0 && self.tokens[self.at - 1].end == self.offset() {
                return Err(self.error_with(
                    "Syntax error: concatenated string literals must be separated by whitespace \
                     or comments",
                    self.offset(),
                    true,
                ));
            }
        }
    }

    /// `string_literal`: one or more string chunks, joined; bytes are refused.
    pub(crate) fn string_lit(&mut self) -> Parsing<String> {
        let start: usize = self.offset();
        match self.literal_chunks()? {
            (text, false) => Ok(text),
            (_, true) => {
                Err(self.error_with("Syntax error: Unexpected bytes literal", start, true))
            }
        }
    }

    /// A string or bytes literal; one raw string chunk stays raw.
    pub(crate) fn string_literal(&mut self) -> Parsing<Expression> {
        if let Some(token) = self.peek().cloned()
            && token.kind
                == (TokenKind::String {
                    raw: true,
                    bytes: false,
                })
            && !self.string_at(1)
        {
            self.at += 1;
            return Ok(literal(Literal::RawString(token.value)));
        }
        Ok(match self.literal_chunks()? {
            (text, false) => string(&text),
            (text, true) => literal(Literal::ByteString(text)),
        })
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
            TokenKind::String { .. } => self.string_literal(),
            TokenKind::Char('(') => self.parenthesized_primary(),
            TokenKind::Char('*') => {
                self.at += 1;
                let except: Option<Vec<Identifier>> = self.star_except()?;
                let mut node = star(None, except);
                node.replace = self.star_replace()?;
                Ok(Expression::Star(node))
            }
            TokenKind::Char('?') => {
                self.at += 1;
                Ok(parameter(None))
            }
            TokenKind::Char('[') => self.array_literal(),
            TokenKind::Char('@')
                if self.is_at(1, |token| {
                    matches!(token.kind, TokenKind::Word | TokenKind::Quoted)
                }) =>
            {
                self.at += 1;
                let name: Token = self.advance()?;
                Ok(parameter(Some(name.value)))
            }
            TokenKind::Op if token.value == SYSTEM_VARIABLE_PREFIX => {
                self.at += 1;
                let mut parts: Vec<Identifier> = vec![
                    self.name(|token| matches!(token.kind, TokenKind::Word | TokenKind::Quoted))?,
                ];
                while self.is_char('.')
                    && self.is_at(1, |token| {
                        matches!(token.kind, TokenKind::Word | TokenKind::Quoted)
                    })
                {
                    self.at += 1;
                    parts.push(self.advance().map(|token| identifier(&token))?);
                }
                Ok(call("@@", vec![string(&joined(parts).name)]))
            }
            TokenKind::Char(':') if self.adjacent(0) && self.is_at(1, is_identifier) => {
                self.at += 1;
                let name: Token = self.advance()?;
                Ok(parameter(Some(name.value)))
            }
            TokenKind::Word => self.word_expression(&token),
            TokenKind::Quoted => self.name_expression(),
            _ => Err(self.error()),
        }
    }

    /// `{ GRAPH ... }`, a graph query in braces, kept as a gap.
    pub(crate) fn graph_subquery(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_char('{')?;
        let mut depth: usize = 1;
        while depth > 0 {
            match self.advance()?.kind {
                TokenKind::Char('{') => depth += 1,
                TokenKind::Char('}') => depth -= 1,
                _ => {}
            }
        }
        self.gap("graph_subquery", start);
        Ok(null())
    }

    /// `[ expression, ... ]`, possibly empty, as the call `ARRAY` of its elements.
    fn array_literal(&mut self) -> Parsing<Expression> {
        self.expect_char('[')?;
        let items: Vec<Expression> = match self.is_char(']') {
            true => Vec::new(),
            false => self.comma_list(|parser| parser.expression())?,
        };
        self.expect_char(']')?;
        Ok(call("ARRAY", items))
    }

    /// `REPLACE (expression AS name, ...)` after a star.
    fn star_replace(
        &mut self,
    ) -> Parsing<Option<Vec<sqlquack_syntax::expressions::operators::models::Alias>>> {
        if !(self.kw("REPLACE") && self.char_at(1, '(')) {
            return Ok(None);
        }
        self.at += 1;
        let replaced = self.parenthesized(|parser| {
            let value: Expression = parser.expression()?;
            parser.expect_kw("AS")?;
            let name: Identifier = parser.ident()?;
            Ok(sqlquack_syntax::expressions::operators::models::Alias::new(
                value, name,
            ))
        })?;
        Ok(Some(replaced))
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
                return Ok(boolean(word == TRUE_LITERAL));
            }
            "CASE" => return self.case_expression(),
            "CAST" | "SAFE_CAST" if call_follows => return self.cast_call(word == SAFE_CAST_CALL),
            "STRUCT" if call_follows => {
                self.at += 1;
                return self.struct_call();
            }
            "INTERVAL" => return self.interval(),
            "ARRAY" | "EXISTS" | "VALUE" if self.char_at(1, '{') => {
                self.at += 1;
                return self.graph_subquery();
            }
            "WITH" if call_follows => {
                let start: usize = self.offset();
                self.at += 2;
                let mut last: Expression = self.expression()?;
                while self.eat_kw("AS") {
                    self.expression()?;
                    self.expect_char(',')?;
                    last = self.expression()?;
                }
                self.expect_char(')')?;
                self.gap("with_expression", start);
                return Ok(last);
            }
            "RANGE" if self.char_at(1, '<') => {
                let to: DataType = self.data_type()?;
                let value: String = self.string_lit()?;
                return Ok(typed(string(&value), to));
            }
            "ARRAY" if call_follows && self.starts_query_at(1) => {
                self.at += 2;
                let query: Expression = self.query()?;
                self.expect_char(')')?;
                return Ok(call("ARRAY", vec![query]));
            }
            "ARRAY" if self.char_at(1, '[') => {
                self.at += 1;
                return self.array_literal();
            }
            "ARRAY" if self.char_at(1, '<') => {
                let to: DataType = self.data_type()?;
                return Ok(typed(self.array_literal()?, to));
            }
            "STRUCT" if self.char_at(1, '<') => {
                let to: DataType = self.data_type()?;
                return Ok(typed(self.struct_call()?, to));
            }
            "EXISTS" if call_follows => {
                self.at += 2;
                let query: Expression = self.query()?;
                self.expect_char(')')?;
                return Ok(crate::parse::_helpers::tree::nodes::exists(subquery(
                    query, None, false,
                )));
            }
            "SAFE" if self.char_at(1, '.') => {}
            reserved if call_follows && RESERVED_CALLS.contains(&reserved) => {
                if let Some(special) = self.special_call(&word)? {
                    return Ok(special);
                }
                self.at += 1;
                return self.function_call(vec![identifier(token)]);
            }
            literal_type if LITERAL_TYPES.contains(&literal_type) && self.string_at(1) => {
                if self.is_at(1, |token| {
                    matches!(token.kind, TokenKind::String { bytes: true, .. })
                }) {
                    return Err(self.error_with(
                        "Syntax error: Expected end of input but got bytes literal",
                        token.end,
                        true,
                    ));
                }
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
        loop {
            if let Some(part) = self.number_part() {
                parts.push(part);
                continue;
            }
            if !self.is_char('.') {
                break;
            }
            if self.char_at(1, '*') {
                self.at += 2;
                let except: Option<Vec<Identifier>> = self.star_except()?;
                let mut node = star(parts.pop(), except);
                node.replace = self.star_replace()?;
                return Ok(Expression::Star(node));
            }
            if !self.is_at(1, is_path_word) {
                break;
            }
            self.at += 1;
            parts.push(self.name(is_path_word)?);
        }
        if self.is_char('(') {
            return self.function_call(parts);
        }
        Ok(column_from(parts))
    }

    /// A number as a later path part (`subsequent_part`), scanned with its dot: `.1` is name `1`.
    fn number_part(&mut self) -> Option<Identifier> {
        let token: &Token = self.peek()?;
        let digits: &str = token.value.strip_prefix('.')?;
        if !(matches!(token.kind, TokenKind::Number(_))
            && !digits.is_empty()
            && digits.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return None;
        }
        let part: Identifier = Identifier::quoted(digits.to_owned()).with_span(token.span);
        self.at += 1;
        Some(part)
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
        (count >= MIN_LAMBDA_LIST_PARAMETERS && self.char_at(at, ')') && self.op_at(at + 1, "->"))
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
        let mut node = Box::new(cast(this, to, false));
        if self.eat_kw("FORMAT") {
            let mut format: Expression = self.expression()?;
            if self.eat_kws(&["AT", "TIME", "ZONE"]) {
                let zone: Expression = self.expression()?;
                format = Expression::AtTimeZone(Box::new(
                    sqlquack_syntax::expressions::typed_functions::models::AtTimeZone {
                        this: format,
                        zone,
                    },
                ));
            }
            node.format = Some(Box::new(format));
        }
        self.expect_char(')')?;
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
            false => self.comma_list(|parser| {
                let value: Expression = parser.expression()?;
                match parser.eat_kw("AS") {
                    true => {
                        let name: Identifier = parser.ident()?;
                        Ok(crate::parse::_helpers::tree::nodes::alias(
                            value, name, true,
                        ))
                    }
                    false => Ok(value),
                }
            })?,
        };
        self.expect_char(')')?;
        Ok(call("struct", args))
    }

    /// `INTERVAL value unit [TO unit]`: the value is any expression, the unit a date part.
    fn interval(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let value: Expression = self.expression()?;
        let word: String = self.word_at(0).ok_or_else(|| self.error())?;
        self.at += 1;
        if self.eat_kw("TO") {
            let to: String = self.word_at(0).ok_or_else(|| self.error())?;
            self.at += 1;
            return match (interval_unit(&word), interval_unit(&to)) {
                (Some((this, _)), Some((expression, _))) => Ok(interval(
                    value,
                    Some(IntervalUnitSpec::Span(IntervalSpan { this, expression })),
                )),
                _ => Err(self.error()),
            };
        }
        Ok(match interval_unit(&word) {
            Some((unit, use_plural)) => {
                interval(value, Some(IntervalUnitSpec::Simple { unit, use_plural }))
            }
            None => call("INTERVAL", vec![value, string(&word)]),
        })
    }

    /// A typed literal: a literal node for the date and time types, else a self-typed cast.
    fn typed_literal(&mut self, word: &str) -> Parsing<Expression> {
        let type_at: usize = self.at;
        self.at += 1;
        let value: String = self.string_lit()?;
        Ok(match word {
            "DATE" => literal(Literal::Date(value)),
            "TIME" => literal(Literal::Time(value)),
            "DATETIME" => literal(Literal::Datetime(value)),
            "TIMESTAMP" => literal(Literal::Timestamp(value)),
            _ => {
                let end: usize = self.at;
                self.at = type_at;
                let to: DataType = self.data_type()?;
                self.at = end;
                typed(string(&value), to)
            }
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
                let field: Token = parser.expect_token(is_path_word)?;
                parser.expect_kw("FROM")?;
                let source: Expression = parser.value_expression(0)?;
                if parser.kw("AT") && parser.kw_at(1, "TIME") && parser.kw_at(2, "ZONE") {
                    parser.at += 3;
                    parser.value_expression(0)?;
                }
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
            Ok(expression) => Ok(Some(expression)),
            Err((_not_this_form, _reached)) => Ok(None),
        }
    }

    /// `( [args] )` of a chained call.
    fn call_arguments(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_char('(')?;
        let mut args: Vec<Expression> = Vec::new();
        if !self.is_char(')') {
            args = self.comma_list(|parser| parser.expression())?;
        }
        self.expect_char(')')?;
        Ok(args)
    }

    /// The postfixes of a primary, left to right.
    fn postfixes(&mut self, mut this: Expression) -> Parsing<Expression> {
        loop {
            if self.eat_char('[') {
                let index: Expression = self.value_expression(0)?;
                self.expect_char(']')?;
                this = Expression::Subscript(Box::new(Subscript { this, index }));
            } else if self.is_char('.') && self.char_at(1, '(') {
                let start: usize = self.offset();
                self.at += 2;
                let path: Vec<Identifier> = self.multipart_identifier()?;
                self.expect_char(')')?;
                match self.is_char('(') {
                    true => {
                        let args: Vec<Expression> = self.call_arguments()?;
                        this = method_call(this, joined(path), args);
                    }
                    false => self.gap("proto_extension_field", start),
                }
            } else if self.is_char('.') && self.is_at(1, is_path_word) && self.char_at(2, '(') {
                self.at += 1;
                let method: Identifier = self.name(is_path_word)?;
                let args: Vec<Expression> = self.call_arguments()?;
                this = method_call(this, method, args);
            } else if self.is_char('.') && self.char_at(1, '*') {
                let start: usize = self.offset();
                self.at += 2;
                let except: Option<Vec<Identifier>> = self.star_except()?;
                let replace = self.star_replace()?;
                if except.is_some() || replace.is_some() {
                    self.gap("dot_star_modifiers", start);
                }
                this = dot(this, Identifier::new("*"));
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
            } else {
                return Ok(this);
            }
        }
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
