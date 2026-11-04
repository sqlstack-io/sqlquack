//! Strict parsing never panics, and every refusal it causes renders with a code and an in-bounds span.

#![no_main]

use libfuzzer_sys::fuzz_target;
use sqlquack_core::diagnostics::main::{diagnose::diagnose, render_diagnostics::render_diagnostics};
use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::{SourceDialect, SourceLines};
use sqlquack_core::refusals::main::refuse_unparsed::refuse_unparsed;

fuzz_target!(|data: &[u8]| {
    let Ok(sql) = std::str::from_utf8(data) else {
        return;
    };
    let Err(failure) = parse_source(sql, SourceDialect::Snowflake) else {
        return;
    };
    let refusal = refuse_unparsed(&failure, &SourceLines::new(sql));
    let diagnostics = diagnose(std::slice::from_ref(&refusal), sql);
    let length = sql.chars().count();
    for diagnostic in &diagnostics {
        assert!(diagnostic.code.starts_with("SQ"));
        if let Some(span) = diagnostic.span {
            assert!(span.start <= span.end && span.end <= length, "{span:?}");
            assert!(span.line >= 1 && span.column >= 1 && span.end_line >= span.line);
        }
    }
    assert!(render_diagnostics(&diagnostics, sql, "query.sql").starts_with("error[SQ"));
});
