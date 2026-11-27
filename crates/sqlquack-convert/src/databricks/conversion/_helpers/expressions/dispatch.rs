//! One expression to DuckDB: each node kind a rule handles, every other kind refused.

use serde_json::Value;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::bitwise::{
    bitwise, bitwise_not, concat_operator, plus, truth_test,
};
use crate::databricks::conversion::_helpers::expressions::functions::call;
use crate::databricks::conversion::_helpers::expressions::literals::{
    boolean, cast, literal, negated, null,
};
use crate::databricks::conversion::_helpers::expressions::operators::{
    arithmetic, between, case, comparison, division, in_list, logical, not, null_test,
};
use crate::databricks::conversion::_helpers::expressions::patterns::{like, trim};
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::_helpers::support::tree::{
    field, identifier, node_kind, quoted, refused, understood,
};
use crate::databricks::conversion::constants::COLUMN_RULE;
use crate::databricks::conversion::models::Emitted;

/// `node` as DuckDB SQL with its Databricks type, or the refusal of the first unhandled part.
pub(crate) fn expression(node: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let Some((kind, payload)) = node_kind(node) else {
        return Err(refused(
            "expression",
            "the expression is no single node".to_owned(),
        ));
    };
    match kind {
        "column" => column(payload, site),
        "literal" => literal(payload, site),
        "boolean" => boolean(payload, site),
        "null" => Ok(null(site)),
        "cast" => cast(payload, site),
        "paren" => {
            let paren = understood(payload, "paren", &["this"])?;
            let inner = expression(field(paren, "this"), site)?;
            Ok(Emitted {
                sql: format!("({})", inner.sql),
                ..inner
            })
        }
        "neg" => {
            let neg = understood(payload, "neg", &["this"])?;
            negated(expression(field(neg, "this"), site)?)
        }
        "add" | "sub" | "mul" => arithmetic(kind, payload, site),
        "div" | "mod" => division(kind, payload, site),
        "eq" | "neq" | "lt" | "lte" | "gt" | "gte" | "null_safe_eq" => {
            comparison(kind, payload, site)
        }
        "and" | "or" => logical(kind, payload, site),
        "not" => not(payload, site),
        "is_null" => null_test(payload, site),
        "between" => between(payload, site),
        "in" => in_list(payload, site),
        "case" => case(payload, site),
        "like" => like(payload, site),
        "trim" => trim(payload, site),
        "bitwise_and" | "bitwise_or" | "bitwise_xor" => bitwise(kind, payload, site),
        "bitwise_not" => bitwise_not(payload, site),
        "pos" => plus(payload, site),
        "concat" => concat_operator(payload, site),
        "is_true" | "is_false" => truth_test(kind, payload, site),
        "function" | "aggregate_function" => call(kind, payload, site),
        _ => Err(refused(kind, format!("no rule converts {kind}"))),
    }
}

/// A column of the relation the block reads, or in `ORDER BY` one of the block's outputs.
fn column(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let column = understood(payload, "column", &["name", "table"])?;
    let name = identifier(field(column, "name"))?;
    let _rule = site.log.rule(COLUMN_RULE);
    let qualifier = match field(column, "table") {
        Value::Null => None,
        table => Some(identifier(table)?),
    };
    if let Some(qualifier) = &qualifier {
        let named = site
            .scope
            .relation
            .as_deref()
            .is_some_and(|relation| relation.eq_ignore_ascii_case(qualifier));
        if !named {
            return Err(refused(
                "column",
                format!("`{qualifier}.{name}` names no relation the query reads"),
            ));
        }
    }
    let input = site
        .scope
        .column(&name)
        .map(|(written, data_type)| Emitted {
            sql: quoted(written),
            data_type: *data_type,
            literal: None,
        });
    let output = match (site.outputs, &qualifier) {
        (true, None) => site.scope.output(&name).map(|(_, emitted)| emitted),
        _ => None,
    };
    match (input, output) {
        (Some(input), None) => Ok(input),
        (Some(input), Some(output)) if input.sql == output.sql => Ok(input),
        (None, Some(output)) => Ok(Emitted {
            sql: quoted(&name),
            literal: None,
            ..output.clone()
        }),
        (Some(_), Some(_)) => Err(refused(
            "column",
            format!("`{name}` names both an input column and a different output"),
        )),
        (None, None) => Err(refused(
            "column",
            format!("`{name}` names no column the query reads"),
        )),
    }
}
