//! Queries: SELECT with its projection and clauses, and WITH.

use serde_json::Value;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::queries::clauses::{
    clause_condition, condition_sql, group_by, order_and_limit,
};
use crate::conversion::_helpers::queries::set_operations::convert_set_operation;
use crate::conversion::_helpers::queries::sources::convert_from;
use crate::conversion::_helpers::support::scope::{Context, Emitted, Scope};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, field, flag, identifier_name, list, missing, node_construct, node_kind,
    outside_conditions, payload_object, require_understood, unresolved,
};
use crate::conversion::constants::{
    ALIAS, COMMENT, COMMON_TABLE, DISTINCT, GROUP_BY, HAVING, QUALIFY, SELECT, STAR, WHERE,
};
use crate::conversion::models::ColumnSpec;

/// The SELECT clauses this rule converts; anything else set on the node is refused.
const SELECT_FIELDS: &[&str] = &[
    "expressions",
    "from",
    "joins",
    "where_clause",
    "group_by",
    "having",
    "qualify",
    "order_by",
    "limit",
    "offset",
    "distinct",
    "with",
];

/// A converted query and its result columns.
pub(crate) type ConvertedQuery = (String, Vec<ColumnSpec>);

/// Where a query is converted: the enclosing query's scope, the context and whether names matter.
#[derive(Clone, Copy, Debug)]
pub(crate) struct QueryRequest<'a> {
    pub(crate) outer: Option<&'a Scope<'a>>,
    pub(crate) context: &'a Context<'a>,
    pub(crate) named: bool,
}

/// One result column: its name, when it has one, and its value.
type Projected = (Option<String>, Emitted);

/// Converts a query; `named` requires every result column to have a Snowflake name.
pub(crate) fn convert_query(
    node: &Value,
    outer: Option<&Scope<'_>>,
    context: &Context<'_>,
    named: bool,
) -> Result<ConvertedQuery, Refusal> {
    let Some((kind, payload)) = node_kind(node) else {
        return Err(missing(
            node_construct("unknown"),
            "a query has an unexpected shape".to_owned(),
        ));
    };
    match kind {
        "select" => convert_select(payload, outer, context, named),
        "union" | "except" | "intersect" => {
            let request = QueryRequest {
                outer,
                context,
                named,
            };
            convert_set_operation(kind, payload, request)
        }
        "subquery" => {
            let construct = node_construct("subquery");
            let payload = payload_object(payload, &construct)?;
            require_understood(payload, &["this"], &construct)?;
            let (sql, columns) =
                convert_query(field(payload, "this", &construct)?, outer, context, named)?;
            Ok((format!("({sql})"), columns))
        }
        other => Err(missing(
            node_construct(other),
            format!("no rule converts node:{other} as a query"),
        )),
    }
}

fn convert_select(
    payload: &Value,
    outer: Option<&Scope<'_>>,
    context: &Context<'_>,
    named: bool,
) -> Result<ConvertedQuery, Refusal> {
    let construct = node_construct("select");
    let select = payload_object(payload, &construct)?;
    require_understood(select, SELECT_FIELDS, &construct)?;
    let _rule = context.log.rule(SELECT);
    let (with_sql, context) = convert_with(select.get("with"), outer, context)?;
    let (from_sql, relations) = convert_from(select, outer, &context)?;
    let mut scope = Scope::new(relations, outer);
    let mut items: Vec<String> = Vec::new();
    let mut outputs: Vec<(String, Emitted)> = Vec::new();
    for item in list(select, "expressions") {
        let site = Site {
            scope: &scope,
            context: &context,
            placement: Placement::PROJECTION,
        };
        for (name, emitted) in convert_projection(item, site, named)? {
            items.push(projected_sql(name.as_deref(), &emitted, &context));
            outputs.push((name.unwrap_or_default(), emitted));
        }
        scope.aliases = outputs.clone();
    }
    let columns: Vec<ColumnSpec> = outputs
        .iter()
        .map(|(name, emitted)| ColumnSpec {
            name: name.clone(),
            data_type: emitted.data_type,
        })
        .collect();
    if named {
        require_distinct_names(&columns)?;
    }
    let distinct = match flag(select, "distinct") {
        true => {
            let _rule = context.log.rule(DISTINCT);
            "DISTINCT "
        }
        false => "",
    };
    let tail = select_tail(select, &scope, &context, &outputs)?;
    Ok((
        format!(
            "{with_sql}SELECT {distinct}{}{from_sql}{tail}",
            items.join(", ")
        ),
        columns,
    ))
}

fn projected_sql(name: Option<&str>, emitted: &Emitted, context: &Context<'_>) -> String {
    match name {
        Some(name) => format!(
            "{} AS {}",
            emitted.sql,
            quoted_identifier(name, context.log)
        ),
        None => emitted.sql.clone(),
    }
}

