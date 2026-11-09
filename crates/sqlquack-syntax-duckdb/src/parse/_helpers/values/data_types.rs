//! `Type` (common.gram): the named, numeric, time, interval and nested types and array bounds.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::types::models::StructField;

use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, is_col_label, is_identifier, is_type_name,
};
use crate::parse::constants::{
    INTEGER_TYPE, INTERVAL_UNITS, MAP_TYPE_PARAMETERS, REAL_TYPE, WITH_TIME_ZONE,
};

/// `ENUM`, whose string modifiers are its members.
const ENUM_TYPE: &str = "ENUM";

/// The transformer's refusal of an array size that is no constant.
const ARRAY_SIZE_CONSTANT: &str = "Expected a constant number as array size";

/// The transformer's refusal of a negative array size.
const ARRAY_SIZE_POSITIVE: &str = "Array size must be greater than 0";

/// The largest negated integer the grammar folds into a constant, the magnitude of `i64::MIN`.
const NEGATED_INTEGER_LIMIT: u128 = 1 << 63;

/// `GEOMETRY`, whose modifier is its coordinate reference system.
const GEOMETRY_TYPE: &str = "GEOMETRY";

/// Whether a type modifier is a constant: a literal, `NULL`, a boolean or a negated number.
fn is_constant(modifier: &Expression) -> bool {
    match modifier {
        Expression::Literal(_) | Expression::Null(_) | Expression::Boolean(_) => true,
        Expression::Neg(operand) => matches!(&operand.this, Expression::Literal(_)),
        _ => false,
    }
}

