//! BigQuery's tokenizer, as SQLClean read it.

use std::sync::LazyLock;

use sqlquack_syntax::tokens::models::{Tokenizer, TokenizerConfig};

/// BigQuery's tokens as SQLClean read them: backtick names, three string quotes, `#` comments.
pub(crate) static BIGQUERY_TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| {
    let mut config: TokenizerConfig = TokenizerConfig::default();
    config.identifiers.remove(&'"');
    config.identifiers.insert('`', '`');
    config.identifier_backslash_escapes = true;
    for quote in ["\"", "'''", "\"\"\""] {
        config.quotes.insert(quote.to_owned(), quote.to_owned());
    }
    config.string_escapes = vec!['\'', '\\'];
    config.b_prefix_is_byte_string = true;
    config.hex_number_strings = true;
    config.hex_string_is_integer_type = true;
    config.hash_comments = true;
    Tokenizer::new(config)
});
