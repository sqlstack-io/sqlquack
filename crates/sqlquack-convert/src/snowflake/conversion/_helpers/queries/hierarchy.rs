//! START WITH ... CONNECT BY PRIOR over one relation, as a DuckDB recursive CTE.

use serde_json::{Map, Value, json};
use sqlquack_blocks::blocks::main::column_type::column_type;
use sqlquack_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_blocks::blocks::main::string_literal::string_literal;
use sqlquack_blocks::raises::main::raised::raised;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::snowflake::conversion::_helpers::expressions::scalar::convert_expression;
use crate::snowflake::conversion::_helpers::queries::clauses::condition_sql;
use crate::snowflake::conversion::_helpers::queries::sources::convert_from;
use crate::snowflake::conversion::_helpers::support::scope::{Context, Scope};
use crate::snowflake::conversion::_helpers::support::site::{Placement, Site};
use crate::snowflake::conversion::_helpers::support::tree::{
    Payload, node_construct, require_understood,
};
use crate::snowflake::conversion::constants::{
    HIERARCHY_COLUMN_PREFIX, HIERARCHY_DEPTH_TAG, HIERARCHY_LEVEL, HIERARCHY_MAX_DEPTH,
    HIERARCHY_NESTED_QUERIES, HIERARCHY_PRIOR, HIERARCHY_RULE, HIERARCHY_WALK,
};
use sqlquack_semantics::snowflake::scopes::models::ScopeRelation;
use sqlquack_semantics::snowflake::types::models::ColumnSpec;

/// A hierarchical query's FROM: its walk, the select rewritten to read the walk's columns.
pub(crate) struct Walk {
    /// The select with LEVEL, SYS_CONNECT_BY_PATH and CONNECT_BY_ROOT read from the walk.
    pub(crate) select: Payload,
    /// ` FROM (<recursive walk>) AS <alias>`.
    pub(crate) from_sql: String,
    /// The walk: the relation's columns, then the hidden columns the rewrite reads.
    pub(crate) relations: Vec<ScopeRelation>,
}

/// A value the walk carries per row: LEVEL, a path or a root.
enum Carried {
    Level,
    Path { value: Value, separator: String },
    Root(Value),
}

