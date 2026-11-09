//! `tryParseOperand`: literals, names, brackets, `CASE`, `INTERVAL` and the asterisk.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, Star};
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::types::models::{Interval, IntervalUnit, IntervalUnitSpec};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    boolean, call, case, column_from, literal, null, star, string, subquery, tuple,
};
use crate::parse::constants::{COLUMN_NAME_PARTS, INTERVAL_UNITS, TRUE_KEYWORD};
use crate::scan::main::scan::scan;
use crate::scan::main::unquote::unquote;
use crate::scan::models::{Token, TokenKind};

/// The interval kind `word` spells, as `parseIntervalKind` reads it.
pub(crate) fn interval_unit(word: &str) -> Option<IntervalUnit> {
    INTERVAL_UNITS
        .iter()
        .find(|(_, spellings)| spells(spellings, word))
        .map(|(unit, _)| *unit)
}

/// Whether `word` is one of `spellings`, compared without case.
fn spells(spellings: &[&str], word: &str) -> bool {
    spellings
        .iter()
        .any(|spelling| spelling.eq_ignore_ascii_case(word))
}

fn interval(value: Expression, unit: IntervalUnit) -> Expression {
    Expression::Interval(Box::new(Interval {
        this: Some(value),
        unit: Some(IntervalUnitSpec::Simple {
            unit,
            use_plural: false,
        }),
    }))
}

/// The parts of `INTERVAL 'n unit ...'`, `None` for a bare number, or an error for other text.
fn interval_parts(text: &str) -> Result<Option<Vec<(String, IntervalUnit)>>, ()> {
    let tokens: Vec<Token> = scan(text);
    let mut parts: Vec<(String, IntervalUnit)> = Vec::new();
    let mut rest: &[Token] = &tokens;
    loop {
        let (sign, unsigned): (&str, &[Token]) = match rest.split_first() {
            Some((first, after)) if first.kind == TokenKind::Minus => ("-", after),
            Some((first, after)) if first.kind == TokenKind::Plus => ("", after),
            _ => ("", rest),
        };
        let Some((number, after_number)) = unsigned.split_first() else {
            return Err(());
        };
        if number.kind != TokenKind::Number {
            return Err(());
        }
        let Some((unit, after_unit)) = after_number.split_first() else {
            return match parts.is_empty() {
                true => Ok(None),
                false => Err(()),
            };
        };
        let unit: IntervalUnit = match unit.kind {
            TokenKind::BareWord => interval_unit(&unit.value).ok_or(())?,
            _ => return Err(()),
        };
        parts.push((format!("{sign}{}", number.value), unit));
        if after_unit.is_empty() {
            return Ok(Some(parts));
        }
        rest = after_unit;
    }
}

