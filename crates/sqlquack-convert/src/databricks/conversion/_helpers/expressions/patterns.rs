//! Text patterns and trimming: `LIKE`, `LIKE ANY` and `ALL`, and `TRIM ... FROM`.

use serde_json::Value;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::dispatch::expression;
use crate::databricks::conversion::_helpers::expressions::literals::{quoted_text, typed};
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::_helpers::support::tree::{
    field, flag, list, node_kind, refused, understood,
};
use crate::databricks::conversion::constants::{
    CASE_MAPPING_ERROR, ILIKE_KIND, ILIKE_RULE, LIKE_ESCAPE_ERROR, LIKE_ESCAPES, LIKE_RULE,
    LOWERED_CHARACTERS, TRIM_RULE,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Literal};

/// `[NOT] LIKE` or `ILIKE` a pattern, or `ANY` or `ALL` of patterns as an `OR` or `AND`; `ILIKE` lowers both sides first.
pub(crate) fn like(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let like = understood(payload, kind, &["left", "right", "negated", "quantifier"])?;
    let folds = kind == ILIKE_KIND;
    let subject = expression(field(like, "left"), site)?;
    let _rule = site.log.rule(match folds {
        true => ILIKE_RULE,
        false => LIKE_RULE,
    });
    if subject.data_type != DatabricksType::String {
        return Err(refused(
            "like",
            "only a string LIKE a pattern converts".to_owned(),
        ));
    }
    let (patterns, joiner) = match field(like, "quantifier").as_str() {
        None => (vec![field(like, "right")], ""),
        Some(quantifier) => {
            let joiner = match quantifier {
                "ANY" => " OR ",
                "ALL" => " AND ",
                other => {
                    return Err(refused("like", format!("no rule reads LIKE {other}")));
                }
            };
            let Some(("tuple", tuple)) = node_kind(field(like, "right")) else {
                return Err(refused(
                    "like",
                    "LIKE ANY and ALL take a list of patterns".to_owned(),
                ));
            };
            let tuple = understood(tuple, "tuple", &["expressions"])?;
            (list(tuple, "expressions").iter().collect(), joiner)
        }
    };
    let not = match flag(like, "negated") {
        true => "NOT ",
        false => "",
    };
    let subject = match folds {
        true => Emitted {
            sql: folded(subject.sql.clone()),
            ..subject
        },
        false => subject,
    };
    let mut tests: Vec<String> = Vec::with_capacity(patterns.len());
    for pattern in patterns {
        let pattern = expression(pattern, site)?;
        let test = match (&pattern.literal, pattern.data_type) {
            (Some(Literal::Text(text)), _) if folds && escapes_well(text) => {
                format!(
                    "({not}{} LIKE {} ESCAPE '\\')",
                    subject.sql,
                    folded(quoted_text(text))
                )
            }
            (Some(Literal::Text(text)), _) if !text.contains('\\') => {
                format!("({not}{} LIKE {})", subject.sql, quoted_text(text))
            }
            (Some(Literal::Text(text)), _) if !escapes_well(text) => {
                return Err(refused(
                    "like",
                    "a constant pattern whose backslash Databricks rejects".to_owned(),
                ));
            }
            (Some(Literal::Text(_)), _) => {
                format!("({not}{} LIKE {} ESCAPE '\\')", subject.sql, pattern.sql)
            }
            (_, DatabricksType::Null) => format!("({not}{} LIKE NULL)", subject.sql),
            (_, DatabricksType::String) => format!(
                "(CASE WHEN NOT regexp_full_match({p}, '{LIKE_ESCAPES}') THEN error('{LIKE_ESCAPE_ERROR}') \
                 ELSE ({not}{s} LIKE {f} ESCAPE '\\') END)",
                p = pattern.sql,
                f = match folds {
                    true => folded(pattern.sql.clone()),
                    false => pattern.sql.clone(),
                },
                s = subject.sql
            ),
            _ => {
                return Err(refused("like", "only a string pattern converts".to_owned()));
            }
        };
        tests.push(test);
    }
    match tests.is_empty() {
        true => Err(refused(
            "like",
            "LIKE ANY and ALL need a pattern".to_owned(),
        )),
        false => Ok(typed(
            format!("({})", tests.join(joiner)),
            DatabricksType::Boolean,
        )),
    }
}

/// `text` lowered by DuckDB, raising for a character outside those whose lowering Java and DuckDB were seen to agree on.
fn folded(text: String) -> String {
    case_mapped(&text, "lower", LOWERED_CHARACTERS)
}

/// `text` mapped by DuckDB's `function`, raising for a character outside `characters`, the class body it maps as Java does.
pub(crate) fn case_mapped(text: &str, function: &str, characters: &str) -> String {
    format!(
        "(CASE WHEN regexp_matches({text}, '[^{characters}]') THEN error('{CASE_MAPPING_ERROR}') \
         ELSE {function}({text}) END)"
    )
}

/// Whether every backslash in `pattern` escapes `_`, `%` or a backslash, as Databricks requires.
fn escapes_well(pattern: &str) -> bool {
    let mut characters = pattern.chars();
    while let Some(character) = characters.next() {
        if character == '\\' && !matches!(characters.next(), Some('_' | '%' | '\\')) {
            return false;
        }
    }
    true
}

/// `TRIM([BOTH | LEADING | TRAILING] [chars] FROM s)`: every character of `chars`, spaces by default.
pub(crate) fn trim(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let trim = understood(
        payload,
        "trim",
        &[
            "this",
            "characters",
            "position",
            "sql_standard_syntax",
            "position_explicit",
        ],
    )?;
    let subject = expression(field(trim, "this"), site)?;
    let characters = match field(trim, "characters") {
        Value::Null => None,
        node => Some(expression(node, site)?),
    };
    let _rule = site.log.rule(TRIM_RULE);
    let function = match field(trim, "position").as_str() {
        None | Some("Both") => "trim",
        Some("Leading") => "ltrim",
        Some("Trailing") => "rtrim",
        Some(other) => return Err(refused("trim", format!("no rule reads TRIM {other}"))),
    };
    let characters_sql = match &characters {
        None => "' '".to_owned(),
        Some(characters) if text_or_null(characters.data_type) => characters.sql.clone(),
        Some(_) => return Err(refused("trim", "only TRIM of strings converts".to_owned())),
    };
    match text_or_null(subject.data_type) {
        true => Ok(typed(
            format!("{function}({}, {characters_sql})", subject.sql),
            DatabricksType::String,
        )),
        false => Err(refused("trim", "only TRIM of strings converts".to_owned())),
    }
}

fn text_or_null(data_type: DatabricksType) -> bool {
    matches!(data_type, DatabricksType::String | DatabricksType::Null)
}
