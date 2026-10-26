//! Tolerant parsing never panics, and its tokens always reproduce the input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use sqlquack_syntax::{parse_tolerant, Dialect, ParseOptions};

fuzz_target!(|data: &[u8]| {
    let Ok(sql) = std::str::from_utf8(data) else {
        return;
    };
    for dialect in [Dialect::Snowflake, Dialect::DuckDB] {
        let parse = parse_tolerant(sql, dialect, &ParseOptions::default());
        if parse.token_failure.is_none() {
            let joined: String = parse.tokens.iter().map(|token| token.text(sql)).collect();
            assert_eq!(joined, sql);
        }
    }
});
