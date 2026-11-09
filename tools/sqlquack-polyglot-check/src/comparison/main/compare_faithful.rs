//! `compare_faithful`: SQLQuack's faithful parse against its lowered parse.

use serde_json::Value;
use sqlquack_syntax::{CallMode, Dialect, ParseOptions};
use sqlquack_syntax_snowflake::parse_statements_with;

use crate::comparison::_helpers::faithful::{comments, relower, with_flat_paths, without_comments};
use crate::comparison::models::{FaithfulComparison, Spelling};

/// The spellings the faithful parse keeps, and any difference they do not explain.
pub fn compare_faithful(sql: &str, dialect: Dialect) -> FaithfulComparison {
    let options = ParseOptions::default();
    let lowered = parse_statements_with(sql, dialect, &options, CallMode::Lowered);
    let faithful = parse_statements_with(sql, dialect, &options, CallMode::FaithfulWithSpans);
    let (spellings, unexplained): (Vec<Spelling>, Option<String>) = match (faithful, lowered) {
        (Ok(faithful), lowered) => {
            let chars: Vec<char> = sql.chars().collect();
            let kept = serde_json::to_value(&faithful).unwrap_or_default();
            let (relowered, spellings) = relower(kept.clone(), &chars);
            let rejected = spellings.iter().any(Spelling::is_rejection);
            let unexplained = match lowered {
                Ok(lowered) if !rejected => {
                    let lowered = serde_json::to_value(&lowered).unwrap_or_default();
                    let (flat_relowered, flat_lowered) =
                        (with_flat_paths(&relowered), with_flat_paths(&lowered));
                    (flat_relowered != flat_lowered
                        && !only_comments_moved(&flat_relowered, &flat_lowered, &kept))
                    .then(|| format!("faithful, put back: {relowered}\nlowered: {lowered}"))
                }
                Ok(_) => Some("a call polyglot rejects alone parses in context".to_owned()),
                Err(_) if rejected => None,
                Err(error) => Some(format!("only the lowered parse fails: {error}")),
            };
            (spellings, unexplained)
        }
        (Err(faithful), Err(lowered)) => (
            Vec::new(),
            (faithful.to_string() != lowered.to_string())
                .then(|| format!("faithful error: {faithful}\nlowered error: {lowered}")),
        ),
        (Err(error), Ok(_)) => (
            Vec::new(),
            Some(format!("only the faithful parse fails: {error}")),
        ),
    };
    FaithfulComparison {
        spellings,
        unexplained,
    }
}

/// Whether only comment placement differs and the faithful tree keeps every distinct comment.
fn only_comments_moved(relowered: &Value, lowered: &Value, faithful: &Value) -> bool {
    if without_comments(relowered) != without_comments(lowered) {
        return false;
    }
    let kept: Vec<String> = comments(faithful);
    comments(lowered)
        .iter()
        .all(|comment| kept.contains(comment))
}
