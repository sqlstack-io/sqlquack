//! Type aliases and traits of parser.

use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::tables::models::TiDBTableOption;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::{Span, TokenType, TokenizerConfig};

/// Configuration hooks and per-expression behaviour that distinguish one dialect from another.
pub(crate) trait DialectImpl {
    /// This dialect's tokenizer settings: quoting, string escapes, comment styles and other lexing.
    fn tokenizer_config(&self) -> TokenizerConfig {
        TokenizerConfig::default()
    }
}

/// MySQL table options and the TiDB options among them.
pub(crate) type MySqlTableOptions = (Vec<(String, String)>, Vec<TiDBTableOption>);

/// Snowflake inline table properties and the `WITH` properties parsed beside them.
pub(crate) type CreateTableInlineProperties = (Vec<Expression>, Vec<(String, String)>);

/// The `PARTITION BY` expression and the `PARTITION (...)` values of an `INSERT`.
pub(crate) type InsertPartition = (
    Option<Box<Expression>>,
    Vec<(Identifier, Option<Expression>)>,
);

/// Each significant token's span and type, in the order the parser read them.
pub(crate) type SignificantTokens = Vec<(Span, TokenType)>;
