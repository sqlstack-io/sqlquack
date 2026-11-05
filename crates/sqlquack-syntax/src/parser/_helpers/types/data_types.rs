//! Parser types: data types.

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Star;
use crate::expressions::operators::models::Alias;
use crate::expressions::queries::models::Subquery;
use crate::expressions::statements::models::Command;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::OracleCharacterKind;
use crate::expressions::types::models::OracleCharacterLengthSemantics;
use crate::expressions::types::models::OracleDataType;
use crate::expressions::types::models::OracleTimestampTimeZone;
use crate::expressions::types::models::StructField;
use crate::parser::constants::ARRAY;
use crate::parser::constants::EXCLUDE;
use crate::parser::constants::INCLUDE;
use crate::parser::constants::MAP;
use crate::parser::constants::NCHAR;
use crate::parser::constants::ROW;
use crate::parser::constants::STRUCT;
use crate::parser::models::Parser;
use crate::tokens::errors::Error;
use crate::tokens::models::DialectType;
use crate::tokens::models::ParserToken;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::Span;
use crate::tokens::models::Token;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// Whether an upper-cased type name is a known custom type (DATETIME2, SYSNAME), kept upper.
pub(crate) fn convert_name_is_known_custom(name: &str) -> bool {
    matches!(
        name,
        "DATETIME2"
            | "DATETIMEOFFSET"
            | "SMALLDATETIME"
            | "DATETIME"
            | "NVARCHAR2"
            | "VARCHAR2"
            | "NCHAR"
            | "MONEY"
            | "SMALLMONEY"
            | "TINYINT"
            | "MEDIUMINT"
            | "BYTEINT"
            | "SUPER"
            | "HLLSKETCH"
            | "TIMETZ"
            | "TIMESTAMPTZ"
            | "SYSNAME"
            | "XML"
            | "SQL_VARIANT"
            | "HIERARCHYID"
            | "ROWVERSION"
            | "IMAGE"
            | "CURSOR"
            | "TABLE"
            | "UNIQUEIDENTIFIER"
            | "VARIANT"
            | "OBJECT"
            | "NUMBER"
            | "BINARY_FLOAT"
            | "BINARY_DOUBLE"
            | "CLOB"
            | "NCLOB"
            | "RAW"
            | "LONG"
            | "MEDIUMTEXT"
            | "LONGTEXT"
            | "MEDIUMBLOB"
            | "LONGBLOB"
            | "TINYTEXT"
            | "TINYBLOB"
            | "INT2"
            | "INT4"
            | "INT8"
            | "FLOAT4"
            | "FLOAT8"
            | "SERIAL"
            | "BIGSERIAL"
            | "SMALLSERIAL"
            | "YEAR"
            | "FIXED"
            | "SIGNED"
            | "UNSIGNED"
            | "ROW"
            | "BIT"
            | "BOOLEAN"
            | "BOOL"
            | "TEXT"
            | "STRING"
            | "NTEXT"
            | "INT128"
            | "INT256"
            | "UINT8"
            | "UINT16"
            | "UINT32"
            | "UINT64"
            | "UINT128"
            | "UTINYINT"
            | "USMALLINT"
            | "UINTEGER"
            | "UBIGINT"
            | "UHUGEINT"
            | "UINT256"
            | "FLOAT32"
            | "FLOAT64"
            | "LOWCARDINALITY"
            | "NULLABLE"
            | "IPADDRESS"
            | "IPV4"
            | "IPV6"
            | "AGGREGATEFUNCTION"
            | "SIMPLEAGGREGATEFUNCTION"
            | "FIXEDSTRING"
            | "RING"
            | "NESTED"
    )
}

