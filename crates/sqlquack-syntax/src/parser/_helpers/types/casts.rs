//! Parser types: casts.

use crate::expressions::names::models::Identifier;
use crate::expressions::operators::models::Case;
use crate::expressions::operators::models::Cast;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::parser::_helpers::types::data_types::convert_name_is_known_custom;
use crate::parser::constants::CHAR;
use crate::parser::constants::CHARACTER;
use crate::parser::constants::INTEGER;
use crate::parser::constants::NATIONAL;
use crate::parser::constants::NCHAR;
use crate::parser::constants::NVARCHAR;
use crate::parser::constants::REAL;
use crate::parser::models::Parser;
use crate::parser::models::ParserDepthScope;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Check (without consuming) if the next tokens match a sequence of keywords by text (case-insensitive)
    pub(crate) fn check_text_seq(&self, keywords: &[&str]) -> bool {
        for (i, &kw) in keywords.iter().enumerate() {
            if self.current + i >= self.tokens.len() {
                return false;
            }
            let token = &self.tokens[self.current + i];
            if Self::text_match_excludes_token(token.token_type)
                || !token.text.eq_ignore_ascii_case(kw)
            {
                return false;
            }
        }
        true
    }

    /// Match any of the given texts (case-insensitive)
    pub(crate) fn match_texts(&mut self, texts: &[&str]) -> bool {
        if self.is_at_end() {
            return false;
        }
        if !self.current_token_allows_text_match() {
            return false;
        }
        for text in texts {
            if self.peek_text().eq_ignore_ascii_case(text) {
                self.skip();
                return true;
            }
        }
        false
    }

    /// Parse CASE expression
    pub(crate) fn parse_case(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Case)?;
        let case_comments = self.previous_trailing_comments().to_vec();

        let operand = if !self.check(TokenType::When) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let mut whens: Vec<(Expression, Expression)> = Vec::new();
        while self.match_token(TokenType::When) {
            let condition = self.parse_expression()?;
            self.expect_token(TokenType::Then)?;
            let result = self.parse_expression()?;

            whens.push((condition, result));
        }

        let else_ = if self.match_token(TokenType::Else) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_token(TokenType::End)?;

        Ok(Expression::Case(Box::new(Case {
            operand,
            whens,
            else_,
            comments: case_comments,
            inferred_type: None,
            span: SourceSpan::default(),
        })))
    }

    /// Parses a cast operand without its AS/type separator, as a nested expression context.
    pub(crate) fn parse_cast_operand(&mut self) -> Result<Expression> {
        let outer_path = std::mem::take(&mut self.parsing_duckdb_json_path);
        let result = self.with_recursive_stack(|parser| parser.parse_duckdb_json_arrow());
        self.parsing_duckdb_json_path = outer_path;
        result
    }

    /// Parse CAST expression
    pub(crate) fn parse_cast(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Cast)?;
        self.expect_token(TokenType::LParen)?;
        let expr = self.parse_cast_operand()?;

        let expr = { expr };

        let expr = self.try_clickhouse_implicit_alias(expr);

        self.expect_token(TokenType::As)?;

        let expr = {
            {
                expr
            }
        };

        let data_type = self.parse_data_type()?;

        let default = if self.match_token(TokenType::Default) {
            let default_val = self.parse_primary()?;
            if !self.match_text_seq(&["ON", "CONVERSION", "ERROR"]) {
                return Err(self.parse_error("Expected ON CONVERSION ERROR"));
            }
            Some(Box::new(default_val))
        } else {
            None
        };

        let format = if self.match_token(TokenType::Format) {
            let wrapped = self.match_token(TokenType::LParen);
            let fmt_expr = self.parse_primary()?;
            if wrapped {
                self.expect_token(TokenType::RParen)?;
            }
            let fmt_with_tz = if self.match_text_seq(&["AT", "TIME", "ZONE"]) {
                let zone = self.parse_primary()?;
                Expression::AtTimeZone(Box::new(
                    crate::expressions::typed_functions::models::AtTimeZone {
                        this: fmt_expr,
                        zone,
                    },
                ))
            } else {
                fmt_expr
            };
            Some(Box::new(fmt_with_tz))
        } else if self.match_token(TokenType::Comma) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.expect_token(TokenType::RParen)?;
        let trailing_comments = self.previous_trailing_comments().to_vec();

        Ok(Expression::Cast(Box::new(Cast {
            this: expr,
            to: data_type,
            trailing_comments,
            double_colon_syntax: false,
            format,
            default,
            inferred_type: None,
            span: SourceSpan::default(),
        })))
    }

    /// Parse TRY_CAST expression
    pub(crate) fn parse_try_cast(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::TryCast)?;
        self.expect_token(TokenType::LParen)?;
        let expr = self.parse_cast_operand()?;
        self.expect_token(TokenType::As)?;
        let data_type = self.parse_data_type()?;

        let format = if self.match_token(TokenType::Format) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.expect_token(TokenType::RParen)?;
        let trailing_comments = self.previous_trailing_comments().to_vec();

        Ok(Expression::TryCast(Box::new(Cast {
            this: expr,
            to: data_type,
            trailing_comments,
            double_colon_syntax: false,
            format,
            default: None,
            inferred_type: None,
            span: SourceSpan::default(),
        })))
    }

    /// Parse SAFE_CAST expression (BigQuery)
    pub(crate) fn parse_safe_cast(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::SafeCast)?;
        self.expect_token(TokenType::LParen)?;
        let expr = self.parse_cast_operand()?;
        self.expect_token(TokenType::As)?;
        let data_type = self.parse_data_type()?;

        let format = if self.match_token(TokenType::Format) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.expect_token(TokenType::RParen)?;
        let trailing_comments = self.previous_trailing_comments().to_vec();

        Ok(Expression::SafeCast(Box::new(Cast {
            this: expr,
            to: data_type,
            trailing_comments,
            double_colon_syntax: false,
            format,
            default: None,
            inferred_type: None,
            span: SourceSpan::default(),
        })))
    }

    /// Recognize equivalent signed 128-bit type names across parser entry points.
    pub(crate) fn is_int128_type_name(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case("INT128") || name.eq_ignore_ascii_case("HUGEINT")
    }

    /// Vertica interval qualifiers may carry a seconds precision: `SECOND(3)`.
    pub(crate) fn parse_interval_field_precision(&mut self, unit: String) -> Result<String> {
        Ok(unit)
    }

    /// Parse a data type.
    pub(crate) fn parse_data_type(&mut self) -> Result<DataType> {
        let data_type = self.with_parser_depth(|parser| parser.parse_data_type_inner())?;
        Ok(self.normalize_dialect_data_type(data_type))
    }

    /// Applies dialect type aliasing after parsing a type, such as Vertica's 64-bit integers.
    pub(crate) fn normalize_dialect_data_type(&self, data_type: DataType) -> DataType {
        data_type
    }

    #[inline(never)]
    pub(crate) fn parse_data_type_inner(&mut self) -> Result<DataType> {
        if let Some(data_type) = self.parse_vertica_data_type()? {
            return Ok(data_type);
        }
        if self.check(TokenType::Geometry) {
            let _token = self.advance()?;
            let (subtype, srid) = self.parse_spatial_type_args()?;
            return Ok(DataType::Geometry { subtype, srid });
        }
        let mut raw_name = self.expect_identifier_or_keyword()?;
        while self.match_token(TokenType::Dot) {
            let part = self.expect_identifier_or_keyword()?;
            raw_name.push('.');
            raw_name.push_str(&part);
        }
        let mut name = raw_name.to_ascii_uppercase();

        if name == NATIONAL {
            let next_upper = if !self.is_at_end() {
                self.peek_text().to_ascii_uppercase()
            } else {
                String::new()
            };
            if next_upper == CHAR || next_upper == CHARACTER {
                self.skip();
                name = "NCHAR".to_string();
                if next_upper == CHARACTER && self.check_identifier("VARYING") {
                    self.skip();
                    let length = if self.match_token(TokenType::LParen) {
                        if self.check(TokenType::RParen) {
                            self.skip();
                            None
                        } else {
                            let n = self.expect_number()? as u32;
                            self.expect_token(TokenType::RParen)?;
                            Some(n)
                        }
                    } else {
                        None
                    };
                    return Ok(DataType::VarChar {
                        length,
                        parenthesized_length: false,
                    });
                }
            }
        }

        let base_type = match name.as_str() {
            "ARRAY" => self.parse_array_data_type(),
            "MAP" => self.parse_map_data_type(),
            "STRUCT" => self.parse_struct_data_type(),
            "ROW" => self.parse_row_data_type(),
            "RECORD" => self.parse_record_data_type(),
            _ => self.parse_scalar_data_type(name, raw_name),
        }?;

        self.finish_data_type(base_type)
    }

    #[inline(never)]
    pub(crate) fn finish_data_type(&mut self, base_type: DataType) -> Result<DataType> {
        let mut result_type = base_type;

        result_type = self.maybe_parse_collated_data_type(result_type)?;

        let result_type = self.maybe_parse_array_dimensions(result_type)?;

        Ok(result_type)
    }

    #[inline(never)]
    pub(crate) fn parse_scalar_data_type(
        &mut self,
        name: String,
        raw_name: String,
    ) -> Result<DataType> {
        match name.as_str() {
            name if DataType::from_unsigned_name(name).is_some() => {
                Ok(DataType::from_unsigned_name(name).ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?)
            }
            name if self.is_int128_type_name(name) => Ok(DataType::Int128),
            "INT" | "INTEGER" => self.parse_int_data_type(name),
            "BIGINT" => self.parse_bigint_data_type(),
            "SMALLINT" => self.parse_smallint_data_type(),
            "TINYINT" => self.parse_tinyint_data_type(),
            "FLOAT" | "REAL" => self.parse_float_data_type(name),
            "BINARY_FLOAT" => Ok(DataType::Float {
                precision: None,
                scale: None,
                real_spelling: false,
            }),
            "BINARY_DOUBLE" => Ok(DataType::Double {
                precision: None,
                scale: None,
            }),
            "DOUBLE" => self.parse_double_data_type(),
            "DECIMAL" | "NUMERIC" => self.parse_decimal_data_type(),
            "BOOLEAN" | "BOOL" => Ok(DataType::Boolean),
            "CHAR" | "CHARACTER" | "NCHAR" => self.parse_char_data_type(name),
            "VARCHAR" | "NVARCHAR" => self.parse_varchar_data_type(name),
            "TEXT" | "NTEXT" => {
                if self.match_token(TokenType::LParen) {
                    let n = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    Ok(DataType::TextWithLength { length: n })
                } else {
                    Ok(DataType::Text)
                }
            }
            "STRING" => {
                let length = if self.match_token(TokenType::LParen) {
                    let n = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    Some(n)
                } else {
                    None
                };
                Ok(DataType::String { length })
            }
            "DATE" => Ok(DataType::Date),
            "TIME" => self.parse_time_data_type(),
            "TIMETZ" => {
                let precision = if self.match_token(TokenType::LParen) {
                    let p = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    Some(p)
                } else {
                    None
                };
                Ok(DataType::Time {
                    precision,
                    timezone: true,
                })
            }
            "TIMESTAMP" => self.parse_timestamp_data_type(),
            "TIMESTAMPTZ" | "TIMESTAMP_TZ" => {
                let precision = if self.match_token(TokenType::LParen) {
                    let p = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    Some(p)
                } else {
                    None
                };
                Ok(DataType::Timestamp {
                    precision,
                    timezone: true,
                })
            }
            "TIMESTAMPLTZ" | "TIMESTAMP_LTZ" => self.parse_timestampltz_data_type(),
            "TIMESTAMPNTZ" | "TIMESTAMP_NTZ" => self.parse_timestampntz_data_type(),
            "INTERVAL" => self.parse_interval_data_type(),
            _ => self.parse_other_scalar_data_type(name, raw_name),
        }
    }

    fn parse_other_scalar_data_type(&mut self, name: String, raw_name: String) -> Result<DataType> {
        match name.as_str() {
            "JSON" => Ok(DataType::Json),
            "JSONB" => Ok(DataType::JsonB),
            "UUID" => Ok(DataType::Uuid),
            "BLOB" => Ok(DataType::Blob),
            "BYTEA" => Ok(DataType::VarBinary { length: None }),
            "BIT" => {
                let length = if self.match_token(TokenType::LParen) {
                    let n = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    Some(n)
                } else {
                    None
                };
                Ok(DataType::Bit { length })
            }
            "VARBIT" | "BIT VARYING" => {
                let length = if self.match_token(TokenType::LParen) {
                    let n = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    Some(n)
                } else {
                    None
                };
                Ok(DataType::VarBit { length })
            }
            "BINARY" => self.parse_binary_data_type(),
            "VARBINARY" => self.parse_varbinary_data_type(),

            "VECTOR" => self.parse_vector_data_type(),
            "OBJECT" => self.parse_object_data_type(),

            "ENUM" => self.parse_enum_data_type(),
            "SET" => self.parse_set_data_type(),
            "UNION" if self.check(TokenType::LParen) => {
                self.skip();
                let struct_fields = self.parse_struct_type_fields(true)?;
                self.expect_token(TokenType::RParen)?;
                let fields: Vec<(String, DataType)> = struct_fields
                    .into_iter()
                    .map(|f| (f.name, f.data_type))
                    .collect();
                Ok(DataType::Union { fields })
            }
            "GEOMETRY" => {
                let (subtype, srid) = self.parse_spatial_type_args()?;
                Ok(DataType::Geometry { subtype, srid })
            }
            "GEOGRAPHY" => {
                let (subtype, srid) = self.parse_spatial_type_args()?;
                Ok(DataType::Geography { subtype, srid })
            }
            "POINT" | "LINESTRING" | "POLYGON" | "MULTIPOINT" | "MULTILINESTRING"
            | "MULTIPOLYGON" | "GEOMETRYCOLLECTION" => {
                let srid = if self.match_identifier("SRID") {
                    Some(self.expect_number()? as u32)
                } else {
                    None
                };
                Ok(DataType::Geometry {
                    subtype: Some(name),
                    srid,
                })
            }
            "ANY" => {
                if self.match_token(TokenType::Type) {
                    Ok(DataType::Custom {
                        name: "ANY TYPE".to_string(),
                    })
                } else {
                    Ok(DataType::Custom {
                        name: "ANY".to_string(),
                    })
                }
            }
            "LONG" => self.parse_long_data_type(),
            "SIGNED" | "UNSIGNED" => {
                if self.check_identifier("INTEGER")
                    || self.check_keyword_text("INTEGER")
                    || self.check_keyword_text("INT")
                {
                    self.skip();
                }
                Ok(DataType::Custom { name })
            }
            "NULLABLE" => {
                self.expect_token(TokenType::LParen)?;
                let inner = self.parse_data_type()?;
                self.expect_token(TokenType::RParen)?;
                Ok(DataType::Nullable {
                    inner: Box::new(inner),
                })
            }
            _ => self.parse_named_data_type(name, raw_name),
        }
    }

    fn parse_int_data_type(&mut self, name: String) -> Result<DataType> {
        let length = if self.match_token(TokenType::LParen) {
            if self.check(TokenType::RParen) {
                self.skip();
                None
            } else {
                let n = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Some(n)
            }
        } else {
            None
        };
        let integer_spelling = name == INTEGER;
        Ok(DataType::Int {
            length,
            integer_spelling,
        })
    }

    fn parse_bigint_data_type(&mut self) -> Result<DataType> {
        let length = if self.match_token(TokenType::LParen) {
            if self.check(TokenType::RParen) {
                self.skip();
                None
            } else {
                let n = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Some(n)
            }
        } else {
            None
        };
        Ok(DataType::BigInt { length })
    }

    fn parse_smallint_data_type(&mut self) -> Result<DataType> {
        let length = if self.match_token(TokenType::LParen) {
            if self.check(TokenType::RParen) {
                self.skip();
                None
            } else {
                let n = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Some(n)
            }
        } else {
            None
        };
        Ok(DataType::SmallInt { length })
    }

    fn parse_tinyint_data_type(&mut self) -> Result<DataType> {
        let length = if self.match_token(TokenType::LParen) {
            if self.check(TokenType::RParen) {
                self.skip();
                None
            } else {
                let n = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Some(n)
            }
        } else {
            None
        };
        Ok(DataType::TinyInt { length })
    }

    fn parse_float_data_type(&mut self, name: String) -> Result<DataType> {
        let real_spelling = name == REAL;
        let (precision, scale) = if self.match_token(TokenType::LParen) {
            let p = self.expect_number()? as u32;
            let s = if self.match_token(TokenType::Comma) {
                Some(self.expect_number()? as u32)
            } else {
                None
            };
            self.expect_token(TokenType::RParen)?;
            (Some(p), s)
        } else {
            (None, None)
        };
        Ok(DataType::Float {
            precision,
            scale,
            real_spelling,
        })
    }

    fn parse_double_data_type(&mut self) -> Result<DataType> {
        let _ = self.match_identifier("PRECISION");
        let (precision, scale) = if self.match_token(TokenType::LParen) {
            let p = self.expect_number()? as u32;
            let s = if self.match_token(TokenType::Comma) {
                Some(self.expect_number()? as u32)
            } else {
                None
            };
            self.expect_token(TokenType::RParen)?;
            (Some(p), s)
        } else {
            (None, None)
        };
        Ok(DataType::Double { precision, scale })
    }

    fn parse_decimal_data_type(&mut self) -> Result<DataType> {
        let (precision, scale) = if self.match_token(TokenType::LParen) {
            let p = self.expect_number()? as u32;
            let s = if self.match_token(TokenType::Comma) {
                Some(self.expect_number()? as u32)
            } else {
                None
            };
            self.expect_token(TokenType::RParen)?;
            (Some(p), s)
        } else {
            (None, None)
        };
        Ok(DataType::Decimal { precision, scale })
    }

    fn parse_char_data_type(&mut self, name: String) -> Result<DataType> {
        let is_nchar = name == NCHAR;
        if self.match_identifier("LARGE") && self.match_identifier("OBJECT") {
            return Ok(DataType::Text);
        }
        if self.match_identifier("VARYING") {
            let length = if self.match_token(TokenType::LParen) {
                if self.check(TokenType::RParen) {
                    self.skip();
                    None
                } else {
                    let n = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    Some(n)
                }
            } else {
                None
            };
            Ok(DataType::VarChar {
                length,
                parenthesized_length: false,
            })
        } else {
            let length = if self.match_token(TokenType::LParen) {
                if self.check(TokenType::RParen) {
                    self.skip();
                    None
                } else {
                    let n = self.expect_number()? as u32;
                    self.expect_token(TokenType::RParen)?;
                    Some(n)
                }
            } else {
                None
            };
            if length.is_none()
                && self.match_identifier("CHARACTER")
                && self.match_token(TokenType::Set)
            {
                let charset = self.expect_identifier_or_keyword()?;
                return Ok(DataType::CharacterSet { name: charset });
            }
            if is_nchar {
                let name = if let Some(len) = length {
                    format!("NCHAR({})", len)
                } else {
                    "NCHAR".to_string()
                };
                return Ok(DataType::Custom { name });
            }
            Ok(DataType::Char { length })
        }
    }

    fn parse_varchar_data_type(&mut self, name: String) -> Result<DataType> {
        let is_nvarchar = name == NVARCHAR;
        if self.match_token(TokenType::LParen) {
            if self.check(TokenType::RParen) {
                self.skip();
                if is_nvarchar {
                    return Ok(DataType::Custom {
                        name: "NVARCHAR".to_string(),
                    });
                }
                Ok(DataType::VarChar {
                    length: None,
                    parenthesized_length: false,
                })
            } else if self.check_identifier("MAX") {
                self.skip();
                self.expect_token(TokenType::RParen)?;
                let type_name = if is_nvarchar {
                    "NVARCHAR(MAX)"
                } else {
                    "VARCHAR(MAX)"
                };
                Ok(DataType::Custom {
                    name: type_name.to_string(),
                })
            } else {
                let parenthesized_length = self.match_token(TokenType::LParen);
                let n = self.expect_number()? as u32;
                if parenthesized_length {
                    self.expect_token(TokenType::RParen)?;
                }
                self.expect_token(TokenType::RParen)?;
                if is_nvarchar {
                    return Ok(DataType::Custom {
                        name: format!("NVARCHAR({})", n),
                    });
                }
                Ok(DataType::VarChar {
                    length: Some(n),
                    parenthesized_length,
                })
            }
        } else {
            if is_nvarchar {
                return Ok(DataType::Custom {
                    name: "NVARCHAR".to_string(),
                });
            }
            Ok(DataType::VarChar {
                length: None,
                parenthesized_length: false,
            })
        }
    }

    fn parse_time_data_type(&mut self) -> Result<DataType> {
        let precision = if self.match_token(TokenType::LParen) {
            let p = self.expect_number()? as u32;
            self.expect_token(TokenType::RParen)?;
            Some(p)
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
        Ok(DataType::Time {
            precision,
            timezone,
        })
    }

    fn parse_timestamp_data_type(&mut self) -> Result<DataType> {
        let precision = if self.match_token(TokenType::LParen) {
            let p = self.expect_number()? as u32;
            self.expect_token(TokenType::RParen)?;
            Some(p)
        } else {
            None
        };
        if self.match_token(TokenType::With) {
            if self.match_token(TokenType::Local) {
                self.match_keyword("TIME");
                self.match_keyword("ZONE");
                Ok(DataType::Custom {
                    name: "TIMESTAMPLTZ".to_string(),
                })
            } else {
                self.match_keyword("TIME");
                self.match_keyword("ZONE");
                Ok(DataType::Timestamp {
                    precision,
                    timezone: true,
                })
            }
        } else if self.match_keyword("WITHOUT") {
            self.match_keyword("TIME");
            self.match_keyword("ZONE");
            Ok(DataType::Timestamp {
                precision,
                timezone: false,
            })
        } else {
            Ok(DataType::Timestamp {
                precision,
                timezone: false,
            })
        }
    }

    fn parse_timestampltz_data_type(&mut self) -> Result<DataType> {
        let precision = if self.match_token(TokenType::LParen) {
            let p = self.expect_number()? as u32;
            self.expect_token(TokenType::RParen)?;
            Some(p)
        } else {
            None
        };
        let name = if let Some(p) = precision {
            format!("TIMESTAMPLTZ({})", p)
        } else {
            "TIMESTAMPLTZ".to_string()
        };
        Ok(DataType::Custom { name })
    }

    fn parse_timestampntz_data_type(&mut self) -> Result<DataType> {
        let precision = if self.match_token(TokenType::LParen) {
            let p = self.expect_number()? as u32;
            self.expect_token(TokenType::RParen)?;
            Some(p)
        } else {
            None
        };
        let name = if let Some(p) = precision {
            format!("TIMESTAMPNTZ({})", p)
        } else {
            "TIMESTAMPNTZ".to_string()
        };
        Ok(DataType::Custom { name })
    }

    fn parse_interval_data_type(&mut self) -> Result<DataType> {
        let unit = if (self.check(TokenType::Identifier)
            || self.check(TokenType::Var)
            || self.check_keyword())
            && !self.check(TokenType::Generated)
            && !self.check(TokenType::As)
            && !self.check(TokenType::From)
            && !self.check(TokenType::Not)
            && !self.check(TokenType::Null)
            && !self.check(TokenType::Default)
            && !self.check(TokenType::PrimaryKey)
            && !self.check(TokenType::Unique)
            && !self.check(TokenType::Check)
            && !self.check(TokenType::Constraint)
            && !self.check(TokenType::References)
            && !self.check(TokenType::Collate)
            && !self.check(TokenType::Comment)
            && !self.check(TokenType::RParen)
            && !self.check(TokenType::Comma)
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
        Ok(DataType::Interval { unit, to })
    }

    fn parse_binary_data_type(&mut self) -> Result<DataType> {
        if self.match_identifier("LARGE") && self.match_identifier("OBJECT") {
            return Ok(DataType::Blob);
        }
        if self.match_identifier("VARYING") {
            let length = if self.match_token(TokenType::LParen) {
                let len = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Some(len)
            } else {
                None
            };
            Ok(DataType::VarBinary { length })
        } else {
            let length = if self.match_token(TokenType::LParen) {
                let len = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Some(len)
            } else {
                None
            };
            Ok(DataType::Binary { length })
        }
    }

    fn parse_varbinary_data_type(&mut self) -> Result<DataType> {
        if self.match_token(TokenType::LParen) {
            if self.check_identifier("MAX") {
                self.skip();
                self.expect_token(TokenType::RParen)?;
                Ok(DataType::Custom {
                    name: "VARBINARY(MAX)".to_string(),
                })
            } else {
                let len = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Ok(DataType::VarBinary { length: Some(len) })
            }
        } else {
            Ok(DataType::VarBinary { length: None })
        }
    }

    fn parse_vector_data_type(&mut self) -> Result<DataType> {
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
                Ok(DataType::Vector {
                    element_type,
                    dimension: Some(dimension),
                })
            } else {
                let element_type = self.parse_data_type()?;
                self.expect_token(TokenType::Comma)?;
                let dimension = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Ok(DataType::Vector {
                    element_type: Some(Box::new(element_type)),
                    dimension: Some(dimension),
                })
            }
        } else {
            Ok(DataType::Custom {
                name: "VECTOR".to_string(),
            })
        }
    }

    /// Snowflake `OBJECT(<name> <type> [NOT NULL], ...)`, or `OBJECT` alone as a named type.
    pub(crate) fn parse_object_data_type(&mut self) -> Result<DataType> {
        if self.match_token(TokenType::LParen) {
            let mut fields: Vec<(String, DataType, bool)> = Vec::new();
            if !self.check(TokenType::RParen) {
                loop {
                    let quoted = self.check(TokenType::QuotedIdentifier);
                    let name = self.expect_identifier_or_keyword()?;
                    let field_name = if quoted {
                        Identifier::quoted(name).to_type_field_name()
                    } else {
                        name
                    };
                    let field_type = self.parse_data_type()?;
                    let not_null = if self.match_keyword("NOT") {
                        self.match_keyword("NULL");
                        true
                    } else {
                        false
                    };
                    fields.push((field_name, field_type, not_null));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
            let modifier = if self.match_keyword("RENAME") {
                if self.match_keyword("FIELDS") {
                    Some("RENAME FIELDS".to_string())
                } else {
                    Some("RENAME".to_string())
                }
            } else if self.match_keyword("ADD") {
                if self.match_keyword("FIELDS") {
                    Some("ADD FIELDS".to_string())
                } else {
                    Some("ADD".to_string())
                }
            } else {
                None
            };
            Ok(DataType::Object { fields, modifier })
        } else {
            Ok(DataType::Custom {
                name: "OBJECT".to_string(),
            })
        }
    }

    fn parse_enum_data_type(&mut self) -> Result<DataType> {
        if self.match_token(TokenType::LParen) {
            let mut values: Vec<String> = Vec::new();
            let mut assignments: Vec<Option<String>> = Vec::new();
            if !self.check(TokenType::RParen) {
                loop {
                    let val = { self.expect_string()? };
                    values.push(val);
                    if self.match_token(TokenType::Eq) {
                        let negative = self.match_token(TokenType::Dash);
                        let num_token = self.advance()?;
                        let val = if negative {
                            format!("-{}", num_token.text)
                        } else {
                            num_token.text.to_string()
                        };
                        assignments.push(Some(val));
                    } else {
                        assignments.push(None);
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
            Ok(DataType::Enum {
                values,
                assignments,
            })
        } else {
            Ok(DataType::Custom {
                name: "ENUM".to_string(),
            })
        }
    }

    fn parse_set_data_type(&mut self) -> Result<DataType> {
        if self.match_token(TokenType::LParen) {
            let mut values: Vec<String> = Vec::new();
            if !self.check(TokenType::RParen) {
                loop {
                    let val = self.expect_string()?;
                    values.push(val);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
            Ok(DataType::Set { values })
        } else {
            Ok(DataType::Custom {
                name: "SET".to_string(),
            })
        }
    }

    fn parse_long_data_type(&mut self) -> Result<DataType> {
        if self.match_identifier("VARCHAR") {
            if self.match_token(TokenType::LParen) {
                let length = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                Ok(DataType::TextWithLength { length })
            } else {
                Ok(DataType::Text)
            }
        } else if self.match_identifier("VARBINARY") {
            if self.match_token(TokenType::LParen) {
                let length = self.expect_number()?;
                self.expect_token(TokenType::RParen)?;
                Ok(DataType::Custom {
                    name: format!("LONG VARBINARY({})", length),
                })
            } else {
                Ok(DataType::Blob)
            }
        } else {
            Ok(DataType::Custom {
                name: "LONG".to_string(),
            })
        }
    }

    fn parse_named_data_type(&mut self, name: String, raw_name: String) -> Result<DataType> {
        let is_known = convert_name_is_known_custom(&name);
        let custom_name = if is_known {
            name.clone()
        } else {
            raw_name.clone()
        };
        if self.match_token(TokenType::LParen) {
            {
                let mut args: Vec<String> = Vec::new();
                let mut after_comma = true;
                loop {
                    if self.check(TokenType::RParen) {
                        if after_comma && !(args.is_empty() && name == "TABLE") {
                            return Err(self.parse_error("Expected a type parameter"));
                        }
                        break;
                    }
                    let token = self.advance()?;
                    if !after_comma && !args.is_empty() {
                        if let Some(last) = args.last_mut() {
                            *last = format!("{} {}", last, token.text);
                        }
                    } else {
                        args.push(token.text.to_string());
                    }
                    after_comma = self.match_token(TokenType::Comma);
                }
                self.expect_token(TokenType::RParen)?;
                Ok(DataType::Custom {
                    name: format!("{}({})", custom_name, args.join(", ")),
                })
            }
        } else {
            Ok(DataType::Custom { name: custom_name })
        }
    }

    #[inline(never)]
    pub(crate) fn parse_array_data_type(&mut self) -> Result<DataType> {
        let close = if self.match_token(TokenType::Lt) {
            TokenType::Gt
        } else if self.match_token(TokenType::LParen) {
            TokenType::RParen
        } else {
            return Ok(DataType::Custom {
                name: "ARRAY".to_string(),
            });
        };
        let mut closes = vec![close];
        let mut scopes: Vec<ParserDepthScope> = Vec::new();
        while !self.is_at_end()
            && !self.check(TokenType::QuotedIdentifier)
            && self.peek_text().eq_ignore_ascii_case("ARRAY")
            && (self.check_next(TokenType::Lt) || self.check_next(TokenType::LParen))
        {
            scopes.push(self.enter_parser_depth(1)?);
            self.skip();
            closes.push(if self.match_token(TokenType::Lt) {
                TokenType::Gt
            } else {
                self.expect_token(TokenType::LParen)?;
                TokenType::RParen
            });
        }
        let mut element = self.parse_data_type()?;
        while let Some(close) = closes.pop() {
            if close == TokenType::Gt {
                self.expect_gt()?;
            } else {
                self.expect_token(TokenType::RParen)?;
            }
            element = DataType::Array {
                element_type: Box::new(element),
                dimension: None,
            };
            if !closes.is_empty() {
                element = self.finish_data_type(element)?;
                scopes.pop();
            }
        }
        Ok(element)
    }

    #[inline(never)]
    pub(crate) fn parse_map_data_type(&mut self) -> Result<DataType> {
        if self.match_token(TokenType::Lt) {
            let key_type = self.parse_data_type()?;
            self.expect_token(TokenType::Comma)?;
            let value_type = self.parse_data_type()?;
            self.expect_gt()?;
            Ok(DataType::Map {
                key_type: Box::new(key_type),
                value_type: Box::new(value_type),
            })
        } else if self.match_token(TokenType::LBracket) {
            let key_type = self.parse_data_type()?;
            self.expect_token(TokenType::FArrow)?;
            let value_type = self.parse_data_type()?;
            self.expect_token(TokenType::RBracket)?;
            Ok(DataType::Map {
                key_type: Box::new(key_type),
                value_type: Box::new(value_type),
            })
        } else if self.match_token(TokenType::LParen) {
            let key_type = self.parse_data_type()?;
            self.expect_token(TokenType::Comma)?;
            let value_type = self.parse_data_type()?;
            self.expect_token(TokenType::RParen)?;
            Ok(DataType::Map {
                key_type: Box::new(key_type),
                value_type: Box::new(value_type),
            })
        } else {
            Ok(DataType::Custom {
                name: "MAP".to_string(),
            })
        }
    }

    #[inline(never)]
    pub(crate) fn parse_struct_data_type(&mut self) -> Result<DataType> {
        if self.match_token(TokenType::Lt) {
            let fields = self.parse_struct_type_fields(false)?;
            self.expect_gt()?;
            Ok(DataType::Struct {
                fields,
                nested: false,
            })
        } else if self.match_token(TokenType::LParen) {
            let fields = self.parse_struct_type_fields(true)?;
            self.expect_token(TokenType::RParen)?;
            Ok(DataType::Struct {
                fields,
                nested: true,
            })
        } else {
            Ok(DataType::Custom {
                name: "STRUCT".to_string(),
            })
        }
    }

    #[inline(never)]
    pub(crate) fn parse_row_data_type(&mut self) -> Result<DataType> {
        if self.match_token(TokenType::LParen) {
            let fields = self.parse_struct_type_fields(true)?;
            self.expect_token(TokenType::RParen)?;
            Ok(DataType::Struct {
                fields,
                nested: true,
            })
        } else {
            Ok(DataType::Custom {
                name: "ROW".to_string(),
            })
        }
    }

    #[inline(never)]
    pub(crate) fn parse_record_data_type(&mut self) -> Result<DataType> {
        if self.match_token(TokenType::LParen) {
            let fields = self.parse_struct_type_fields(true)?;
            self.expect_token(TokenType::RParen)?;
            Ok(DataType::Struct {
                fields,
                nested: true,
            })
        } else {
            Ok(DataType::Custom {
                name: "RECORD".to_string(),
            })
        }
    }

    pub(crate) fn maybe_parse_collated_data_type(
        &mut self,
        data_type: DataType,
    ) -> Result<DataType> {
        if !(self.match_token(TokenType::Collate) || self.match_identifier("COLLATE")) {
            return Ok(data_type);
        }

        let collation_start = self.current;
        if self.check(TokenType::String) {
            self.skip();
        } else {
            self.expect_identifier_or_keyword()?;
            while self.match_token(TokenType::Dot) {
                self.expect_identifier_or_keyword()?;
            }
        }
        let collation = if self.current > collation_start {
            let start_span = self.tokens[collation_start].span.start;
            let end_span = self.tokens[self.current - 1].span.end;
            let raw = self.source_text_range(start_span, end_span);
            if raw.is_empty() {
                self.tokens_to_sql(collation_start, self.current)
            } else {
                raw
            }
        } else {
            String::new()
        };
        let base = self.data_type_to_string(&data_type);

        Ok(DataType::Custom {
            name: format!("{} COLLATE {}", base, collation),
        })
    }

    /// Parses a `::TYPE` cast type; DuckDB reads `INT[3]` as a fixed-size array.
    pub(crate) fn parse_data_type_for_cast(&mut self) -> Result<DataType> {
        let data_type = self.with_parser_depth(|parser| parser.parse_data_type_for_cast_inner())?;
        Ok(self.normalize_dialect_data_type(data_type))
    }
}
