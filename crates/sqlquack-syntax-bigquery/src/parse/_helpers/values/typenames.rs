//! `type` (googlesql.tm): `ARRAY<...>`, `STRUCT<...>`, `RANGE<...>` and the named types.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::types::models::StructField;

use crate::keywords::main::is_reserved::is_reserved;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::var;
use crate::parse::constants::TYPE_LIST_SEPARATOR;
use crate::scan::models::{NumberKind, TokenKind};

impl Parser<'_> {
    /// `dataType`.
    pub(crate) fn data_type(&mut self) -> Parsing<DataType> {
        self.deeper(|parser| parser.data_type_inner())
    }

    /// An unsigned integer in parentheses' place, such as a length or precision.
    fn integer(&mut self) -> Parsing<u32> {
        let value: u32 = match self.peek() {
            Some(token) if token.kind == TokenKind::Number(NumberKind::Integer) => {
                match token.value.parse::<u32>() {
                    Ok(value) => value,
                    Err(_out_of_range) => return Err(self.error()),
                }
            }
            _ => return Err(self.error()),
        };
        self.at += 1;
        Ok(value)
    }

    /// `( n )` after a type name, when written.
    fn optional_length(&mut self) -> Parsing<Option<u32>> {
        if !self.eat_char('(') {
            return Ok(None);
        }
        let length: u32 = self.integer()?;
        self.expect_char(')')?;
        Ok(Some(length))
    }

    /// The type written from token `from` to here, as GoogleSQL prints it: `ARRAY<STRING(10)>`.
    fn type_text(&self, from: usize) -> String {
        let mut text: String = String::new();
        for token in &self.tokens[from..self.at] {
            let piece: String = match token.kind {
                TokenKind::Word => token.value.to_ascii_uppercase(),
                TokenKind::Quoted => format!("`{}`", token.value),
                TokenKind::String { .. } => format!("'{}'", token.value),
                _ => token.value.clone(),
            };
            let glued: bool = matches!(piece.as_str(), "<" | ">" | "(" | ")" | ",")
                || text.ends_with(['<', '('])
                || text.is_empty();
            if !glued {
                text.push(' ');
            }
            text.push_str(&piece);
            if piece == TYPE_LIST_SEPARATOR {
                text.push(' ');
            }
        }
        text
    }

    fn incomplete(&self, position: usize) -> crate::parse::errors::ParseError {
        self.error_with("incomplete complex type definition", position, false)
    }

    fn data_type_inner(&mut self) -> Parsing<DataType> {
        let start: usize = self.offset();
        let type_at: usize = self.at;
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        if token.kind == TokenKind::Quoted {
            self.at += 1;
            return Ok(DataType::Custom { name: token.value });
        }
        let Some(word) = self.word_at(0) else {
            return Err(self.error());
        };
        self.at += 1;
        let data_type: DataType = match word.as_str() {
            "ARRAY" => {
                if !self.eat_char('<') {
                    return Err(self.incomplete(start));
                }
                let element: DataType = self.data_type()?;
                self.expect_char('>')?;
                DataType::Array {
                    element_type: Box::new(element),
                    dimension: None,
                }
            }
            "STRUCT" => {
                let fields: Vec<StructField> = if self.eat_op("<>") {
                    Vec::new()
                } else if self.eat_char('<') {
                    let fields: Vec<StructField> = match self.is_char('>') {
                        true => Vec::new(),
                        false => self.comma_list(|parser| parser.struct_field())?,
                    };
                    self.expect_char('>')?;
                    fields
                } else {
                    return Err(self.incomplete(start));
                };
                DataType::Struct {
                    fields,
                    nested: true,
                }
            }
            "RANGE" => {
                self.expect_char('<')?;
                let element_at: usize = self.at;
                self.data_type()?;
                let element: String = self.type_text(element_at);
                self.expect_char('>')?;
                DataType::Custom {
                    name: format!("RANGE<{element}>"),
                }
            }
            "STRING" => DataType::String {
                length: self.optional_length()?,
            },
            "BYTES" => DataType::VarBinary {
                length: self.optional_length()?,
            },
            "NUMERIC" | "DECIMAL" | "BIGNUMERIC" | "BIGDECIMAL" => {
                let (mut precision, mut scale) = (None, None);
                if self.eat_char('(') {
                    precision = Some(self.integer()?);
                    if self.eat_char(',') {
                        scale = Some(self.integer()?);
                    }
                    self.expect_char(')')?;
                }
                match word.as_str() {
                    "NUMERIC" | "DECIMAL" => DataType::Decimal { precision, scale },
                    _ => DataType::Custom {
                        name: match (precision, scale) {
                            (Some(precision), Some(scale)) => {
                                format!("BIGNUMERIC({precision}, {scale})")
                            }
                            (Some(precision), None) => format!("BIGNUMERIC({precision})"),
                            _ => "BIGNUMERIC".to_owned(),
                        },
                    },
                }
            }
            "INTERVAL" => DataType::Interval {
                unit: None,
                to: None,
            },
            "TIMESTAMP" => DataType::Timestamp {
                precision: None,
                timezone: true,
            },
            "DATETIME" => DataType::Timestamp {
                precision: None,
                timezone: false,
            },
            "TIME" => DataType::Time {
                precision: None,
                timezone: false,
            },
            "GEOGRAPHY" => DataType::Geography {
                subtype: None,
                srid: None,
            },
            "JSON" => DataType::Json,
            "BOOL" | "BOOLEAN" => DataType::Boolean,
            "INT64" | "INT" | "SMALLINT" | "INTEGER" | "BIGINT" | "TINYINT" | "BYTEINT" => {
                DataType::BigInt { length: None }
            }
            "FLOAT64" => DataType::Double {
                precision: None,
                scale: None,
            },
            "DATE" => DataType::Date,
            _ if is_reserved(&word) => return Err(self.error()),
            _ => {
                if self.eat_char('(') {
                    self.comma_list(|parser| parser.expression())?;
                    self.expect_char(')')?;
                }
                DataType::Custom { name: token.value }
            }
        };
        if self.kw("COLLATE") {
            let base: String = self.type_text(type_at);
            self.at += 1;
            let collation: String = self.string_lit()?;
            return Ok(DataType::Custom {
                name: format!("{base} COLLATE '{}'", collation.replace('\'', "\\'")),
            });
        }
        Ok(data_type)
    }

    /// A struct field: `[name] type [NOT NULL] [OPTIONS (...)]`; an unnamed field has no name.
    fn struct_field(&mut self) -> Parsing<StructField> {
        let named: bool = self.is(crate::parse::_helpers::tree::cursor::is_identifier)
            && !self.char_at(1, ',')
            && !self.char_at(1, '>')
            && !self.char_at(1, '<')
            && !self.char_at(1, '(');
        let name: String = match named {
            true => self.ident()?.name,
            false => String::new(),
        };
        let data_type: DataType = self.data_type()?;
        let mut options: Vec<Expression> = Vec::new();
        if self.kw("NOT") && self.kw_at(1, "NULL") {
            self.at += 2;
            options.push(var("NOT NULL"));
        }
        if self.kw("OPTIONS") && self.char_at(1, '(') {
            self.at += 1;
            self.options_list()?;
        }
        Ok(StructField::with_options_and_comment(
            name, data_type, options, None,
        ))
    }
}
