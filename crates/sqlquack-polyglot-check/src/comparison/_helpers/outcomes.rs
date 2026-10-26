//! Parse results rendered as comparable text, and the lossless-token check.

use std::panic::{AssertUnwindSafe, catch_unwind};

use sqlquack_syntax::{Dialect, TokenizeFailure, tokenize};

/// `ok:` and the serialised trees, `err:` and the error's debug form, or `panic`.
pub(crate) fn rendered<T: serde::Serialize, E: std::fmt::Debug>(
    parse: impl FnOnce() -> Result<T, E>,
) -> String {
    match catch_unwind(AssertUnwindSafe(parse)) {
        Ok(Ok(trees)) => match serde_json::to_string(&trees) {
            Ok(json) => format!("ok:{json}"),
            Err(error) => format!("unserialisable:{error}"),
        },
        Ok(Err(error)) => format!("err:{error:?}"),
        Err(_) => "panic".to_owned(),
    }
}

/// Why the lossless tokens of `sql` do not reproduce it, if they do not.
pub(crate) fn lossless_failure(sql: &str, dialect: Dialect, parsed: bool) -> Option<String> {
    let tokens = match catch_unwind(|| tokenize(sql, dialect)) {
        Ok(Ok(tokens)) => tokens,
        Ok(Err(TokenizeFailure::Tokenizer(_))) if !parsed => return None,
        Ok(Err(failure)) => return Some(failure.to_string()),
        Err(_) => return Some("the tokenizer panicked".to_owned()),
    };
    let mut cursor = 0;
    for token in &tokens {
        if token.start != cursor || token.end <= token.start {
            return Some(format!(
                "token {}..{} does not continue at byte {cursor}",
                token.start, token.end
            ));
        }
        cursor = token.end;
    }
    match cursor == sql.len() {
        true => None,
        false => Some(format!("tokens end at byte {cursor} of {}", sql.len())),
    }
}

/// polyglot's Snowflake rendering of `data_type`, cut to its upper-case base name.
pub(crate) fn polyglot_base_type_name(
    data_type: &sqlquack_syntax::DataType,
    dialect: polyglot_sql::DialectType,
) -> String {
    let converted =
        serde_json::to_value(data_type).and_then(serde_json::from_value::<polyglot_sql::DataType>);
    let theirs = match converted {
        Ok(theirs) => theirs,
        Err(_) => return "<unconvertible>".to_owned(),
    };
    match polyglot_sql::generate_data_type(&theirs, dialect) {
        Ok(text) => text
            .split(['(', '<', ' '])
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_uppercase(),
        Err(_) => "<unprintable>".to_owned(),
    }
}
