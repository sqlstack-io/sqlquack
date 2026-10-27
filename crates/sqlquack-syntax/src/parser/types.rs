//! Type aliases and traits of parser.

use crate::expressions::names::models::Identifier;
use crate::expressions::tables::models::TiDBTableOption;
use crate::expressions::tree::models::Expression;
use crate::tokens::models::TokenizerConfig;

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
