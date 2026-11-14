//! `BaseExpression` (expression.gram): `SingleExpression` and its indirections.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::Comprehension;
use sqlquack_syntax::expressions::functions::models::Columns;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::operators::models::{
    ArraySlice, MethodCall, Parameter, ParameterStyle, Subscript,
};
use sqlquack_syntax::expressions::typed_functions::models::{Map, Slice};
use sqlquack_syntax::expressions::types::models::{Array, Interval, IntervalUnitSpec, Struct};
use sqlquack_syntax::tokens::models::SourceSpan;

use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, identifier, is_col_label, is_function_name, is_identifier,
};
use crate::parse::_helpers::tree::nodes::{
    boolean, call, case, cast, column_from, dot, literal, null, paren, string, tuple, var,
};
use crate::parse::constants::{DOUBLE_TYPE, INTERVAL_UNITS, TRUE_WORD, TRY_CAST};
use crate::scan::models::{StringPrefix, Token, TokenKind};

/// The transformer's refusal of a star with more than a table in front of it.
const STAR_QUALIFIERS: &str = "Did not expect more than one column in front of a star expression";

/// The transformer's refusal of `[]` after a value.
const EMPTY_SUBSCRIPT: &str = "Empty subscript '[]' is not allowed";

/// The most words a typed literal's type has after its modifiers, as in `WITH TIME ZONE`.
const TYPE_SUFFIX_WORDS: usize = 3;

/// The `unit TO unit` ranges `IntervalToInterval` reads, each refused by the transformer.
const INTERVAL_RANGES: [(&str, &str); 7] = [
    ("YEAR", "MONTH"),
    ("DAY", "HOUR"),
    ("DAY", "MINUTE"),
    ("DAY", "SECOND"),
    ("HOUR", "MINUTE"),
    ("HOUR", "SECOND"),
    ("MINUTE", "SECOND"),
];

/// A string token's literal.
fn string_token(prefix: StringPrefix, value: String) -> Expression {
    literal(match prefix {
        StringPrefix::Plain => Literal::String(value),
        StringPrefix::Escape => Literal::EscapeString(value),
        StringPrefix::National => Literal::NationalString(value),
        StringPrefix::Hex => Literal::HexString(value),
        StringPrefix::Bit => Literal::BitString(value),
    })
}

fn parameter(name: Option<String>, index: Option<u32>, style: ParameterStyle) -> Expression {
    Expression::Parameter(Box::new(Parameter {
        name,
        index,
        style,
        quoted: false,
        string_quoted: false,
        expression: None,
        span: SourceSpan::default(),
    }))
}

