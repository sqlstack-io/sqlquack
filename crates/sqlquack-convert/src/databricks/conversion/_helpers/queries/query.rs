//! A query to DuckDB: the input fixture, one query block over it, and its clauses.

use serde_json::Value;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::dispatch::expression;
use crate::databricks::conversion::_helpers::expressions::operators::{as_type, common_type};
use crate::databricks::conversion::_helpers::support::scope::{Columns, Scope, Site};
use crate::databricks::conversion::_helpers::support::tree::{
    digits_value, field, flag, identifier, list, node_kind, quoted, refused, understood,
};
use crate::databricks::conversion::constants::{
    CONTROL_CHARACTERS, CONTROL_ERROR, FIXTURE_RULE, LAMBDA_ELEMENT, ORDER_RULE, RENDERED_ELEMENTS,
    SELECT_RULE,
};
use crate::databricks::conversion::models::{DatabricksType, Element, Emitted};

/// A common table expression: its name as written, its DuckDB definition and its columns.
struct Table {
    name: String,
    sql: String,
    columns: Columns,
}

/// The DuckDB SQL of a statement and its result columns, or the refusal of its first unhandled part.
pub(crate) fn statement(
    node: &Value,
    log: &ProvenanceLog,
) -> Result<(String, Vec<(String, DatabricksType)>), Refusal> {
    match node_kind(node) {
        Some(("select", payload)) => select(payload, log),
        Some((kind, _)) => Err(refused(
            kind,
            format!("no rule converts a {kind} statement"),
        )),
        None => Err(refused(
            "statement",
            "the statement is no single node".to_owned(),
        )),
    }
}

/// One query block: `[WITH input AS (...)] SELECT ... [FROM ...] [WHERE] [GROUP BY] [HAVING] ...`.
fn select(
    payload: &Value,
    log: &ProvenanceLog,
) -> Result<(String, Vec<(String, DatabricksType)>), Refusal> {
    let select = understood(
        payload,
        "select",
        &[
            "expressions",
            "from",
            "where_clause",
            "group_by",
            "having",
            "order_by",
            "limit",
            "distinct",
            "with",
        ],
    )?;
    let _rule = log.rule(SELECT_RULE);
    let tables = match field(select, "with") {
        Value::Null => Vec::new(),
        with => common_tables(with, log)?,
    };
    let mut scope = from(field(select, "from"), &tables)?;
    let mut sql = String::new();
    if !tables.is_empty() {
        let definitions: Vec<&str> = tables.iter().map(|table| table.sql.as_str()).collect();
        sql.push_str(&format!("WITH {} ", definitions.join(", ")));
    }
    sql.push_str(match flag(select, "distinct") {
        true => "SELECT DISTINCT ",
        false => "SELECT ",
    });
    let mut outputs: Vec<(String, Emitted)> = Vec::new();
    for item in list(select, "expressions") {
        let projection = Site {
            log,
            scope: &scope,
            aggregates: true,
            outputs: false,
        };
        outputs.extend(projected(item, projection)?);
    }
    if outputs.is_empty() {
        return Err(refused("select", "the query projects nothing".to_owned()));
    }
    let items: Vec<String> = outputs
        .iter()
        .map(|(name, emitted)| format!("{} AS {}", rendered(emitted), quoted(name)))
        .collect();
    sql.push_str(&items.join(", "));
    if let Some(relation) = &scope.relation {
        let table = tables
            .iter()
            .find(|table| table.name.eq_ignore_ascii_case(relation))
            .map_or(relation.as_str(), |table| table.name.as_str());
        sql.push_str(&format!(" FROM {}", quoted(table)));
        if table != relation {
            sql.push_str(&format!(" AS {}", quoted(relation)));
        }
    }
    let row = Site {
        log,
        scope: &scope,
        aggregates: false,
        outputs: false,
    };
    if let Some(filter) = clause(select, "where_clause")? {
        sql.push_str(&format!(" WHERE {}", condition(filter, row, "where")?));
    }
    if let Some(group_by) = select.get("group_by").filter(|group| !group.is_null()) {
        let group = understood(group_by, "group_by", &["expressions"])?;
        let mut keys: Vec<String> = Vec::new();
        for key in list(group, "expressions") {
            keys.push(not_positional(expression(key, row)?, "group_by")?.sql);
        }
        sql.push_str(&format!(" GROUP BY {}", keys.join(", ")));
    }
    let aggregated = Site {
        aggregates: true,
        ..row
    };
    if let Some(having) = clause(select, "having")? {
        sql.push_str(&format!(
            " HAVING {}",
            condition(having, aggregated, "having")?
        ));
    }
    let columns: Vec<(String, DatabricksType)> = outputs
        .iter()
        .map(|(name, emitted)| (name.clone(), emitted.data_type))
        .collect();
    scope.outputs = outputs;
    if let Some(order_by) = select.get("order_by").filter(|order| !order.is_null()) {
        let ordering = Site {
            log,
            scope: &scope,
            aggregates: true,
            outputs: true,
        };
        sql.push_str(&format!(" ORDER BY {}", order(order_by, ordering)?));
    }
    if let Some(limit) = select.get("limit").filter(|limit| !limit.is_null()) {
        sql.push_str(&format!(" LIMIT {}", row_limit(limit)?));
    }
    Ok((sql, columns))
}

