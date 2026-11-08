//! Every construct the probe nests: recursive paths, flat chains, and chains nested in a chain's first operand.

use crate::depth::constants::{MIXED_SUFFIX_RUN, NESTED_CHAIN_LINKS};
use crate::depth::models::Construct;
use crate::formats::models::LocalParser;

/// `open` `depth` times, then `inner`, then `close` `depth` times.
fn nest(open: &str, inner: &str, close: &str, depth: usize) -> String {
    format!("{}{inner}{}", open.repeat(depth), close.repeat(depth))
}

/// `depth` copies of `term` joined by `separator`.
fn chain(term: &str, separator: &str, depth: usize) -> String {
    vec![term; depth].join(separator)
}

/// A parenthesised chain of `link` after `inner`, nested in the first operand of the next, `depth` links in all.
fn nested_chain(inner: &str, link: &str, depth: usize) -> String {
    let levels: usize = (depth / NESTED_CHAIN_LINKS).max(1);
    let close: String = format!("{})", link.repeat(NESTED_CHAIN_LINKS));
    nest("(", inner, &close, levels)
}

/// A type built by the parser's own nested type constructor, where it has one.
fn constructed_type(parser: LocalParser, depth: usize) -> Option<String> {
    match parser {
        LocalParser::Snowflake => Some(nest("ARRAY(", "INT", ")", depth)),
        LocalParser::BigQuery => Some(nest("STRUCT<x ", "INT64", ">", depth)),
        LocalParser::DuckDb => Some(nest("STRUCT(a ", "INT", ")", depth)),
        LocalParser::Databricks => Some(nest("ARRAY<", "INT", ">", depth)),
        LocalParser::Postgres
        | LocalParser::SqlServer
        | LocalParser::Sqlite
        | LocalParser::None => None,
    }
}

/// Constructor levels alternating with runs of `[]`, about `depth` type levels in all.
fn mixed_type(parser: LocalParser, depth: usize) -> Option<String> {
    let levels: usize = (depth / MIXED_SUFFIX_RUN).max(1);
    let run: String = "[]".repeat(MIXED_SUFFIX_RUN);
    let open: &str = match parser {
        LocalParser::Snowflake => "ARRAY(",
        LocalParser::DuckDb => "STRUCT(a ",
        LocalParser::Postgres
        | LocalParser::BigQuery
        | LocalParser::Databricks
        | LocalParser::SqlServer
        | LocalParser::Sqlite
        | LocalParser::None => return None,
    };
    Some(format!(
        "{}INT{}{run}",
        open.repeat(levels),
        format!("{run})").repeat(levels)
    ))
}

/// Casts nested about `depth / MIXED_SUFFIX_RUN` deep, each to a type with a run of `[]`.
fn cast_with_suffix(depth: usize) -> String {
    let levels: usize = (depth / MIXED_SUFFIX_RUN).max(1);
    let cast: String = format!(" AS INT{})", "[]".repeat(MIXED_SUFFIX_RUN));
    format!(
        "SELECT {}NULL{}",
        "CAST(".repeat(levels),
        cast.repeat(levels)
    )
}

/// Whether `parser` reads `[]` subscripts and type suffixes; T-SQL and SQLite (`[x]` is a name) don't.
fn array_syntax(parser: LocalParser) -> bool {
    !matches!(parser, LocalParser::SqlServer | LocalParser::Sqlite)
}

