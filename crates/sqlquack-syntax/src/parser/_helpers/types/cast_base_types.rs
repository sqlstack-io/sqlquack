//! Parser types: the base type names a cast target can start with.

use crate::expressions::types::models::DataType;
use crate::parser::constants::INTEGER;
use crate::parser::constants::NVARCHAR;
use crate::parser::constants::REAL;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    pub(crate) fn parse_cast_other_base_type(
        &mut self,
        name: String,
        raw_name: &str,
    ) -> Result<DataType> {
        Ok(match name.as_str() {
            "TIME" => self.parse_cast_time_type()?,
            "TIMETZ" => self.parse_cast_timetz_type()?,
            "TIMESTAMP" => self.parse_cast_timestamp_type()?,
            "TIMESTAMPTZ" | "TIMESTAMP_TZ" => self.parse_cast_timestamp_tz_type()?,
            "TIMESTAMPLTZ" | "TIMESTAMP_LTZ" => self.parse_cast_timestamp_ltz_type()?,
            "TIMESTAMPNTZ" | "TIMESTAMP_NTZ" => self.parse_cast_timestamp_ntz_type()?,
            "INTERVAL" => self.parse_cast_interval_type()?,
            "VARCHAR" | "NVARCHAR" => self.parse_cast_varchar_type(name)?,
            "VARBINARY" => self.parse_cast_varbinary_type()?,
            "DECIMAL" | "NUMERIC" | "NUMBER" => self.parse_cast_decimal_type()?,
            "INT" | "INTEGER" => {
                if self.is_snowflake() && self.check(TokenType::LParen) {
                    return Err(self.parse_error("Snowflake's INT and INTEGER take no precision"));
                }
                let length = if self.match_token(TokenType::LParen) {
                    let n = Some(self.expect_number()? as u32);
                    self.expect_token(TokenType::RParen)?;
                    n
                } else {
                    None
                };
                DataType::Int {
                    length,
                    integer_spelling: name == INTEGER,
                }
            }
            "BIGINT" => {
                let length = if self.match_token(TokenType::LParen) {
                    let n = Some(self.expect_number()? as u32);
                    self.expect_token(TokenType::RParen)?;
                    n
                } else {
                    None
                };
                DataType::BigInt { length }
            }
            "SMALLINT" => {
                let length = if self.match_token(TokenType::LParen) {
                    let n = Some(self.expect_number()? as u32);
                    self.expect_token(TokenType::RParen)?;
                    n
                } else {
                    None
                };
                DataType::SmallInt { length }
            }
            "TINYINT" => {
                let length = if self.match_token(TokenType::LParen) {
                    let n = Some(self.expect_number()? as u32);
                    self.expect_token(TokenType::RParen)?;
                    n
                } else {
                    None
                };
                DataType::TinyInt { length }
            }
            "FLOAT" | "REAL" | "BINARY_FLOAT" => self.parse_cast_float_type(name)?,
            "BINARY_DOUBLE" => DataType::Double {
                precision: None,
                scale: None,
            },
            "BINARY" => {
                let length = if self.match_token(TokenType::LParen) {
                    let n = Some(self.expect_number()? as u32);
                    self.expect_token(TokenType::RParen)?;
                    n
                } else {
                    None
                };
                DataType::Binary { length }
            }
            "SIGNED" | "UNSIGNED" => {
                if self.check_identifier("INTEGER")
                    || self.check_keyword_text("INTEGER")
                    || self.check_keyword_text("INT")
                {
                    self.skip();
                }
                DataType::Custom { name }
            }
            "NULLABLE" => {
                self.expect_token(TokenType::LParen)?;
                let inner = self.parse_data_type_for_cast()?;
                self.expect_token(TokenType::RParen)?;
                DataType::Nullable {
                    inner: Box::new(inner),
                }
            }
            "VECTOR" => self.parse_cast_vector_type()?,
            "OBJECT" => self.parse_object_data_type()?,
            _ => {
                let base = self.convert_name_to_type(&name)?;
                {
                    if matches!(base, DataType::Custom { .. }) && self.check(TokenType::Dot) {
                        let mut type_name = raw_name.to_string();
                        while self.match_token(TokenType::Dot) {
                            let tok = self.advance()?;
                            type_name = format!("{}.{}", type_name, tok.text);
                        }
                        DataType::Custom { name: type_name }
                    } else if matches!(base, DataType::Custom { .. })
                        && self.config.dialect.is_none()
                    {
                        DataType::Custom {
                            name: raw_name.to_string(),
                        }
                    } else {
                        base
                    }
                }
            }
        })
    }

    pub(crate) fn parse_cast_array_type(&mut self, name: String) -> Result<DataType> {
        Ok({
            if self.match_token(TokenType::Lt) {
                let element_type = self.parse_data_type()?;
                self.expect_gt()?;
                DataType::Array {
                    element_type: Box::new(element_type),
                    dimension: None,
                }
            } else if self.match_token(TokenType::LParen) {
                let element_type = self.parse_data_type_for_cast()?;
                self.expect_token(TokenType::RParen)?;
                DataType::Array {
                    element_type: Box::new(element_type),
                    dimension: None,
                }
            } else {
                DataType::Custom { name }
            }
        })
    }

    pub(crate) fn parse_cast_map_type(&mut self, name: String) -> Result<DataType> {
        Ok({
            if self.match_token(TokenType::Lt) {
                let key_type = self.parse_data_type()?;
                self.expect_token(TokenType::Comma)?;
                let value_type = self.parse_data_type()?;
                self.expect_gt()?;
                DataType::Map {
                    key_type: Box::new(key_type),
                    value_type: Box::new(value_type),
                }
            } else if self.match_token(TokenType::LParen) {
                let key_type = self.parse_data_type_for_cast()?;
                self.expect_token(TokenType::Comma)?;
                let value_type = self.parse_data_type_for_cast()?;
                self.expect_token(TokenType::RParen)?;
                DataType::Map {
                    key_type: Box::new(key_type),
                    value_type: Box::new(value_type),
                }
            } else if self.match_token(TokenType::LBracket) {
                let key_type = self.parse_data_type_for_cast()?;
                self.expect_token(TokenType::FArrow)?;
                let value_type = self.parse_data_type_for_cast()?;
                self.expect_token(TokenType::RBracket)?;
                DataType::Map {
                    key_type: Box::new(key_type),
                    value_type: Box::new(value_type),
                }
            } else {
                DataType::Custom { name }
            }
        })
    }

    pub(crate) fn parse_cast_struct_type(&mut self, name: String) -> Result<DataType> {
        Ok({
            if self.match_token(TokenType::Lt) {
                let fields = self.parse_struct_type_fields(false)?;
                self.expect_gt()?;
                DataType::Struct {
                    fields,
                    nested: false,
                }
            } else if self.match_token(TokenType::LParen) {
                let fields = self.parse_struct_type_fields(true)?;
                self.expect_token(TokenType::RParen)?;
                DataType::Struct {
                    fields,
                    nested: true,
                }
            } else {
                DataType::Custom { name }
            }
        })
    }

    pub(crate) fn parse_cast_row_type(&mut self, name: String) -> Result<DataType> {
        Ok({
            if self.match_token(TokenType::LParen) {
                let fields = self.parse_struct_type_fields(true)?;
                self.expect_token(TokenType::RParen)?;
                DataType::Struct {
                    fields,
                    nested: true,
                }
            } else {
                DataType::Custom { name }
            }
        })
    }

    pub(crate) fn parse_cast_record_type(&mut self, name: String) -> Result<DataType> {
        Ok({
            if self.match_token(TokenType::LParen) {
                let fields = self.parse_struct_type_fields(true)?;
                self.expect_token(TokenType::RParen)?;
                DataType::Struct {
                    fields,
                    nested: true,
                }
            } else {
                DataType::Custom { name }
            }
        })
    }

    pub(crate) fn parse_cast_double_type(&mut self) -> Result<DataType> {
        Ok({
            let _ = self.match_identifier("PRECISION");
            let (precision, scale) = if self.match_token(TokenType::LParen) {
                let p = Some(self.expect_number()? as u32);
                let s = if self.match_token(TokenType::Comma) {
                    Some(self.expect_number()? as u32)
                } else {
                    None
                };
                self.expect_token(TokenType::RParen)?;
                (p, s)
            } else {
                (None, None)
            };
            DataType::Double { precision, scale }
        })
    }

    pub(crate) fn parse_cast_time_type(&mut self) -> Result<DataType> {
        Ok({
            let precision = if self.match_token(TokenType::LParen) {
                let p = Some(self.expect_number()? as u32);
                self.expect_token(TokenType::RParen)?;
                p
            } else {
                None
            };
            let timezone = if self.match_token(TokenType::With) {
                self.match_keyword("TIME");
                self.match_keyword("ZONE");
                true
            } else if self.match_keyword("WITHOUT") {
                self.match_keyword("TIME");
                self.match_keyword("ZONE");
                false
            } else {
                false
            };
            DataType::Time {
                precision,
                timezone,
            }
        })
    }

    pub(crate) fn parse_cast_timetz_type(&mut self) -> Result<DataType> {
        Ok({
            let precision = if self.match_token(TokenType::LParen) {
                let p = Some(self.expect_number()? as u32);
                self.expect_token(TokenType::RParen)?;
                p
            } else {
                None
            };
            DataType::Time {
                precision,
                timezone: true,
            }
        })
    }

    /// Snowflake's VECTOR holds INT or FLOAT elements only.
    pub(crate) fn expect_snowflake_vector_element(&self) -> Result<()> {
        if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
            && !matches!(
                self.peek_text().to_ascii_uppercase().as_str(),
                "INT" | "FLOAT"
            )
        {
            return Err(self.parse_error("Expected INT or FLOAT as the VECTOR element type"));
        }
        Ok(())
    }

    /// `TIME ZONE` after `WITH [LOCAL]` or `WITHOUT`: both words in Snowflake, either elsewhere.
    pub(crate) fn match_time_zone_words(&mut self) -> Result<()> {
        let time = self.match_keyword("TIME");
        let zone = self.match_keyword("ZONE");
        if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
            && !(time && zone)
        {
            return Err(self.parse_error("Expected TIME ZONE"));
        }
        Ok(())
    }

    pub(crate) fn parse_cast_timestamp_type(&mut self) -> Result<DataType> {
        Ok({
            let precision = if self.match_token(TokenType::LParen) {
                let p = Some(self.expect_number()? as u32);
                self.expect_token(TokenType::RParen)?;
                p
            } else {
                None
            };
            if self.match_token(TokenType::With) {
                if self.match_token(TokenType::Local) {
                    self.match_time_zone_words()?;
                    DataType::Custom {
                        name: "TIMESTAMPLTZ".to_string(),
                    }
                } else {
                    self.match_time_zone_words()?;
                    DataType::Timestamp {
                        precision,
                        timezone: true,
                    }
                }
            } else if self.match_keyword("WITHOUT") {
                self.match_time_zone_words()?;
                self.timestamp_without_time_zone(precision)
            } else {
                DataType::Timestamp {
                    precision,
                    timezone: false,
                }
            }
        })
    }

    /// `TIMESTAMP WITHOUT TIME ZONE`: Snowflake's TIMESTAMP_NTZ, whatever TIMESTAMP maps to.
    pub(crate) fn timestamp_without_time_zone(&self, precision: Option<u32>) -> DataType {
        match (self.is_snowflake(), precision) {
            (true, Some(precision)) => DataType::Custom {
                name: format!("TIMESTAMPNTZ({precision})"),
            },
            (true, None) => DataType::Custom {
                name: "TIMESTAMPNTZ".to_owned(),
            },
            (false, _) => DataType::Timestamp {
                precision,
                timezone: false,
            },
        }
    }

    pub(crate) fn parse_cast_timestamp_tz_type(&mut self) -> Result<DataType> {
        Ok({
            let precision = if self.match_token(TokenType::LParen) {
                let p = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Some(p)
            } else {
                None
            };
            DataType::Timestamp {
                precision,
                timezone: true,
            }
        })
    }

    pub(crate) fn parse_cast_timestamp_ltz_type(&mut self) -> Result<DataType> {
        Ok({
            let precision = if self.match_token(TokenType::LParen) {
                let p = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Some(p)
            } else {
                None
            };
            let dt_name = if let Some(p) = precision {
                format!("TIMESTAMPLTZ({})", p)
            } else {
                "TIMESTAMPLTZ".to_string()
            };
            DataType::Custom { name: dt_name }
        })
    }

    pub(crate) fn parse_cast_timestamp_ntz_type(&mut self) -> Result<DataType> {
        Ok({
            let precision = if self.match_token(TokenType::LParen) {
                let p = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Some(p)
            } else {
                None
            };
            let dt_name = if let Some(p) = precision {
                format!("TIMESTAMPNTZ({})", p)
            } else {
                "TIMESTAMPNTZ".to_string()
            };
            DataType::Custom { name: dt_name }
        })
    }

    pub(crate) fn parse_cast_interval_type(&mut self) -> Result<DataType> {
        Ok({
            let unit = if (self.check(TokenType::Identifier)
                || self.check(TokenType::Var)
                || self.check_keyword())
                && !self.check(TokenType::RParen)
                && !self.check(TokenType::Comma)
                && !self.check(TokenType::As)
                && !self.check(TokenType::From)
                && !self.check(TokenType::Not)
                && !self.check(TokenType::Null)
            {
                {
                    let unit = self.advance_text()?.to_ascii_uppercase();
                    Some(self.parse_interval_field_precision(unit)?)
                }
            } else {
                None
            };
            let to = if self.match_token(TokenType::To) {
                if self.check(TokenType::Identifier)
                    || self.check(TokenType::Var)
                    || self.check_keyword()
                {
                    {
                        let unit = self.advance_text()?.to_ascii_uppercase();
                        Some(self.parse_interval_field_precision(unit)?)
                    }
                } else {
                    None
                }
            } else {
                None
            };
            DataType::Interval { unit, to }
        })
    }

    pub(crate) fn parse_cast_varchar_type(&mut self, name: String) -> Result<DataType> {
        Ok({
            let is_nvarchar = name == NVARCHAR;
            if self.match_token(TokenType::LParen) {
                if self.check(TokenType::RParen) {
                    self.skip();
                    if is_nvarchar {
                        DataType::Custom {
                            name: "NVARCHAR".to_string(),
                        }
                    } else {
                        DataType::VarChar {
                            length: None,
                            parenthesized_length: false,
                        }
                    }
                } else if self.check_identifier("MAX") {
                    self.skip();
                    self.expect_token(TokenType::RParen)?;
                    let type_name = if is_nvarchar {
                        "NVARCHAR(MAX)"
                    } else {
                        "VARCHAR(MAX)"
                    };
                    DataType::Custom {
                        name: type_name.to_string(),
                    }
                } else {
                    let n = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    if is_nvarchar {
                        DataType::Custom {
                            name: format!("NVARCHAR({})", n),
                        }
                    } else {
                        DataType::VarChar {
                            length: Some(n),
                            parenthesized_length: false,
                        }
                    }
                }
            } else {
                if is_nvarchar {
                    DataType::Custom {
                        name: "NVARCHAR".to_string(),
                    }
                } else {
                    DataType::VarChar {
                        length: None,
                        parenthesized_length: false,
                    }
                }
            }
        })
    }

    pub(crate) fn parse_cast_varbinary_type(&mut self) -> Result<DataType> {
        Ok({
            if self.match_token(TokenType::LParen) {
                if self.check(TokenType::RParen) {
                    self.skip();
                    DataType::VarBinary { length: None }
                } else if self.check_identifier("MAX") {
                    self.skip();
                    self.expect_token(TokenType::RParen)?;
                    DataType::Custom {
                        name: "VARBINARY(MAX)".to_string(),
                    }
                } else {
                    let n = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    DataType::VarBinary { length: Some(n) }
                }
            } else {
                DataType::VarBinary { length: None }
            }
        })
    }

    pub(crate) fn parse_cast_decimal_type(&mut self) -> Result<DataType> {
        Ok({
            if self.match_token(TokenType::LParen) {
                let precision = Some(self.expect_number()? as u32);
                let scale = if self.match_token(TokenType::Comma) {
                    Some(self.expect_number()? as u32)
                } else {
                    None
                };
                self.expect_token(TokenType::RParen)?;
                DataType::Decimal { precision, scale }
            } else {
                DataType::Decimal {
                    precision: None,
                    scale: None,
                }
            }
        })
    }

    pub(crate) fn parse_cast_float_type(&mut self, name: String) -> Result<DataType> {
        Ok({
            let (precision, scale) = if self.match_token(TokenType::LParen) {
                let n = Some(self.expect_number()? as u32);
                let s = if self.match_token(TokenType::Comma) {
                    Some(self.expect_number()? as u32)
                } else {
                    None
                };
                self.expect_token(TokenType::RParen)?;
                (n, s)
            } else {
                (None, None)
            };
            DataType::Float {
                precision,
                scale,
                real_spelling: name == REAL,
            }
        })
    }

    pub(crate) fn parse_cast_vector_type(&mut self) -> Result<DataType> {
        Ok({
            if self.match_token(TokenType::LParen) {
                if self.check(TokenType::Number) {
                    let dimension = self.expect_number()? as u32;
                    let element_type = if self.match_token(TokenType::Comma) {
                        let type_alias = self.expect_identifier_or_keyword()?;
                        let mapped_type = match type_alias.to_ascii_uppercase().as_str() {
                            "I8" => DataType::TinyInt { length: None },
                            "I16" => DataType::SmallInt { length: None },
                            "I32" => DataType::Int {
                                length: None,
                                integer_spelling: false,
                            },
                            "I64" => DataType::BigInt { length: None },
                            "F32" => DataType::Float {
                                precision: None,
                                scale: None,
                                real_spelling: false,
                            },
                            "F64" => DataType::Double {
                                precision: None,
                                scale: None,
                            },
                            _ => DataType::Custom {
                                name: type_alias.to_string(),
                            },
                        };
                        Some(Box::new(mapped_type))
                    } else {
                        None
                    };
                    self.expect_token(TokenType::RParen)?;
                    DataType::Vector {
                        element_type,
                        dimension: Some(dimension),
                    }
                } else {
                    self.expect_snowflake_vector_element()?;
                    let element_type = self.parse_data_type()?;
                    self.expect_token(TokenType::Comma)?;
                    let dimension = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    DataType::Vector {
                        element_type: Some(Box::new(element_type)),
                        dimension: Some(dimension),
                    }
                }
            } else {
                DataType::Custom {
                    name: "VECTOR".to_string(),
                }
            }
        })
    }
}
