//! `term` and the operands of `expr` (parse.y).

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{BooleanLiteral, Identifier, Literal};
use sqlquack_syntax::expressions::operators::models::{Parameter, ParameterStyle, Placeholder};
use sqlquack_syntax::expressions::types::models::DataType;
use sqlquack_syntax::tokens::models::SourceSpan;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_idj, is_ids, is_nm};
use crate::parse::_helpers::tree::nodes::{
    case, cast, column_from, exists, literal, null, paren, tuple,
};
use crate::parse::constants::{
    MAX_COLUMN_PARTS, MAX_TYPE_MODIFIERS, MAX_VARIABLE_NUMBER, VARIABLE_RANGE,
};
use crate::scan::constants::{FIRST_NON_ASCII, UNRECOGNIZED_TOKEN};
use crate::scan::models::{NumberKind, Token, TokenKind};

/// `sqlite3AffinityType`'s spellings the shared tree has a type for, with no modifiers.
fn named_type(name: &str, modifiers: &[u32]) -> Option<DataType> {
    let first: Option<u32> = modifiers.first().copied();
    let second: Option<u32> = modifiers.get(1).copied();
    let plain: bool = modifiers.is_empty();
    Some(match name {
        "INT" if plain => DataType::Int {
            length: None,
            integer_spelling: false,
        },
        "INTEGER" if plain => DataType::Int {
            length: None,
            integer_spelling: true,
        },
        "BIGINT" if plain => DataType::BigInt { length: None },
        "SMALLINT" if plain => DataType::SmallInt { length: None },
        "TINYINT" if plain => DataType::TinyInt { length: None },
        "REAL" if plain => DataType::Float {
            precision: None,
            scale: None,
            real_spelling: true,
        },
        "FLOAT" if plain => DataType::Float {
            precision: None,
            scale: None,
            real_spelling: false,
        },
        "DOUBLE" | "DOUBLE PRECISION" if plain => DataType::Double {
            precision: None,
            scale: None,
        },
        "NUMERIC" | "DECIMAL" if modifiers.len() <= MAX_TYPE_MODIFIERS => DataType::Decimal {
            precision: first,
            scale: second,
        },
        "TEXT" if plain => DataType::Text,
        "VARCHAR" if modifiers.len() <= 1 => DataType::VarChar {
            length: first,
            parenthesized_length: first.is_some(),
        },
        "CHAR" | "CHARACTER" if modifiers.len() <= 1 => DataType::Char { length: first },
        "BLOB" if plain => DataType::Blob,
        "BOOLEAN" if plain => DataType::Boolean,
        "DATE" if plain => DataType::Date,
        "TIMESTAMP" if plain => DataType::Timestamp {
            precision: None,
            timezone: false,
        },
        _ => return None,
    })
}