impl Parser<'_> {
    /// `BaseExpression`: a `SingleExpression` and its indirections.
    pub(crate) fn primary_expression(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let base: Expression = self.single_expression()?;
        let node: Expression = self.indirections(base)?;
        Ok(self.spanned(first, node))
    }

    /// `StringLiteral`: a string, and the plain strings continuing it on later lines.
    pub(crate) fn string_lit(&mut self) -> Parsing<String> {
        if !self.string_at(0) {
            return Err(self.error());
        }
        let first: Token = self.advance()?;
        let mut text: String = first.value;
        if matches!(first.kind, TokenKind::String(_)) {
            while self.continues_string() {
                text.push_str(&self.advance()?.value);
            }
        }
        Ok(text)
    }

    pub(crate) fn single_expression(&mut self) -> Parsing<Expression> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        match token.kind {
            TokenKind::Char('(') => self.parenthesized_primary(),
            TokenKind::String(prefix) => {
                let value: String = self.string_lit()?;
                self.check_string(prefix, &value, token.start)?;
                Ok(string_token(prefix, value))
            }
            TokenKind::DollarString => {
                self.at += 1;
                Ok(literal(Literal::DollarString(token.value)))
            }
            TokenKind::Number if token.value.matches('.').count() > 1 => Err(self.error()),
            TokenKind::Number => {
                self.at += 1;
                Ok(literal(Literal::Number(token.value)))
            }
            TokenKind::Char('?') => {
                self.at += 1;
                match self
                    .peek()
                    .filter(|next| next.kind == TokenKind::Number)
                    .cloned()
                {
                    Some(number) => {
                        self.at += 1;
                        let Ok(index) = number.value.parse::<u32>() else {
                            return Ok(parameter(None, None, ParameterStyle::Question));
                        };
                        Ok(parameter(None, Some(index), ParameterStyle::Question))
                    }
                    None => Ok(parameter(None, None, ParameterStyle::Question)),
                }
            }
            TokenKind::Char('$') => {
                self.at += 1;
                let next: Token = self.advance()?;
                match next.kind {
                    TokenKind::Number => {
                        let Ok(index) = next.value.parse::<u32>() else {
                            return Ok(parameter(None, None, ParameterStyle::Dollar));
                        };
                        Ok(parameter(None, Some(index), ParameterStyle::Dollar))
                    }
                    _ if is_col_label(&next) => {
                        Ok(parameter(Some(next.value), None, ParameterStyle::Dollar))
                    }
                    _ => {
                        self.at -= 1;
                        Err(self.error())
                    }
                }
            }
            TokenKind::Char('#') => {
                self.at += 1;
                let number: Token = self.advance()?;
                if number.kind != TokenKind::Number {
                    self.at -= 1;
                    return Err(self.error());
                }
                Ok(var(&format!("#{}", number.value)))
            }
            TokenKind::Char('[') => self.list_expression(),
            TokenKind::Char('{') => self.struct_expression(),
            TokenKind::Char('*') => {
                self.at += 1;
                if self.kw("COLUMNS") && self.char_at(1, '(') {
                    self.at += 1;
                    return self.columns_expression(true);
                }
                self.star_modifiers(None)
            }
            TokenKind::Word | TokenKind::Quoted => self.word_expression(&token),
            _ => Err(self.error()),
        }
    }

    /// The checks the transformer makes of a blob or bit-string literal's digits.
    fn check_string(&self, prefix: StringPrefix, value: &str, position: usize) -> Parsing<()> {
        let message: Option<String> = match prefix {
            StringPrefix::Hex => match value.chars().find(|digit| !digit.is_ascii_hexdigit()) {
                Some(digit) => Some(format!(
                    "Hex string literal contains a non-hexadecimal character '{digit}'"
                )),
                None if value.len() % 2 == 1 => {
                    Some("Hex string literal must have an even number of hex digits".to_owned())
                }
                None => None,
            },
            StringPrefix::Bit => {
                value
                    .chars()
                    .find(|digit| !matches!(digit, '0' | '1'))
                    .map(|digit| {
                        format!("Bit string literal contains a non-binary character '{digit}'")
                    })
            }
            _ => None,
        };
        match message {
            Some(message) => Err(self.error_with(&message, position, false)),
            None => Ok(()),
        }
    }

    /// The forms that start with a word: keywords' own forms, typed literals, calls and columns.
    fn word_expression(&mut self, token: &Token) -> Parsing<Expression> {
        let word: String = match token.kind {
            TokenKind::Word => token.value.to_ascii_uppercase(),
            _ => String::new(),
        };
        let call_follows: bool = self.char_at(1, '(');
        match word.as_str() {
            "NULL" => {
                self.at += 1;
                return Ok(null());
            }
            "TRUE" | "FALSE" => {
                self.at += 1;
                return Ok(boolean(word == TRUE_WORD));
            }
            "DEFAULT" => {
                self.at += 1;
                return Ok(var("DEFAULT"));
            }
            "EXISTS" if call_follows && self.starts_query_at(1) => {
                self.at += 1;
                let query: Expression = self.parenthesized_query()?;
                return Ok(crate::parse::_helpers::tree::nodes::exists(query));
            }
            "CASE" => return self.case_expression(),
            "CAST" | "TRY_CAST" if call_follows => return self.cast_call(word == TRY_CAST),
            "INTERVAL"
                if self.string_at(1)
                    || self.is_at(1, |next| next.kind == TokenKind::Number)
                    || call_follows =>
            {
                return self.interval_literal();
            }
            "MAP" if self.char_at(1, '{') => return self.map_expression(),
            "ARRAY" if self.char_at(1, '[') => {
                self.at += 1;
                return self.list_expression();
            }
            "ARRAY" if call_follows && self.starts_query_at(1) => {
                self.at += 1;
                let query: Expression = self.parenthesized_query()?;
                return Ok(call("array", vec![query]));
            }
            "LAMBDA" => return self.lambda_keyword(),
            "COLUMNS" if call_follows => {
                self.at += 1;
                return self.columns_expression(false);
            }
            _ => {}
        }
        if call_follows && let Some(special) = self.special_function(&word)? {
            return Ok(special);
        }
        if let Some(typed) = self.typed_literal(&word)? {
            return Ok(typed);
        }
        self.name_expression()
    }

    /// `TypeLiteral`: a type and a string, such as `DATE '2024-01-01'`, when one is here.
    fn typed_literal(&mut self, word: &str) -> Parsing<Option<Expression>> {
        let likely: bool = self.string_at(1)
            || (matches!(word, "TIMESTAMP" | "TIME") && self.kw_in(1, &["WITH", "WITHOUT"]))
            || (word == DOUBLE_TYPE && self.kw_at(1, "PRECISION"))
            || ((self.char_at(1, '(') || self.char_at(1, '[') || self.char_at(1, '.'))
                && self.string_after_modifiers());
        if !likely {
            return Ok(None);
        }
        let attempt = self.attempt(|parser| {
            let to: DataType = parser.data_type_unchecked()?;
            let value: String = parser.string_lit()?;
            Ok((to, value))
        });
        let Ok((to, value)) = attempt else {
            return Ok(None);
        };
        self.check_modifiers()?;
        Ok(Some(match to {
            DataType::Date => literal(Literal::Date(value)),
            DataType::Time {
                timezone: false, ..
            } => literal(Literal::Time(value)),
            DataType::Timestamp {
                timezone: false,
                precision: None,
            } => literal(Literal::Timestamp(value)),
            other => Expression::Cast(Box::new(cast(string(&value), other, false))),
        }))
    }

    /// Whether a string follows the type name's qualification, bracketed modifiers and few words, as a typed literal needs.
    fn string_after_modifiers(&self) -> bool {
        let mut offset: usize = 1;
        let mut words: usize = 0;
        loop {
            let Some(token) = self.peek_at(offset) else {
                return false;
            };
            match token.kind {
                TokenKind::String(_) | TokenKind::DollarString => return true,
                TokenKind::Char('(' | '[') => match self.after_group(offset) {
                    Some(next) => offset = next,
                    None => return false,
                },
                TokenKind::Char('.') => offset += 1,
                TokenKind::Word if words < TYPE_SUFFIX_WORDS => {
                    words += 1;
                    offset += 1;
                }
                _ => return false,
            }
        }
    }

    /// The offset after the bracket group opening `offset` tokens ahead, if it closes.
    fn after_group(&self, mut offset: usize) -> Option<usize> {
        let mut depth: usize = 0;
        loop {
            match self.peek_at(offset)?.kind {
                TokenKind::Char('(' | '[') => depth += 1,
                TokenKind::Char(')' | ']') => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(offset + 1);
                    }
                }
                _ => {}
            }
            offset += 1;
        }
    }

    /// `ColumnReference`, `FunctionExpression` or a qualified `StarExpression`.
    pub(crate) fn name_expression(&mut self) -> Parsing<Expression> {
        let first: Token = match self.peek() {
            Some(token) if is_col_label(token) => token.clone(),
            _ => return Err(self.error()),
        };
        let qualified: bool = self.char_at(1, '.');
        let call: bool = self.char_at(1, '(');
        let allowed: bool = match (qualified, call) {
            (true, _) => is_identifier(&first),
            (false, true) => is_function_name(&first),
            (false, false) => is_identifier(&first),
        };
        if !allowed {
            return Err(self.error());
        }
        self.at += 1;
        let mut parts: Vec<Identifier> = vec![identifier(&first)];
        while self.is_char('.') {
            if self.char_at(1, '*') {
                if parts.len() > 1 {
                    let start: usize = self.offset();
                    self.refuse_later(STAR_QUALIFIERS, start);
                }
                self.at += 2;
                let table: Option<Identifier> = parts.pop();
                return self.star_modifiers(table);
            }
            if !self.is_at(1, is_col_label) {
                break;
            }
            self.at += 1;
            parts.push(identifier(&self.advance()?));
        }
        if self.is_char('(') {
            return self.function_call(parts);
        }
        Ok(column_from(parts))
    }

    /// `ParensExpression`, `SubqueryExpression` and `ParenthesisExpression`.
    fn parenthesized_primary(&mut self) -> Parsing<Expression> {
        if self.starts_query_at(1) {
            let attempt = self.attempt(|parser| parser.parenthesized_query());
            if let Ok(query) = attempt {
                return Ok(query);
            }
        }
        self.at += 1;
        if self.eat_char(')') {
            return Ok(tuple(Vec::new()));
        }
        let first: Expression = self.expression()?;
        if self.eat_char(')') {
            return Ok(paren(first));
        }
        let mut items: Vec<Expression> = vec![first];
        while self.eat_char(',') {
            if self.is_char(')') {
                break;
            }
            items.push(self.expression()?);
        }
        self.expect_char(')')?;
        Ok(tuple(items))
    }

    /// `[ ... ]`: a list, or a list comprehension `[expr FOR x IN list IF cond]`.
    fn list_expression(&mut self) -> Parsing<Expression> {
        self.expect_char('[')?;
        if self.eat_char(']') {
            return Ok(Expression::Array(Box::new(Array {
                expressions: Vec::new(),
                inferred_type: None,
            })));
        }
        let first: Expression = self.expression()?;
        if self.eat_kw("FOR") {
            let names: Vec<Identifier> = self.comma_list(|parser| parser.col_id_or_string())?;
            self.expect_kw("IN")?;
            let source: Expression = self.expression()?;
            let condition: Option<Box<Expression>> = match self.eat_kw("IF") {
                true => Some(Box::new(self.expression()?)),
                false => None,
            };
            self.expect_char(']')?;
            let iterator: Expression = match names.len() {
                1 => column_from(names),
                _ => tuple(
                    names
                        .into_iter()
                        .map(|name| column_from(vec![name]))
                        .collect(),
                ),
            };
            return Ok(Expression::Comprehension(Box::new(Comprehension {
                this: Box::new(first),
                expression: Box::new(source),
                position: None,
                iterator: Some(Box::new(iterator)),
                condition,
            })));
        }
        let mut expressions: Vec<Expression> = vec![first];
        while self.eat_char(',') {
            if self.is_char(']') {
                break;
            }
            expressions.push(self.expression()?);
        }
        self.expect_char(']')?;
        Ok(Expression::Array(Box::new(Array {
            expressions,
            inferred_type: None,
        })))
    }

    /// `ColIdOrString`: a name or a string, as a name.
    pub(crate) fn col_id_or_string(&mut self) -> Parsing<Identifier> {
        match self.string_at(0) {
            true => {
                let first: usize = self.at;
                let name: String = self.string_lit()?;
                Ok(Identifier::new(name).with_span(self.tokens[first].span))
            }
            false => self.name(is_identifier),
        }
    }

    /// `{ key: value, ... }`, a struct literal.
    fn struct_expression(&mut self) -> Parsing<Expression> {
        self.expect_char('{')?;
        let mut fields: Vec<(Option<String>, Expression)> = Vec::new();
        while !self.is_char('}') {
            let key: Identifier = self.col_id_or_string()?;
            self.expect_char(':')?;
            fields.push((Some(key.name), self.expression()?));
            if !self.eat_char(',') {
                break;
            }
        }
        self.expect_char('}')?;
        Ok(Expression::Struct(Box::new(Struct { fields })))
    }

    /// `MAP { key: value, ... }`.
    fn map_expression(&mut self) -> Parsing<Expression> {
        self.at += 1;
        self.expect_char('{')?;
        let (mut keys, mut values) = (Vec::new(), Vec::new());
        while !self.is_char('}') {
            keys.push(self.expression()?);
            self.expect_char(':')?;
            values.push(self.expression()?);
            if !self.eat_char(',') {
                break;
            }
        }
        self.expect_char('}')?;
        Ok(Expression::Map(Box::new(Map { keys, values })))
    }

    /// `[*]COLUMNS ( expression )`, after `COLUMNS`.
    fn columns_expression(&mut self, unpacked: bool) -> Parsing<Expression> {
        self.expect_char('(')?;
        let this: Expression = self.expression()?;
        self.expect_char(')')?;
        Ok(Expression::Columns(Box::new(Columns {
            this: Box::new(this),
            unpack: unpacked.then(|| Box::new(boolean(true))),
        })))
    }

    /// `LAMBDA a, b : body`.
    fn lambda_keyword(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let parameters: Vec<Identifier> = self.comma_list(|parser| parser.col_id_or_string())?;
        self.expect_char(':')?;
        let body: Expression = self.expression()?;
        let parameter_types = vec![None; parameters.len()];
        Ok(Expression::Lambda(Box::new(
            sqlquack_syntax::expressions::operators::models::LambdaExpr {
                parameters,
                body,
                colon: true,
                parameter_types,
            },
        )))
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

    /// `CAST (expression AS Type)` or `TRY_CAST (...)`.
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

    /// `INTERVAL 'text' | number | (expr)` and an optional unit or unit range.
    fn interval_literal(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let value: Expression = match self.is_char('(') {
            true => {
                self.at += 1;
                let value: Expression = self.expression()?;
                self.expect_char(')')?;
                paren(value)
            }
            false => self.single_expression()?,
        };
        let unit: Option<IntervalUnitSpec> = self.interval_unit()?;
        Ok(Expression::Interval(Box::new(Interval {
            this: Some(value),
            unit,
        })))
    }

    /// `Interval`: a unit word, or `unit TO unit`, consumed.
    fn interval_unit(&mut self) -> Parsing<Option<IntervalUnitSpec>> {
        let Some(word) = self.word_at(0) else {
            return Ok(None);
        };
        let Some((_, unit)) = INTERVAL_UNITS.iter().find(|(name, _)| *name == word) else {
            return Ok(None);
        };
        let start: usize = self.offset();
        self.at += 1;
        let range: Option<(&str, &str)> = match (self.kw("TO"), self.word_at(1)) {
            (true, Some(to)) => INTERVAL_RANGES
                .iter()
                .find(|(from, upper)| singular(&word) == *from && singular(&to) == *upper)
                .copied(),
            _ => None,
        };
        if let Some((from, upper)) = range {
            let to: String = self.word_at(1).unwrap_or_default();
            self.at += 2;
            self.refuse_later(&format!("{from} TO {upper} is not supported"), start);
            return Ok(Some(IntervalUnitSpec::Expr(Box::new(var(&format!(
                "{word} TO {to}"
            ))))));
        }
        Ok(Some(match unit {
            Some(unit) => IntervalUnitSpec::Simple {
                unit: *unit,
                use_plural: word.ends_with('S') || word.ends_with('A'),
            },
            None => IntervalUnitSpec::Expr(Box::new(var(&word))),
        }))
    }

    /// The indirections after a single expression, left to right.
    fn indirections(&mut self, mut this: Expression) -> Parsing<Expression> {
        let mut links: usize = 0;
        loop {
            if self.eat_op("::") {
                let to: DataType = self.data_type()?;
                this = Expression::Cast(Box::new(cast(this, to, true)));
            } else if self.is_char('.') && self.is_at(1, is_col_label) {
                self.at += 1;
                let name: Identifier = self.name(is_col_label)?;
                this = match self.is_char('(') {
                    true => {
                        let (args, distinct, order_by) = self.method_arguments()?;
                        Expression::MethodCall(Box::new(MethodCall {
                            this,
                            method: name,
                            args,
                            distinct,
                            order_by,
                            schema_omitted: false,
                        }))
                    }
                    false => dot(this, name),
                };
            } else if self.is_char('[') {
                this = self.slice(this)?;
            } else if self.is_char('!') && !self.char_at(1, '=') {
                self.at += 1;
                this = call("!", vec![this]);
            } else {
                return Ok(this);
            }
            links = self.link(links)?;
        }
    }

    /// `[index]` or `[start:end:step]` after `this`.
    fn slice(&mut self, this: Expression) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_char('[')?;
        if self.eat_char(']') {
            self.refuse_later(EMPTY_SUBSCRIPT, start);
            let index: Expression = null();
            return Ok(Expression::Subscript(Box::new(Subscript { this, index })));
        }
        let bound = |parser: &mut Self, stops: &[char]| -> Parsing<Option<Expression>> {
            match stops.iter().any(|stop| parser.is_char(*stop)) {
                true => Ok(None),
                false => parser.expression().map(Some),
            }
        };
        let start: Option<Expression> = bound(self, &[':', ']'])?;
        if !self.eat_char(':') {
            self.expect_char(']')?;
            let index: Expression = start.ok_or_else(|| self.error())?;
            return Ok(Expression::Subscript(Box::new(Subscript { this, index })));
        }
        let end: Option<Expression> = match self.is_char('-') && self.char_at(1, ':') {
            true => {
                self.at += 1;
                None
            }
            false => bound(self, &[':', ']'])?,
        };
        let step: Option<Expression> = match self.eat_char(':') {
            true => bound(self, &[']'])?,
            false => None,
        };
        self.expect_char(']')?;
        if step.is_none() {
            return Ok(Expression::ArraySlice(Box::new(ArraySlice {
                this,
                start,
                end,
            })));
        }
        let index: Expression = Expression::Slice(Box::new(Slice {
            this: start.map(Box::new),
            expression: end.map(Box::new),
            step: step.map(Box::new),
        }));
        Ok(Expression::Subscript(Box::new(Subscript { this, index })))
    }

    /// `AliasedExpression`: `name: expr`, `expr AS label`, or `expr [bare label]`.
    pub(crate) fn named_expression(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        if self.is(is_identifier) && self.char_at(1, ':') && !self.char_at(2, ':') {
            let name: Identifier = self.name(is_identifier)?;
            self.at += 1;
            let this: Expression = self.expression()?;
            let node = crate::parse::_helpers::tree::nodes::alias(this, name, false);
            return Ok(self.spanned(first, node));
        }
        let this: Expression = self.expression()?;
        let node: Expression = if self.eat_kw("AS") {
            let name: Identifier = match self.string_at(0) {
                true => self.col_id_or_string()?,
                false => self.name(is_col_label)?,
            };
            crate::parse::_helpers::tree::nodes::alias(this, name, true)
        } else if self.is(crate::parse::_helpers::tree::cursor::is_bare_label) {
            let name: Identifier =
                self.name(crate::parse::_helpers::tree::cursor::is_bare_label)?;
            crate::parse::_helpers::tree::nodes::alias(this, name, false)
        } else {
            return Ok(this);
        };
        Ok(self.spanned(first, node))
    }
}

/// A unit word without its plural `S`.
fn singular(word: &str) -> &str {
    word.strip_suffix('S').unwrap_or(word)
}
