//! `Typename` (gram.y): the SQL-standard type spellings and generic names, with array bounds.

use sqlquack_syntax::DataType;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_type_function_name};
use crate::parse::constants::{
    BIT_TYPE, CATALOGUE_SCHEMA, DEFAULT_LENGTH, FLOAT4_MAX_PRECISION, FLOAT8_MAX_PRECISION,
    INTERVAL_FIELDS, INTERVAL_TYPE, NUMERIC_TYPE, SECOND_FIELD, SETOF_PREFIX, VARBIT_TYPE,
};
use crate::parse::errors::ParseError;
use crate::parse::main::format_type::format_type;
use crate::scan::constants::DECIMAL_RADIX;
use crate::scan::main::digits::digits_value;
use crate::scan::models::TokenKind;

/// Catalogue type names with a shared type, by modifiers; bare `bpchar` has no length, so stays generic.
fn builtin(name: &str, modifiers: &[u32]) -> Option<DataType> {
    let allowed: usize = if name == NUMERIC_TYPE { 2 } else { 1 };
    if modifiers.len() > allowed {
        return None;
    }
    let one: Option<u32> = modifiers.first().copied();
    Some(match (name, one) {
        ("int4", None) => DataType::Int {
            length: None,
            integer_spelling: false,
        },
        ("int2", None) => DataType::SmallInt { length: None },
        ("int8", None) => DataType::BigInt { length: None },
        ("float4", None) => DataType::Float {
            precision: None,
            scale: None,
            real_spelling: true,
        },
        ("float8", None) => DataType::Double {
            precision: None,
            scale: None,
        },
        ("bool", None) => DataType::Boolean,
        ("text", None) => DataType::Text,
        ("date", None) => DataType::Date,
        ("json", None) => DataType::Json,
        ("jsonb", None) => DataType::JsonB,
        ("uuid", None) => DataType::Uuid,
        (NUMERIC_TYPE, _) => DataType::Decimal {
            precision: one,
            scale: modifiers.get(1).copied(),
        },
        ("varchar", length) => DataType::VarChar {
            length,
            parenthesized_length: false,
        },
        ("bpchar", Some(length)) => DataType::Char {
            length: Some(length),
        },
        (VARBIT_TYPE, length) => DataType::VarBit { length },
        (BIT_TYPE, length) => DataType::Bit { length },
        ("timestamp", precision) => DataType::Timestamp {
            precision,
            timezone: false,
        },
        ("timestamptz", precision) => DataType::Timestamp {
            precision,
            timezone: true,
        },
        ("time", precision) => DataType::Time {
            precision,
            timezone: false,
        },
        ("timetz", precision) => DataType::Time {
            precision,
            timezone: true,
        },
        _ => return None,
    })
}

/// `SETOF type`: a name the shared tree keeps apart from the type, as `format_type` reads it.
pub(crate) fn setof(data_type: DataType) -> DataType {
    let text: String = format_type(&data_type).unwrap_or_else(|| format!("{data_type:?}"));
    DataType::Custom {
        name: format!("{SETOF_PREFIX}{text}"),
    }
}

fn integer(text: &str) -> Option<u32> {
    digits_value(text, DECIMAL_RADIX)
}