impl Parser<'_> {
    /// One operand of `expr` that starts with no prefix operator.
    pub(crate) fn primary(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let node: Expression = self.primary_unspanned()?;
        Ok(self.spanned(first, node))
    }

    fn primary_unspanned(&mut self) -> Parsing<Expression> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        match token.kind {
            TokenKind::Keyword("NULL") => {
                self.at += 1;
                Ok(null())
            }
            TokenKind::Number(kind) => {
                self.at += 1;
                if kind == NumberKind::Misseparated {
                    let message: String = format!("{UNRECOGNIZED_TOKEN} \"{}\"", token.value);
                    self.defer(self.error_with(&message, token.start, true));
                }
                let hex: Option<&str> = token
                    .value
                    .strip_prefix("0x")
                    .or_else(|| token.value.strip_prefix("0X"));
                Ok(literal(match (kind, hex) {
                    (NumberKind::Float, _) | (_, None) => Literal::Number(token.value),
                    (_, Some(digits)) => Literal::HexNumber(digits.to_owned()),
                }))
            }
            TokenKind::String if !self.op_at(1, "DOT") => {
                self.at += 1;
                Ok(literal(Literal::String(token.value)))
            }
            TokenKind::Blob => {
                self.at += 1;
                Ok(literal(Literal::HexString(token.value)))
            }
            TokenKind::Variable => {
                self.at += 1;
                self.variable(&token)
            }
            TokenKind::Keyword("CTIME_KW") => {
                self.at += 1;
                let mut node = crate::parse::_helpers::tree::nodes::function(
                    vec![Identifier::new(token.value.to_ascii_uppercase())],
                    Vec::new(),
                );
                node.no_parens = true;
                Ok(Expression::Function(Box::new(node)))
            }
            TokenKind::Keyword("CAST") => self.cast_expression(),
            TokenKind::Keyword("CASE") => self.case_expression(),
            TokenKind::Keyword("EXISTS") => {
                self.at += 1;
                let query: Expression = self.subquery_expression()?;
                let Expression::Subquery(node) = query else {
                    return Err(self.error());
                };
                Ok(exists(node.this))
            }
            TokenKind::Keyword("RAISE") => self.raise_expression(),
            TokenKind::Op("LP") => self.parenthesized(),
            _ if is_idj(&token) && self.op_at(1, "LP") => {
                self.at += 2;
                self.function_call(identifier(&token))
            }
            _ if is_nm(&token) => self.column_reference(),
            _ => Err(self.error()),
        }
    }

    /// `idj`, `nm.nm` or `nm.nm.nm`: a column, with its table and schema when written.
    fn column_reference(&mut self) -> Parsing<Expression> {
        let first = self.expect_token(is_nm)?;
        if !self.op("DOT") {
            if first.kind == TokenKind::String || !is_idj(&first) {
                return Err(self.error_at(&first));
            }
            if first.kind == TokenKind::Ident {
                let truth: Option<bool> = match first.value.to_ascii_lowercase().as_str() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                };
                if let Some(value) = truth {
                    return Ok(Expression::Boolean(BooleanLiteral { value }));
                }
            }
            return Ok(column_from(vec![identifier(&first)]));
        }
        let mut parts: Vec<Identifier> = vec![identifier(&first)];
        while parts.len() < MAX_COLUMN_PARTS && self.eat_op("DOT") {
            parts.push(self.name(is_nm)?);
        }
        Ok(column_from(parts))
    }

    /// The syntax error at a token already consumed.
    fn error_at(&self, token: &Token) -> crate::parse::errors::ParseError {
        self.error_with(
            &format!(
                "near \"{}\": syntax error",
                &self.sql[token.start..token.end]
            ),
            token.start,
            true,
        )
    }

    /// `VARIABLE`: plain parameters as nodes, Tcl and `#` forms as a gap, `#NNN` refused.
    fn variable(&mut self, token: &Token) -> Parsing<Expression> {
        let text: &str = token.value.as_str();
        let (sigil, rest) = text.split_at(1);
        let numbered: bool = rest.starts_with(|character: char| character.is_ascii_digit());
        match (sigil, rest.is_empty(), numbered) {
            ("?", true, _) => return Ok(Expression::Placeholder(Placeholder { index: None })),
            ("?", false, _) => {
                let Ok(index) = rest.parse::<u32>() else {
                    return Err(self.error_with(VARIABLE_RANGE, token.start, false));
                };
                return match (1..=MAX_VARIABLE_NUMBER).contains(&index) {
                    true => Ok(Expression::Placeholder(Placeholder { index: Some(index) })),
                    false => Err(self.error_with(VARIABLE_RANGE, token.start, false)),
                };
            }
            ("#", _, true) => return Err(self.error_at(token)),
            _ => {}
        }
        let plain: bool = rest
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte >= FIRST_NON_ASCII);
        let style: Option<ParameterStyle> = match sigil {
            ":" => Some(ParameterStyle::Colon),
            "@" => Some(ParameterStyle::At),
            "$" => Some(ParameterStyle::Dollar),
            _ => None,
        };
        match (style, plain) {
            (Some(style), true) => Ok(Expression::Parameter(Box::new(Parameter {
                name: Some(rest.to_owned()),
                index: None,
                style,
                quoted: false,
                string_quoted: false,
                expression: None,
                span: SourceSpan::default(),
            }))),
            _ => {
                self.gap("parameter", token.start);
                Ok(null())
            }
        }
    }

    /// `( select )`, `( expr )` or the row value `( expr, expr, ... )`.
    fn parenthesized(&mut self) -> Parsing<Expression> {
        if self.starts_select_at(1) {
            return self.subquery_expression();
        }
        self.expect_op("LP")?;
        let items: Vec<Expression> = self.comma_list(|parser| parser.expression())?;
        self.expect_op("RP")?;
        let mut items = items;
        Ok(match items.len() {
            1 => paren(items.remove(0)),
            _ => tuple(items),
        })
    }

    /// `CAST ( expr AS typetoken )`.
    fn cast_expression(&mut self) -> Parsing<Expression> {
        self.expect_kw("CAST")?;
        self.expect_op("LP")?;
        let this: Expression = self.expression()?;
        self.expect_kw("AS")?;
        let to: DataType = self.type_token()?;
        self.expect_op("RP")?;
        Ok(Expression::Cast(Box::new(cast(this, to, false))))
    }

    /// `typetoken`: the type its name spells, or a custom type holding the text as written.
    pub(crate) fn type_token(&mut self) -> Parsing<DataType> {
        let first: usize = self.at;
        let mut words: Vec<Token> = Vec::new();
        while self.is(is_ids) {
            words.push(self.advance()?);
        }
        let mut modifiers: Vec<Option<u32>> = Vec::new();
        if !words.is_empty() && self.eat_op("LP") {
            modifiers.push(self.signed_number()?);
            if self.eat_op("COMMA") {
                modifiers.push(self.signed_number()?);
            }
            self.expect_op("RP")?;
        }
        let written: &str = self.text_from(first);
        let plain_words: bool = words
            .iter()
            .all(|word| matches!(word.kind, TokenKind::Ident | TokenKind::Keyword(_)));
        let lengths: Option<Vec<u32>> = modifiers.into_iter().collect();
        let name: String = words
            .iter()
            .map(|word| word.value.to_ascii_uppercase())
            .collect::<Vec<String>>()
            .join(" ");
        let known: Option<DataType> = match (plain_words, lengths) {
            (true, Some(lengths)) => named_type(&name, &lengths),
            _ => None,
        };
        Ok(known.unwrap_or_else(|| DataType::Custom {
            name: written.to_owned(),
        }))
    }

    /// `signed`: `[+|-]` an integer or a float; its value when it is a plain unsigned integer.
    fn signed_number(&mut self) -> Parsing<Option<u32>> {
        let sign: bool = self.eat_op("PLUS") || self.eat_op("MINUS");
        let token = self.expect_token(|token| {
            matches!(
                token.kind,
                TokenKind::Number(NumberKind::Integer | NumberKind::Float)
            )
        })?;
        let unsigned: bool = !sign && token.kind == TokenKind::Number(NumberKind::Integer);
        Ok(match token.value.parse::<u32>() {
            Ok(value) if unsigned => Some(value),
            _ => None,
        })
    }

    /// `CASE [operand] WHEN expr THEN expr ... [ELSE expr] END`.
    fn case_expression(&mut self) -> Parsing<Expression> {
        self.expect_kw("CASE")?;
        let operand: Option<Expression> = match self.kw("WHEN") {
            true => None,
            false => Some(self.expression()?),
        };
        let mut whens: Vec<(Expression, Expression)> = Vec::new();
        loop {
            self.expect_kw("WHEN")?;
            let condition: Expression = self.expression()?;
            self.expect_kw("THEN")?;
            whens.push((condition, self.expression()?));
            if !self.kw("WHEN") {
                break;
            }
        }
        let otherwise: Option<Expression> = match self.eat_kw("ELSE") {
            true => Some(self.expression()?),
            false => None,
        };
        self.expect_kw("END")?;
        Ok(case(operand, whens, otherwise))
    }

    /// `RAISE (...)`, which only a trigger program may run: a gap.
    fn raise_expression(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("RAISE")?;
        self.expect_op("LP")?;
        if !self.eat_kw("IGNORE") {
            match self.kind_at(0) {
                Some(TokenKind::Keyword("ROLLBACK" | "ABORT" | "FAIL")) => self.at += 1,
                _ => return Err(self.error()),
            }
            self.expect_op("COMMA")?;
            self.expression()?;
        }
        self.expect_op("RP")?;
        self.gap("raise", start);
        Ok(null())
    }
}
