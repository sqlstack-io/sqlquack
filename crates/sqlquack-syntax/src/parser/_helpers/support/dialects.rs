//! Parser support: dialects.

use std::sync::Arc;
use std::sync::LazyLock;

use crate::parser::models::CachedDialectConfig;
use crate::parser::models::DialectConfigs;
use crate::parser::models::DuckDBDialect;
use crate::parser::models::GenericDialect;
use crate::parser::models::PostgresDialect;
use crate::parser::models::SnowflakeDialect;
use crate::parser::types::DialectImpl;
use crate::tokens::models::DialectType;

pub(crate) static CACHED_GENERIC: LazyLock<CachedDialectConfig> =
    LazyLock::new(|| cached(GenericDialect));

pub(crate) static CACHED_SNOWFLAKE: LazyLock<CachedDialectConfig> =
    LazyLock::new(|| cached(SnowflakeDialect));

pub(crate) static CACHED_DUCKDB: LazyLock<CachedDialectConfig> =
    LazyLock::new(|| cached(DuckDBDialect));

pub(crate) static CACHED_POSTGRES: LazyLock<CachedDialectConfig> =
    LazyLock::new(|| cached(PostgresDialect));

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
        DialectType::Generic => &CACHED_GENERIC,
    };
    DialectConfigs {
        tokenizer_config: cache.tokenizer_config.clone(),
    }
}