impl Parser {
    #[inline(never)]
    pub(crate) fn parse_data_type_for_cast_inner(&mut self) -> Result<DataType> {
        if let Some(data_type) = self.parse_vertica_data_type()? {
            return Ok(data_type);
        }
        let supports_array_type_suffix = matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::DuckDB)
                | Some(crate::tokens::models::DialectType::PostgreSQL)
        );

        let is_quoted = self.check(TokenType::QuotedIdentifier);
        let raw_name = self.expect_identifier_or_keyword()?;
        if is_quoted {
            let known_type = self.convert_name_to_type(&raw_name);
            if let Ok(ref dt) = known_type {
                if !matches!(dt, DataType::Custom { .. }) {
                    return known_type;
                }
            }
            return Ok(DataType::Custom {
                name: format!("\"{}\"", raw_name),
            });
        }
        let name = raw_name.to_ascii_uppercase();

        let base_type = match name.as_str() {
            name if DataType::from_unsigned_name(name).is_some() => {
                DataType::from_unsigned_name(name).ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?
            }
            name if self.is_int128_type_name(name) => DataType::Int128,
            "ARRAY" => self.parse_cast_array_type(name)?,
            "MAP" => self.parse_cast_map_type(name)?,
            "STRUCT" => self.parse_cast_struct_type(name)?,
            "ROW" => self.parse_cast_row_type(name)?,
            "RECORD" => self.parse_cast_record_type(name)?,
            "DOUBLE" => self.parse_cast_double_type()?,
            "BPCHAR" | "CHARACTER" | "CHAR" | "NCHAR" => {
                let is_nchar = name == NCHAR;
                if self.match_identifier("VARYING") {
                    let length = if self.match_token(TokenType::LParen) {
                        let len = Some(self.expect_number()? as u32);
                        self.expect_token(TokenType::RParen)?;
                        len
                    } else {
                        None
                    };
                    if is_nchar {
                        let name = if let Some(len) = length {
                            format!("NVARCHAR({})", len)
                        } else {
                            "NVARCHAR".to_string()
                        };
                        DataType::Custom { name }
                    } else {
                        DataType::VarChar {
                            length,
                            parenthesized_length: false,
                        }
                    }
                } else {
                    let length = if self.match_token(TokenType::LParen) {
                        let len = Some(self.expect_number()? as u32);
                        self.expect_token(TokenType::RParen)?;
                        len
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
                        DataType::Custom { name }
                    } else {
                        DataType::Char { length }
                    }
                }
            }
            _ => self.parse_cast_other_base_type(name, &raw_name)?,
        };

        let mut result_type = base_type;

        result_type = self.maybe_parse_collated_data_type(result_type)?;

        if supports_array_type_suffix {
            self.maybe_parse_array_dimensions(result_type)
        } else {
            Ok(result_type)
        }
    }

    /// Parses a data type if one is present, for JSON_TABLE columns.
    pub(crate) fn parse_data_type_optional(&mut self) -> Result<Option<DataType>> {
        if !self.check(TokenType::Identifier)
            && !self.check(TokenType::Var)
            && !self.check_keyword()
        {
            return Ok(None);
        }

        if self.check_identifier("PATH") {
            return Ok(None);
        }

        let saved_pos = self.current;
        match self.parse_data_type() {
            Ok(dt) => Ok(Some(dt)),
            Err(_) => {
                self.current = saved_pos;
                Ok(None)
            }
        }
    }

    pub(crate) fn oracle_data_type_to_string(&self, data_type: &OracleDataType) -> String {
        match data_type {
            OracleDataType::Number { precision, scale } => {
                if precision.is_none() && scale.is_none() {
                    "NUMBER".to_string()
                } else {
                    let precision = precision
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "*".to_string());
                    if let Some(scale) = scale {
                        format!("NUMBER({precision}, {scale})")
                    } else {
                        format!("NUMBER({precision})")
                    }
                }
            }
            OracleDataType::BinaryFloat => "BINARY_FLOAT".to_string(),
            OracleDataType::BinaryDouble => "BINARY_DOUBLE".to_string(),
            OracleDataType::Float { precision } => precision
                .map(|value| format!("FLOAT({value})"))
                .unwrap_or_else(|| "FLOAT".to_string()),
            OracleDataType::Character {
                kind,
                length,
                semantics,
            } => {
                let name = match kind {
                    OracleCharacterKind::Char => "CHAR",
                    OracleCharacterKind::VarChar => "VARCHAR2",
                    OracleCharacterKind::NChar => "NCHAR",
                    OracleCharacterKind::NVarChar => "NVARCHAR2",
                };
                if let Some(length) = length {
                    let semantics = match semantics {
                        Some(OracleCharacterLengthSemantics::Byte) => " BYTE",
                        Some(OracleCharacterLengthSemantics::Char) => " CHAR",
                        None => "",
                    };
                    format!("{name}({length}{semantics})")
                } else {
                    name.to_string()
                }
            }
            OracleDataType::Date => "DATE".to_string(),
            OracleDataType::Timestamp {
                precision,
                timezone,
            } => {
                let mut sql = "TIMESTAMP".to_string();
                if let Some(precision) = precision {
                    sql.push_str(&format!("({precision})"));
                }
                sql.push_str(match timezone {
                    OracleTimestampTimeZone::None => "",
                    OracleTimestampTimeZone::WithTimeZone => " WITH TIME ZONE",
                    OracleTimestampTimeZone::WithLocalTimeZone => " WITH LOCAL TIME ZONE",
                });
                sql
            }
            OracleDataType::IntervalYearToMonth { year_precision } => {
                let precision = year_precision
                    .map(|value| format!("({value})"))
                    .unwrap_or_default();
                format!("INTERVAL YEAR{precision} TO MONTH")
            }
            OracleDataType::IntervalDayToSecond {
                day_precision,
                fractional_seconds_precision,
            } => {
                let day_precision = day_precision
                    .map(|value| format!("({value})"))
                    .unwrap_or_default();
                let fractional_seconds_precision = fractional_seconds_precision
                    .map(|value| format!("({value})"))
                    .unwrap_or_default();
                format!("INTERVAL DAY{day_precision} TO SECOND{fractional_seconds_precision}")
            }
            OracleDataType::Clob { national } => {
                if *national {
                    "NCLOB".to_string()
                } else {
                    "CLOB".to_string()
                }
            }
            OracleDataType::Blob => "BLOB".to_string(),
            OracleDataType::Raw { length } => length
                .map(|value| format!("RAW({value})"))
                .unwrap_or_else(|| "RAW".to_string()),
            OracleDataType::Long { raw } => {
                if *raw {
                    "LONG RAW".to_string()
                } else {
                    "LONG".to_string()
                }
            }
            OracleDataType::RowId => "ROWID".to_string(),
        }
    }

    /// Convert a DataType to a string representation for JSONColumnDef.kind
    pub(crate) fn data_type_to_string(&self, dt: &DataType) -> String {
        match dt {
            DataType::Hana { hana_type } => {
                if hana_type.parameters.is_empty() {
                    hana_type.name.clone()
                } else {
                    format!(
                        "{}({})",
                        hana_type.name,
                        hana_type
                            .parameters
                            .iter()
                            .map(u32::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }

            DataType::Int {
                length: Some(n),
                integer_spelling: true,
            } => format!("INTEGER({})", n),
            DataType::Int {
                length: Some(n), ..
            } => format!("INT({})", n),
            DataType::Int {
                length: None,
                integer_spelling: true,
            } => "INTEGER".to_string(),
            DataType::Int { length: None, .. } => "INT".to_string(),
            DataType::BigInt { length: Some(n) } => format!("BIGINT({})", n),
            DataType::BigInt { length: None } => "BIGINT".to_string(),
            DataType::Int128 => "INT128".to_string(),
            DataType::UInt8 => "UTINYINT".to_string(),
            DataType::UInt16 => "USMALLINT".to_string(),
            DataType::UInt32 => "UINTEGER".to_string(),
            DataType::UInt64 => "UBIGINT".to_string(),
            DataType::UInt128 => "UINT128".to_string(),
            DataType::SmallInt { length: Some(n) } => format!("SMALLINT({})", n),
            DataType::SmallInt { length: None } => "SMALLINT".to_string(),
            DataType::TinyInt { length: Some(n) } => format!("TINYINT({})", n),
            DataType::TinyInt { length: None } => "TINYINT".to_string(),
            DataType::Float {
                precision: Some(p),
                scale: Some(s),
                ..
            } => format!("FLOAT({}, {})", p, s),
            DataType::Float {
                precision: Some(p),
                scale: None,
                ..
            } => format!("FLOAT({})", p),
            DataType::Float {
                precision: None, ..
            } => "FLOAT".to_string(),
            DataType::Double {
                precision: Some(p),
                scale: Some(s),
            } => format!("DOUBLE({}, {})", p, s),
            DataType::Double {
                precision: Some(p),
                scale: None,
            } => format!("DOUBLE({})", p),
            DataType::Double {
                precision: None, ..
            } => "DOUBLE".to_string(),
            DataType::Decimal {
                precision: Some(p),
                scale: Some(s),
            } => format!("DECIMAL({}, {})", p, s),
            DataType::Decimal {
                precision: Some(p),
                scale: None,
            } => format!("DECIMAL({})", p),
            DataType::Decimal {
                precision: None, ..
            } => "DECIMAL".to_string(),
            DataType::Oracle { oracle_type } => self.oracle_data_type_to_string(oracle_type),
            DataType::VarChar {
                length: Some(n), ..
            } => format!("VARCHAR({})", n),
            DataType::VarChar { length: None, .. } => "VARCHAR".to_string(),
            DataType::Char { length: Some(n) } => format!("CHAR({})", n),
            DataType::Char { length: None } => "CHAR".to_string(),
            DataType::Text => "TEXT".to_string(),
            DataType::Boolean => "BOOLEAN".to_string(),
            DataType::Date => "DATE".to_string(),
            DataType::Time {
                precision: Some(p), ..
            } => format!("TIME({})", p),
            DataType::Time {
                precision: None, ..
            } => "TIME".to_string(),
            DataType::Timestamp {
                precision: Some(p),
                timezone: true,
            } => format!("TIMESTAMPTZ({})", p),
            DataType::Timestamp {
                precision: Some(p),
                timezone: false,
            } => format!("TIMESTAMP({})", p),
            DataType::Timestamp {
                precision: None,
                timezone: true,
            } => "TIMESTAMPTZ".to_string(),
            DataType::Timestamp {
                precision: None,
                timezone: false,
            } => "TIMESTAMP".to_string(),
            DataType::Json => "JSON".to_string(),
            DataType::JsonB => "JSONB".to_string(),
            DataType::Binary { length: Some(n) } => format!("BINARY({})", n),
            DataType::Binary { length: None } => "BINARY".to_string(),
            DataType::VarBinary { length: Some(n) } => format!("VARBINARY({})", n),
            DataType::VarBinary { length: None } => "VARBINARY".to_string(),
            DataType::String { length: Some(n) } => format!("STRING({})", n),
            DataType::String { length: None } => "STRING".to_string(),
            DataType::Array { element_type, .. } => {
                format!("ARRAY({})", self.data_type_to_string(element_type))
            }
            DataType::Nullable { inner } => {
                format!("Nullable({})", self.data_type_to_string(inner))
            }
            DataType::Custom { name } => name.clone(),
            _ => format!("{:?}", dt),
        }
    }

    /// Parse optional array dimensions after a type: [], [N], [N][M], ARRAY, ARRAY[N], etc.
    pub(crate) fn maybe_parse_array_dimensions(&mut self, base_type: DataType) -> Result<DataType> {
        let mut current_type = base_type;

        if self.check_identifier("ARRAY") {
            self.skip();
            let dimension = if self.match_token(TokenType::LBracket) {
                let dim = if self.check(TokenType::Number) {
                    let n = self.expect_number()? as u32;
                    Some(n)
                } else {
                    None
                };
                self.expect_token(TokenType::RBracket)?;
                dim
            } else {
                None
            };
            current_type = DataType::Array {
                element_type: Box::new(current_type),
                dimension,
            };
        }

        while self.match_token(TokenType::LBracket) {
            let dimension = if self.check(TokenType::Number) {
                let n = self.expect_number()? as u32;
                Some(n)
            } else {
                None
            };
            self.expect_token(TokenType::RBracket)?;

            current_type = DataType::Array {
                element_type: Box::new(current_type),
                dimension,
            };
        }

        Ok(current_type)
    }

    /// Parse spatial type arguments like GEOMETRY(Point, 4326) or GEOGRAPHY
    pub(crate) fn parse_spatial_type_args(&mut self) -> Result<(Option<String>, Option<u32>)> {
        if self.match_token(TokenType::LParen) {
            if self.check(TokenType::Number) {
                let n = self.expect_number()? as u32;
                self.expect_token(TokenType::RParen)?;
                return Ok((None, Some(n)));
            }
            let subtype = Some(self.expect_identifier()?.to_ascii_uppercase());

            let srid = if self.match_token(TokenType::Comma) {
                Some(self.expect_number()? as u32)
            } else {
                None
            };

            self.expect_token(TokenType::RParen)?;
            Ok((subtype, srid))
        } else {
            Ok((None, None))
        }
    }

    /// Parses struct, row or union fields `name TYPE, ...` in parentheses or angle brackets.
    pub(crate) fn parse_struct_type_fields(
        &mut self,
        paren_style: bool,
    ) -> Result<Vec<StructField>> {
        self.with_parser_depth(|parser| parser.parse_struct_type_fields_inner(paren_style))
    }

    #[inline(never)]
    pub(crate) fn parse_struct_type_fields_inner(
        &mut self,
        paren_style: bool,
    ) -> Result<Vec<StructField>> {
        let mut fields: Vec<StructField> = Vec::new();
        if paren_style && self.check(TokenType::RParen)
            || !paren_style && (self.check(TokenType::Gt) || self.check(TokenType::GtGt))
        {
            return Ok(fields);
        }
        loop {
            let is_quoted = self.check(TokenType::QuotedIdentifier);
            let first = self.expect_identifier_or_keyword()?;
            let first_upper = first.to_ascii_uppercase();

            let is_parametric_type = (first_upper == ARRAY
                || first_upper == MAP
                || first_upper == STRUCT
                || first_upper == ROW)
                && (self.check(TokenType::Lt) || self.check(TokenType::LParen));

            let (field_name, field_type) = if is_parametric_type {
                let field_type = self.parse_data_type_from_name(&first_upper)?;
                (String::new(), field_type)
            } else if self.check(TokenType::Comma)
                || self.match_identifier("OPTIONS")
                || paren_style && self.check(TokenType::RParen)
                || !paren_style && (self.check(TokenType::Gt) || self.check(TokenType::GtGt))
            {
                if self.previous_text().eq_ignore_ascii_case("OPTIONS") {
                    self.current -= 1;
                }
                let field_type = self.convert_name_to_type(&first)?;
                (String::new(), field_type)
            } else if self.is_identifier_token()
                || self.is_safe_keyword_as_identifier()
                || self.check(TokenType::Lt)
                || self.check(TokenType::LParen)
                || self.check(TokenType::Colon)
            {
                self.match_token(TokenType::Colon);
                let field_type = self.parse_data_type()?;
                let field_name = if is_quoted {
                    Identifier::quoted(first).to_type_field_name()
                } else {
                    first
                };
                (field_name, field_type)
            } else {
                let field_type = self.convert_name_to_type(&first)?;
                (String::new(), field_type)
            };

            let comment = if self.match_token(TokenType::Comment) {
                Some(self.expect_string()?)
            } else {
                None
            };

            let options = if self.match_identifier("OPTIONS") {
                self.parse_options_list()?
            } else {
                Vec::new()
            };

            fields.push(StructField::with_options_and_comment(
                field_name, field_type, options, comment,
            ));

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(fields)
    }

    /// Parses a data type whose name was already consumed, as in ARRAY<T>.
    pub(crate) fn parse_data_type_from_name(&mut self, name: &str) -> Result<DataType> {
        match name {
            name if DataType::from_unsigned_name(name).is_some() => {
                Ok(DataType::from_unsigned_name(name).ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?)
            }
            name if self.is_int128_type_name(name) => Ok(DataType::Int128),
            "ARRAY" => {
                if self.match_token(TokenType::Lt) {
                    let element_type = self.parse_data_type()?;
                    self.expect_gt()?;
                    Ok(DataType::Array {
                        element_type: Box::new(element_type),
                        dimension: None,
                    })
                } else {
                    Ok(DataType::Custom {
                        name: "ARRAY".to_string(),
                    })
                }
            }
            "MAP" => {
                if self.match_token(TokenType::Lt) {
                    let key_type = self.parse_data_type()?;
                    self.expect_token(TokenType::Comma)?;
                    let value_type = self.parse_data_type()?;
                    self.expect_gt()?;
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
            "STRUCT" => {
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
            "ROW" => {
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
            _ => Ok(DataType::Custom {
                name: name.to_string(),
            }),
        }
    }

    /// A data type from a bare type name, for anonymous struct fields.
    pub(crate) fn convert_name_to_type(&self, name: &str) -> Result<DataType> {
        let upper = name.to_ascii_uppercase();
        Ok(match upper.as_str() {
            name if DataType::from_unsigned_name(name).is_some() => {
                DataType::from_unsigned_name(name).ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?
            }
            name if self.is_int128_type_name(name) => DataType::Int128,
            "INT" => DataType::Int {
                length: None,
                integer_spelling: false,
            },
            "INTEGER" => DataType::Int {
                length: None,
                integer_spelling: true,
            },
            "BIGINT" => DataType::BigInt { length: None },
            "SMALLINT" => DataType::SmallInt { length: None },
            "TINYINT" => DataType::TinyInt { length: None },
            "FLOAT" | "BINARY_FLOAT" => DataType::Float {
                precision: None,
                scale: None,
                real_spelling: false,
            },
            "REAL" => DataType::Float {
                precision: None,
                scale: None,
                real_spelling: true,
            },
            "DOUBLE" | "BINARY_DOUBLE" => DataType::Double {
                precision: None,
                scale: None,
            },
            "DECIMAL" | "NUMERIC" => DataType::Decimal {
                precision: None,
                scale: None,
            },
            "BOOLEAN" | "BOOL" => DataType::Boolean,
            "CHAR" | "CHARACTER" | "NCHAR" => DataType::Char { length: None },
            "VARCHAR" | "NVARCHAR" => DataType::VarChar {
                length: None,
                parenthesized_length: false,
            },
            "TEXT" | "STRING" | "NTEXT" => DataType::Text,
            "DATE" => DataType::Date,
            "TIME" => DataType::Time {
                precision: None,
                timezone: false,
            },
            "TIMETZ" => DataType::Time {
                precision: None,
                timezone: true,
            },
            "TIMESTAMP" => DataType::Timestamp {
                precision: None,
                timezone: false,
            },
            "INTERVAL" => DataType::Interval {
                unit: None,
                to: None,
            },
            "JSON" => DataType::Json,
            "JSONB" => DataType::JsonB,
            "UUID" => DataType::Uuid,
            "BLOB" => DataType::Blob,
            "BYTEA" => DataType::VarBinary { length: None },
            "BINARY" => DataType::Binary { length: None },
            "VARBINARY" => DataType::VarBinary { length: None },
            "BIT" => DataType::Bit { length: None },
            "VARBIT" => DataType::VarBit { length: None },
            _ => DataType::Custom {
                name: name.to_string(),
            },
        })
    }

    /// Parses star modifiers ILIKE 'pattern' (a string literal) or EXCLUDE/EXCEPT, then REPLACE and RENAME.
    pub(crate) fn parse_star_modifiers(&mut self, table: Option<Identifier>) -> Result<Star> {
        self.parse_star_modifiers_with_comments(table, Vec::new())
    }

    /// One EXCLUDE name; Snowflake names columns there, never strings.
    fn parse_star_exclude_name(&mut self) -> Result<String> {
        if self.config.dialect == Some(DialectType::Snowflake) && self.check(TokenType::String) {
            return Err(self.parse_error("Snowflake EXCLUDE names columns, not strings"));
        }
        if self.check(TokenType::String) || self.is_safe_keyword_as_identifier() {
            self.advance_text()
        } else {
            self.expect_identifier()
        }
    }

    /// Parse star modifiers with explicit trailing comments from the star token
    pub(crate) fn parse_star_modifiers_with_comments(
        &mut self,
        table: Option<Identifier>,
        star_trailing_comments: Vec<String>,
    ) -> Result<Star> {
        let mut ilike = None;
        let mut except = None;
        let mut replace = None;
        let mut rename = None;

        if self.check(TokenType::ILike) && self.check_next(TokenType::String) {
            self.skip();
            ilike = Some(self.advance_text()?);
        } else if self.match_token(TokenType::Exclude) || self.match_token(TokenType::Except) {
            let _ = self.match_text_seq(&["STRICT"]);
            let mut columns: Vec<Identifier> = Vec::new();
            if self.match_token(TokenType::LParen) {
                loop {
                    let start = self.peek().span;
                    let quoted = self.check(TokenType::QuotedIdentifier);
                    let col = self.parse_star_exclude_name()?;
                    if self.match_token(TokenType::Dot) {
                        let subcol = if self.is_safe_keyword_as_identifier() {
                            self.advance_text()?
                        } else {
                            self.expect_identifier()?
                        };
                        columns.push(
                            Identifier::new(format!("{}.{}", col, subcol))
                                .with_span(start.through(self.previous().span)),
                        );
                    } else {
                        let mut column =
                            Identifier::new(col).with_span(start.through(self.previous().span));
                        column.quoted = quoted;
                        columns.push(column);
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
            } else {
                let start = self.peek().span;
                let quoted = self.check(TokenType::QuotedIdentifier);
                let col = self.parse_star_exclude_name()?;
                let mut column =
                    Identifier::new(col).with_span(start.through(self.previous().span));
                column.quoted = quoted;
                columns.push(column);
            }
            except = Some(columns);
        }

        if self.match_token(TokenType::Replace) {
            let _ = self.match_text_seq(&["STRICT"]);
            let mut replacements: Vec<Alias> = Vec::new();
            if self.match_token(TokenType::LParen) {
                loop {
                    let expr = self.parse_expression()?;
                    self.expect_token(TokenType::As)?;
                    let alias =
                        self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?;
                    replacements.push(Alias::new(expr, alias));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
            } else {
                return Err(self.parse_error("Expected LParen after REPLACE"));
            }
            replace = Some(replacements);
        }

        if self.match_token(TokenType::Rename) {
            let mut renames: Vec<(Identifier, Identifier)> = Vec::new();
            if self.match_token(TokenType::LParen) {
                loop {
                    let old_name = self.parse_quoting_identifier(Self::expect_identifier)?;
                    self.expect_token(TokenType::As)?;
                    let new_name = self.parse_quoting_identifier(Self::expect_identifier)?;
                    renames.push((old_name, new_name));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
            } else {
                let old_name = self.parse_quoting_identifier(Self::expect_identifier)?;
                self.expect_token(TokenType::As)?;
                let new_name = self.parse_quoting_identifier(Self::expect_identifier)?;
                renames.push((old_name, new_name));
            }
            rename = Some(renames);
        }

        Ok(Star {
            table,
            ilike,
            except,
            replace,
            rename,
            trailing_comments: star_trailing_comments,
            span: None,
        })
    }

    /// Whether the input is exhausted; a caller-supplied `TokenType::Eof` also ends it.
    #[inline]
    pub(crate) fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || self.tokens[self.current].token_type == TokenType::Eof
    }

    /// Whether a query modifier keyword or the end of input comes next.
    pub(crate) fn is_at_query_modifier_or_end(&self) -> bool {
        if self.is_at_end() {
            return true;
        }
        matches!(
            self.peek().token_type,
            TokenType::Having
                | TokenType::Qualify
                | TokenType::Window
                | TokenType::Order
                | TokenType::Limit
                | TokenType::Fetch
                | TokenType::Offset
                | TokenType::For
                | TokenType::Lock
                | TokenType::Union
                | TokenType::Except
                | TokenType::Intersect
                | TokenType::RParen
                | TokenType::Semicolon
                | TokenType::Where
        )
    }

    /// Create a parse error with position from the current token
    pub(crate) fn parse_error(&self, message: impl Into<String>) -> Error {
        if let Some(error) = self.recursion.error() {
            return error;
        }
        let span = self
            .tokens
            .get(self.current)
            .or_else(|| self.tokens.last())
            .map(|token| token.span)
            .unwrap_or_default();
        Error::parse(message, span)
    }

    /// The current token, or the last one at the end.
    #[inline]
    pub(crate) fn peek(&self) -> &ParserToken {
        &self.tokens[self.current.min(self.tokens.len().saturating_sub(1))]
    }

    /// Look ahead by n positions (0 = current token)
    pub(crate) fn peek_nth(&self, n: usize) -> Option<&ParserToken> {
        let idx = self.current + n;
        if idx < self.tokens.len() {
            Some(&self.tokens[idx])
        } else {
            None
        }
    }

    /// Advances one token, erroring at the end of input instead of repeating the last token.
    #[inline]
    pub(crate) fn advance(&mut self) -> Result<Token> {
        if self.is_at_end() {
            return Err(self.end_of_input_error());
        }
        let token = self.materialize_token(&self.tokens[self.current]);
        self.current += 1;
        Ok(token)
    }

    /// Advances one token and takes its text, erroring at the end of input.
    #[inline]
    pub(crate) fn advance_text(&mut self) -> Result<String> {
        if self.is_at_end() {
            return Err(self.end_of_input_error());
        }
        let text = self.tokens[self.current].text_owned();
        self.current += 1;
        Ok(text)
    }

    /// The error for running off the end of the tokens, at the end of the last one.
    pub(crate) fn end_of_input_error(&self) -> Error {
        let span = self
            .tokens
            .last()
            .map(|token| token.span)
            .unwrap_or_default();
        Error::parse(
            "Unexpected end of input",
            Span {
                start: span.end,
                end: span.end,
                line: span.line,
                column: span.column,
            },
        )
    }

    #[inline]
    pub(crate) fn peek_text(&self) -> &str {
        self.peek().text()
    }

    #[inline]
    pub(crate) fn previous_text(&self) -> &str {
        self.previous().text()
    }

    pub(crate) fn materialize_token(&self, token: &ParserToken) -> Token {
        Token {
            token_type: token.token_type,
            text: token.text_owned(),
            span: token.span,
            comments: token.comments.clone(),
            trailing_comments: token.trailing_comments.clone(),
        }
    }

    /// Advance to next token without returning it (when result is unused)
    #[inline]
    pub(crate) fn skip(&mut self) {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
    }

    /// Get the previous token (last consumed)
    pub(crate) fn previous(&self) -> &ParserToken {
        &self.tokens[self.current - 1]
    }

    /// Get trailing comments from the previous token
    pub(crate) fn previous_trailing_comments(&self) -> &[String] {
        if self.current > 0 {
            &self.tokens[self.current - 1].trailing_comments
        } else {
            &[]
        }
    }

    /// Get the token type of the previous token (the one before current).
    pub(crate) fn previous_token_type(&self) -> Option<TokenType> {
        if self.current > 0 {
            Some(self.tokens[self.current - 1].token_type)
        } else {
            None
        }
    }

    /// Wraps a query (not a plain expression) in a Subquery node.
    pub(crate) fn maybe_wrap_in_subquery(&self, inner: Expression) -> Expression {
        if matches!(
            &inner,
            Expression::Select(_)
                | Expression::Union(_)
                | Expression::Intersect(_)
                | Expression::Except(_)
        ) {
            Expression::Subquery(Box::new(Subquery {
                this: inner,
                alias: None,
                column_aliases: Vec::new(),
                alias_explicit_as: false,
                alias_keyword: None,
                order_by: None,
                limit: None,
                offset: None,
                distribute_by: None,
                sort_by: None,
                cluster_by: None,
                lateral: false,
                modifiers_inside: false,
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            }))
        } else {
            inner
        }
    }

    /// Clears trailing comments of an expression's rightmost leaf, so AND/OR do not repeat them.
    pub(crate) fn clear_rightmost_trailing_comments(expr: &mut Expression) {
        match expr {
            Expression::Column(col) => col.trailing_comments.clear(),
            Expression::And(op) | Expression::Or(op) => {
                Self::clear_rightmost_trailing_comments(&mut op.right);
            }
            Expression::Not(op) => {
                Self::clear_rightmost_trailing_comments(&mut op.this);
            }
            Expression::Eq(op)
            | Expression::Neq(op)
            | Expression::Lt(op)
            | Expression::Lte(op)
            | Expression::Gt(op)
            | Expression::Gte(op)
            | Expression::Add(op)
            | Expression::Sub(op)
            | Expression::Mul(op)
            | Expression::Div(op) => {
                Self::clear_rightmost_trailing_comments(&mut op.right);
            }
            _ => {}
        }
    }

    /// Get leading comments from the current token (comments that appeared before it)
    pub(crate) fn current_leading_comments(&self) -> &[String] {
        if !self.is_at_end() {
            &self.tokens[self.current].comments
        } else {
            &[]
        }
    }

    /// Convert a slice of tokens to SQL string with proper quoting for strings
    pub(crate) fn tokens_to_sql(&self, start: usize, end: usize) -> String {
        let mut result = String::new();
        let mut prev_line: Option<usize> = None;
        let mut prev_end_offset: Option<usize> = None;

        for t in &self.tokens[start..end] {
            let is_new_line = prev_line.is_some_and(|line| t.span.line > line);

            if is_new_line {
                result.push('\n');
                let text_len = t.text.chars().count();
                let start_col = t.span.column.saturating_sub(text_len);
                let start_col = if t.token_type == TokenType::String {
                    start_col.saturating_sub(2)
                } else {
                    start_col
                };
                let indent = start_col.saturating_sub(1);
                for _ in 0..indent {
                    result.push(' ');
                }
            } else if !result.is_empty() {
                let had_space = prev_end_offset.is_some_and(|prev_end| t.span.start > prev_end);
                if had_space {
                    result.push(' ');
                }
            }

            if t.token_type == TokenType::String {
                result.push('\'');
                result.push_str(&t.text.replace('\'', "''"));
                result.push('\'');
            } else {
                result.push_str(&t.text);
            }

            prev_line = Some(t.span.line);
            prev_end_offset = Some(t.span.end);
        }
        result
    }

    /// Source text between tokenizer offsets, which count characters, not bytes.
    pub(crate) fn source_text_range(&self, start: usize, end: usize) -> String {
        let Some(source) = self.source.as_ref() else {
            return String::new();
        };

        if source.is_ascii() {
            source
                .get(start..end)
                .map(ToString::to_string)
                .unwrap_or_default()
        } else {
            source.chars().skip(start).take(end - start).collect()
        }
    }

    /// Keeps a ClickHouse `(EXPLAIN ... SELECT ...)` subquery as command text.
    pub(crate) fn parse_clickhouse_command_until_rparen(&mut self) -> Result<Expression> {
        let start_index = self.current;
        let start_span = self
            .tokens
            .get(start_index)
            .map(|token| token.span.start)
            .unwrap_or_default();

        let mut depth = 0usize;
        while !self.is_at_end() {
            match self.peek().token_type {
                TokenType::LParen => {
                    depth += 1;
                    self.skip();
                }
                TokenType::RParen if depth == 0 => break,
                TokenType::RParen => {
                    depth = depth.saturating_sub(1);
                    self.skip();
                }
                TokenType::Semicolon if depth == 0 => break,
                _ => self.skip(),
            }
        }

        let end_span = if self.current > start_index {
            self.tokens[self.current - 1].span.end
        } else {
            start_span
        };

        let command_text = if self.source.is_some() {
            self.source_text_range(start_span, end_span)
                .trim()
                .to_string()
        } else {
            self.tokens_to_sql(start_index, self.current)
        };

        Ok(Expression::Command(Box::new(Command {
            this: command_text,
        })))
    }

    /// CREATE STAGE tokens as SQL, writing FILE_FORMAT as FILE_FORMAT=(FORMAT_NAME=...).
    pub(crate) fn tokens_to_sql_stage_format(&self, start: usize, end: usize) -> String {
        let mut result = String::new();
        let mut prev_token_type: Option<TokenType> = None;
        let mut i = start;

        while i < end {
            let t = &self.tokens[i];

            if (t.token_type == TokenType::Var || t.token_type == TokenType::Identifier)
                && t.text.eq_ignore_ascii_case("FILE_FORMAT")
                && i + 1 < end
                && self.tokens[i + 1].token_type == TokenType::Eq
                && (i + 2 >= end || self.tokens[i + 2].token_type != TokenType::LParen)
            {
                if !result.is_empty() && prev_token_type != Some(TokenType::LParen) {
                    result.push(' ');
                }
                result.push_str("FILE_FORMAT=(FORMAT_NAME=");

                i += 2;

                if i < end {
                    let val = &self.tokens[i];
                    if val.token_type == TokenType::String {
                        result.push('\'');
                        result.push_str(&val.text.replace('\'', "''"));
                        result.push('\'');
                        i += 1;
                    } else if val.token_type == TokenType::Var
                        || val.token_type == TokenType::Identifier
                    {
                        result.push_str(&val.text);
                        i += 1;
                        if i < end && self.tokens[i].token_type == TokenType::Dot {
                            result.push('.');
                            i += 1;
                            if i < end {
                                result.push_str(&self.tokens[i].text);
                                i += 1;
                            }
                        }
                    }
                }
                result.push(')');
                prev_token_type = Some(TokenType::RParen);
                continue;
            }

            let needs_space = !result.is_empty()
                && prev_token_type != Some(TokenType::LParen)
                && prev_token_type != Some(TokenType::Eq)
                && prev_token_type != Some(TokenType::Dot)
                && t.token_type != TokenType::Comma
                && t.token_type != TokenType::RParen
                && t.token_type != TokenType::LParen
                && t.token_type != TokenType::Eq
                && t.token_type != TokenType::Dot;

            if needs_space {
                result.push(' ');
            }

            if t.token_type == TokenType::String {
                result.push('\'');
                result.push_str(&t.text.replace('\'', "''"));
                result.push('\'');
            } else {
                result.push_str(&t.text);
            }

            prev_token_type = Some(t.token_type);
            i += 1;
        }
        result
    }

    /// Like tokens_to_sql but also uppercases keyword tokens and adds space after commas
    pub(crate) fn tokens_to_sql_uppercased(&self, start: usize, end: usize) -> String {
        let mut result = String::new();
        let mut prev_token_type: Option<TokenType> = None;
        let mut prev_token_text: Option<String> = None;

        for t in &self.tokens[start..end] {
            let is_lparen_after_keyword = t.token_type == TokenType::LParen
                && prev_token_type.is_some_and(|p: TokenType| {
                    matches!(
                        p,
                        TokenType::PrimaryKey
                            | TokenType::ForeignKey
                            | TokenType::Unique
                            | TokenType::Check
                            | TokenType::Index
                            | TokenType::Key
                            | TokenType::Constraint
                            | TokenType::References
                            | TokenType::Not
                            | TokenType::Null
                            | TokenType::Default
                            | TokenType::Values
                            | TokenType::In
                            | TokenType::Exists
                            | TokenType::Select
                            | TokenType::From
                            | TokenType::Where
                            | TokenType::Having
                            | TokenType::Using
                            | TokenType::On
                            | TokenType::Set
                            | TokenType::Into
                            | TokenType::Table
                            | TokenType::View
                            | TokenType::Create
                            | TokenType::Insert
                            | TokenType::Update
                            | TokenType::Delete
                            | TokenType::Join
                            | TokenType::Left
                            | TokenType::Right
                            | TokenType::Inner
                            | TokenType::Outer
                            | TokenType::Full
                            | TokenType::Cross
                            | TokenType::Case
                            | TokenType::When
                            | TokenType::Then
                            | TokenType::Else
                            | TokenType::End
                            | TokenType::If
                            | TokenType::Partition
                            | TokenType::Over
                            | TokenType::Between
                            | TokenType::Like
                            | TokenType::Replace
                            | TokenType::Grant
                            | TokenType::Revoke
                    )
                })
                || t.token_type == TokenType::LParen
                    && prev_token_text.as_ref().is_some_and(|text| {
                        let upper = text.to_ascii_uppercase();
                        matches!(
                            upper.as_str(),
                            "CLUSTERED"
                                | "NONCLUSTERED"
                                | "HASH"
                                | "RANGE"
                                | "INCLUDE"
                                | "FILLFACTOR"
                                | "PAD_INDEX"
                        )
                    });
            let needs_space = !result.is_empty()
                && prev_token_type != Some(TokenType::LParen)
                && prev_token_type != Some(TokenType::Dot)
                && t.token_type != TokenType::Comma
                && t.token_type != TokenType::RParen
                && t.token_type != TokenType::Dot
                && (t.token_type != TokenType::LParen || is_lparen_after_keyword);

            if prev_token_type == Some(TokenType::Comma) || needs_space {
                result.push(' ');
            }

            if t.token_type == TokenType::String {
                result.push('\'');
                result.push_str(&t.text.replace('\'', "''"));
                result.push('\'');
            } else if t.token_type.is_keyword() {
                result.push_str(&t.text.to_ascii_uppercase());
            } else {
                result.push_str(&t.text);
            }

            prev_token_type = Some(t.token_type);
            prev_token_text = Some(t.text.to_string());
        }
        result
    }

    /// Check if current token matches type
    #[inline]
    pub(crate) fn check(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.peek().token_type == token_type
        }
    }

    /// Whether a token's text came from delimited source, so it is never read as a keyword.
    pub(crate) fn text_match_excludes_token(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::QuotedIdentifier
                | TokenType::String
                | TokenType::DollarString
                | TokenType::TripleDoubleQuotedString
                | TokenType::TripleSingleQuotedString
                | TokenType::BitString
                | TokenType::HexString
                | TokenType::ByteString
                | TokenType::NationalString
                | TokenType::EscapeString
                | TokenType::RawString
                | TokenType::HeredocString
                | TokenType::HeredocStringAlternative
                | TokenType::UnicodeString
        )
    }

    pub(crate) fn current_token_allows_text_match(&self) -> bool {
        !self.is_at_end() && !Self::text_match_excludes_token(self.peek().token_type)
    }

    /// Check if current token is a keyword
    pub(crate) fn check_keyword(&self) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.peek().token_type.is_keyword()
        }
    }

    /// Whether UNPIVOT starts a clause (`UNPIVOT(`, INCLUDE, EXCLUDE) rather than an alias.
    pub(crate) fn is_unpivot_clause_start(&self) -> bool {
        if !self.check(TokenType::Unpivot) {
            return false;
        }
        let next_idx = self.current + 1;
        if next_idx >= self.tokens.len() {
            return false;
        }
        let next = &self.tokens[next_idx];
        if next.token_type == TokenType::LParen {
            return true;
        }
        let next_text = next.text.to_ascii_uppercase();
        next_text == INCLUDE || next_text == EXCLUDE
    }

    /// Check if current token text matches (case-insensitive), does not advance
    pub(crate) fn check_keyword_text(&self, keyword: &str) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.current_token_allows_text_match() && self.peek_text().eq_ignore_ascii_case(keyword)
        }
    }
}
