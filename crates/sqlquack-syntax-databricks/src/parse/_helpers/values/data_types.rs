//! `dataType` (SqlBaseParser.g4): `ARRAY<...>`, `MAP<...>`, `STRUCT<...>` and the primitive types.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::types::models::StructField;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::var;
use crate::parse::constants::{DAY_TIME, INTEGER_SPELLING, MONTH, REAL_SPELLING, YEAR_MONTH};
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

    /// `COLLATE name` after a string type, read and dropped.
    fn skip_collate(&mut self) -> Parsing<()> {
        if self.eat_kw("COLLATE") {
            self.multipart_identifier()?;
        }
        Ok(())
    }

    fn incomplete(&self, position: usize) -> crate::parse::errors::ParseError {
        self.error_with("incomplete complex type definition", position, false)
    }

    fn data_type_inner(&mut self) -> Parsing<DataType> {
        let start: usize = self.offset();
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
            "MAP" => {
                if !self.eat_char('<') {
                    return Err(self.incomplete(start));
                }
                let key: DataType = self.data_type()?;
                self.expect_char(',')?;
                let value: DataType = self.data_type()?;
                self.expect_char('>')?;
                DataType::Map {
                    key_type: Box::new(key),
                    value_type: Box::new(value),
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
            "STRING" => {
                self.skip_collate()?;
                DataType::String { length: None }
            }
            "CHARACTER" | "CHAR" => {
                let length: Option<u32> = self.optional_length()?;
                self.skip_collate()?;
                DataType::Char { length }
            }
            "VARCHAR" => {
                let length: Option<u32> = self.optional_length()?;
                self.skip_collate()?;
                DataType::VarChar {
                    length,
                    parenthesized_length: length.is_some(),
                }
            }
            "DECIMAL" | "DEC" | "NUMERIC" => {
                let (mut precision, mut scale) = (None, None);
                if self.eat_char('(') {
                    precision = Some(self.integer()?);
                    if self.eat_char(',') {
                        scale = Some(self.integer()?);
                    }
                    self.expect_char(')')?;
                }
                DataType::Decimal { precision, scale }
            }
            "INTERVAL" => {
                let (unit, to) = self.interval_fields()?;
                DataType::Interval { unit, to }
            }
            "TIMESTAMP" => {
                let timezone: bool = if self.eat_kws(&["WITH", "LOCAL", "TIME", "ZONE"]) {
                    true
                } else {
                    !self.eat_kws(&["WITHOUT", "TIME", "ZONE"])
                };
                DataType::Timestamp {
                    precision: None,
                    timezone,
                }
            }
            "TIME" => {
                let precision: Option<u32> = self.optional_length()?;
                self.eat_kws(&["WITHOUT", "TIME", "ZONE"]);
                DataType::Time {
                    precision,
                    timezone: false,
                }
            }
            "GEOGRAPHY" | "GEOMETRY" => {
                self.expect_char('(')?;
                let srid: Option<u32> = match self.eat_kw("ANY") {
                    true => None,
                    false => Some(self.integer()?),
                };
                self.expect_char(')')?;
                match word.as_str() {
                    "GEOGRAPHY" => DataType::Geography {
                        subtype: None,
                        srid,
                    },
                    _ => DataType::Geometry {
                        subtype: None,
                        srid,
                    },
                }
            }
            "BOOLEAN" => DataType::Boolean,
            "TINYINT" | "BYTE" => DataType::TinyInt { length: None },
            "SMALLINT" | "SHORT" => DataType::SmallInt { length: None },
            "INT" | "INTEGER" => DataType::Int {
                length: None,
                integer_spelling: word == INTEGER_SPELLING,
            },
            "BIGINT" | "LONG" => DataType::BigInt { length: None },
            "FLOAT" | "REAL" => DataType::Float {
                precision: None,
                scale: None,
                real_spelling: word == REAL_SPELLING,
            },
            "DOUBLE" => DataType::Double {
                precision: None,
                scale: None,
            },
            "DATE" => DataType::Date,
            "TIMESTAMP_LTZ" => DataType::Timestamp {
                precision: None,
                timezone: true,
            },
            "TIMESTAMP_NTZ" => DataType::Timestamp {
                precision: None,
                timezone: false,
            },
            "BINARY" => DataType::Binary { length: None },
            "VOID" | "VARIANT" => DataType::Custom { name: word },
            _ => {
                if self.eat_char('(') {
                    self.comma_list(|parser| parser.integer())?;
                    self.expect_char(')')?;
                }
                return Err(self.error_with(
                    &format!("unsupported data type {word}"),
                    start,
                    false,
                ));
            }
        };
        Ok(data_type)
    }

    /// The optional fields of an `INTERVAL` type: `YEAR [TO MONTH]`, `DAY [TO SECOND]` and so on.
    fn interval_fields(&mut self) -> Parsing<(Option<String>, Option<String>)> {
        let Some(from) = self.word_at(0) else {
            return Ok((None, None));
        };
        let year_month: bool = YEAR_MONTH.contains(&from.as_str());
        if !year_month && !DAY_TIME.contains(&from.as_str()) {
            return Ok((None, None));
        }
        self.at += 1;
        if !self.eat_kw("TO") {
            return Ok((Some(from), None));
        }
        let to: Option<String> = self.word_at(0).filter(|to| match year_month {
            true => to == MONTH,
            false => ["HOUR", "MINUTE", "SECOND"].contains(&to.as_str()),
        });
        match to {
            Some(to) => {
                self.at += 1;
                Ok((Some(from), Some(to)))
            }
            None => Err(self.error()),
        }
    }

    /// `complexColType`: `name [:] dataType [NOT NULL] [COMMENT '...']`.
    fn struct_field(&mut self) -> Parsing<StructField> {
        let name = self.ident()?;
        self.eat_char(':');
        let data_type: DataType = self.data_type()?;
        let mut options: Vec<Expression> = Vec::new();
        if self.kw("NOT") && self.kw_at(1, "NULL") {
            self.at += 2;
            options.push(var("NOT NULL"));
        }
        let comment: Option<String> = match self.eat_kw("COMMENT") {
            true => Some(self.string_lit()?),
            false => None,
        };
        Ok(StructField::with_options_and_comment(
            name.name, data_type, options, comment,
        ))
    }
}
