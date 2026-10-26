//! Dialects: per-engine tokenizer settings, parser behaviour and source semantics.

mod generic; // Always compiled

mod duckdb;
mod snowflake;

pub(crate) use generic::GenericDialect; // Always available

pub(crate) use duckdb::DuckDBDialect;
pub(crate) use snowflake::SnowflakeDialect;

use crate::error::Result;
use crate::expressions::{DataType, Expression};
use crate::guard::{enforce_input, ComplexityGuardOptions, ParseOptions};
use crate::parser::Parser;
use crate::tokens::{Tokenizer, TokenizerConfig};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};

/// Every supported SQL dialect; `Generic` is standard SQL and the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DialectType {
    /// Standard SQL with no dialect-specific behavior (default).
    #[default]
    Generic,
    /// Snowflake -- cloud data platform with QUALIFY clause, FLATTEN, and variant types.
    Snowflake,
    /// DuckDB -- in-process analytical database with modern SQL extensions.
    DuckDB,
}

impl DialectType {
    /// Whether projections may use string literals as aliases (DuckDB only with an explicit AS).
    pub(crate) const fn supports_string_aliases(self, explicit_as: bool) -> bool {
        (matches!(self, DialectType::DuckDB) && explicit_as)
    }

    /// Whether `ORDER BY ALL` (every projected column, in order) is accepted natively.
    pub(crate) const fn supports_order_by_all(self) -> bool {
        matches!(self, DialectType::DuckDB | DialectType::Snowflake)
    }
}

impl std::fmt::Display for DialectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialectType::Generic => write!(f, "generic"),

            DialectType::Snowflake => write!(f, "snowflake"),
            DialectType::DuckDB => write!(f, "duckdb"),
        }
    }
}

impl std::str::FromStr for DialectType {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "generic" | "" => Ok(DialectType::Generic),

            "snowflake" => Ok(DialectType::Snowflake),
            "duckdb" => Ok(DialectType::DuckDB),

            _ => Err(crate::error::Error::parse(
                format!("Unknown dialect: {}", s),
                0,
                0,
                0,
                0,
            )),
        }
    }
}

/// Configuration hooks and per-expression behaviour that distinguish one dialect from another.
pub(crate) trait DialectImpl {
    /// This dialect's tokenizer settings: quoting, string escapes, comment styles and other lexing.
    fn tokenizer_config(&self) -> TokenizerConfig {
        TokenizerConfig::default()
    }
}

// ---------------------------------------------------------------------------
// Cached dialect configurations
// ---------------------------------------------------------------------------

/// Tokenizer and generator settings precomputed per dialect and cached in a `LazyLock`.
struct CachedDialectConfig {
    tokenizer_config: Arc<TokenizerConfig>,
}

struct DialectConfigs {
    tokenizer_config: Arc<TokenizerConfig>,
}

/// A dialect's cached configuration.
fn cached(dialect: impl DialectImpl) -> CachedDialectConfig {
    CachedDialectConfig {
        tokenizer_config: Arc::new(dialect.tokenizer_config()),
    }
}

static CACHED_GENERIC: LazyLock<CachedDialectConfig> = LazyLock::new(|| cached(GenericDialect));
static CACHED_SNOWFLAKE: LazyLock<CachedDialectConfig> = LazyLock::new(|| cached(SnowflakeDialect));
static CACHED_DUCKDB: LazyLock<CachedDialectConfig> = LazyLock::new(|| cached(DuckDBDialect));

fn configs_for_dialect_type(dt: DialectType) -> DialectConfigs {
    let cache: &CachedDialectConfig = match dt {
        DialectType::Snowflake => &CACHED_SNOWFLAKE,
        DialectType::DuckDB => &CACHED_DUCKDB,
        DialectType::Generic => &CACHED_GENERIC,
    };
    DialectConfigs {
        tokenizer_config: cache.tokenizer_config.clone(),
    }
}

// ---------------------------------------------------------------------------
// Custom dialect registry
// ---------------------------------------------------------------------------

/// Dialect-specific SQL operations: tokenizing and parsing with one engine's settings.
pub(crate) struct Dialect {
    dialect_type: DialectType,
    tokenizer: Tokenizer,
}