impl Parser<'_> {
    /// `Typename`: a simple type, then `[n]...` bounds or `ARRAY [ [n] ]`.
    pub(crate) fn typename(&mut self) -> Parsing<DataType> {
        if self.eat_kw("setof") {
            return Ok(setof(self.typename()?));
        }
        let mut data_type: DataType = self.simple_typename()?;
        if self.eat_kw("array") {
            let dimension: Option<u32> = match self.eat_char('[') {
                true => {
                    let bound = self.advance()?;
                    if bound.kind != TokenKind::Integer {
                        self.at -= 1;
                        return Err(self.error());
                    }
                    self.expect_char(']')?;
                    integer(&bound.value)
                }
                false => None,
            };
            return Ok(DataType::Array {
                element_type: Box::new(data_type),
                dimension,
            });
        }
        while self.is_char('[') {
            let dimension: Option<u32> = self.array_bound()?;
            data_type = DataType::Array {
                element_type: Box::new(data_type),
                dimension,
            };
        }
        Ok(data_type)
    }

    /// `[ ]` or `[ Iconst ]`.
    fn array_bound(&mut self) -> Parsing<Option<u32>> {
        self.expect_char('[')?;
        let bound: Option<u32> = match self.is_kind(TokenKind::Integer) {
            true => integer(&self.advance()?.value),
            false => None,
        };
        self.expect_char(']')?;
        Ok(bound)
    }

    /// `( Iconst )`, when present.
    fn precision(&mut self) -> Parsing<Option<u32>> {
        if !self.eat_char('(') {
            return Ok(None);
        }
        let token = self.advance()?;
        if token.kind != TokenKind::Integer {
            self.at -= 1;
            return Err(self.error());
        }
        self.expect_char(')')?;
        Ok(integer(&token.value))
    }

    /// `( expr_list )`, when present: the integer modifiers, or none when one is not an integer.
    fn modifiers(&mut self) -> Parsing<Option<Vec<Option<u32>>>> {
        if !self.is_char('(') {
            return Ok(None);
        }
        let values = self.parenthesized(|parser| parser.a_expr())?;
        Ok(Some(
            values
                .iter()
                .map(|value| match value {
                    sqlquack_syntax::Expression::Literal(literal) => match &**literal {
                        Literal::Number(text) => integer(text),
                        _ => None,
                    },
                    _ => None,
                })
                .collect(),
        ))
    }

    pub(crate) fn simple_typename(&mut self) -> Parsing<DataType> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        let keyword: Option<&str> = match token.kind {
            TokenKind::Keyword(keyword) => Some(keyword.name),
            _ => None,
        };
        match keyword {
            Some("int" | "integer") => {
                self.at += 1;
                Ok(DataType::Int {
                    length: None,
                    integer_spelling: keyword == Some("integer"),
                })
            }
            Some("smallint") => {
                self.at += 1;
                Ok(DataType::SmallInt { length: None })
            }
            Some("bigint") => {
                self.at += 1;
                Ok(DataType::BigInt { length: None })
            }
            Some("real") => {
                self.at += 1;
                Ok(DataType::Float {
                    precision: None,
                    scale: None,
                    real_spelling: true,
                })
            }
            Some("float") => {
                self.at += 1;
                let start: usize = self.offset();
                let precision: Option<u32> = self.precision()?;
                let refusal: Option<&str> = match precision {
                    Some(0) => Some("precision for type float must be at least 1 bit"),
                    Some(bits) if bits > FLOAT8_MAX_PRECISION => {
                        Some("precision for type float must be less than 54 bits")
                    }
                    _ => None,
                };
                if let Some(message) = refusal {
                    return Err(ParseError {
                        message: message.to_owned(),
                        position: start,
                        syntax: false,
                    });
                }
                Ok(match precision {
                    Some(bits) if bits <= FLOAT4_MAX_PRECISION => DataType::Float {
                        precision: None,
                        scale: None,
                        real_spelling: true,
                    },
                    _ => DataType::Double {
                        precision: None,
                        scale: None,
                    },
                })
            }
            Some("double") if self.kw_at(1, "precision") => {
                self.at += 2;
                Ok(DataType::Double {
                    precision: None,
                    scale: None,
                })
            }
            Some("decimal" | "dec" | "numeric") => {
                self.at += 1;
                self.modified_type(NUMERIC_TYPE)
            }
            Some("boolean") => {
                self.at += 1;
                Ok(DataType::Boolean)
            }
            Some("bit") => {
                self.at += 1;
                let name: &str = match self.eat_kw("varying") {
                    true => VARBIT_TYPE,
                    false => BIT_TYPE,
                };
                if name == BIT_TYPE && !self.is_char('(') {
                    return Ok(DataType::Bit {
                        length: Some(DEFAULT_LENGTH),
                    });
                }
                self.modified_type(name)
            }
            Some("character" | "char" | "varchar" | "national" | "nchar") => self.character(),
            Some("timestamp" | "time") => {
                self.at += 1;
                let precision: Option<u32> = self.precision()?;
                let timezone: bool = self.timezone()?;
                Ok(match keyword {
                    Some("time") => DataType::Time {
                        precision,
                        timezone,
                    },
                    _ => DataType::Timestamp {
                        precision,
                        timezone,
                    },
                })
            }
            Some("interval") => {
                self.at += 1;
                if let Some(precision) = self.precision()? {
                    return Ok(DataType::Custom {
                        name: format!("{INTERVAL_TYPE}({precision})"),
                    });
                }
                self.interval_fields()
            }
            Some("json") => {
                self.at += 1;
                Ok(DataType::Json)
            }
            _ if is_type_function_name(&token) => self.generic_type(),
            _ => Err(self.error()),
        }
    }

    /// `CHARACTER [VARYING] [(n)]` and its spellings `CHAR`, `VARCHAR`, `NATIONAL ...`, `NCHAR`.
    fn character(&mut self) -> Parsing<DataType> {
        let first: String = self.advance()?.value;
        let varchar: bool = match first.as_str() {
            "national" if !(self.eat_kw("character") || self.eat_kw("char")) => {
                return Err(self.error());
            }
            "varchar" => true,
            _ => false,
        };
        let varying: bool = varchar || self.eat_kw("varying");
        let length: Option<u32> = self.precision()?;
        Ok(match varying {
            true => DataType::VarChar {
                length,
                parenthesized_length: false,
            },
            false => DataType::Char {
                length: length.or(Some(DEFAULT_LENGTH)),
            },
        })
    }

    /// `ConstTypename`: as `SimpleTypename`, but `BIT` and `CHAR` without a length take none.
    pub(crate) fn const_typename(&mut self) -> Parsing<DataType> {
        let start: usize = self.at;
        let data_type: DataType = self.simple_typename()?;
        let sized: bool = self.tokens[start..self.at]
            .iter()
            .any(|token| token.kind == TokenKind::Char('('));
        Ok(match data_type {
            DataType::Bit { .. } if !sized => DataType::Bit { length: None },
            DataType::Char { .. } if !sized => DataType::Char { length: None },
            other => other,
        })
    }

    /// `WITH TIME ZONE` or `WITHOUT TIME ZONE`, when present: whether it is with.
    fn timezone(&mut self) -> Parsing<bool> {
        let with: bool = self.kw("with");
        if (with || self.kw("without")) && self.kw_at(1, "time") {
            self.at += 2;
            self.expect_kw("zone")?;
            return Ok(with);
        }
        Ok(false)
    }

    /// `opt_interval`: a field, or `field TO field`, with `SECOND` taking a precision.
    pub(crate) fn interval_fields(&mut self) -> Parsing<DataType> {
        let Some(unit) = INTERVAL_FIELDS.iter().find(|field| self.kw(field)) else {
            return Ok(DataType::Interval {
                unit: None,
                to: None,
            });
        };
        self.at += 1;
        if *unit == SECOND_FIELD {
            return Ok(DataType::Interval {
                unit: Some(self.field_text(unit)?),
                to: None,
            });
        }
        if !self.eat_kw("to") {
            return Ok(DataType::Interval {
                unit: Some(unit.to_uppercase()),
                to: None,
            });
        }
        let allowed: &[&str] = match *unit {
            "year" => &["month"],
            "day" => &["hour", "minute", "second"],
            "hour" => &["minute", "second"],
            "minute" => &["second"],
            _ => &[],
        };
        let Some(to) = allowed.iter().find(|field| self.kw(field)) else {
            return Err(self.error());
        };
        self.at += 1;
        Ok(DataType::Interval {
            unit: Some(unit.to_uppercase()),
            to: Some(self.field_text(to)?),
        })
    }

    /// An interval field's text, with the precision `SECOND (n)` takes.
    fn field_text(&mut self, field: &str) -> Parsing<String> {
        let precision: Option<u32> = match field == SECOND_FIELD {
            true => self.precision()?,
            false => None,
        };
        Ok(match precision {
            Some(precision) => format!("{}({precision})", field.to_uppercase()),
            None => field.to_uppercase(),
        })
    }

    /// Catalogue type `name` with `[( expr_list )]`: its shared type, else a name keeping the modifiers.
    fn modified_type(&mut self, name: &str) -> Parsing<DataType> {
        let start: usize = self.offset();
        let modifiers: Option<Vec<Option<u32>>> = self.modifiers()?;
        let integers: Option<Vec<u32>> =
            modifiers.clone().unwrap_or_default().into_iter().collect();
        if let Some(known) = integers.and_then(|integers| builtin(name, &integers)) {
            return Ok(known);
        }
        let end: usize = self.tokens[self.at - 1].end;
        Ok(DataType::Custom {
            name: format!("{name}{}", &self.sql[start..end]),
        })
    }

    /// `GenericType`: `type_function_name [. ColLabel ...] [( expr_list )]`.
    pub(crate) fn generic_type(&mut self) -> Parsing<DataType> {
        let first = self.advance()?;
        let mut parts: Vec<Identifier> =
            vec![crate::parse::_helpers::tree::cursor::identifier(&first)];
        while self.is_char('.') && self.peek_at(1).is_some_and(is_col_label) {
            self.at += 1;
            parts.push(self.name(is_col_label)?);
        }
        let start: usize = self.offset();
        let modifiers = self.modifiers()?;
        let end: usize = self
            .at
            .checked_sub(1)
            .and_then(|last| self.tokens.get(last))
            .map_or(start, |token| token.end);
        let name: Option<&str> = match parts.as_slice() {
            [name] => Some(&name.name),
            [schema, name] if schema.name == CATALOGUE_SCHEMA => Some(&name.name),
            _ => None,
        };
        let integers: Option<Vec<u32>> = modifiers
            .as_deref()
            .unwrap_or_default()
            .iter()
            .copied()
            .collect();
        if let Some(known) = name
            .zip(integers)
            .and_then(|(name, integers)| builtin(name, &integers))
        {
            return Ok(known);
        }
        let name: String = parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<String>>()
            .join(".");
        Ok(DataType::Custom {
            name: match modifiers {
                Some(_) => format!("{name}{}", &self.sql[start..end.max(start)]),
                None => name,
            },
        })
    }
}
