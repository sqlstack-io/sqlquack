//! `ParserDataType`; a type is kept as its source text, since type names are case-sensitive.

use sqlquack_syntax::DataType;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::constants::{TYPE_NAME_REFUSED, TYPE_SUFFIXES, VERSIONED_AGGREGATE_TYPE};
use crate::scan::main::unquote::unquote;
use crate::scan::models::TokenKind;

impl Parser<'_> {
    /// `ParserDataType`: a type name, its suffix words, and its arguments in brackets.
    pub(crate) fn data_type(&mut self) -> Parsing<DataType> {
        let start: usize = self.offset();
        self.type_tokens()?;
        let end: usize = self
            .at
            .checked_sub(1)
            .and_then(|last| self.tokens.get(last))
            .map_or(start, |token| token.end);
        Ok(DataType::Custom {
            name: self.sql[start..end].to_owned(),
        })
    }

    /// The tokens of one type, consumed, one level deeper.
    fn type_tokens(&mut self) -> Parsing<()> {
        self.deeper(|parser| parser.type_tokens_inner())
    }

    fn type_tokens_inner(&mut self) -> Parsing<()> {
        let Some(token) = self.peek().cloned().filter(is_identifier) else {
            return Err(self.error());
        };
        let name: String = match token.kind {
            TokenKind::QuotedIdentifier => unquote(&token.value),
            _ => token.value.clone(),
        };
        let upper: String = name.to_ascii_uppercase();
        if !name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '$'
        }) || TYPE_NAME_REFUSED.contains(&upper.as_str())
        {
            return Err(self.error());
        }
        self.at += 1;
        self.type_suffix(&upper)?;
        if self.is(TokenKind::Comma) && self.kind_at(1, TokenKind::ClosingRoundBracket) {
            self.at += 1;
        }
        if ["ENUM", "ENUM8", "ENUM16"].contains(&upper.as_str())
            && self.is(TokenKind::OpeningRoundBracket)
        {
            let enumeration: bool = self.parses(|parser| {
                parser.at += 1;
                parser.comma_list(|parser| parser.enum_element())?;
                parser.expect_kind(TokenKind::ClosingRoundBracket)
            });
            if enumeration {
                return Ok(());
            }
        }
        if !self.eat(TokenKind::OpeningRoundBracket) {
            return Ok(());
        }
        let versioned: bool = name == VERSIONED_AGGREGATE_TYPE && self.eat(TokenKind::Number);
        let mut index: usize = usize::from(versioned);
        loop {
            if index > 0 && !self.eat(TokenKind::Comma) {
                break;
            }
            let parsed: bool = self.type_argument(&name, index, versioned);
            if !parsed {
                break;
            }
            index += 1;
        }
        self.expect_kind(TokenKind::ClosingRoundBracket)
    }

    /// The words some type names take after them: `DOUBLE PRECISION`, `INT UNSIGNED`, ….
    fn type_suffix(&mut self, upper: &str) -> Parsing<()> {
        if let Some((_, suffixes)) = TYPE_SUFFIXES.iter().find(|(name, _)| *name == upper) {
            if let Some(words) = suffixes.iter().find(|words| self.kws(words)) {
                self.at += words.len();
            }
            return Ok(());
        }
        if !upper.contains("INT") {
            return Ok(());
        }
        if self.eat(TokenKind::OpeningRoundBracket) {
            self.eat(TokenKind::Number);
            self.expect_kind(TokenKind::ClosingRoundBracket)?;
        }
        if !self.eat_kw("SIGNED") {
            self.eat_kw("UNSIGNED");
        }
        Ok(())
    }

    /// `'name' = [-]number` of an enum type.
    fn enum_element(&mut self) -> Parsing<()> {
        let name = self.peek().filter(|token| {
            token.kind == TokenKind::StringLiteral && !token.value.starts_with(['b', 'B', 'x', 'X'])
        });
        if name.is_none() {
            return Err(self.error());
        }
        self.at += 1;
        self.expect_kind(TokenKind::Equals)?;
        self.eat(TokenKind::Minus);
        self.expect_kind(TokenKind::Number)
    }

    /// One argument of type `name` at `index`; whether one parsed, as each parser tries in turn.
    fn type_argument(&mut self, name: &str, index: usize, versioned: bool) -> bool {
        match name {
            "Nested" => self.parses(|parser| parser.name_type_pair()),
            "Tuple" => {
                self.parses(|parser| parser.name_type_pair())
                    || self.parses(|parser| parser.type_tokens())
            }
            "Dynamic" => self.parses(|parser| {
                parser.ident()?;
                parser.expect_kind(TokenKind::Equals)?;
                parser.expect_kind(TokenKind::Number)
            }),
            "AggregateFunction" | "SimpleAggregateFunction" => {
                match index == usize::from(versioned) {
                    true => self.parses(|parser| {
                        let parts = parser.compound_identifier()?;
                        if parser.eat(TokenKind::OpeningRoundBracket) {
                            parser.function_layer(parts)?;
                        }
                        Ok(())
                    }),
                    false => self.parses(|parser| parser.type_tokens()),
                }
            }
            _ if name.eq_ignore_ascii_case("json") => self.parses(|parser| parser.json_argument()),
            _ => {
                self.parses(|parser| {
                    parser.type_literal()?;
                    while parser.eat(TokenKind::Equals) {
                        parser.type_literal()?;
                    }
                    Ok(())
                }) || self.parses(|parser| parser.type_tokens())
            }
        }
    }

    /// `ParserNameTypePair`: `name Type`.
    fn name_type_pair(&mut self) -> Parsing<()> {
        self.ident()?;
        self.type_tokens()
    }

    /// `ParserLiteral` in a type's arguments: a number, string, `NULL`, `TRUE` or `FALSE`.
    fn type_literal(&mut self) -> Parsing<()> {
        if self.is(TokenKind::Minus) || self.is(TokenKind::Plus) {
            self.at += 1;
            return self.expect_kind(TokenKind::Number);
        }
        if self.is(TokenKind::Number)
            || self.is(TokenKind::StringLiteral)
            || self.kw_in(0, &["NULL", "TRUE", "FALSE"])
        {
            self.at += 1;
            return Ok(());
        }
        Err(self.error())
    }

    /// `ObjectArgumentParser`: `SKIP REGEXP 'r'`, `SKIP path`, `path = n` or `path Type`.
    fn json_argument(&mut self) -> Parsing<()> {
        if self.eat_kw("SKIP") {
            if self.eat_kw("REGEXP") {
                return self.expect_kind(TokenKind::StringLiteral);
            }
            return self.compound_identifier().map(|_| ());
        }
        self.compound_identifier()?;
        if self.eat(TokenKind::Equals) {
            return self.expect_kind(TokenKind::Number);
        }
        self.type_tokens()
    }
}