impl Dialect {
    /// A fully configured [`Dialect`] for `dialect_type`.
    pub(crate) fn get(dialect_type: DialectType) -> Self {
        let configs = configs_for_dialect_type(dialect_type);
        let tokenizer_config = configs.tokenizer_config;

        // Set up expression-specific generator config for hybrid dialects

        Self {
            dialect_type,
            tokenizer: Tokenizer::from_shared_config(tokenizer_config),
        }
    }

    #[cfg(test)]
    /// Parses SQL, possibly several semicolon-separated statements, into one AST per statement.
    pub(crate) fn parse(&self, sql: &str) -> Result<Vec<Expression>> {
        self.parse_with_guard(sql, self.default_complexity_guard())
    }

    #[cfg(test)]
    /// Parse using per-call limits without changing this dialect's defaults.
    pub(crate) fn parse_with_options(
        &self,
        sql: &str,
        options: &ParseOptions,
    ) -> Result<Vec<Expression>> {
        self.parse_with_guard(
            sql,
            options
                .complexity_guard
                .unwrap_or_else(|| self.default_complexity_guard()),
        )
    }

    /// SQLQuack: parses with polyglot's call lowering (`faithful_calls` false) or without it.
    pub(crate) fn parse_with_mode(
        &self,
        sql: &str,
        options: &ParseOptions,
        (faithful_calls, record_call_spans): (bool, bool),
    ) -> Result<Vec<Expression>> {
        let complexity_guard = options
            .complexity_guard
            .unwrap_or_else(|| self.default_complexity_guard());
        self.parse_with_config(sql, complexity_guard, (faithful_calls, record_call_spans))
    }

    #[cfg(test)]
    fn parse_with_guard(
        &self,
        sql: &str,
        complexity_guard: ComplexityGuardOptions,
    ) -> Result<Vec<Expression>> {
        self.parse_with_config(sql, complexity_guard, (false, false))
    }

    fn parse_with_config(
        &self,
        sql: &str,
        complexity_guard: ComplexityGuardOptions,
        (faithful_calls, record_call_spans): (bool, bool),
    ) -> Result<Vec<Expression>> {
        enforce_input(sql, &complexity_guard)?;
        let source: Arc<str> = Arc::from(sql);
        let (tokens, token_guard_stats) = self.tokenizer.tokenize_for_parser(&source)?;
        let config = crate::parser::ParserConfig {
            dialect: Some(self.dialect_type),
            complexity_guard,
            faithful_calls,
            record_call_spans,
            ..Default::default()
        };
        let mut parser = Parser::with_parser_tokens(tokens, token_guard_stats, config, source);
        parser.parse()
    }

    fn default_complexity_guard(&self) -> ComplexityGuardOptions {
        ComplexityGuardOptions::default()
    }

    /// Parses a standalone data type such as `DECIMAL(10, 2)` or `STRUCT(a INT, b VARCHAR)`.
    pub(crate) fn parse_data_type(&self, sql: &str) -> Result<DataType> {
        self.parse_data_type_with_options(sql, &ParseOptions::default())
    }

    /// Parse a standalone data type using per-call complexity limits.
    pub(crate) fn parse_data_type_with_options(
        &self,
        sql: &str,
        options: &ParseOptions,
    ) -> Result<DataType> {
        let complexity_guard = options
            .complexity_guard
            .unwrap_or_else(|| self.default_complexity_guard());
        enforce_input(sql, &complexity_guard)?;
        let source: Arc<str> = Arc::from(sql);
        let (tokens, token_guard_stats) = self.tokenizer.tokenize_for_parser(&source)?;
        let config = crate::parser::ParserConfig {
            dialect: Some(self.dialect_type),
            complexity_guard,
            ..Default::default()
        };
        let mut parser = Parser::with_parser_tokens(tokens, token_guard_stats, config, source);
        parser.parse_standalone_data_type()
    }

    #[cfg(test)]
    /// Tokenize SQL using this dialect's tokenizer configuration.
    pub(crate) fn tokenize(&self, sql: &str) -> Result<Vec<crate::tokens::Token>> {
        self.tokenizer.tokenize(sql)
    }
}

// Transpile-only methods: cross-dialect normalization and helpers

#[cfg(test)]
mod tests {

    // =========================================================================
    // REGEXP function transpilation tests
    // =========================================================================
}