/// The walk of `select`, which has a `connect` clause, over its one FROM relation.
pub(crate) fn hierarchy(
    select: &Payload,
    outer: Option<&Scope<'_>>,
    context: &Context<'_>,
) -> Result<Walk, Refusal> {
    let construct = node_construct("connect");
    let log = context.log;
    let _rule = log.rule(HIERARCHY_RULE);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    if let Some(reason) = unsupported_shape(select) {
        return Err(refuse(&reason));
    }
    let connect = select
        .get("connect")
        .and_then(Value::as_object)
        .ok_or_else(|| refuse("a CONNECT BY of an unexpected shape"))?;
    require_understood(connect, &["connect", "start"], &construct)?;
    let start = connect
        .get("start")
        .ok_or_else(|| refuse("CONNECT BY without START WITH"))?;
    let condition = connect
        .get("connect")
        .ok_or_else(|| refuse("CONNECT BY without a condition"))?;
    let (from_sql, relations) = convert_from(select, outer, context)?;
    let [relation] = relations.as_slice() else {
        return Err(refuse("CONNECT BY over other than one relation"));
    };
    if relation.columns.iter().any(|column| {
        column.name.eq_ignore_ascii_case(HIERARCHY_LEVEL)
            || column.name.starts_with(HIERARCHY_COLUMN_PREFIX)
    }) {
        return Err(refuse("CONNECT BY over a relation with a LEVEL column"));
    }
    let mut carried: Vec<Carried> = Vec::new();
    let mut rewritten = select.clone();
    rewritten.remove("connect");
    for clause in ["expressions", "order_by"] {
        if let Some(value) = rewritten.remove(clause) {
            let (value, next) = rewrite(value, carried).map_err(|reason| refuse(&reason))?;
            carried = next;
            rewritten.insert(clause.to_owned(), value);
        }
    }
    let scope = Scope::new(relations.clone(), outer);
    let site = Site {
        scope: &scope,
        context,
        placement: Placement::ROW,
    };
    let prior_relation = ScopeRelation {
        alias: HIERARCHY_PRIOR.to_owned(),
        columns: relation.columns.clone(),
        merged: Vec::new(),
    };
    let both = Scope::new(vec![relation.clone(), prior_relation], outer);
    let joined = Site {
        scope: &both,
        context,
        placement: Placement::ROW,
    };
    let on = condition_sql(
        &prior_condition(condition, &relation.alias, &relation.alias)
            .map_err(|reason| refuse(&reason))?,
        joined.placed(Placement::FILTER),
    )?;
    let start = condition_sql(start, site.placed(Placement::FILTER))?;
    let source = quoted_identifier(&relation.alias, log);
    let prior = quoted_identifier(HIERARCHY_PRIOR, log);
    let mut anchor: Vec<String> = Vec::new();
    let mut step: Vec<String> = Vec::new();
    let mut columns: Vec<ColumnSpec> = Vec::new();
    for column in &relation.columns {
        let name = quoted_identifier(&column.name, log);
        anchor.push(format!("{source}.{name} AS {name}"));
        step.push(format!("{source}.{name} AS {name}"));
        columns.push(column.clone());
    }
    let depth_name = format!("{HIERARCHY_COLUMN_PREFIX}DEPTH");
    let depth = quoted_identifier(&depth_name, log);
    anchor.push(format!("CAST(1 AS BIGINT) AS {depth}"));
    step.push(format!("{prior}.{depth} + 1 AS {depth}"));
    columns.push(ColumnSpec::new(
        depth_name,
        SnowflakeType::Number {
            precision: 19,
            scale: 0,
        },
    ));
    let too_deep = raised(&format!(
        "{HIERARCHY_DEPTH_TAG}: a CONNECT BY walk deeper than {HIERARCHY_MAX_DEPTH} levels, as a \
         cycle makes it"
    ));
    let on = format!(
        "({on}) AND (CASE WHEN {prior}.{depth} < {HIERARCHY_MAX_DEPTH} THEN TRUE \
         ELSE CAST({too_deep} AS BOOLEAN) END)"
    );
    for (position, value) in carried.iter().enumerate() {
        let hidden = format!("{HIERARCHY_COLUMN_PREFIX}{position}");
        let name = quoted_identifier(&hidden, log);
        let (first, next, data_type) = match value {
            Carried::Level => {
                let data_type = SnowflakeType::Number {
                    precision: 38,
                    scale: 0,
                };
                let held = column_type(data_type, log).map_err(|reason| refuse(&reason))?;
                (
                    format!("CAST(1 AS {held})"),
                    format!("CAST({prior}.{name} + 1 AS {held})"),
                    data_type,
                )
            }
            Carried::Path { value, separator } => {
                let text = convert_expression(value, site)?;
                if !matches!(text.data_type, SnowflakeType::Varchar) {
                    return Err(refuse(&format!(
                        "SYS_CONNECT_BY_PATH of {}, not VARCHAR",
                        text.data_type
                    )));
                }
                let separator = string_literal(separator, log);
                (
                    format!("CAST({separator} || {} AS VARCHAR)", text.sql),
                    format!(
                        "CAST({prior}.{name} || {separator} || {} AS VARCHAR)",
                        text.sql
                    ),
                    SnowflakeType::Varchar,
                )
            }
            Carried::Root(value) => {
                let root = convert_expression(value, site)?;
                if root.is_variant_held() || root.data_type == SnowflakeType::SemiStructured {
                    return Err(refuse("CONNECT_BY_ROOT of a VARIANT"));
                }
                (root.sql, format!("{prior}.{name}"), root.data_type)
            }
        };
        anchor.push(format!("{first} AS {name}"));
        step.push(format!("{next} AS {name}"));
        columns.push(ColumnSpec::new(hidden, data_type));
    }
    let walk = quoted_identifier(HIERARCHY_WALK, log);
    let from_sql = format!(
        " FROM (WITH RECURSIVE {walk} AS (SELECT {}{from_sql} WHERE {start} \
         UNION ALL SELECT {}{from_sql} INNER JOIN {walk} AS {prior} ON {on}) \
         SELECT * FROM {walk}) AS {source}",
        anchor.join(", "),
        step.join(", ")
    );
    Ok(Walk {
        select: rewritten,
        from_sql,
        relations: vec![ScopeRelation {
            alias: relation.alias.clone(),
            columns,
            merged: Vec::new(),
        }],
    })
}

