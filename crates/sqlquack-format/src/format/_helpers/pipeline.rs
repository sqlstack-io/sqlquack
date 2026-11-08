//! The formatting pipeline: lex, parse, lay out against the guide, then check the output.

use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{CallMode, ExpressionWalk, parse_statements_with};

use crate::format::_helpers::bigquery::parse_bigquery;
use crate::format::_helpers::guide::layout_guide;
use crate::format::_helpers::identifiers::parsed_names;
use crate::format::_helpers::lexemes::{Lexeme, ends_operand, is_unquoted_word, lex};
use crate::format::_helpers::native::parse_postgres;
use crate::format::_helpers::printed_layout::{PrintedSql, wrap_lines};
use crate::format::_helpers::sqlserver::parse_sqlserver;
use crate::format::_helpers::token_layout::{
    AuthoredPrint, AuthoredSql, Oracle, print_authored_tokens, verify_printed_tokens,
};
use crate::format::_helpers::touching::verify_touching_tokens;
use crate::format::constants::{
    DEPTH_FAILURE, FORMAT_GATED_DIALECTS, MAX_TREE_DEPTH, MAX_TYPE_NESTING, NOT_IDEMPOTENT_FAILURE,
    PARSE_FAILURE, SELECT_LIST_FAILURE, TOKEN_PRESERVATION_FAILURE, TYPE_CONSTRUCTOR_WORDS,
    TYPE_DEPTH_FAILURE, UNSUPPORTED_DIALECT_FAILURE, UNSUPPORTED_SYNTAX_FAILURE,
};
use crate::format::models::{FormatDialect, FormatOptions, Formatted};

/// Format `sql`, or return why it is left as written.
pub(crate) fn format_checked(sql: &str, options: &FormatOptions) -> Result<Formatted, String> {
    let dialect = options.dialect;
    if !FORMAT_GATED_DIALECTS.contains(&dialect) {
        return Err(UNSUPPORTED_DIALECT_FAILURE.to_string());
    }
    let (lexemes, comments) = lex(sql, dialect)?;
    if lexemes.is_empty() {
        return Ok(Formatted {
            sql: sql.to_string(),
            changed: false,
        });
    }
    if dialect == FormatDialect::Snowflake && has_unsupported_syntax(&lexemes) {
        return Err(UNSUPPORTED_SYNTAX_FAILURE.to_string());
    }
    if nests_types_deeply(&lexemes) {
        return Err(TYPE_DEPTH_FAILURE.to_string());
    }
    let statements = match dialect {
        FormatDialect::BigQuery => parse_bigquery(sql)?,
        FormatDialect::PostgreSQL => parse_postgres(sql)?,
        FormatDialect::SqlServer => parse_sqlserver(sql)?,
        _ => {
            let shared = dialect.shared().ok_or(UNSUPPORTED_DIALECT_FAILURE)?;
            parse_statements_with(sql, shared, &options.parse, CallMode::FaithfulLocated)
                .map_err(|error| format!("{PARSE_FAILURE}: {error}"))?
        }
    };
    if statements
        .iter()
        .any(|statement| statement.tree_depth() > MAX_TREE_DEPTH)
    {
        return Err(DEPTH_FAILURE.to_string());
    }
    let names = parsed_names(sql, &statements, &lexemes)?;
    let guide = layout_guide(&lexemes, &names, dialect);
    if dialect == FormatDialect::Snowflake && guide.selects != names.selects {
        return Err(SELECT_LIST_FAILURE.to_string());
    }
    let oracle = Oracle::new(guide);
    let authored = AuthoredSql {
        sql,
        lexemes: &lexemes,
        comments: &comments,
    };
    let print = print_layout(&authored, &oracle, options)?;
    if print.sql == sql {
        return Ok(Formatted {
            sql: print.sql,
            changed: false,
        });
    }
    let (printed_lexemes, printed_comments) = lex(&print.sql, dialect)?;
    let printed = AuthoredSql {
        sql: &print.sql,
        lexemes: &printed_lexemes,
        comments: &printed_comments,
    };
    verify_printed_tokens(&authored, &printed, &print.recases)?;
    verify_touching_tokens(&authored, &printed, &names.binary_operators, dialect)?;
    if print_layout(&printed, &oracle, options)?.sql != print.sql {
        return Err(NOT_IDEMPOTENT_FAILURE.to_string());
    }
    Ok(Formatted {
        sql: print.sql,
        changed: true,
    })
}

/// Print `authored` against the guide, then apply the clause rules and optional line wrapping.
fn print_layout(
    authored: &AuthoredSql<'_>,
    oracle: &Oracle,
    options: &FormatOptions,
) -> Result<AuthoredPrint, String> {
    let print = print_authored_tokens(authored, oracle)?;
    let Some(mut sql) = PrintedSql::new(&print.sql, &print.tokens, &options.token_widths) else {
        return Ok(print);
    };
    sql.set_indent(options.indent);
    sql.attach_commas_before_comments();
    if let Some(line_width) = options.line_width {
        sql.join_fitting_groups(line_width);
    }
    sql.arrange_clauses(options.line_width.unwrap_or(usize::MAX));
    if let Some(line_width) = options.line_width {
        wrap_lines(&mut sql, line_width);
    }
    sql.lead_operators_of_multiline_operands();
    Ok(AuthoredPrint {
        sql: keep_verbatim(authored, sql.render(), &oracle.verbatim, options.dialect)?,
        ..print
    })
}

