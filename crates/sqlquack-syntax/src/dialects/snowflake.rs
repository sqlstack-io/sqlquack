//! The Snowflake dialect: TRY_ functions, FLATTEN, QUALIFY, VARIANT and OBJECT/ARRAY constructors.

use super::DialectImpl;
use crate::tokens::TokenizerConfig;

/// Snowflake dialect
pub(crate) struct SnowflakeDialect;

impl DialectImpl for SnowflakeDialect {
    fn tokenizer_config(&self) -> TokenizerConfig {
        let mut config = TokenizerConfig::default();
        // Snowflake uses double quotes for identifiers
        config.identifiers.insert('"', '"');
        // Snowflake supports $$ string literals
        config.quotes.insert("$$".to_string(), "$$".to_string());
        // Snowflake accepts both doubled quotes and backslash escape sequences.
        config.string_escapes.push('\\');
        // Unknown backslash sequences discard the backslash. Listing the supported
        // followers also prevents MySQL-only escapes from being decoded here.
        config.escape_follow_chars = vec![
            '\'', '"', '\\', 'b', 'f', 'n', 'r', 't', '0', '1', '2', '3', '4', '5', '6', '7', 'x',
            'u',
        ];
        // Snowflake does NOT support nested comments (per Python sqlglot)
        config.nested_comments = false;
        // Snowflake supports // as single-line comments (in addition to --)
        config.comments.insert("//".to_string(), None);
        config
    }
}

#[cfg(test)]
mod tests {}