/// A result column as Databricks returns it: an array as a JSON list of each element's text.
fn rendered(emitted: &Emitted) -> String {
    match emitted.data_type {
        DatabricksType::Array(Element::String) => format!(
            "CASE WHEN list_bool_or(list_transform({x}, lambda {LAMBDA_ELEMENT}: \
             regexp_matches({LAMBDA_ELEMENT}, '{CONTROL_CHARACTERS}'))) THEN error('{CONTROL_ERROR}') \
             ELSE CAST(to_json({x}) AS VARCHAR) END",
            x = emitted.sql
        ),
        DatabricksType::Array(_) => format!(
            "CAST(to_json(list_transform({}, lambda {LAMBDA_ELEMENT}: CAST({LAMBDA_ELEMENT} AS VARCHAR))) AS VARCHAR)",
            emitted.sql
        ),
        _ => emitted.sql.clone(),
    }
}

/// A clause holding one expression, such as `WHERE` or `HAVING`.
fn clause<'a>(
    select: &'a serde_json::Map<String, Value>,
    name: &str,
) -> Result<Option<&'a Value>, Refusal> {
    match select.get(name).filter(|clause| !clause.is_null()) {
        None => Ok(None),
        Some(clause) => Ok(Some(field(understood(clause, name, &["this"])?, "this"))),
    }
}

/// A boolean condition's SQL.
fn condition(node: &Value, site: Site<'_>, kind: &str) -> Result<String, Refusal> {
    let condition = expression(node, site)?;
    match condition.data_type {
        DatabricksType::Boolean | DatabricksType::Null => Ok(condition.sql),
        other => Err(refused(
            kind,
            format!("no rule reads a {} as a condition", other.name()),
        )),
    }
}

/// An expression that is no literal: a literal number in `GROUP BY` or `ORDER BY` is a position.
fn not_positional(emitted: Emitted, kind: &str) -> Result<Emitted, Refusal> {
    match emitted.literal {
        Some(_) => Err(refused(
            kind,
            format!("no rule converts a literal in {kind}, which may name a position"),
        )),
        None => Ok(emitted),
    }
}

/// The `WITH` list: only the input fixture, `input AS (SELECT * FROM VALUES ... AS input(...))`.
fn common_tables(with: &Value, log: &ProvenanceLog) -> Result<Vec<Table>, Refusal> {
    let with = understood(with, "with", &["ctes"])?;
    let mut tables: Vec<Table> = Vec::new();
    for cte in list(with, "ctes") {
        let cte = understood(cte, "cte", &["alias", "this"])?;
        let name = identifier(field(cte, "alias"))?;
        if tables
            .iter()
            .any(|table| table.name.eq_ignore_ascii_case(&name))
        {
            return Err(refused("cte", format!("`{name}` is defined twice")));
        }
        tables.push(fixture(&name, field(cte, "this"), log)?);
    }
    Ok(tables)
}

