use std::collections::BTreeMap;

use crate::verdicts::models::{DialectVerdicts, Verdicts};

/// One corpus file's verdicts, from (dialect, statements, refused positions).
pub(crate) fn by_dialect(
    file: Vec<(&'static str, usize, Vec<usize>)>,
) -> BTreeMap<String, DialectVerdicts> {
    file.into_iter()
        .map(|(dialect, statements, refused)| {
            (
                dialect.to_owned(),
                DialectVerdicts {
                    statements,
                    refused,
                },
            )
        })
        .collect()
}

/// Verdicts holding one corpus file, `c.json`.
pub(crate) fn one_file(file: Vec<(&'static str, usize, Vec<usize>)>) -> Verdicts {
    Verdicts {
        about: String::new(),
        corpora: BTreeMap::from([("c.json".to_owned(), by_dialect(file))]),
    }
}