/// Whether a type name part needs no quotes: a word of letters, digits, `_` and `$`.
fn is_bare(part: &Identifier) -> bool {
    !part.name.is_empty()
        && !part.name.starts_with(|c: char| c.is_ascii_digit())
        && part
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// One type name part, quoted when it was quoted and needs quotes, quotes doubled.
fn type_name_part(part: &Identifier) -> String {
    match part.quoted && !is_bare(part) {
        true => format!("\"{}\"", part.name.replace('"', "\"\"")),
        false => part.name.clone(),
    }
}

/// Whether a modifier is a string literal.
fn is_string(modifier: &Expression) -> bool {
    string_value(modifier).is_some()
}

/// A string literal modifier's value.
fn string_value(modifier: &Expression) -> Option<String> {
    match modifier {
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::String(value) => Some(value.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// The first modifier, when it is a number that fits.
fn first_length(modifiers: &[Expression]) -> Option<u32> {
    match modifiers.first() {
        Some(Expression::Literal(literal)) => match literal.as_ref() {
            Literal::Number(number) => {
                let Ok(length) = number.replace('_', "").parse::<u32>() else {
                    return None;
                };
                Some(length)
            }
            _ => None,
        },
        _ => None,
    }
}

/// The second modifier, when it is a number that fits.
fn second_length(modifiers: &[Expression]) -> Option<u32> {
    first_length(modifiers.get(1..).unwrap_or_default())
}

/// Whether a word is one of the interval units.
fn is_interval_unit(word: &str) -> bool {
    INTERVAL_UNITS.iter().any(|(name, _)| *name == word)
}

/// The shared-tree type a DuckDB type name stands for, with its modifiers.
fn named_type(name: &str, modifiers: &[Expression]) -> DataType {
    let length: Option<u32> = first_length(modifiers);
    match name {
        "BOOLEAN" | "BOOL" | "LOGICAL" => DataType::Boolean,
        "TINYINT" | "INT1" => DataType::TinyInt { length: None },
        "SMALLINT" | "INT2" | "SHORT" => DataType::SmallInt { length: None },
        "INTEGER" | "INT" | "INT4" | "SIGNED" => DataType::Int {
            length: None,
            integer_spelling: name == INTEGER_TYPE,
        },
        "BIGINT" | "INT8" | "LONG" => DataType::BigInt { length: None },
        "HUGEINT" | "INT128" => DataType::Int128,
        "UTINYINT" => DataType::UInt8,
        "USMALLINT" => DataType::UInt16,
        "UINTEGER" => DataType::UInt32,
        "UBIGINT" => DataType::UInt64,
        "UHUGEINT" => DataType::UInt128,
        "FLOAT" | "FLOAT4" | "REAL" => DataType::Float {
            precision: length,
            scale: None,
            real_spelling: name == REAL_TYPE,
        },
        "DOUBLE" | "FLOAT8" => DataType::Double {
            precision: None,
            scale: None,
        },
        "DECIMAL" | "DEC" | "NUMERIC" => DataType::Decimal {
            precision: length,
            scale: second_length(modifiers),
        },
        "VARCHAR" | "NVARCHAR" => DataType::VarChar {
            length,
            parenthesized_length: length.is_some(),
        },
        "CHAR" | "CHARACTER" | "BPCHAR" | "NCHAR" => DataType::Char { length },
        "TEXT" => DataType::Text,
        "STRING" => DataType::String { length },
        "BLOB" | "BYTEA" | "VARBINARY" => DataType::Blob,
        "BINARY" => DataType::Binary { length },
        "BIT" | "BITSTRING" => DataType::Bit { length },
        "DATE" => DataType::Date,
        "TIMETZ" => DataType::Time {
            precision: length,
            timezone: true,
        },
        "TIMESTAMPTZ" => DataType::Timestamp {
            precision: length,
            timezone: true,
        },
        "DATETIME" => DataType::Timestamp {
            precision: length,
            timezone: false,
        },
        "UUID" => DataType::Uuid,
        "JSON" => DataType::Json,
        _ => DataType::Custom {
            name: name.to_owned(),
        },
    }
}

/// The transformer's refusal of an array size: a negated positive integer, or anything but a constant.
fn array_size_refusal(size: &Expression) -> Option<&'static str> {
    match size {
        Expression::Paren(paren) => array_size_refusal(&paren.this),
        Expression::Literal(_) | Expression::Null(_) | Expression::Boolean(_) => None,
        Expression::Neg(unary) => match unparenthesized(&unary.this) {
            Expression::Literal(literal) => match literal.as_ref() {
                Literal::Number(text) if text.bytes().all(|b| b.is_ascii_digit() || b == b'_') => {
                    match text.replace('_', "").parse::<u128>() {
                        Ok(value) if (1..=NEGATED_INTEGER_LIMIT).contains(&value) => {
                            Some(ARRAY_SIZE_POSITIVE)
                        }
                        Ok(_) | Err(_) => None,
                    }
                }
                Literal::Number(_) => None,
                _ => Some(ARRAY_SIZE_CONSTANT),
            },
            _ => Some(ARRAY_SIZE_CONSTANT),
        },
        _ => Some(ARRAY_SIZE_CONSTANT),
    }
}

/// `expression` with its parentheses removed.
fn unparenthesized(expression: &Expression) -> &Expression {
    match expression {
        Expression::Paren(paren) => unparenthesized(&paren.this),
        other => other,
    }
}

impl Parser<'_> {
    /// `Type`: one of the type variations and its array bounds.
    pub(crate) fn data_type(&mut self) -> Parsing<DataType> {
        let data_type: DataType = self.data_type_unchecked()?;
        self.check_modifiers()?;
        Ok(data_type)
    }

    /// `Type`, with a modifier that is no constant only noted, as a typed literal tries it.
    pub(crate) fn data_type_unchecked(&mut self) -> Parsing<DataType> {
        self.deeper(|parser| {
            let siblings: usize = std::mem::take(&mut parser.type_height);
            let parsed: Parsing<DataType> = parser.type_variation().and_then(|base| {
                parser.type_height += 1;
                parser.array_bounds(base)
            });
            parser.type_height = match parsed {
                Ok(_) => siblings.max(parser.type_height),
                Err(_) => siblings,
            };
            parsed
        })
    }

    /// The transformer's refusal of a type modifier that is no constant, once a type is read.
    pub(crate) fn check_modifiers(&mut self) -> Parsing<()> {
        match self.nonconstant_modifier.take() {
            Some(position) => {
                self.refuse_later("Expected a constant as type modifier", position);
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// `ArrayBounds*`: `[]` and `ARRAY` make a list, `[n]` and `ARRAY[n]` a fixed-size array.
    pub(crate) fn array_bounds(&mut self, mut data_type: DataType) -> Parsing<DataType> {
        loop {
            let keyword: bool = self.eat_kw("ARRAY");
            if !(keyword || self.is_char('[')) {
                return Ok(data_type);
            }
            self.type_height += 1;
            if Self::type_too_tall(self.type_height) {
                return Err(self.nests_too_deeply());
            }
            if !self.is_char('[') {
                data_type = DataType::List {
                    element_type: Box::new(data_type),
                };
                continue;
            }
            self.at += 1;
            let position: usize = self.offset();
            let dimension: Option<Expression> = match self.is_char(']') {
                true => None,
                false => Some(self.expression()?),
            };
            self.expect_char(']')?;
            if let Some(message) = dimension.as_ref().and_then(array_size_refusal) {
                self.refuse_later(message, position);
            }
            data_type = match dimension {
                None => DataType::List {
                    element_type: Box::new(data_type),
                },
                Some(size) => DataType::Array {
                    element_type: Box::new(data_type),
                    dimension: first_length(&[size]),
                },
            };
        }
    }

    /// `TypeModifiers`: `( expression, ... )`, possibly empty; none when no `(` follows.
    fn type_modifiers(&mut self) -> Parsing<Vec<Expression>> {
        if !self.eat_char('(') {
            return Ok(Vec::new());
        }
        let position: usize = self.offset();
        let modifiers: Vec<Expression> = match self.is_char(')') {
            true => Vec::new(),
            false => self.expression_list(')')?,
        };
        self.expect_char(')')?;
        if !modifiers.iter().all(is_constant) && self.nonconstant_modifier.is_none() {
            self.nonconstant_modifier = Some(position);
        }
        Ok(modifiers)
    }

    /// `TypeVariations`, in the grammar's order.
    fn type_variation(&mut self) -> Parsing<DataType> {
        let word: String = self.word_at(0).unwrap_or_default();
        match word.as_str() {
            "TIME" | "TIMESTAMP" => {
                self.at += 1;
                let modifiers: Vec<Expression> = self.type_modifiers()?;
                let timezone: bool = match self.word_at(0).as_deref() {
                    Some(with @ ("WITH" | "WITHOUT"))
                        if self.kw_at(1, "TIME") && self.kw_at(2, "ZONE") =>
                    {
                        let with: bool = with == WITH_TIME_ZONE;
                        self.at += 3;
                        with
                    }
                    _ => false,
                };
                let precision: Option<u32> = first_length(&modifiers);
                return Ok(match word.as_str() {
                    "TIME" => DataType::Time {
                        precision,
                        timezone,
                    },
                    _ => DataType::Timestamp {
                        precision,
                        timezone,
                    },
                });
            }
            "INTERVAL" => {
                self.at += 1;
                if self.is_char('(') {
                    self.type_modifiers()?;
                    return Ok(DataType::Interval {
                        unit: None,
                        to: None,
                    });
                }
                return Ok(self.interval_fields());
            }
            "BIT" => {
                self.at += 1;
                let varying: bool = self.eat_kw("VARYING");
                let length: Option<u32> = first_length(&self.type_modifiers()?);
                return Ok(match varying {
                    true => DataType::VarBit { length },
                    false => DataType::Bit { length },
                });
            }
            "ROW" | "STRUCT" if self.char_at(1, '(') => {
                self.at += 1;
                let fields: Vec<StructField> = self.parenthesized(|parser| {
                    let name: Identifier = parser.name(is_identifier)?;
                    let data_type: DataType = parser.data_type()?;
                    Ok(StructField::new(name.name, data_type))
                })?;
                return Ok(DataType::Struct {
                    fields,
                    nested: true,
                });
            }
            "VARIANT" => {
                self.at += 1;
                return Ok(DataType::Custom { name: word });
            }
            "MAP" if self.char_at(1, '(') => {
                self.at += 1;
                let mut types: Vec<DataType> = self.parenthesized(|parser| parser.data_type())?;
                if types.len() != MAP_TYPE_PARAMETERS {
                    return Err(self.error());
                }
                let value: DataType = types.pop().unwrap_or(DataType::Unknown);
                let key: DataType = types.pop().unwrap_or(DataType::Unknown);
                return Ok(DataType::Map {
                    key_type: Box::new(key),
                    value_type: Box::new(value),
                });
            }
            "TUPLE" if self.char_at(1, '(') => {
                self.at += 1;
                let start: usize = self.at;
                self.parenthesized(|parser| parser.data_type())?;
                return Ok(DataType::Custom {
                    name: format!("{word}{}", self.text_from(start)),
                });
            }
            "GEOMETRY" => return self.geometry_type(),
            "UNION" if self.char_at(1, '(') => {
                self.at += 1;
                let fields: Vec<(String, DataType)> = self.parenthesized(|parser| {
                    let name: Identifier = parser.name(is_identifier)?;
                    Ok((name.name, parser.data_type()?))
                })?;
                return Ok(DataType::Union { fields });
            }
            "DOUBLE" if self.kw_at(1, "PRECISION") => {
                self.at += 2;
                return Ok(named_type("DOUBLE", &[]));
            }
            "FLOAT" if self.char_at(1, '(') => {
                self.at += 1;
                let modifiers: Vec<Expression> = self.type_modifiers()?;
                return Ok(named_type("FLOAT", &modifiers));
            }
            "SETOF" => {
                self.at += 1;
                return self.data_type();
            }
            "CHARACTER" | "CHAR" | "NCHAR" | "NATIONAL" => return self.character_type(),
            "VARCHAR" | "DECIMAL" | "DEC" | "NUMERIC" => {
                self.at += 1;
                let modifiers: Vec<Expression> = self.type_modifiers()?;
                return Ok(named_type(&word, &modifiers));
            }
            "INT" | "INTEGER" | "SMALLINT" | "BIGINT" | "REAL" | "BOOLEAN" | "FLOAT" => {
                self.at += 1;
                return Ok(named_type(&word, &[]));
            }
            "ROW" | "STRUCT" => {
                self.at += 1;
                return Ok(DataType::Custom { name: word });
            }
            "MAP" => {
                self.at += 1;
                return Ok(DataType::Custom { name: word });
            }
            _ => {}
        }
        let mut parts: Vec<Identifier> = vec![self.name(is_type_name)?];
        while self.is_char('.') && self.is_at(1, is_col_label) {
            self.at += 1;
            parts.push(self.name(is_col_label)?);
        }
        let start: usize = self.at;
        let modifiers: Vec<Expression> = self.type_modifiers()?;
        let name: String = match parts.as_slice() {
            [part] if is_bare(part) => part.name.to_ascii_uppercase(),
            _ => parts
                .iter()
                .map(type_name_part)
                .collect::<Vec<String>>()
                .join("."),
        };
        Ok(match named_type(&name, &modifiers) {
            DataType::Custom { .. }
                if name == ENUM_TYPE
                    && !modifiers.is_empty()
                    && modifiers.iter().all(is_string) =>
            {
                DataType::Enum {
                    values: modifiers.iter().filter_map(string_value).collect(),
                    assignments: Vec::new(),
                }
            }
            DataType::Custom { name } => DataType::Custom {
                name: format!("{name}{}", self.text_from(start)),
            },
            known => known,
        })
    }

    /// `GeometryType`: `GEOMETRY`, or `GEOMETRY(crs)` with the modifier written out in its name.
    fn geometry_type(&mut self) -> Parsing<DataType> {
        self.at += 1;
        let start: usize = self.at;
        if !self.eat_char('(') {
            return Ok(DataType::Geometry {
                subtype: None,
                srid: None,
            });
        }
        let position: usize = self.offset();
        let modifier: Expression = self.expression()?;
        self.expect_char(')')?;
        if !is_constant(&modifier) && self.nonconstant_modifier.is_none() {
            self.nonconstant_modifier = Some(position);
        }
        Ok(DataType::Custom {
            name: format!("{GEOMETRY_TYPE}{}", self.text_from(start)),
        })
    }

    /// The source text from token `first` to the last token read, empty when none was read.
    fn text_from(&self, first: usize) -> &str {
        match (
            self.tokens.get(first),
            self.at
                .checked_sub(1)
                .and_then(|last| self.tokens.get(last)),
        ) {
            (Some(from), Some(to)) if self.at > first => &self.sql[from.start..to.end],
            _ => "",
        }
    }

    /// `CharacterType`: `[NATIONAL] CHARACTER|CHAR [VARYING]`, `NCHAR [VARYING]`, with modifiers.
    fn character_type(&mut self) -> Parsing<DataType> {
        self.eat_kw("NATIONAL");
        let Some(word) = self.word_at(0) else {
            return Err(self.error());
        };
        if !matches!(word.as_str(), "CHARACTER" | "CHAR" | "NCHAR") {
            return Err(self.error());
        }
        self.at += 1;
        let varying: bool = self.eat_kw("VARYING");
        let modifiers: Vec<Expression> = self.type_modifiers()?;
        Ok(named_type(
            match varying {
                true => "VARCHAR",
                false => "CHAR",
            },
            &modifiers,
        ))
    }

    /// The fields after `INTERVAL`: a unit, or `unit TO unit`, when written.
    fn interval_fields(&mut self) -> DataType {
        let unit_at = |parser: &Self, offset: usize| {
            parser.word_at(offset).filter(|word| is_interval_unit(word))
        };
        let Some(unit) = unit_at(self, 0) else {
            return DataType::Interval {
                unit: None,
                to: None,
            };
        };
        self.at += 1;
        let to: Option<String> = match self.kw("TO") {
            true => unit_at(self, 1),
            false => None,
        };
        if to.is_some() {
            self.at += 2;
        }
        DataType::Interval {
            unit: Some(unit),
            to,
        }
    }
}