/// The fixture's rows as a DuckDB `VALUES` list, each column cast to its one Databricks type.
fn fixture(name: &str, node: &Value, log: &ProvenanceLog) -> Result<Table, Refusal> {
    let not_fixture = || {
        refused(
            "cte",
            format!(
                "no rule converts `{name}` unless it is `SELECT * FROM VALUES ... AS {name}(...)`"
            ),
        )
    };
    let Some(("select", payload)) = node_kind(node) else {
        return Err(not_fixture());
    };
    let select = understood(payload, "select", &["expressions", "from"])?;
    match list(select, "expressions") {
        [star] => match node_kind(star) {
            Some(("star", star)) => understood(star, "star", &[]).map(drop)?,
            _ => return Err(not_fixture()),
        },
        _ => return Err(not_fixture()),
    }
    let from = understood(field(select, "from"), "from", &["expressions"])?;
    let [source] = list(from, "expressions") else {
        return Err(not_fixture());
    };
    let Some(("values", values)) = node_kind(source) else {
        return Err(not_fixture());
    };
    let values = understood(
        values,
        "values",
        &["expressions", "alias", "column_aliases"],
    )?;
    let alias = identifier(field(values, "alias"))?;
    if !alias.eq_ignore_ascii_case(name) {
        return Err(not_fixture());
    }
    let names: Vec<String> = list(values, "column_aliases")
        .iter()
        .map(identifier)
        .collect::<Result<_, _>>()?;
    let _rule = log.rule(FIXTURE_RULE);
    let empty = Scope::default();
    let site = Site {
        log,
        scope: &empty,
        aggregates: false,
        outputs: false,
    };
    let mut rows: Vec<Vec<Emitted>> = Vec::new();
    for tuple in list(values, "expressions") {
        let tuple = understood(tuple, "tuple", &["expressions"])?;
        let cells = list(tuple, "expressions");
        if cells.len() != names.len() {
            return Err(refused(
                "values",
                format!(
                    "a row has {} values for {} columns",
                    cells.len(),
                    names.len()
                ),
            ));
        }
        rows.push(
            cells
                .iter()
                .map(|cell| expression(cell, site))
                .collect::<Result<_, _>>()?,
        );
    }
    if rows.is_empty() || names.is_empty() {
        return Err(not_fixture());
    }
    let mut columns: Columns = Vec::new();
    for (index, column) in names.iter().enumerate() {
        let cells: Vec<Emitted> = rows.iter().map(|row| row[index].clone()).collect();
        match common_type(&cells) {
            Some(data_type) if data_type != DatabricksType::Null => {
                columns.push((column.clone(), data_type));
            }
            _ => {
                return Err(refused(
                    "values",
                    format!("no rule types the input column `{column}` as both engines do"),
                ));
            }
        }
    }
    let rows_sql: Vec<String> = rows.iter().map(|row| typed_row(row, &columns)).collect();
    let quoted_names: Vec<String> = names.iter().map(|name| quoted(name)).collect();
    Ok(Table {
        name: name.to_owned(),
        sql: format!(
            "{} AS (SELECT * FROM (VALUES {}) AS {}({}))",
            quoted(name),
            rows_sql.join(", "),
            quoted(name),
            quoted_names.join(", ")
        ),
        columns,
    })
}

/// One fixture row, each value cast to its column's type, NULLs included.
fn typed_row(row: &[Emitted], columns: &Columns) -> String {
    let mut cells: Vec<String> = Vec::with_capacity(row.len());
    for (cell, (_, data_type)) in row.iter().zip(columns) {
        cells.push(match cell.data_type {
            DatabricksType::Null => format!("CAST({} AS {})", cell.sql, data_type.duckdb_name()),
            _ => as_type(cell, *data_type),
        });
    }
    format!("({})", cells.join(", "))
}

/// The relation the block reads: none, or one common table by name.
fn from(node: &Value, tables: &[Table]) -> Result<Scope, Refusal> {
    if node.is_null() {
        return Ok(Scope::default());
    }
    let from = understood(node, "from", &["expressions"])?;
    let [source] = list(from, "expressions") else {
        return Err(refused(
            "from",
            "no rule converts a FROM of several relations".to_owned(),
        ));
    };
    let Some(("table", table)) = node_kind(source) else {
        let kind = node_kind(source).map_or("from", |(kind, _)| kind);
        return Err(refused(kind, format!("no rule reads FROM a {kind}")));
    };
    let table = understood(table, "table", &["name", "alias"])?;
    let name = identifier(field(table, "name"))?;
    let Some(defined) = tables
        .iter()
        .find(|table| table.name.eq_ignore_ascii_case(&name))
    else {
        return Err(refused(
            "table",
            format!("`{name}` is no relation the statement defines"),
        ));
    };
    let relation = match field(table, "alias") {
        Value::Null => defined.name.clone(),
        alias => identifier(alias)?,
    };
    Ok(Scope {
        relation: Some(relation),
        columns: defined.columns.clone(),
        outputs: Vec::new(),
    })
}

