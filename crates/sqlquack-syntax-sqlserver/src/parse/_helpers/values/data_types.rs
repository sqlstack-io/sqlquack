//! Data types: the built-in types with their lengths, precisions and scales, and user types.

use sqlquack_syntax::DataType;

use crate::keywords::main::is_reserved::is_reserved;
use crate::parse::constants::{INTEGER_SPELLING, NCHAR_TYPE, XML_TYPE};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_any_name, is_name};
use crate::scan::models::{NumberKind, Token, TokenKind};

/// A type modifier: a number, or `MAX` for the variable-length types.
enum Modifier {
    Number(u32),
    Max,
}

/// The shared-tree type for a built-in name and its modifiers; others keep their written text.
fn built_in(name: &str, modifiers: &[Modifier], text: &str) -> DataType {
    let first: Option<u32> = match modifiers.first() {
        Some(Modifier::Number(number)) => Some(*number),
        _ => None,
    };
    let second: Option<u32> = match modifiers.get(1) {
        Some(Modifier::Number(number)) => Some(*number),
        _ => None,
    };
    let max: bool = matches!(modifiers.first(), Some(Modifier::Max));
    match (name, max) {
        ("INT" | "INTEGER", _) => DataType::Int {
            length: None,
            integer_spelling: name == INTEGER_SPELLING,
        },
        ("BIGINT", _) => DataType::BigInt { length: None },
        ("SMALLINT", _) => DataType::SmallInt { length: None },
        ("TINYINT", _) => DataType::TinyInt { length: None },
        ("BIT", _) => DataType::Bit { length: None },
        ("DECIMAL" | "DEC" | "NUMERIC", _) => DataType::Decimal {
            precision: first,
            scale: second,
        },
        ("REAL", _) => DataType::Float {
            precision: None,
            scale: None,
            real_spelling: true,
        },
        ("FLOAT", _) => DataType::Double {
            precision: first,
            scale: None,
        },
        ("DATE", _) => DataType::Date,
        ("TIME", _) => DataType::Time {
            precision: first,
            timezone: false,
        },
        ("CHAR" | "CHARACTER", _) => DataType::Char { length: first },
        ("VARCHAR", false) => DataType::VarChar {
            length: first,
            parenthesized_length: first.is_some(),
        },
        ("BINARY", _) => DataType::Binary { length: first },
        ("VARBINARY", false) => DataType::VarBinary { length: first },
        _ => DataType::Custom {
            name: text.to_owned(),
        },
    }
}

impl<'sql> Parser<'sql> {
    /// A data type: `[schema.]name [( n [, m] | MAX )]`, with the two-word spellings.
    pub(crate) fn data_type(&mut self) -> Parsing<DataType> {
        let first: Token = self.expect_token(is_any_name)?;
        let quoted: bool = first.kind != TokenKind::Word;
        let mut name: String = first.value.to_ascii_uppercase();
        let mut written: String = self.sql[first.start..first.end].to_owned();
        if self.is_char('.') {
            if !quoted && is_reserved(&first.value) {
                return Err(self.error_at(self.at - 1));
            }
            self.at += 1;
            let second: Token = self.expect_token(is_name)?;
            written = format!("{written}.{}", &self.sql[second.start..second.end]);
            name = written.clone();
        } else if !quoted {
            let two_words: Option<&str> = match (name.as_str(), self.word_at(0).as_deref()) {
                ("DOUBLE", Some("PRECISION")) => Some("FLOAT"),
                ("CHAR" | "CHARACTER", Some("VARYING")) => Some("VARCHAR"),
                ("NATIONAL", Some("CHAR" | "CHARACTER")) => Some("NCHAR"),
                ("NATIONAL", Some("TEXT")) => Some("NTEXT"),
                ("BINARY", Some("VARYING")) => Some("VARBINARY"),
                _ => None,
            };
            if two_words.is_none() && is_reserved(&name) {
                return Err(self.error_at(self.at - 1));
            }
            if let Some(two_words) = two_words {
                self.at += 1;
                name = two_words.to_owned();
                if name == NCHAR_TYPE && self.eat_kw("VARYING") {
                    name = "NVARCHAR".to_owned();
                }
                written = name.clone();
            } else {
                written = name.clone();
            }
        }
        let mut modifiers: Vec<Modifier> = Vec::new();
        if name == XML_TYPE && self.is_char('(') {
            return Err(self.gap("xml_schema_collection", self.offset()));
        }
        if self.is_char('(') {
            self.at += 1;
            loop {
                let token: Token = self.advance()?;
                match token.kind {
                    TokenKind::Number(NumberKind::Integer) => match token.value.parse::<u32>() {
                        Ok(number) => modifiers.push(Modifier::Number(number)),
                        Err(_) => return Err(self.error_at(self.at - 1)),
                    },
                    TokenKind::Word if token.value.eq_ignore_ascii_case("MAX") => {
                        modifiers.push(Modifier::Max);
                    }
                    _ => return Err(self.error_at(self.at - 1)),
                }
                if !self.eat_char(',') {
                    break;
                }
            }
            self.expect_char(')')?;
            let rendered: Vec<String> = modifiers
                .iter()
                .map(|modifier| match modifier {
                    Modifier::Number(number) => number.to_string(),
                    Modifier::Max => "MAX".to_owned(),
                })
                .collect();
            written = format!("{written}({})", rendered.join(", "));
        }
        Ok(match quoted || name.contains('.') {
            true => DataType::Custom { name: written },
            false => built_in(&name, &modifiers, &written),
        })
    }
}
