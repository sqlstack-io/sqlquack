//! Databricks's tokenizer, as SQLClean read it.

use std::sync::LazyLock;

use sqlquack_syntax::tokens::models::{TokenType, Tokenizer, TokenizerConfig};

/// Databricks' tokens as SQLClean read them: backtick names, `"` strings, `\` escapes, typed number suffixes.
pub(crate) static DATABRICKS_TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| {
    let mut config: TokenizerConfig = TokenizerConfig::default();
    config.identifiers.clear();
    config.identifiers.insert('`', '`');
    config.quotes.insert("\"".to_owned(), "\"".to_owned());
    config.string_escapes.push('\\');
    config.keywords.insert("DIV".to_owned(), TokenType::Div);
    for command in ["REPAIR", "MSCK"] {
        config
            .keywords
            .insert(command.to_owned(), TokenType::Command);
    }
    for (suffix, kind) in [
        ("L", "BIGINT"),
        ("S", "SMALLINT"),
        ("Y", "TINYINT"),
        ("D", "DOUBLE"),
        ("F", "FLOAT"),
        ("BD", "DECIMAL"),
    ] {
        config
            .numeric_literals
            .insert(suffix.to_owned(), kind.to_owned());
    }
    config.identifiers_can_start_with_digit = true;
    config.string_escapes_allowed_in_raw_strings = false;
    Tokenizer::new(config)
});
