//! Data models of parser.

use crate::parser::constants::POSTGRES_REPLICATION_COMMANDS;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::OnceLock;

use crate::expressions::names::models::TableRef;
use crate::expressions::queries::models::Hint;
use crate::expressions::queries::models::JoinKind;
use crate::expressions::queries::models::Limit;
use crate::expressions::queries::models::Offset;
use crate::expressions::queries::models::Over;
use crate::expressions::queries::models::SelectInto;
use crate::expressions::queries::models::Top;
use crate::expressions::schema_objects::models::FunctionSecurity;
use crate::expressions::tables::models::OnCluster;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::guard::main::enforce_input::enforce_input;
use crate::guard::models::ComplexityGuardOptions;
use crate::guard::models::ParseOptions;
use crate::guard::models::TokenGuardStats;
use crate::parser::_helpers::support::dialects::configs_for_dialect_type;
use crate::parser::types::DialectImpl;
use crate::tokens::errors::Error;
use crate::tokens::models::CallMode;
use crate::tokens::models::DialectType;
use crate::tokens::models::ParserToken;
use crate::tokens::models::Span;
use crate::tokens::models::TokenType;
use crate::tokens::models::Tokenizer;
use crate::tokens::models::TokenizerConfig;
use crate::tokens::types::Result;

/// Tokenizer and generator settings precomputed per dialect and cached in a `LazyLock`.
#[derive(Debug)]
pub(crate) struct CachedDialectConfig {
    pub(crate) tokenizer_config: Arc<TokenizerConfig>,
}

#[derive(Debug)]
pub(crate) struct DialectConfigs {
    pub(crate) tokenizer_config: Arc<TokenizerConfig>,
}

/// Dialect-specific SQL operations: tokenizing and parsing with one engine's settings.
#[derive(Debug)]
pub(crate) struct Dialect {
    pub(crate) dialect_type: DialectType,
    pub(crate) tokenizer: Tokenizer,
}

impl Dialect {
    /// A fully configured [`Dialect`] for `dialect_type`.
    pub(crate) fn get(dialect_type: DialectType) -> Self {
        let configs = configs_for_dialect_type(dialect_type);
        let tokenizer_config = configs.tokenizer_config;

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
        mode: CallMode,
    ) -> Result<Vec<Expression>> {
        let complexity_guard = options
            .complexity_guard
            .unwrap_or_else(|| self.default_complexity_guard());
        self.parse_with_config(sql, complexity_guard, mode)
    }

    #[cfg(test)]
    pub(crate) fn parse_with_guard(
        &self,
        sql: &str,
        complexity_guard: ComplexityGuardOptions,
    ) -> Result<Vec<Expression>> {
        self.parse_with_config(sql, complexity_guard, CallMode::Lowered)
    }

    pub(crate) fn parse_with_config(
        &self,
        sql: &str,
        complexity_guard: ComplexityGuardOptions,
        mode: CallMode,
    ) -> Result<Vec<Expression>> {
        enforce_input(sql, &complexity_guard)?;
        let source: Arc<str> = Arc::from(sql);
        let (tokens, token_guard_stats) = self.tokenizer.tokenize_for_parser(&source)?;
        let config = crate::parser::models::ParserConfig {
            dialect: Some(self.dialect_type),
            complexity_guard,
            faithful_calls: mode != CallMode::Lowered,
            record_call_spans: mode == CallMode::FaithfulWithSpans,
            record_node_spans: mode == CallMode::FaithfulLocated,
            ..Default::default()
        };
        let mut parser = Parser::with_parser_tokens(tokens, token_guard_stats, config, source);
        parser.parse()
    }

    pub(crate) fn default_complexity_guard(&self) -> ComplexityGuardOptions {
        ComplexityGuardOptions::default()
    }

    /// Parses text as a standalone data type, or `None` when its first token cannot start one.
    pub(crate) fn parse_data_type_text(&self, sql: &str) -> Result<Option<DataType>> {
        let mut parser = self.data_type_parser(sql, &ParseOptions::default())?;
        parser.parse_standalone_data_type_if_present()
    }

    #[cfg(test)]
    /// Parse a standalone data type using per-call complexity limits.
    pub(crate) fn parse_data_type_with_options(
        &self,
        sql: &str,
        options: &ParseOptions,
    ) -> Result<DataType> {
        let mut parser = self.data_type_parser(sql, options)?;
        parser.parse_standalone_data_type()
    }

    fn data_type_parser(&self, sql: &str, options: &ParseOptions) -> Result<Parser> {
        let complexity_guard = options
            .complexity_guard
            .unwrap_or_else(|| self.default_complexity_guard());
        enforce_input(sql, &complexity_guard)?;
        let source: Arc<str> = Arc::from(sql);
        let (tokens, token_guard_stats) = self.tokenizer.tokenize_for_parser(&source)?;
        let config = crate::parser::models::ParserConfig {
            dialect: Some(self.dialect_type),
            complexity_guard,
            ..Default::default()
        };
        Ok(Parser::with_parser_tokens(
            tokens,
            token_guard_stats,
            config,
            source,
        ))
    }

