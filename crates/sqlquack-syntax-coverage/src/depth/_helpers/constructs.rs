//! Every construct the probe nests: recursive paths through queries, expressions and types, and flat chains.

use crate::depth::constants::MIXED_SUFFIX_RUN;
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

/// A type built by the parser's own nested type constructor, where it has one.
fn constructed_type(parser: LocalParser, depth: usize) -> Option<String> {
    match parser {
        LocalParser::Snowflake => Some(nest("ARRAY(", "INT", ")", depth)),
        LocalParser::Postgres | LocalParser::BigQuery | LocalParser::None => None,
    }
}

/// Constructor levels alternating with runs of `[]`, about `depth` type levels in all.
fn mixed_type(parser: LocalParser, depth: usize) -> Option<String> {
    let levels: usize = (depth / MIXED_SUFFIX_RUN).max(1);
    let run: String = "[]".repeat(MIXED_SUFFIX_RUN);
    match parser {
        LocalParser::Snowflake => Some(format!(
            "{}INT{}{run}",
            "ARRAY(".repeat(levels),
            format!("{run})").repeat(levels)
        )),
        LocalParser::Postgres | LocalParser::BigQuery | LocalParser::None => None,
    }
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
            text: |_, depth| {
                Some(format!(
                    "SELECT * FROM {}",
                    nest("(SELECT * FROM ", "t", ")", depth)
                ))
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
            text: |_, depth| Some(nest("(", "SELECT 1", ")", depth)),
        },
        Construct {
            name: "parenthesised_set_operation",
            text: |_, depth| {
                Some(format!(
                    "{}SELECT 1{}",
                    "(".repeat(depth),
                    ") UNION ALL (SELECT 1)".repeat(depth)
                ))
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
            text: |_, depth| Some(format!("SELECT {}TRUE", "NOT ".repeat(depth))),
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
            text: |_, depth| Some(format!("SELECT {}", nest("coalesce(", "1", ")", depth))),
        },
        Construct {
            name: "cast",
            text: |_, depth| Some(format!("SELECT {}", nest("CAST(", "1", " AS INT)", depth))),
        },
        Construct {
            name: "subscript",
            text: |_, depth| Some(format!("SELECT {} FROM t", nest("a[", "0", "]", depth))),
        },
        Construct {
            name: "type_suffix",
            text: |_, depth| Some(format!("SELECT CAST(NULL AS INT{})", "[]".repeat(depth))),
        },
        Construct {
            name: "type_suffix_sized",
            text: |_, depth| Some(format!("SELECT CAST(NULL AS INT{})", "[3]".repeat(depth))),
        },
        Construct {
            name: "double_colon_type_suffix",
            text: |_, depth| Some(format!("SELECT NULL::INT{}", "[]".repeat(depth))),
        },
        Construct {
            name: "column_type_suffix",
            text: |_, depth| Some(format!("CREATE TABLE t (c INT{})", "[]".repeat(depth))),
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