/// `node` with LEVEL, SYS_CONNECT_BY_PATH and CONNECT_BY_ROOT read from hidden columns.
fn rewrite(node: Value, carried: Vec<Carried>) -> Result<(Value, Vec<Carried>), String> {
    let mut carried = carried;
    if let Value::Object(object) = &node
        && let Some(value) = carried_value(object)?
    {
        let position = carried.len();
        carried.push(value);
        let column = json!({ "column": { "name": {
            "name": format!("{HIERARCHY_COLUMN_PREFIX}{position}"),
            "quoted": true,
        }}});
        return Ok((column, carried));
    }
    match node {
        Value::Object(object) => {
            if object.contains_key("prior") {
                return Err("PRIOR outside the CONNECT BY condition".to_owned());
            }
            if let Some(query) = HIERARCHY_NESTED_QUERIES
                .iter()
                .find(|query| object.contains_key(**query))
            {
                return Err(format!(
                    "a {query} in a hierarchical query's projection or ordering, where LEVEL and \
                     the CONNECT BY functions would not read the walk"
                ));
            }
            let mut rewritten = Map::new();
            for (key, value) in object {
                let (value, next) = rewrite(value, carried)?;
                carried = next;
                rewritten.insert(key, value);
            }
            Ok((Value::Object(rewritten), carried))
        }
        Value::Array(items) => {
            let mut rewritten = Vec::with_capacity(items.len());
            for item in items {
                let (item, next) = rewrite(item, carried)?;
                carried = next;
                rewritten.push(item);
            }
            Ok((Value::Array(rewritten), carried))
        }
        other => Ok((other, carried)),
    }
}

/// The carried value an expression node stands for, if any.
fn carried_value(object: &Map<String, Value>) -> Result<Option<Carried>, String> {
    if let Some(column) = object.get("column") {
        let name = column.get("name");
        let unquoted = name
            .and_then(|name| name.get("quoted"))
            .and_then(Value::as_bool)
            != Some(true);
        let level = name
            .and_then(|name| name.get("name"))
            .and_then(Value::as_str)
            .is_some_and(|name| name.eq_ignore_ascii_case(HIERARCHY_LEVEL));
        if level && unquoted && column.get("table").is_none_or(Value::is_null) {
            return Ok(Some(Carried::Level));
        }
        return Ok(None);
    }
    if let Some(root) = object.get("connect_by_root") {
        let value = match root.get("this") {
            Some(Value::Object(this)) if this.contains_key("var") => {
                let name = this
                    .get("var")
                    .and_then(|var| var.get("this"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| "a CONNECT_BY_ROOT of an unexpected shape".to_owned())?;
                json!({ "column": { "name": { "name": name } } })
            }
            Some(Value::Object(this)) if this.contains_key("column") => Value::Object(this.clone()),
            Some(Value::Object(this)) if this.contains_key("dot") => qualified_root(this)
                .ok_or_else(|| "CONNECT_BY_ROOT of other than a column".to_owned())?,
            _ => return Err("CONNECT_BY_ROOT of other than a column".to_owned()),
        };
        return Ok(Some(Carried::Root(value)));
    }
    if let Some(call) = object.get("function")
        && call
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| name.eq_ignore_ascii_case("SYS_CONNECT_BY_PATH"))
    {
        let arguments = call.get("args").and_then(Value::as_array);
        let Some([value, separator]) = arguments.map(Vec::as_slice) else {
            return Err("SYS_CONNECT_BY_PATH with other than two arguments".to_owned());
        };
        let separator = separator
            .get("literal")
            .filter(|literal| literal.get("literal_type").and_then(Value::as_str) == Some("string"))
            .and_then(|literal| literal.get("value"))
            .and_then(Value::as_str)
            .filter(|separator| !separator.is_empty())
            .ok_or_else(|| "a SYS_CONNECT_BY_PATH separator other than literal text".to_owned())?;
        return Ok(Some(Carried::Path {
            value: value.clone(),
            separator: separator.to_owned(),
        }));
    }
    Ok(None)
}

