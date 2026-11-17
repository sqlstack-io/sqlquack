//! `dataType` (SqlBaseParser.g4): `ARRAY<...>`, `MAP<...>`, `STRUCT<...>` and the primitive types.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::types::models::{StructField, TypeSpelling};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{joined, var};
use crate::parse::constants::{
    DAY_TIME, INTEGER_SPELLING, LOCAL_TIMESTAMP, MONTH, REAL_SPELLING, YEAR_MONTH,
};
use crate::scan::models::{NumberKind, TokenKind};

impl Parser<'_> {
    /// `dataType`, without the collation written after a string type.
    pub(crate) fn data_type(&mut self) -> Parsing<DataType> {
        Ok(self.collated_type()?.0)
    }

    /// `dataType` and the collation written after a string type, when written.
    pub(crate) fn collated_type(&mut self) -> Parsing<(DataType, Option<Identifier>)> {
        let data_type: DataType = self.deeper(|parser| parser.data_type_inner())?;
        let string: bool = matches!(
            data_type,
            DataType::String { .. } | DataType::Char { .. } | DataType::VarChar { .. }
        );
        let collation: Option<Identifier> = match string && self.eat_kw("COLLATE") {
            true => Some(joined(self.multipart_identifier()?)),
            false => None,
        };
        Ok((data_type, collation))
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
            "STRING" => DataType::String { length: None },
            "CHARACTER" | "CHAR" => {
                let length: Option<u32> = self.optional_length()?;
                DataType::Char { length }
            }
            "VARCHAR" => {
                let length: Option<u32> = self.optional_length()?;
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
                DataType::Decimal {
                    precision,
                    scale,
                    spelling: synonym(&word),
                }
            }
            "INTERVAL" => {
                let (unit, to) = self.interval_fields()?;
                DataType::Interval { unit, to }
            }
            "TIMESTAMP" => {
                let precision: Option<u32> = self.optional_length()?;
                let timezone: bool = if self.eat_kws(&["WITH", "LOCAL", "TIME", "ZONE"]) {
                    true
                } else {
                    !self.eat_kws(&["WITHOUT", "TIME", "ZONE"])
                };
                DataType::Timestamp {
                    precision,
                    timezone,
                    spelling: None,
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
            "TINYINT" | "BYTE" => DataType::TinyInt {
                length: None,
                spelling: synonym(&word),
            },
            "SMALLINT" | "SHORT" => DataType::SmallInt {
                length: None,
                spelling: synonym(&word),
            },
            "INT" | "INTEGER" => DataType::Int {
                length: None,
                integer_spelling: word == INTEGER_SPELLING,
            },
            "BIGINT" | "LONG" => DataType::BigInt {
                length: None,
                spelling: synonym(&word),
            },
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
            "TIMESTAMP_LTZ" | "TIMESTAMP_NTZ" => DataType::Timestamp {
                precision: self.optional_length()?,
                timezone: word == LOCAL_TIMESTAMP,
                spelling: (word == LOCAL_TIMESTAMP).then_some(TypeSpelling::TimestampLtz),
            },
            "DECFLOAT" => DataType::Named {
                name: word,
                precision: self.optional_length()?,
                scale: None,
                length: None,
            },
            "BINARY" => DataType::Binary { length: None },
            "FILE" => match self.word_at(0).as_deref() {
                Some(kind @ ("MANAGED" | "EXTERNAL")) => {
                    let name: String = format!("FILE {kind}");
                    self.at += 1;
                    DataType::Custom { name }
                }
                _ => return Err(self.error()),
            },
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
        let name: Identifier = self.ident()?;
        self.eat_char(':');
        let (data_type, collation) = self.collated_type()?;
        let mut options: Vec<Expression> = Vec::new();
        if self.kw("NOT") && self.kw_at(1, "NULL") {
            self.at += 2;
            options.push(var("NOT NULL"));
        }
        let comment: Option<String> = match self.eat_kw("COMMENT") {
            true => Some(self.string_lit()?),
            false => None,
        };
        let name: String = match name.quoted {
            true => format!("`{}`", name.name.replace('`', "``")),
            false => name.name,
        };
        let mut field: StructField =
            StructField::with_options_and_comment(name, data_type, options, comment);
        field.collation = collation.map(Box::new);
        Ok(field)
    }
}

/// The synonym `word` names, when it is one rather than the type's own name.
fn synonym(word: &str) -> Option<TypeSpelling> {
    match word {
        "BYTE" => Some(TypeSpelling::Byte),
        "SHORT" => Some(TypeSpelling::Short),
        "LONG" => Some(TypeSpelling::Long),
        "DEC" => Some(TypeSpelling::Dec),
        "NUMERIC" => Some(TypeSpelling::Numeric),
        _ => None,
    }
}