impl Parser<'_> {
    /// Whether the word `offset` ahead is one an alias without `AS` cannot be.
    pub(crate) fn restricted_at(&self, offset: usize) -> bool {
        self.word_at(offset)
            .is_some_and(|word| crate::parse::constants::ALIAS_RESTRICTED.contains(&word.as_str()))
    }

    /// One operand, as `tryParseOperand` tries its parsers in order.
    pub(crate) fn operand(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let node: Expression = self.operand_base()?;
        Ok(self.spanned(first, node))
    }

    fn operand_base(&mut self) -> Parsing<Expression> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        if token.kind == TokenKind::BareWord
            && let Some(node) = self.keyword_operand()?
        {
            return Ok(node);
        }
        match token.kind {
            TokenKind::Minus if self.kind_at(1, TokenKind::Number) => {
                self.at += 2;
                let digits: &str = &self.tokens[self.at - 1].value;
                Ok(literal(Literal::Number(format!("-{digits}"))))
            }
            TokenKind::Number => {
                self.at += 1;
                Ok(literal(Literal::Number(token.value)))
            }
            TokenKind::StringLiteral => {
                self.at += 1;
                Ok(string(&unquote(&token.value)))
            }
            TokenKind::HereDoc => {
                self.at += 1;
                Ok(string(&crate::scan::main::heredoc::heredoc_body(
                    &token.value,
                )))
            }
            TokenKind::Asterisk => {
                self.at += 1;
                let mut node: Star = star(None, None);
                self.star_transformers(&mut node)?;
                Ok(Expression::Star(Box::new(node)))
            }
            TokenKind::BareWord | TokenKind::QuotedIdentifier if is_identifier(&token) => {
                self.name_operand()
            }
            TokenKind::OpeningCurlyBrace => self.substitution(),
            TokenKind::OpeningRoundBracket => self.round_brackets(),
            TokenKind::OpeningSquareBracket => {
                self.at += 1;
                let items: Vec<Expression> =
                    self.layer_elements(TokenKind::ClosingSquareBracket)?;
                Ok(Expression::Array(Box::new(
                    sqlquack_syntax::expressions::types::models::Array {
                        expressions: items,
                        inferred_type: None,
                    },
                )))
            }
            _ => Err(self.error()),
        }
    }

    /// Keyword operands; `INTERVAL` and `CASE` fall back to names when their layer fails.
    fn keyword_operand(&mut self) -> Parsing<Option<Expression>> {
        let word: String = self.word_at(0).unwrap_or_default();
        match word.as_str() {
            "INTERVAL" => Ok(self.optional(|parser| {
                parser.at += 1;
                parser.interval_layer()
            })),
            "CASE" => Ok(self.optional(|parser| {
                parser.at += 1;
                parser.case_layer()
            })),
            "DATE" | "TIMESTAMP" if self.kind_at(1, TokenKind::StringLiteral) => {
                let text: String = unquote(&self.tokens[self.at + 1].value);
                self.at += 2;
                let name: &str = match word.as_str() {
                    "DATE" => "toDate",
                    _ => "toDateTime",
                };
                Ok(Some(call(name, vec![string(&text)])))
            }
            "NULL" => {
                self.at += 1;
                Ok(Some(null()))
            }
            "TRUE" | "FALSE" => {
                self.at += 1;
                Ok(Some(boolean(word == TRUE_KEYWORD)))
            }
            _ => Ok(None),
        }
    }

    /// `IntervalLayer`: `INTERVAL expr unit`, `INTERVAL 'n unit ...'` or `INTERVAL 'n' unit`.
    fn interval_layer(&mut self) -> Parsing<Expression> {
        if self.is(TokenKind::StringLiteral) {
            let text: String = unquote(
                &self
                    .peek()
                    .map(|token| token.value.clone())
                    .unwrap_or_default(),
            );
            match interval_parts(&text) {
                Err(()) => return Err(self.error()),
                Ok(Some(parts)) => {
                    self.at += 1;
                    let mut nodes: Vec<Expression> = parts
                        .into_iter()
                        .map(|(number, unit)| interval(literal(Literal::Number(number)), unit))
                        .collect();
                    return Ok(match nodes.len() {
                        1 => nodes.remove(0),
                        _ => tuple(nodes),
                    });
                }
                Ok(None) => {}
            }
        }
        let value: Expression = self.expression()?;
        let unit: IntervalUnit = self
            .peek()
            .filter(|token| token.kind == TokenKind::BareWord)
            .and_then(|token| interval_unit(&token.value))
            .ok_or_else(|| self.error())?;
        self.at += 1;
        Ok(interval(value, unit))
    }

    /// `CaseLayer`: `CASE [operand] WHEN a THEN b ... [ELSE c] END`.
    fn case_layer(&mut self) -> Parsing<Expression> {
        let operand: Option<Expression> = match self.kw("WHEN") {
            true => None,
            false => Some(self.element()?),
        };
        let mut whens: Vec<(Expression, Expression)> = Vec::new();
        while self.eat_kw("WHEN") {
            let condition: Expression = self.element()?;
            self.expect_kw("THEN")?;
            let result: Expression = self.element()?;
            whens.push((condition, result));
        }
        let otherwise: Option<Expression> = match self.eat_kw("ELSE") {
            true => Some(self.element()?),
            false => None,
        };
        self.expect_kw("END")?;
        Ok(case(operand, whens, otherwise))
    }

    /// A name: a call when `(` follows, a qualified asterisk, or a compound identifier.
    fn name_operand(&mut self) -> Parsing<Expression> {
        let parts: Vec<Identifier> = self.compound_identifier()?;
        if self.is(TokenKind::OpeningRoundBracket) {
            self.at += 1;
            return self.function_layer(parts);
        }
        if self.is(TokenKind::Dot) && self.kind_at(1, TokenKind::Asterisk) {
            self.at += 2;
            let mut node: Star = star(
                Some(crate::parse::_helpers::tree::nodes::joined(parts)),
                None,
            );
            self.star_transformers(&mut node)?;
            return Ok(Expression::Star(Box::new(node)));
        }
        self.within_depth(parts.len().saturating_sub(COLUMN_NAME_PARTS))?;
        Ok(column_from(parts))
    }

    /// `ParserSubstitution`: a query parameter `{name: Type}`, which the shared tree lacks.
    fn substitution(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kind(TokenKind::OpeningCurlyBrace)?;
        let name: Identifier = self.ident()?;
        self.expect_kind(TokenKind::Colon)?;
        self.data_type()?;
        self.expect_kind(TokenKind::ClosingCurlyBrace)?;
        self.gap("query_parameter", start);
        Ok(crate::parse::_helpers::tree::nodes::parameter(Some(
            name.name,
        )))
    }

    /// Whether `(` at `offset` opens a subquery: past any further `(`, a query keyword is next.
    pub(crate) fn subquery_at(&self, offset: usize) -> bool {
        let mut at: usize = offset;
        while self.kind_at(at, TokenKind::OpeningRoundBracket) {
            at += 1;
        }
        at > offset
            && ["SELECT", "WITH", "FROM", "EXPLAIN"]
                .iter()
                .any(|word| self.kw_at(at, word))
    }

    /// `(`: a subquery when one parses, else `RoundBracketsLayer` (a tuple, or brackets).
    fn round_brackets(&mut self) -> Parsing<Expression> {
        if self.subquery_at(0) {
            let attempt = self.attempt(|parser| {
                parser.at += 1;
                let query: Expression = parser.select_with_union()?;
                parser.expect_kind(TokenKind::ClosingRoundBracket)?;
                Ok(query)
            });
            if let Ok(query) = attempt {
                return Ok(subquery(query, None, false));
            }
        }
        self.at += 1;
        if self.eat(TokenKind::ClosingRoundBracket) {
            return Ok(tuple(Vec::new()));
        }
        let mut items: Vec<Expression> = vec![self.element()?];
        let mut is_tuple: bool = false;
        while self.eat(TokenKind::Comma) {
            is_tuple = true;
            items.push(self.element()?);
        }
        self.expect_kind(TokenKind::ClosingRoundBracket)?;
        if is_tuple || self.is(TokenKind::Arrow) {
            return Ok(tuple(items));
        }
        Ok(crate::parse::_helpers::tree::nodes::paren(items.remove(0)))
    }

    /// `LayerWithSeparator`: elements separated by commas up to `close`, which may come first.
    pub(crate) fn layer_elements(&mut self, close: TokenKind) -> Parsing<Vec<Expression>> {
        if self.eat(close) {
            return Ok(Vec::new());
        }
        let items: Vec<Expression> = self.comma_list(|parser| parser.element())?;
        self.expect_kind(close)?;
        Ok(items)
    }

    /// `ParserColumnsTransformers` after an asterisk: `EXCEPT`, `REPLACE` and `APPLY`, repeated.
    pub(crate) fn star_transformers(&mut self, node: &mut Star) -> Parsing<()> {
        loop {
            let start: usize = self.offset();
            if self.kw("EXCEPT") {
                let attempt = self.attempt(|parser| {
                    parser.at += 1;
                    let strict: bool = parser.eat_kw("STRICT");
                    let names: Vec<Identifier> = parser.except_names()?;
                    if strict {
                        parser.gap("strict_column_transformer", start);
                    }
                    Ok(names)
                });
                match attempt {
                    Ok(names) => node.except.get_or_insert_with(Vec::new).extend(names),
                    Err(_) => return Ok(()),
                }
                continue;
            }
            if self.kw("REPLACE") {
                let attempt = self.attempt(|parser| {
                    parser.at += 1;
                    if parser.eat_kw("STRICT") {
                        parser.gap("strict_column_transformer", start);
                    }
                    match parser.eat(TokenKind::OpeningRoundBracket) {
                        true => {
                            let items: Vec<Alias> =
                                parser.comma_list(|parser| parser.replacement())?;
                            parser.expect_kind(TokenKind::ClosingRoundBracket)?;
                            Ok(items)
                        }
                        false => Ok(vec![parser.replacement()?]),
                    }
                });
                match attempt {
                    Ok(items) => node.replace.get_or_insert_with(Vec::new).extend(items),
                    Err(_) => return Ok(()),
                }
                continue;
            }
            if self.kw("APPLY") {
                self.at += 1;
                self.apply_transformer()?;
                self.gap("apply_column_transformer", start);
                continue;
            }
            return Ok(());
        }
    }

    /// `EXCEPT`'s names: `(a, b)`, `a`, or a regular expression, which the shared tree lacks.
    fn except_names(&mut self) -> Parsing<Vec<Identifier>> {
        let start: usize = self.offset();
        let parenthesized: bool = self.eat(TokenKind::OpeningRoundBracket);
        let names: Vec<Identifier> = match self.string_next() {
            true => {
                self.at += 1;
                self.gap("except_regexp", start);
                Vec::new()
            }
            false if parenthesized => self.comma_list(|parser| parser.ident())?,
            false => vec![self.ident()?],
        };
        if parenthesized {
            self.expect_kind(TokenKind::ClosingRoundBracket)?;
        }
        Ok(names)
    }

    /// One `REPLACE` item: `expr AS name`.
    fn replacement(&mut self) -> Parsing<Alias> {
        let this: Expression = self.expression()?;
        self.expect_kw("AS")?;
        let name: Identifier = self.ident()?;
        Ok(Alias::new(this, name))
    }

    /// `APPLY f`, `APPLY(f(params))`, `APPLY(x -> body[, 'prefix'])`, parsed and not kept.
    fn apply_transformer(&mut self) -> Parsing<()> {
        let bracketed: bool = self.eat(TokenKind::OpeningRoundBracket);
        let lambda = self.attempt(|parser| {
            let this: Expression = parser.expression()?;
            match this {
                Expression::Lambda(_) => Ok(()),
                _ => Err(parser.error()),
            }
        });
        if lambda.is_err() {
            self.ident()?;
            if self.eat(TokenKind::OpeningRoundBracket) {
                self.layer_elements(TokenKind::ClosingRoundBracket)?;
            }
        }
        if bracketed && self.eat(TokenKind::Comma) {
            self.string_literal()?;
        }
        if bracketed {
            self.expect_kind(TokenKind::ClosingRoundBracket)?;
        }
        Ok(())
    }
}