/// A CONNECT BY condition of `child = PRIOR parent` conjunctions; sides may be qualified by `own`.
fn prior_condition(condition: &Value, alias: &str, own: &str) -> Result<Value, String> {
    let object = condition
        .as_object()
        .ok_or_else(|| "a CONNECT BY condition of an unexpected shape".to_owned())?;
    if let Some(both) = object.get("and") {
        let left = prior_condition(both.get("left").unwrap_or(&Value::Null), alias, own)?;
        let right = prior_condition(both.get("right").unwrap_or(&Value::Null), alias, own)?;
        return Ok(json!({ "and": { "left": left, "right": right } }));
    }
    let Some(equal) = object.get("eq") else {
        return Err("a CONNECT BY condition other than child = PRIOR parent".to_owned());
    };
    let (left, right) = (
        equal.get("left").unwrap_or(&Value::Null),
        equal.get("right").unwrap_or(&Value::Null),
    );
    let (child, parent) = match (left.get("prior"), right.get("prior")) {
        (None, Some(parent)) => (left, parent),
        (Some(parent), None) => (right, parent),
        _ => {
            return Err("a CONNECT BY condition other than child = PRIOR parent".to_owned());
        }
    };
    let child = qualified(child, alias, own)?;
    let parent = qualified(
        parent.get("this").unwrap_or(&Value::Null),
        HIERARCHY_PRIOR,
        own,
    )?;
    Ok(json!({ "eq": { "left": child, "right": parent } }))
}

/// A column, unqualified or qualified by the walked relation's alias `own`, read from `alias`.
fn qualified(node: &Value, alias: &str, own: &str) -> Result<Value, String> {
    let column = node
        .get("column")
        .and_then(Value::as_object)
        .filter(|column| {
            column
                .get("table")
                .is_none_or(|table| table.is_null() || names_relation(table, own))
        })
        .ok_or_else(|| "a CONNECT BY side other than a column of the walked relation".to_owned())?;
    let mut column = column.clone();
    column.insert("table".to_owned(), json!({ "name": alias, "quoted": true }));
    Ok(json!({ "column": column }))
}

/// Why the clauses beside CONNECT BY keep `select` from converting, if they do.
fn unsupported_shape(select: &Payload) -> Option<String> {
    for clause in ["where_clause", "group_by", "having", "qualify", "joins"] {
        if select
            .get(clause)
            .is_some_and(|value| !value.is_null() && value != &json!([]))
        {
            return Some(format!(
                "CONNECT BY with {}, which no recording shows",
                clause.replace('_', " ").to_ascii_uppercase()
            ));
        }
    }
    select
        .get("order_by")
        .is_some_and(has_qualified_column)
        .then(|| "ORDER BY a qualified column after CONNECT BY, which Snowflake rejects".to_owned())
}

/// Whether `node` holds a column qualified by a relation name anywhere inside it.
fn has_qualified_column(node: &Value) -> bool {
    match node {
        Value::Object(object) => {
            object
                .get("column")
                .and_then(|column| column.get("table"))
                .is_some_and(|table| !table.is_null())
                || object.values().any(has_qualified_column)
        }
        Value::Array(items) => items.iter().any(has_qualified_column),
        _ => false,
    }
}

/// Whether `table` names the relation aliased `own`: unquoted in any case, quoted exactly.
fn names_relation(table: &Value, own: &str) -> bool {
    let Some(name) = table.get("name").and_then(Value::as_str) else {
        return false;
    };
    match table.get("quoted").and_then(Value::as_bool) == Some(true) {
        true => name == own,
        false => name.to_ascii_uppercase() == own,
    }
}

/// `relation.column` after CONNECT_BY_ROOT, which parses as a field of the relation's name.
fn qualified_root(this: &serde_json::Map<String, Value>) -> Option<Value> {
    let dot = this.get("dot")?;
    let table = dot.get("this")?.get("var")?.get("this")?.as_str()?;
    let field = dot.get("field")?;
    Some(json!({ "column": { "name": field, "table": { "name": table } } }))
}