/// WHERE, GROUP BY, HAVING, QUALIFY, ORDER BY and LIMIT.
fn select_tail(
    select: &Payload,
    scope: &Scope<'_>,
    context: &Context<'_>,
    outputs: &[(String, Emitted)],
) -> Result<String, Refusal> {
    let site = Site {
        scope,
        context,
        placement: Placement::ROW,
    };
    let mut sql = String::new();
    if let Some(condition) = clause_condition(select, "where_clause") {
        let _rule = context.log.rule(WHERE);
        sql.push_str(&format!(
            " WHERE {}",
            condition_sql(condition, site.placed(Placement::FILTER))?
        ));
    }
    if select.get("group_by").is_some_and(|group| !group.is_null()) {
        let _rule = context.log.rule(GROUP_BY);
        sql.push_str(&group_by(select, site, outputs.len())?);
    }
    if let Some(condition) = clause_condition(select, "having") {
        let _rule = context.log.rule(HAVING);
        let site = site.placed(Placement::GROUPED);
        sql.push_str(&format!(" HAVING {}", condition_sql(condition, site)?));
    }
    if let Some(condition) = clause_condition(select, "qualify") {
        let _rule = context.log.rule(QUALIFY);
        let site = site.placed(Placement::PROJECTION);
        sql.push_str(&format!(" QUALIFY {}", condition_sql(condition, site)?));
    }
    sql.push_str(&order_and_limit(select, site, outputs)?);
    Ok(sql)
}

/// WITH: each CTE converted in turn, each visible to the ones after it.
pub(crate) fn convert_with<'a>(
    with: Option<&Value>,
    outer: Option<&Scope<'_>>,
    context: &Context<'a>,
) -> Result<(String, Context<'a>), Refusal> {
    let mut extended = context.clone();
    let Some(with) = with.filter(|with| !with.is_null()) else {
        return Ok((String::new(), extended));
    };
    let construct = node_construct("with");
    let with = payload_object(with, &construct)?;
    require_understood(with, &["ctes"], &construct)?;
    let fixture = |name: &str| {
        outer.is_none()
            && context
                .options
                .fixture
                .as_ref()
                .is_some_and(|fixture| fixture.relation.eq_ignore_ascii_case(name))
    };
    let names: Vec<Option<String>> = list(with, "ctes")
        .iter()
        .map(|cte| cte.get("alias").and_then(identifier_name))
        .collect();
    let only_fixture = names.iter().flatten().all(|name| fixture(name));
    let _rule = (!only_fixture).then(|| context.log.rule(COMMON_TABLE));
    let mut parts: Vec<String> = Vec::new();
    for cte in list(with, "ctes") {
        let cte = payload_object(cte, &construct)?;
        require_understood(cte, &["alias", "this"], &construct)?;
        let name = identifier_name(field(cte, "alias", &construct)?)
            .ok_or_else(|| missing(construct.clone(), "a CTE has no name".to_owned()))?;
        let _muted = fixture(&name).then(|| context.log.muted());
        let (sql, columns) =
            convert_query(field(cte, "this", &construct)?, outer, &extended, true)?;
        parts.push(format!(
            "{} AS ({sql})",
            quoted_identifier(&name, context.log)
        ));
        extended.ctes.push((name, columns));
    }
    Ok((format!("WITH {} ", parts.join(", ")), extended))
}

fn convert_projection(
    item: &Value,
    site: Site<'_>,
    named: bool,
) -> Result<Vec<Projected>, Refusal> {
    match node_kind(item) {
        Some(("star", payload)) => star(payload, site),
        Some(("annotated", payload)) => {
            let construct = node_construct("annotated");
            let _rule = site.context.log.rule(COMMENT);
            let annotated = payload_object(payload, &construct)?;
            require_understood(annotated, &["this", "trailing_comments"], &construct)?;
            convert_projection(field(annotated, "this", &construct)?, site, named)
        }
        Some(("alias", payload)) => {
            let _rule = site.context.log.rule(ALIAS);
            let construct = node_construct("alias");
            let alias = payload_object(payload, &construct)?;
            require_understood(alias, &["this", "alias"], &construct)?;
            let name = identifier_name(field(alias, "alias", &construct)?)
                .ok_or_else(|| missing(construct.clone(), "an alias has no name".to_owned()))?;
            let emitted = convert_expression(field(alias, "this", &construct)?, site)?;
            Ok(vec![(Some(name), emitted)])
        }
        Some(("column", payload)) => {
            let name = payload.get("name").and_then(identifier_name);
            Ok(vec![(name, convert_expression(item, site)?)])
        }
        Some((kind, _)) => {
            let emitted = convert_expression(item, site)?;
            if named {
                return Err(outside_conditions(
                    node_construct(kind),
                    "an unaliased expression gets a Snowflake-generated column name".to_owned(),
                ));
            }
            Ok(vec![(None, emitted)])
        }
        None => Err(missing(
            node_construct("unknown"),
            "a select item has an unexpected shape".to_owned(),
        )),
    }
}

fn star(payload: &Value, site: Site<'_>) -> Result<Vec<Projected>, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(STAR);
    let construct = node_construct("star");
    let star = payload_object(payload, &construct)?;
    require_understood(star, &["table"], &construct)?;
    let qualifier = star.get("table").and_then(identifier_name);
    let mut expanded: Vec<Projected> = Vec::new();
    for relation in site.scope.star_relations(qualifier.as_deref())? {
        for column in &relation.columns {
            let sql = format!(
                "{}.{}",
                quoted_identifier(&relation.alias, log),
                quoted_identifier(&column.name, log)
            );
            expanded.push((
                Some(column.name.clone()),
                Emitted::new(sql, column.data_type),
            ));
        }
    }
    Ok(expanded)
}

/// Refuses result columns whose names repeat, which DuckDB would rename.
pub(crate) fn require_distinct_names(columns: &[ColumnSpec]) -> Result<(), Refusal> {
    for (index, column) in columns.iter().enumerate() {
        if columns[..index]
            .iter()
            .any(|earlier| earlier.name.eq_ignore_ascii_case(&column.name))
        {
            return Err(unresolved(format!(
                "the result column name `{}` repeats, possibly differing only in case",
                column.name
            )));
        }
    }
    Ok(())
}
