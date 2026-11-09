//! Seeded SQL-shaped inputs and the properties every parse and diagnostic of them must keep.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use sqlquack_syntax::{Dialect, ParseOptions};
use sqlquack_syntax_snowflake::parse_tolerant;

use crate::diagnostics::constants::MAX_SHOWN_LINE;
use crate::diagnostics::main::{diagnose::diagnose, render_diagnostics::render_diagnostics};
use crate::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_parse::parsing::main::parse_source::parse_source;
use sqlquack_syntax::tokens::models::SourceLines;

/// Pieces of Snowflake SQL, including ones that open or close what they cannot finish.
const FRAGMENTS: &[&str] = &[
    "SELECT ",
    "FROM ",
    "WHERE ",
    "GROUP BY ",
    "ORDER BY ",
    "QUALIFY ",
    "JOIN ",
    "ON ",
    "AS ",
    "CASE ",
    "WHEN ",
    "THEN ",
    "ELSE ",
    "END ",
    "WITH ",
    "UNION ALL ",
    "LATERAL FLATTEN(",
    "orders",
    "o.order_id",
    "customers",
    "\"Quoted Name\"",
    "price",
    "1",
    "2.50",
    "-3e10",
    "'text'",
    "'unterminated",
    "$$dollar$$",
    "$$open",
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    ",",
    ";",
    "+",
    "-",
    "*",
    "/",
    "%",
    "||",
    "::",
    ":",
    ".",
    "=",
    "<>",
    "<=",
    "IS NULL",
    "NOT ",
    "AND ",
    "OR ",
    "COUNT(*)",
    "IFF(",
    "TO_DATE(",
    "ARRAY_CONSTRUCT(",
    "OVER (PARTITION BY ",
    "/* c */",
    "/*",
    "*/",
    "-- line\n",
    "\n",
    "\t",
    "\r\n",
    " ",
    "é",
    "中",
    "🦆",
    "\u{0}",
    "\\",
    "?",
    "@",
    "$1",
    "IDENTIFIER(",
    "TRY_CAST(",
    "INTERVAL '1 day'",
    "TIMESTAMP_TZ",
    "VARIANT",
];

/// The next state of a xorshift64 generator; never zero for a non-zero seed.
fn next(state: u64) -> u64 {
    let state = state ^ (state << 13);
    let state = state ^ (state >> 7);
    state ^ (state << 17)
}

/// `inputs` SQL-shaped strings from `seed`: random fragments, some nested deep.
pub(crate) fn generated_inputs(seed: u64, inputs: usize) -> Vec<String> {
    std::iter::successors(Some(next(seed | 1)), |state| Some(next(*state)))
        .take(inputs)
        .map(|state| {
            let length = (state % 48) as usize;
            let nests = usize::from(state % 11 == 0);
            let nesting = nests * ((state >> 8) as usize % 1200);
            let pieces: String =
                std::iter::successors(Some(next(state)), |piece| Some(next(*piece)))
                    .take(length)
                    .map(|piece| FRAGMENTS[(piece as usize) % FRAGMENTS.len()])
                    .collect();
            format!("{}{pieces}{}", "(".repeat(nesting), ")".repeat(nesting / 2))
        })
        .collect()
}

/// What `input` breaks: a panic, a slow parse, a lost token, or a diagnostic out of bounds.
pub(crate) fn violations_of(input: &str, limit: Duration) -> Vec<String> {
    let started = Instant::now();
    let strict = catch_unwind(AssertUnwindSafe(|| parse_source(input, Dialect::Snowflake)));
    let tolerant = catch_unwind(AssertUnwindSafe(|| {
        parse_tolerant(input, Dialect::Snowflake, &ParseOptions::default())
    }));
    let elapsed = started.elapsed();
    let length = input.chars().count();
    let lines = input.split('\n').count();
    let rendered = strict
        .as_ref()
        .ok()
        .and_then(|parsed| parsed.as_ref().err())
        .map(|failure| {
            let refusal = refuse_unparsed(failure, &SourceLines::new(input));
            let diagnostics = diagnose(std::slice::from_ref(&refusal), input);
            let text = catch_unwind(AssertUnwindSafe(|| {
                render_diagnostics(&diagnostics, input, "query.sql")
            }));
            (diagnostics, text)
        });
    let checks = [
        (strict.is_err(), "strict parsing panicked"),
        (tolerant.is_err(), "tolerant parsing panicked"),
        (elapsed > limit, "parsing took longer than its limit"),
        (
            tolerant.as_ref().is_ok_and(|parse| {
                parse.token_failure.is_none()
                    && parse
                        .tokens
                        .iter()
                        .map(|token| token.text(input))
                        .collect::<String>()
                        != input
            }),
            "tolerant tokens do not reproduce the input",
        ),
        (
            rendered.as_ref().is_some_and(|(diagnostics, _)| {
                diagnostics.iter().any(|diagnostic| {
                    diagnostic.span.is_none_or(|span| {
                        span.start > span.end
                            || span.end > length
                            || span.line == 0
                            || span.column == 0
                            || span.line > lines
                            || span.end_line < span.line
                    })
                })
            }),
            "a parse diagnostic has no span or one outside the input",
        ),
        (
            rendered.as_ref().is_some_and(|(_, text)| {
                text.as_ref().map_or(true, |text| {
                    !text.starts_with("error[SQ")
                        || text
                            .lines()
                            .any(|line| line.chars().count() > MAX_SHOWN_LINE * 4 + 16)
                })
            }),
            "rendering panicked or printed an unbounded line",
        ),
    ];
    checks
        .into_iter()
        .filter(|(broken, _)| *broken)
        .map(|(_, property)| format!("{property}: {input:?}"))
        .collect()
}

/// Every property violation over the inputs generated from `seed`.
pub(crate) fn property_violations(seed: u64, inputs: usize) -> Vec<String> {
    generated_inputs(seed, inputs)
        .iter()
        .flat_map(|input| violations_of(input, Duration::from_secs(2)))
        .collect()
}