/// The probed constructs, in report order.
pub(crate) fn constructs() -> Vec<Construct> {
    vec![
        Construct {
            name: "from_parentheses",
            text: |_, depth| Some(format!("SELECT * FROM {}", nest("(", "t", ")", depth))),
        },
        Construct {
            name: "from_parenthesised_join",
            text: |_, depth| {
                Some(format!(
                    "SELECT * FROM {}t{}",
                    "(".repeat(depth),
                    " JOIN t ON 1 = 1)".repeat(depth)
                ))
            },
        },
        Construct {
            name: "from_subquery",
            text: |parser, depth| match parser {
                LocalParser::SqlServer => Some(format!(
                    "SELECT * FROM {}",
                    nest("(SELECT * FROM ", "t", ") s", depth)
                )),
                _ => Some(format!(
                    "SELECT * FROM {}",
                    nest("(SELECT * FROM ", "t", ")", depth)
                )),
            },
        },
        Construct {
            name: "scalar_subquery",
            text: |_, depth| Some(format!("SELECT {}", nest("(SELECT ", "1", ")", depth))),
        },
        Construct {
            name: "exists_subquery",
            text: |_, depth| {
                Some(format!(
                    "SELECT 1 FROM t WHERE {}",
                    nest("EXISTS (SELECT 1 FROM t WHERE ", "1 = 1", ")", depth)
                ))
            },
        },
        Construct {
            name: "in_subquery",
            text: |_, depth| {
                Some(format!(
                    "SELECT 1 FROM t WHERE a IN {}",
                    nest("(SELECT a FROM t WHERE a IN ", "(1)", ")", depth)
                ))
            },
        },
        Construct {
            name: "parenthesised_query",
            text: |parser, depth| match parser {
                LocalParser::Sqlite => None,
                _ => Some(nest("(", "SELECT 1", ")", depth)),
            },
        },
        Construct {
            name: "parenthesised_set_operation",
            text: |parser, depth| match parser {
                LocalParser::Sqlite => None,
                _ => Some(format!(
                    "{}SELECT 1{}",
                    "(".repeat(depth),
                    ") UNION ALL (SELECT 1)".repeat(depth)
                )),
            },
        },
        Construct {
            name: "nested_with",
            text: |_, depth| {
                Some(format!(
                    "WITH {}",
                    nest(
                        "a AS (WITH ",
                        "b AS (SELECT 1) SELECT 1",
                        ") SELECT 1",
                        depth
                    )
                ))
            },
        },
        Construct {
            name: "parenthesised_expression",
            text: |_, depth| Some(format!("SELECT {}", nest("(", "1", ")", depth))),
        },
        Construct {
            name: "unary_minus",
            text: |_, depth| Some(format!("SELECT {}1", "- ".repeat(depth))),
        },
        Construct {
            name: "not",
            text: |parser, depth| match parser {
                LocalParser::SqlServer => Some(format!(
                    "SELECT 1 FROM t WHERE {}1 = 1",
                    "NOT ".repeat(depth)
                )),
                _ => Some(format!("SELECT {}TRUE", "NOT ".repeat(depth))),
            },
        },
        Construct {
            name: "case",
            text: |_, depth| {
                Some(format!(
                    "SELECT {} FROM t",
                    nest("CASE WHEN a = 1 THEN ", "1", " END", depth)
                ))
            },
        },
        Construct {
            name: "function_arguments",
            text: |parser, depth| match parser {
                LocalParser::SqlServer => {
                    Some(format!("SELECT {}", nest("coalesce(", "1", ", 1)", depth)))
                }
                _ => Some(format!("SELECT {}", nest("coalesce(", "1", ")", depth))),
            },
        },
        Construct {
            name: "cast",
            text: |_, depth| Some(format!("SELECT {}", nest("CAST(", "1", " AS INT)", depth))),
        },
        Construct {
            name: "subscript",
            text: |parser, depth| {
                array_syntax(parser)
                    .then(|| format!("SELECT {} FROM t", nest("a[", "0", "]", depth)))
            },
        },
        Construct {
            name: "type_suffix",
            text: |parser, depth| {
                array_syntax(parser)
                    .then(|| format!("SELECT CAST(NULL AS INT{})", "[]".repeat(depth)))
            },
        },
        Construct {
            name: "type_suffix_sized",
            text: |parser, depth| {
                array_syntax(parser)
                    .then(|| format!("SELECT CAST(NULL AS INT{})", "[3]".repeat(depth)))
            },
        },
        Construct {
            name: "double_colon_type_suffix",
            text: |parser, depth| {
                array_syntax(parser).then(|| format!("SELECT NULL::INT{}", "[]".repeat(depth)))
            },
        },
        Construct {
            name: "column_type_suffix",
            text: |parser, depth| {
                array_syntax(parser)
                    .then(|| format!("CREATE TABLE t (c INT{})", "[]".repeat(depth)))
            },
        },
        Construct {
            name: "type_constructor",
            text: |parser, depth| {
                constructed_type(parser, depth).map(|kind| format!("SELECT CAST(NULL AS {kind})"))
            },
        },
        Construct {
            name: "column_type_constructor",
            text: |parser, depth| {
                constructed_type(parser, depth).map(|kind| format!("CREATE TABLE t (c {kind})"))
            },
        },
        Construct {
            name: "type_constructor_and_suffix",
            text: |parser, depth| {
                mixed_type(parser, depth).map(|kind| format!("SELECT CAST(NULL AS {kind})"))
            },
        },
        Construct {
            name: "cast_and_type_suffix",
            text: |parser, depth| array_syntax(parser).then(|| cast_with_suffix(depth)),
        },
        Construct {
            name: "flat_plus",
            text: |_, depth| Some(format!("SELECT {}", chain("1", " + ", depth))),
        },
        Construct {
            name: "flat_or",
            text: |_, depth| {
                Some(format!(
                    "SELECT 1 FROM t WHERE {}",
                    chain("a = 1", " OR ", depth)
                ))
            },
        },
        Construct {
            name: "flat_union_all",
            text: |_, depth| Some(chain("SELECT 1", " UNION ALL ", depth)),
        },
        Construct {
            name: "subquery_flat_union_all",
            text: |_, depth| {
                Some(format!(
                    "SELECT * FROM ({}) s",
                    chain("SELECT 1", " UNION ALL ", depth)
                ))
            },
        },
        Construct {
            name: "nested_cast_chain",
            text: |parser, depth| match parser {
                LocalParser::BigQuery | LocalParser::Sqlite => None,
                LocalParser::SqlServer => Some(format!(
                    "SELECT {}",
                    nested_chain("'a'", " COLLATE Latin1_General_CI_AS", depth)
                )),
                _ => Some(format!("SELECT {}", nested_chain("1", "::INT", depth))),
            },
        },
        Construct {
            name: "nested_plus_chain",
            text: |_, depth| Some(format!("SELECT {}", nested_chain("1", " + 1", depth))),
        },
        Construct {
            name: "nested_or_chain",
            text: |parser, depth| match parser {
                LocalParser::SqlServer => Some(format!(
                    "SELECT 1 FROM t WHERE {}",
                    nested_chain("1 = 0", " OR 1 = 0", depth)
                )),
                _ => Some(format!(
                    "SELECT {}",
                    nested_chain("FALSE", " OR FALSE", depth)
                )),
            },
        },
        Construct {
            name: "nested_union_all",
            text: |parser, depth| match parser {
                LocalParser::Sqlite => None,
                _ => Some(nested_chain("SELECT 1", " UNION ALL SELECT 1", depth)),
            },
        },
        Construct {
            name: "nested_pipe_chain",
            text: |parser, depth| match parser {
                LocalParser::BigQuery | LocalParser::Databricks => {
                    Some(nested_chain("FROM t", " |> WHERE TRUE", depth))
                }
                _ => None,
            },
        },
        Construct {
            name: "nested_if",
            text: |parser, depth| match parser {
                LocalParser::SqlServer => Some(format!("{}SELECT 1", "IF 1 = 1 ".repeat(depth))),
                _ => None,
            },
        },
        Construct {
            name: "nested_at_time_zone_chain",
            text: |parser, depth| match parser {
                LocalParser::SqlServer => Some(format!(
                    "SELECT {}",
                    nested_chain("SYSDATETIMEOFFSET()", " AT TIME ZONE 'UTC'", depth)
                )),
                _ => None,
            },
        },
        Construct {
            name: "flat_join",
            text: |_, depth| {
                Some(format!(
                    "SELECT * FROM t{}",
                    " JOIN t ON 1 = 1".repeat(depth)
                ))
            },
        },
    ]
}
