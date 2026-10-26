//! `cast_target`: the type a Snowflake type name denotes.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::typing::_helpers::targets::{fractional, number_target, parse_arguments, text_target};
use crate::typing::constants::{CAST_TARGET, NUMBER_38_0};
use crate::typing::models::{CastTarget, SnowflakeType};

/// The type a Snowflake type name denotes, under `TIMESTAMP_TYPE_MAPPING = TIMESTAMP_NTZ`.
pub fn cast_target(name: &str, log: &ProvenanceLog) -> Result<CastTarget, String> {
    log.typing(CAST_TARGET);
    let compact: String = name
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    let (base, arguments) = match compact.split_once('(') {
        Some((base, rest)) => (base.to_owned(), parse_arguments(rest, name)?),
        None => (compact.clone(), Vec::new()),
    };
    let plain = |data_type: SnowflakeType| -> Result<CastTarget, String> {
        if arguments.is_empty() {
            Ok(CastTarget {
                data_type,
                max_length: None,
            })
        } else {
            Err(format!("type `{name}` takes no arguments"))
        }
    };
    match base.as_str() {
        "NUMBER" | "DECIMAL" | "NUMERIC" => number_target(&arguments, name),
        "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" | "BYTEINT" => plain(NUMBER_38_0),
        "FLOAT" | "FLOAT4" | "FLOAT8" | "DOUBLE" | "DOUBLEPRECISION" | "REAL" => {
            plain(SnowflakeType::Float)
        }
        "VARCHAR" | "STRING" | "TEXT" | "NVARCHAR" | "NVARCHAR2" | "CHARVARYING" => {
            text_target(&arguments, None, name)
        }
        "CHAR" | "CHARACTER" | "NCHAR" => text_target(&arguments, Some(1), name),
        "BOOLEAN" => plain(SnowflakeType::Boolean),
        "DATE" => plain(SnowflakeType::Date),
        "TIME" => fractional(&arguments, name).map(|precision| CastTarget {
            data_type: SnowflakeType::Time { precision },
            max_length: None,
        }),
        "TIMESTAMP" | "TIMESTAMP_NTZ" | "TIMESTAMPNTZ" | "DATETIME" => fractional(&arguments, name)
            .map(|precision| CastTarget {
                data_type: SnowflakeType::TimestampNtz { precision },
                max_length: None,
            }),
        "TIMESTAMP_TZ" | "TIMESTAMPTZ" => {
            fractional(&arguments, name).map(|precision| CastTarget {
                data_type: SnowflakeType::TimestampTz { precision },
                max_length: None,
            })
        }
        "TIMESTAMP_LTZ" | "TIMESTAMPLTZ" => {
            fractional(&arguments, name).map(|precision| CastTarget {
                data_type: SnowflakeType::TimestampLtz { precision },
                max_length: None,
            })
        }
        "VARIANT" | "OBJECT" | "ARRAY" => plain(SnowflakeType::SemiStructured),
        _ => Err(format!("unknown Snowflake type `{name}`")),
    }
}
