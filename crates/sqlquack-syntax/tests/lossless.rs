//! Lossless tokens: every byte belongs to one token, and trivia is classified.

use sqlquack_syntax::{tokenize, Dialect, LosslessToken, TokenKind, TokenizeFailure};

fn kinds(sql: &str, dialect: Dialect) -> Vec<(TokenKind, String)> {
    let tokens = tokenize(sql, dialect).expect("the SQL tokenizes");
    assert_joined(sql, &tokens);
    tokens
        .iter()
        .map(|token| (token.kind, token.text(sql).to_owned()))
        .collect()
}

fn assert_joined(sql: &str, tokens: &[LosslessToken]) {
    let joined: String = tokens.iter().map(|token| token.text(sql)).collect();
    assert_eq!(joined, sql);
    for pair in tokens.windows(2) {
        assert_eq!(pair[0].end, pair[1].start);
    }
}

fn trivia(sql: &str, dialect: Dialect) -> Vec<(TokenKind, String)> {
    kinds(sql, dialect)
        .into_iter()
        .filter(|(kind, _)| !matches!(kind, TokenKind::Significant(_)))
        .collect()
}

#[test]
fn whitespace_and_comments_become_trivia() {
    let sql = "SELECT a -- trailing\n  /* block */ FROM t\t// slashes\n";
    assert_eq!(
        trivia(sql, Dialect::Snowflake),
        vec![
            (TokenKind::Whitespace, " ".to_owned()),
            (TokenKind::Whitespace, " ".to_owned()),
            (TokenKind::LineComment, "-- trailing".to_owned()),
            (TokenKind::Whitespace, "\n  ".to_owned()),
            (TokenKind::BlockComment, "/* block */".to_owned()),
            (TokenKind::Whitespace, " ".to_owned()),
            (TokenKind::Whitespace, " ".to_owned()),
            (TokenKind::Whitespace, "\t".to_owned()),
            (TokenKind::LineComment, "// slashes".to_owned()),
            (TokenKind::Whitespace, "\n".to_owned()),
        ]
    );
}

#[test]
fn string_and_identifier_tokens_keep_their_quotes() {
    let sql = "SELECT 'it''s', \"Quoted Name\", $$dollar 'text'$$ FROM t";
    let significant: Vec<String> = kinds(sql, Dialect::Snowflake)
        .into_iter()
        .filter(|(kind, _)| matches!(kind, TokenKind::Significant(_)))
        .map(|(_, text)| text)
        .collect();
    assert!(
        significant.contains(&"'it''s'".to_owned()),
        "{significant:?}"
    );
    assert!(
        significant.contains(&"\"Quoted Name\"".to_owned()),
        "{significant:?}"
    );
    assert!(
        significant.contains(&"$$dollar 'text'$$".to_owned()),
        "{significant:?}"
    );
}

#[test]
fn byte_ranges_follow_multibyte_text() {
    let sql = "SELECT 'ünï' AS \"größe\" /* 日本 */ FROM t";
    let tokens = tokenize(sql, Dialect::DuckDB).expect("the SQL tokenizes");
    assert_joined(sql, &tokens);
    assert!(tokens
        .iter()
        .any(|token| token.kind == TokenKind::BlockComment && token.text(sql) == "/* 日本 */"));
}

#[test]
fn nested_block_comments_follow_the_dialect() {
    let sql = "SELECT /* a /* b */ c */ 1";
    let duckdb = trivia(sql, Dialect::DuckDB);
    assert!(duckdb.contains(&(TokenKind::BlockComment, "/* a /* b */ c */".to_owned())));
    let snowflake = tokenize(sql, Dialect::Snowflake).expect("the SQL tokenizes");
    assert_joined(sql, &snowflake);
}

#[test]
fn hints_and_empty_input_are_covered() {
    assert!(tokenize("", Dialect::Snowflake)
        .expect("empty SQL tokenizes")
        .is_empty());
    let sql = "SELECT /*+ hint */ 1;\n";
    let tokens = tokenize(sql, Dialect::Snowflake).expect("the SQL tokenizes");
    assert_joined(sql, &tokens);
}

#[test]
fn tokenizer_errors_are_reported() {
    let failure = tokenize("SELECT 'unterminated", Dialect::Snowflake);
    assert!(
        matches!(failure, Err(TokenizeFailure::Tokenizer(_))),
        "{failure:?}"
    );
}