    #[cfg(test)]
    /// Tokenize SQL using this dialect's tokenizer configuration.
    pub(crate) fn tokenize(&self, sql: &str) -> Result<Vec<crate::tokens::models::Token>> {
        self.tokenizer.tokenize(sql)
    }
}

/// Generic SQL dialect (ANSI SQL)
#[derive(Debug)]
pub(crate) struct GenericDialect;

impl DialectImpl for GenericDialect {}

/// DuckDB dialect
#[derive(Debug)]
pub(crate) struct DuckDBDialect;

impl DialectImpl for DuckDBDialect {
    fn tokenizer_config(&self) -> TokenizerConfig {
        let mut config = TokenizerConfig::default();
        config.identifiers.insert('"', '"');
        config.quotes.remove("\"\"\"");
        config.nested_comments = true;
        config.numbers_can_be_underscore_separated = true;
        config.double_slash_int_div = true;
        config
    }
}

/// PostgreSQL dialect
#[derive(Debug)]
pub(crate) struct PostgresDialect;

impl DialectImpl for PostgresDialect {
    fn tokenizer_config(&self) -> TokenizerConfig {
        let mut config = TokenizerConfig::default();
        config.quotes.insert("$$".to_string(), "$$".to_string());
        config.identifiers.insert('"', '"');
        config.nested_comments = true;
        config.keywords.insert(
            "EXEC".to_string(),
            crate::tokens::models::TokenType::Command,
        );
        for command in POSTGRES_REPLICATION_COMMANDS {
            config.keywords.insert(
                command.to_string(),
                crate::tokens::models::TokenType::Command,
            );
        }
        config
    }
}

/// Snowflake dialect
#[derive(Debug)]
pub(crate) struct SnowflakeDialect;

impl DialectImpl for SnowflakeDialect {
    fn tokenizer_config(&self) -> TokenizerConfig {
        let mut config = TokenizerConfig::default();
        config.identifiers.insert('"', '"');
        config.quotes.insert("$$".to_string(), "$$".to_string());
        config.string_escapes.push('\\');
        config.escape_follow_chars = vec![
            '\'', '"', '\\', 'b', 'f', 'n', 'r', 't', '0', '1', '2', '3', '4', '5', '6', '7', 'x',
            'u',
        ];
        config.nested_comments = false;
        config.comments.insert("//".to_string(), None);
        config
    }
}

/// The spellings of a function name that registry parsing dispatches on.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FunctionCallNames<'a> {
    pub(crate) name: &'a str,
    pub(crate) upper_name: &'a str,
    pub(crate) canonical_upper_name: &'a str,
}

/// What `INSERT` has parsed before its target.
#[derive(Debug)]
pub(crate) struct InsertPrefix {
    pub(crate) leading_comments: Vec<String>,
    pub(crate) hint: Option<Hint>,
    pub(crate) conflict_action: Option<String>,
    pub(crate) ignore: bool,
    pub(crate) overwrite: bool,
}

/// The `LIMIT`, `OFFSET`, `LIMIT ... OVER` and `LIMIT ... BY` clauses of a `SELECT`.
#[derive(Debug)]
pub(crate) struct SelectLimitClauses {
    pub(crate) limit: Option<Limit>,
    pub(crate) offset: Option<Offset>,
    pub(crate) limit_over: Option<Over>,
    pub(crate) limit_by: Option<Vec<Expression>>,
}

/// What `CREATE TABLE` has parsed up to the table name, for the `PARTITION OF` form.
#[derive(Debug)]
pub(crate) struct CreateTableHead<'a> {
    pub(crate) name: TableRef,
    pub(crate) if_not_exists: bool,
    pub(crate) temporary: bool,
    pub(crate) or_replace: bool,
    pub(crate) table_modifier: Option<&'a str>,
    pub(crate) leading_comments: Vec<String>,
}

/// What `CREATE TABLE` has parsed right after the table name.
#[derive(Debug)]
pub(crate) struct CreateTableNameSuffix {
    pub(crate) on_cluster: Option<OnCluster>,
    pub(crate) uuid: Option<String>,
    pub(crate) teradata_post_name_options: Vec<String>,
}

/// How `CREATE TABLE ... CLONE` or `COPY` was introduced.
#[derive(Debug)]
pub(crate) struct CreateTableCloneKind {
    pub(crate) shallow_clone: bool,
    pub(crate) deep_clone: bool,
    pub(crate) is_copy: bool,
}

/// The `WITH (...)` properties and `COPY GRANTS` flag written before a `CREATE TABLE` body.
#[derive(Debug)]
pub(crate) struct CreateTableLeadingOptions {
    pub(crate) copy_grants: bool,
    pub(crate) with_properties: Vec<(String, String)>,
}

