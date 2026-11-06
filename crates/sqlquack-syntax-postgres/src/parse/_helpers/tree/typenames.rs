//! `Typename` (gram.y): the SQL-standard type spellings and generic names, with array bounds.

use sqlquack_syntax::DataType;
use sqlquack_syntax::expressions::names::models::Identifier;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_type_function_name};
use crate::parse::constants::{FLOAT4_MAX_PRECISION, INTERVAL_FIELDS, SECOND_FIELD};
use crate::scan::constants::DECIMAL_RADIX;
use crate::scan::main::digits::digits_value;
use crate::scan::models::TokenKind;

/// Generic type names the shared tree has a type for, unquoted and unqualified.
fn generic(name: &str) -> Option<DataType> {
    Some(match name {
        "int4" => DataType::Int {
            length: None,
            integer_spelling: false,
        },
        "int2" => DataType::SmallInt { length: None },
        "int8" => DataType::BigInt { length: None },
        "float4" => DataType::Float {
            precision: None,
            scale: None,
            real_spelling: true,
        },
        "float8" => DataType::Double {
            precision: None,
            scale: None,
        },
        "bool" => DataType::Boolean,
        "text" => DataType::Text,
        "date" => DataType::Date,
        "timestamptz" => DataType::Timestamp {
            precision: None,
            timezone: true,
        },
        "timetz" => DataType::Time {
            precision: None,
            timezone: true,
        },
        "jsonb" => DataType::JsonB,
        "uuid" => DataType::Uuid,
        "varbit" => DataType::VarBit { length: None },
        _ => return None,
    })
}

fn integer(text: &str) -> Option<u32> {
    digits_value(text, DECIMAL_RADIX)
}

impl Parser<'_> {
    /// `Typename`: a simple type, then `[n]...` bounds or `ARRAY [ [n] ]`.
    pub(crate) fn typename(&mut self) -> Parsing<DataType> {
        if self.kw("setof") {
            let start: usize = self.offset();
            self.at += 1;
            self.gap("setof", start);
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
                    sqlquack_syntax::Expression::Literal(literal) => integer(literal.value_str()),
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
                let precision: Option<u32> = self.precision()?;
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
                let modifiers: Vec<Option<u32>> = self.modifiers()?.unwrap_or_default();
                Ok(DataType::Decimal {
                    precision: modifiers.first().copied().flatten(),
                    scale: modifiers.get(1).copied().flatten(),
                })
            }
            Some("boolean") => {
                self.at += 1;
                Ok(DataType::Boolean)
            }
            Some("bit") => {
                self.at += 1;
                let varying: bool = self.eat_kw("varying");
                let length: Option<u32> = self
                    .modifiers()?
                    .and_then(|values| values.first().copied().flatten());
                Ok(match varying {
                    true => DataType::VarBit { length },
                    false => DataType::Bit { length },
                })
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
                if self.is_char('(') {
                    self.precision()?;
                    return Ok(DataType::Interval {
                        unit: None,
                        to: None,
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
            false => DataType::Char { length },
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
            self.precision()?;
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
        if *to == SECOND_FIELD {
            self.precision()?;
        }
        Ok(DataType::Interval {
            unit: Some(unit.to_uppercase()),
            to: Some(to.to_uppercase()),
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
        let plain: bool = parts.len() == 1 && !parts[0].quoted && modifiers.is_none();
        if let Some(known) = plain.then(|| generic(&parts[0].name)).flatten() {
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