/// `printed` with the text of each `verbatim` token range put back as `authored` wrote it.
fn keep_verbatim(
    authored: &AuthoredSql<'_>,
    printed: String,
    verbatim: &[(usize, usize)],
    dialect: FormatDialect,
) -> Result<String, String> {
    if verbatim.is_empty() {
        return Ok(printed);
    }
    let (lexemes, _) = lex(&printed, dialect)?;
    if lexemes.len() != authored.lexemes.len() {
        return Err(TOKEN_PRESERVATION_FAILURE.to_string());
    }
    let mut kept = printed;
    for &(first, last) in verbatim.iter().rev() {
        let text = &authored.sql[authored.lexemes[first].start..authored.lexemes[last].end];
        kept.replace_range(lexemes[first].start..lexemes[last].end, text);
    }
    Ok(kept)
}

/// Whether a data type nests deeper than `MAX_TYPE_NESTING`, counting bracket suffixes such as `INT[]` and constructors such as `ARRAY<...`, `ARRAY(...` or `"ARRAY"(...` along one chain, since the parser does not bound either; each open constructor keeps its fields' starting depth, its deepest field, and whether it opened with `(`.
fn nests_types_deeply(lexemes: &[Lexeme]) -> bool {
    let mut frames: Vec<(usize, usize, bool)> = Vec::new();
    let mut paren_opens_type: Vec<bool> = Vec::new();
    let mut depth: usize = 0;
    let mut closed_type = false;
    for (index, lexeme) in lexemes.iter().enumerate() {
        let after_type_word = index.checked_sub(1).is_some_and(|at| {
            let name = lexemes[at].raw.trim_matches(['"', '`']);
            TYPE_CONSTRUCTOR_WORDS
                .iter()
                .any(|word| name.eq_ignore_ascii_case(word))
        });
        let continues = closed_type
            || index
                .checked_sub(1)
                .is_some_and(|at| lexemes[at].kind == TokenType::RBracket);
        closed_type = false;
        match lexeme.kind {
            TokenType::LBracket if continues => depth += 1,
            TokenType::LBracket => depth = frames.last().map_or(0, |&(base, _, _)| base) + 1,
            TokenType::Lt | TokenType::LParen if after_type_word => {
                depth = if frames.is_empty() { 1 } else { depth + 1 };
                let paren = lexeme.kind == TokenType::LParen;
                frames.push((depth, depth, paren));
                if paren {
                    paren_opens_type.push(true);
                }
            }
            TokenType::LParen => paren_opens_type.push(false),
            TokenType::RParen => {
                if paren_opens_type.pop() == Some(true) {
                    while let Some((_, deepest, paren)) = frames.pop() {
                        depth = depth.max(deepest);
                        if paren {
                            break;
                        }
                    }
                    closed_type = true;
                }
            }
            TokenType::Gt | TokenType::GtGt => {
                let closes = if lexeme.kind == TokenType::GtGt { 2 } else { 1 };
                for _ in 0..closes {
                    if frames.last().is_some_and(|&(_, _, paren)| !paren) {
                        if let Some((_, deepest, _)) = frames.pop() {
                            depth = depth.max(deepest);
                        }
                        closed_type = true;
                    }
                }
            }
            TokenType::Comma => {
                if let Some((base, deepest, _)) = frames.last_mut() {
                    *deepest = (*deepest).max(depth);
                    depth = *base;
                }
            }
            _ => {}
        }
        if depth > MAX_TYPE_NESTING || frames.len() > MAX_TYPE_NESTING {
            return true;
        }
    }
    false
}

/// Bind variables such as `:1`, and string stage references with options, are left as written.
fn has_unsupported_syntax(lexemes: &[Lexeme]) -> bool {
    (0..lexemes.len())
        .any(|index| is_bind_variable(lexemes, index) || is_staged_string(lexemes, index))
}

/// Whether a `:` at `index` starts a bind variable such as `:1` or `:name`.
fn is_bind_variable(lexemes: &[Lexeme], index: usize) -> bool {
    let after_operand = index
        .checked_sub(1)
        .is_some_and(|previous| ends_operand(&lexemes[previous]));
    lexemes[index].kind == TokenType::Colon
        && !after_operand
        && lexemes
            .get(index + 1)
            .is_some_and(|next| next.kind == TokenType::Number || is_unquoted_word(&next.raw))
}

/// Whether a string stage reference such as `'@stage'` at `index` is followed by options.
fn is_staged_string(lexemes: &[Lexeme], index: usize) -> bool {
    lexemes[index].kind == TokenType::String
        && lexemes[index].raw.starts_with("'@")
        && lexemes
            .get(index + 1)
            .is_some_and(|next| next.kind == TokenType::LParen)
}
