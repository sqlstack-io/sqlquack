use sqlquack_syntax::{tokenize, Dialect, TokenizeFailure};

pub(crate) fn labelled_tokens(sql: &str, dialect: Dialect) -> Vec<(String, String)> {
    tokenize(sql, dialect)
        .expect("the SQL tokenizes")
        .iter()
        .map(|token| {
            let kind = format!("{:?}", token.kind);
            (
                kind.split('(').next().unwrap_or_default().to_owned(),
                token.text(sql).to_owned(),
            )
        })
        .collect()
}

pub(crate) fn trivia(sql: &str, dialect: Dialect) -> Vec<(String, String)> {
    labelled_tokens(sql, dialect)
        .into_iter()
        .filter(|(kind, _)| kind != "Significant")
        .collect()
}

pub(crate) fn missing_tokens(
    sql: &str,
    dialect: Dialect,
    wanted: &[(&str, &str)],
) -> Vec<(String, String)> {
    let tokens = labelled_tokens(sql, dialect);
    wanted
        .iter()
        .map(|(kind, text)| ((*kind).to_owned(), (*text).to_owned()))
        .filter(|token| !tokens.contains(token))
        .collect()
}

pub(crate) fn lossless_and_contiguous(sql: &str, dialect: Dialect) -> (bool, usize) {
    let tokens = tokenize(sql, dialect).expect("the SQL tokenizes");
    let joined: String = tokens.iter().map(|token| token.text(sql)).collect();
    (
        joined == sql && tokens.windows(2).all(|pair| pair[0].end == pair[1].start),
        tokens.len(),
    )
}

pub(crate) fn tokenizer_failed(sql: &str, dialect: Dialect) -> bool {
    matches!(tokenize(sql, dialect), Err(TokenizeFailure::Tokenizer(_)))
}
