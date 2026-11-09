//! Parser support: dialects.

use std::sync::Arc;
use std::sync::LazyLock;

use crate::parser::models::CachedDialectConfig;
use crate::parser::models::DialectConfigs;
use crate::parser::models::DuckDBDialect;
use crate::parser::models::GenericDialect;
use crate::parser::models::PostgresDialect;
use crate::parser::models::SnowflakeDialect;
use crate::parser::models::SqliteDialect;
use crate::parser::types::DialectImpl;
use sqlquack_syntax::tokens::errors::Error;
use sqlquack_syntax::tokens::models::Dialect;
use sqlquack_syntax::tokens::models::DialectType;
use sqlquack_syntax::tokens::models::TokenizerConfig;
use sqlquack_syntax::tokens::types::Result;

pub(crate) static CACHED_GENERIC: LazyLock<CachedDialectConfig> =
    LazyLock::new(|| cached(GenericDialect));

pub(crate) static CACHED_SNOWFLAKE: LazyLock<CachedDialectConfig> =
    LazyLock::new(|| cached(SnowflakeDialect));

pub(crate) static CACHED_DUCKDB: LazyLock<CachedDialectConfig> =
    LazyLock::new(|| cached(DuckDBDialect));

pub(crate) static CACHED_POSTGRES: LazyLock<CachedDialectConfig> =
    LazyLock::new(|| cached(PostgresDialect));

pub(crate) static CACHED_SQLITE: LazyLock<CachedDialectConfig> =
    LazyLock::new(|| cached(SqliteDialect));

/// A dialect's cached configuration.
pub(crate) fn cached(dialect: impl DialectImpl) -> CachedDialectConfig {
    CachedDialectConfig {
        tokenizer_config: Arc::new(dialect.tokenizer_config()),
    }
}

pub(crate) fn configs_for_dialect_type(dt: DialectType) -> DialectConfigs {
    let cache: &CachedDialectConfig = match dt {
        DialectType::Snowflake => &CACHED_SNOWFLAKE,
        DialectType::DuckDB => &CACHED_DUCKDB,
        DialectType::PostgreSQL => &CACHED_POSTGRES,
        DialectType::SQLite => &CACHED_SQLITE,
        DialectType::Generic => &CACHED_GENERIC,
    };
    DialectConfigs {
        tokenizer_config: cache.tokenizer_config.clone(),
    }
}

/// The parser configuration for `dialect`, or why this parser does not read it.
pub(crate) fn dialect_type(dialect: Dialect) -> Result<DialectType> {
    match dialect {
        Dialect::Snowflake => Ok(DialectType::Snowflake),
        Dialect::DuckDB => Ok(DialectType::DuckDB),
        Dialect::PostgreSQL => Ok(DialectType::PostgreSQL),
        Dialect::SQLite => Ok(DialectType::SQLite),
        other => Err(Error::unsupported("parsing", format!("{other:?}"))),
    }
}

/// Whether projections may use string literals as aliases (DuckDB only with an explicit AS).
pub(crate) const fn supports_string_aliases(dialect: DialectType, explicit_as: bool) -> bool {
    matches!(dialect, DialectType::DuckDB) && explicit_as
}

/// Whether `ORDER BY ALL` (every projected column, in order) is accepted natively.
pub(crate) const fn supports_order_by_all(dialect: DialectType) -> bool {
    matches!(dialect, DialectType::DuckDB | DialectType::Snowflake)
}

/// The tokenizer configuration of `dialect`, or why this parser does not read it.
pub(crate) fn tokenizer_config(dialect: Dialect) -> Result<TokenizerConfig> {
    match dialect {
        Dialect::Snowflake => Ok(SnowflakeDialect.tokenizer_config()),
        Dialect::DuckDB => Ok(DuckDBDialect.tokenizer_config()),
        Dialect::PostgreSQL => Ok(PostgresDialect.tokenizer_config()),
        Dialect::SQLite => Ok(SqliteDialect.tokenizer_config()),
        other => Err(Error::unsupported("tokenizing", format!("{other:?}"))),
    }
}