/// A projection item's result columns: an aliased expression, a column, or `*`.
fn projected(item: &Value, site: Site<'_>) -> Result<Vec<(String, Emitted)>, Refusal> {
    let named = match node_kind(item) {
        Some(("alias", payload)) => {
            let alias = understood(payload, "alias", &["this", "alias"])?;
            let name = identifier(field(alias, "alias"))?;
            vec![(name, expression(field(alias, "this"), site)?)]
        }
        Some(("star", payload)) => {
            understood(payload, "star", &[])?;
            site.scope
                .columns
                .iter()
                .map(|(name, data_type)| {
                    (
                        name.clone(),
                        Emitted {
                            sql: quoted(name),
                            data_type: *data_type,
                            literal: None,
                        },
                    )
                })
                .collect()
        }
        Some(("column", payload)) => {
            let column = understood(payload, "column", &["name", "table"])?;
            let written = identifier(field(column, "name"))?;
            let emitted = expression(item, site)?;
            let defined = site.scope.column(&written).map(|(name, _)| name.as_str());
            match defined == Some(written.as_str()) {
                true => vec![(written, emitted)],
                false => {
                    return Err(refused(
                        "column",
                        format!(
                            "no rule names the result of `{written}`, spelled unlike its column"
                        ),
                    ));
                }
            }
        }
        _ => {
            return Err(refused(
                "select",
                "no rule names an unaliased result column that is no column".to_owned(),
            ));
        }
    };
    let untyped = named.iter().find(|(_, emitted)| match emitted.data_type {
        DatabricksType::Null => true,
        DatabricksType::Array(element) => !RENDERED_ELEMENTS.contains(&element),
        _ => false,
    });
    match untyped {
        Some((name, emitted)) => Err(refused(
            "select",
            format!(
                "no rule writes the {} result column `{name}` as Databricks returns it",
                emitted.data_type.name()
            ),
        )),
        None => Ok(named),
    }
}

/// `ORDER BY` with each item's null ordering written out as Databricks applies it.
fn order(order_by: &Value, site: Site<'_>) -> Result<String, Refusal> {
    let order_by = understood(order_by, "order_by", &["expressions"])?;
    let _rule = site.log.rule(ORDER_RULE);
    let mut items: Vec<String> = Vec::new();
    for ordered in list(order_by, "expressions") {
        let ordered = understood(ordered, "ordered", &["this", "desc", "nulls_first"])?;
        let key = not_positional(expression(field(ordered, "this"), site)?, "order_by")?;
        if let DatabricksType::Array(_) = key.data_type {
            return Err(refused(
                "order_by",
                "no rule orders by an array as Databricks does".to_owned(),
            ));
        }
        let desc = flag(ordered, "desc");
        let nulls_first = field(ordered, "nulls_first").as_bool().unwrap_or(!desc);
        items.push(format!(
            "{}{}{}",
            key.sql,
            match desc {
                true => " DESC",
                false => " ASC",
            },
            match nulls_first {
                true => " NULLS FIRST",
                false => " NULLS LAST",
            }
        ));
    }
    Ok(items.join(", "))
}

/// A `LIMIT` of a non-negative integer literal.
fn row_limit(limit: &Value) -> Result<String, Refusal> {
    let limit = understood(limit, "limit", &["this"])?;
    let count = match node_kind(field(limit, "this")) {
        Some(("literal", literal)) => {
            let literal = understood(literal, "literal", &["literal_type", "value"])?;
            match field(literal, "literal_type").as_str() {
                Some("number") => field(literal, "value").as_str().and_then(digits_value),
                _ => None,
            }
        }
        _ => None,
    };
    match count {
        Some(count) => Ok(count.to_string()),
        None => Err(refused(
            "limit",
            "no rule converts a LIMIT other than a non-negative integer".to_owned(),
        )),
    }
}