/// A join kind and how it was written: which keywords appeared, and its hint.
#[derive(Debug)]
pub(crate) struct JoinKindParts {
    pub(crate) kind: JoinKind,
    pub(crate) needs_join_keyword: bool,
    pub(crate) use_inner_keyword: bool,
    pub(crate) use_outer_keyword: bool,
    pub(crate) hint: Option<String>,
}

/// The `[STRICT] CORRESPONDING` or `BY NAME` modifiers after a set operator.
#[derive(Debug)]
pub(crate) struct SetOperationCorresponding {
    pub(crate) by_name: bool,
    pub(crate) strict: bool,
    pub(crate) corresponding: bool,
    pub(crate) on_columns: Vec<Expression>,
}

/// The modifiers written before `VIEW` in `CREATE ... VIEW`.
#[derive(Debug)]
pub(crate) struct ViewModifiers {
    pub(crate) or_replace: bool,
    pub(crate) or_alter: bool,
    pub(crate) materialized: bool,
    pub(crate) temporary: bool,
    pub(crate) algorithm: Option<String>,
    pub(crate) definer: Option<String>,
    pub(crate) security: Option<FunctionSecurity>,
    pub(crate) secure: bool,
}

/// Budget shared by the parsers of one parse, fragments included; atomic to stay Send and Sync.
#[derive(Debug, Default)]
pub(crate) struct ParserRecursion {
    pub(crate) depth: AtomicUsize,
    pub(crate) failure: OnceLock<(usize, usize, Span)>,
}

impl ParserRecursion {
    pub(crate) fn error(&self) -> Option<Error> {
        self.failure.get().map(|(actual, limit, span)| {
            Error::parse(format!("E_GUARD_PARSER_DEPTH_EXCEEDED: value {actual} exceeds configured limit {limit}"), *span)
        })
    }
}

#[derive(Debug)]
pub(crate) struct ParserDepthScope {
    pub(crate) recursion: Arc<ParserRecursion>,
    pub(crate) levels: usize,
}

impl Drop for ParserDepthScope {
    fn drop(&mut self) {
        self.recursion
            .depth
            .fetch_sub(self.levels, Ordering::Relaxed);
    }
}

/// Recursive-descent SQL parser from a token stream to [`Expression`] trees.
#[derive(Debug)]
pub(crate) struct Parser {
    pub(crate) tokens: Vec<ParserToken>,
    pub(crate) current: usize,
    pub(crate) config: ParserConfig,
    /// Original source SQL (used for preserving exact text in Command expressions)
    pub(crate) source: Option<Arc<str>>,
    /// Leading comments parse_comparison captured without an operator, for the caller to place.
    pub(crate) pending_leading_comments: Vec<String>,
    /// Whether token-level complexity guards have been applied to this token stream.
    pub(crate) guards_checked: bool,
    /// Token statistics collected during the internal zero-copy tokenization path.
    pub(crate) token_guard_stats: Option<TokenGuardStats>,
    /// The token after a caller-supplied `TokenType::Eof`: its type and span, for the error.
    pub(crate) eof_followed_by: Option<(TokenType, Span)>,
    /// Positions where `IF` was rejected as an if-expression, so retries stay linear.
    pub(crate) if_expr_ruled_out: HashSet<usize>,
    pub(crate) parsing_duckdb_json_path: bool,
    pub(crate) recursion: Arc<ParserRecursion>,
    /// SQLQuack: faithful calls parsed so far, by argument start token, with the token after them.
    pub(crate) faithful_calls: HashMap<usize, (Expression, usize)>,
}

/// Parser configuration; set `dialect` for engine-specific syntax.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParserConfig {
    /// Allow trailing commas in SELECT lists (e.g. BigQuery permits `SELECT a, b, FROM t`).
    pub allow_trailing_commas: bool,
    /// Dialect type for dialect-specific parsing behavior.
    pub dialect: Option<crate::tokens::models::DialectType>,
    /// Complexity guards for recursion-heavy parse paths.
    pub complexity_guard: ComplexityGuardOptions,
    /// SQLQuack: in Snowflake, keep calls as written; only keyword-syntax calls become typed nodes.
    pub faithful_calls: bool,
    /// SQLQuack: record the source span of every call parsed faithfully (for the exact check).
    pub record_call_spans: bool,
    /// SQLQuack: record each expression and statement node's source range (for locations).
    pub record_node_spans: bool,
}

#[derive(Debug)]
pub(crate) struct SelectBodyHead {
    pub(crate) leading_comments: Vec<String>,
    pub(crate) post_select_comments: Vec<String>,
    pub(crate) hint: Option<Hint>,
    pub(crate) top: Option<Top>,
    pub(crate) distinct: bool,
    pub(crate) distinct_on: Option<Vec<Expression>>,
    pub(crate) kind: Option<String>,
    pub(crate) operation_modifiers: Vec<String>,
    pub(crate) expressions: Vec<Expression>,
    pub(crate) exclude: Option<Vec<Expression>>,
    pub(crate) into: Option<SelectInto>,
}
