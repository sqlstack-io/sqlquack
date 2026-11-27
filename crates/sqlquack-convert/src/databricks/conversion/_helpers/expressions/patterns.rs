//! Text patterns and trimming: `LIKE`, `LIKE ANY` and `ALL`, and `TRIM ... FROM`.

use serde_json::Value;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::dispatch::expression;
use crate::databricks::conversion::_helpers::expressions::literals::{quoted_text, typed};
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::_helpers::support::tree::{
    field, flag, list, node_kind, refused, understood,
};
use crate::databricks::conversion::constants::{LIKE_RULE, TRIM_RULE};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Literal};

/// `[NOT] LIKE` a constant pattern, or `LIKE ANY` or `ALL` constant patterns as an `OR` or `AND`.
pub(crate) fn like(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let like = understood(payload, "like", &["left", "right", "negated", "quantifier"])?;
    let subject = expression(field(like, "left"), site)?;
    let _rule = site.log.rule(LIKE_RULE);
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
    let mut tests: Vec<String> = Vec::with_capacity(patterns.len());
    for pattern in patterns {
        let pattern = expression(pattern, site)?;
        let pattern_sql = match (&pattern.literal, pattern.data_type) {
            (Some(Literal::Text(text)), _) if !text.contains('\\') => quoted_text(text),
            (_, DatabricksType::Null) => pattern.sql,
            _ => {
                return Err(refused(
                    "like",
                    "only constant patterns without a backslash, Databricks's escape, convert"
                        .to_owned(),
                ));
            }
        };
        tests.push(format!("({not}{} LIKE {pattern_sql})", subject.sql));
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
