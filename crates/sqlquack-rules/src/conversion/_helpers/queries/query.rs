//! Queries: SELECT with its projection and clauses, and WITH.

use serde_json::{Map, Value, json};
use sqlquack_core::classification::models::Construct;
use sqlquack_core::provenance::constants::QUERY_CONSTRUCT;
use sqlquack_core::provenance::models::ShapeArgument;
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::main::unresolved::unresolved;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::ordered_aggregates::order_checked;
use crate::conversion::_helpers::expressions::scalar::convert_held;
use crate::conversion::_helpers::expressions::windows::canonical_variant;
use crate::conversion::_helpers::queries::clauses::{
    clause_condition, condition_sql, group_by, group_positions, order_and_limit, variant_group_keys,
};
use crate::conversion::_helpers::queries::group_all::group_all_positions;
use crate::conversion::_helpers::queries::hierarchy::hierarchy;
use crate::conversion::_helpers::queries::set_operations::convert_set_operation;
use crate::conversion::_helpers::queries::sources::convert_from;
use crate::conversion::_helpers::support::scope::read_column;
use crate::conversion::_helpers::support::scope::{Context, Emitted, Scope};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, field, flag, function_construct, identifier_name, list, node_construct, node_kind,
    payload_object, require_understood,
};
use crate::conversion::constants::WINDOW_MARKER;
use crate::conversion::constants::{
    ALIAS, COMMENT, COMMON_TABLE, DISTINCT, GENERATOR_SEQUENCE_COLUMN, GROUP_BY, HAVING,
    HIERARCHY_COLUMN_PREFIX, QUALIFY, SELECT, STAR, SYSTEM_TYPEOF_NAME, TYPE_SUFFIX_UNREMOVED,
    USING_ALIAS_PREFIX, VARIANT_HELD_UNREAD, WHERE,
};
use crate::conversion::constants::{VALUES_COLUMN_PREFIX, VALUES_ROWS, VALUES_ROWS_LIMIT};
use sqlquack_semantics::dialects::constants::SNOWFLAKE_TO_DUCKDB;
use sqlquack_semantics::scopes::main::star_relations::star_relations;
use sqlquack_semantics::scopes::models::ScopeRelation;
use sqlquack_semantics::types::models::ColumnSpec;

/// The SELECT clauses this rule converts; anything else set on the node is refused.
const SELECT_FIELDS: &[&str] = &[
    "expressions",
    "connect",
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

/// A grouped VARIANT expression's SQL and the SQL it is grouped as.
type Regrouped = (String, String);

/// One result column: its name, when it has one, and its value.
type Projected = (Option<String>, Emitted);

/// Converts a query; `named` requires every result column to have a Snowflake name.
pub(crate) fn convert_query(
    node: &Value,
    outer: Option<&Scope<'_>>,
    context: &Context<'_>,
    named: bool,
) -> Result<ConvertedQuery, Refusal> {
    context.log.enter(QUERY_CONSTRUCT, true);
    let converted = convert_query_kind(node, outer, context, named);
    let argument = match &converted {
        Ok(_) => Some(ShapeArgument {
            family: QUERY_CONSTRUCT.to_owned(),
            ..ShapeArgument::default()
        }),
        Err(_) => None,
    };
    context
        .log
        .leave(argument, (std::ptr::from_ref(node) as usize, None));
    let (sql, columns) = converted.map_err(|refusal| context.locator.placed(refusal, node))?;
    Ok((context.locator.identified(&sql, node), columns))
}

fn convert_query_kind(
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
        "annotated" => {
            let construct = node_construct("annotated");
            let _rule = context.log.rule(COMMENT);
            let payload = payload_object(payload, &construct)?;
            require_understood(payload, &["this", "trailing_comments"], &construct)?;
            convert_query(field(payload, "this", &construct)?, outer, context, named)
        }
        "values" => {
            let _rule = context.log.rule(VALUES_ROWS);
            let (sql, mut columns) = convert_query(&values_rows(payload)?, outer, context, named)?;
            let filled = filled_columns(payload);
            for (position, column) in columns.iter_mut().enumerate() {
                column.not_null = filled.get(position).copied().unwrap_or(false);
            }
            Ok((sql, columns))
        }
        other => Err(missing(
            node_construct(other),
            format!("no rule converts node:{other} as a query"),
        )),
    }
}

/// For each VALUES column, whether every row holds a non-NULL text or number literal there.
fn filled_columns(payload: &Value) -> Vec<bool> {
    let mut filled: Vec<bool> = Vec::new();
    let rows = payload
        .as_object()
        .map(|values| list(values, "expressions"));
    for (index, row) in rows.unwrap_or_default().iter().enumerate() {
        let cells = row
            .as_object()
            .map(|row| list(row, "expressions"))
            .unwrap_or_default();
        if index == 0 {
            filled = vec![true; cells.len()];
        }
        for (position, cell) in cells.iter().enumerate() {
            let literal = matches!(
                node_kind(cell),
                Some(("literal", literal))
                    if matches!(literal.get("literal_type").and_then(Value::as_str), Some("string" | "number"))
            );
            if let Some(slot) = filled.get_mut(position) {
                *slot &= literal;
            }
        }
    }
    filled
}

/// VALUES rows as the UNION ALL of one SELECT per row, its columns named COLUMN1, COLUMN2 and so on.
fn values_rows(payload: &Value) -> Result<Value, Refusal> {
    let construct = node_construct("values");
    let values = payload_object(payload, &construct)?;
    require_understood(values, &["expressions"], &construct)?;
    if list(values, "expressions").len() > VALUES_ROWS_LIMIT {
        return Err(outside_conditions(
            construct,
            format!(
                "VALUES with more than {VALUES_ROWS_LIMIT} rows, beyond the rows the conversion is \
                 checked with"
            ),
        ));
    }
    let mut selects: Vec<Value> = Vec::new();
    let mut width: Option<usize> = None;
    for row in list(values, "expressions") {
        let row = payload_object(row, &construct)?;
        require_understood(row, &["expressions"], &construct)?;
        let cells = list(row, "expressions");
        if *width.get_or_insert(cells.len()) != cells.len() || cells.is_empty() {
            return Err(outside_conditions(
                construct.clone(),
                "VALUES rows of different or no widths".to_owned(),
            ));
        }
        let expressions: Vec<Value> = cells
            .iter()
            .enumerate()
            .map(|(position, cell)| {
                json!({"alias": {
                    "alias": {"name": format!("{VALUES_COLUMN_PREFIX}{}", position + 1)},
                    "this": cell,
                }})
            })
            .collect();
        selects.push(json!({"select": {"expressions": expressions}}));
    }
    balanced_union(selects).ok_or_else(|| missing(construct, "VALUES without rows".to_owned()))
}

/// The rows' UNION ALL in order, paired level by level so it nests log2(rows) deep, not one per row.
fn balanced_union(mut selects: Vec<Value>) -> Option<Value> {
    while selects.len() > 1 {
        let mut rows = selects.into_iter();
        let mut paired: Vec<Value> = Vec::new();
        while let Some(left) = rows.next() {
            paired.push(match rows.next() {
                Some(right) => union_all(left, right),
                None => left,
            });
        }
        selects = paired;
    }
    selects.pop()
}

/// `left UNION ALL right`, moving both trees in: `json!` would copy `left`, once per row.
fn union_all(left: Value, right: Value) -> Value {
    let union = Map::from_iter([
        ("all".to_owned(), Value::Bool(true)),
        ("left".to_owned(), left),
        ("right".to_owned(), right),
    ]);
    Value::Object(Map::from_iter([("union".to_owned(), Value::Object(union))]))
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
    let walk;
    let (select, from_sql, relations) = match select.get("connect") {
        Some(connect) if !connect.is_null() => {
            walk = hierarchy(select, outer, &context)?;
            (&walk.select, walk.from_sql.clone(), walk.relations.clone())
        }
        _ => {
            let (from_sql, relations) = convert_from(select, outer, &context)?;
            (select, from_sql, relations)
        }
    };
    let mut scope = Scope::new(relations, outer);
    scope.grouped = select.get("group_by").is_some_and(|group| !group.is_null());
    let mut grouped_variants = variant_group_keys(
        select,
        Site {
            scope: &scope,
            context: &context,
            placement: Placement::ROW,
        },
    )?;
    let group_positions = group_positions(select);
    let mut names: Vec<Option<String>> = Vec::new();
    let mut outputs: Vec<(String, Emitted)> = Vec::new();
    for item in list(select, "expressions") {
        let site = Site {
            scope: &scope,
            context: &context,
            placement: Placement {
                order_free: context.free_projections,
                suffixed: true,
                ..Placement::PROJECTION
            },
        };
        for (name, emitted) in convert_projection(item, site, named)? {
            outputs.push((name.clone().unwrap_or_default(), emitted));
            names.push(name);
        }
        scope.aliases = outputs.clone();
    }
    let mut positioned: Vec<usize> = Vec::new();
    for (index, (_, emitted)) in outputs.iter_mut().enumerate() {
        let position = u64::try_from(index + 1).unwrap_or(u64::MAX);
        if group_positions.contains(&position)
            && emitted.data_type == SnowflakeType::SemiStructured
            && !emitted.is_variant_held()
        {
            let grouped =
                canonical_variant(emitted.clone(), &node_construct("group_by"), &context)?;
            grouped_variants.push((emitted.sql.clone(), grouped.sql.clone()));
            *emitted = grouped;
            positioned.push(index);
        }
    }
    let mut items: Vec<String> = Vec::with_capacity(outputs.len());
    for (index, ((_, emitted), name)) in outputs.iter_mut().zip(&names).enumerate() {
        if !positioned.contains(&index) {
            for (converted, grouped) in &grouped_variants {
                emitted.sql = emitted.sql.replace(converted.as_str(), grouped);
            }
        }
        items.push(projected_sql(name.as_deref(), emitted, &context));
    }
    let plain = list(select, "joins").is_empty() && !scope.grouped;
    let columns: Vec<ColumnSpec> = outputs
        .iter()
        .map(|(name, emitted)| ColumnSpec {
            holding: emitted.held,
            untainted: emitted.untainted,
            sourced: emitted.sourced,
            utc_offset: emitted.utc_offset,
            declared_offsets: emitted.declared.clone(),
            not_null: plain && (emitted.not_null || emitted.literal.is_some()),
            unordered: emitted.unordered,
            suffixed: emitted.suffixed,
            ..ColumnSpec::new(name.clone(), emitted.data_type)
        })
        .collect();
    if named {
        require_distinct_names(&columns)?;
    }
    let grouped_all = match select.get("group_by").and_then(Value::as_object) {
        Some(group) if flag(group, "all") => group_all_positions(select).unwrap_or_default(),
        _ => Vec::new(),
    };
    let suffix_read = (!context.free_projections || flag(select, "distinct"))
        || outputs.iter().zip(1_u64..).any(|((_, emitted), position)| {
            emitted.suffixed
                && (group_positions.contains(&position) || grouped_all.contains(&position))
        });
    if suffix_read && outputs.iter().any(|(_, emitted)| emitted.suffixed) {
        return Err(outside_conditions(
            function_construct(SYSTEM_TYPEOF_NAME),
            TYPE_SUFFIX_UNREMOVED.to_owned(),
        ));
    }
    let distinct = match flag(select, "distinct") {
        true if outputs.iter().any(|(_, emitted)| emitted.is_variant_held()) => {
            return Err(outside_conditions(
                node_construct("distinct"),
                VARIANT_HELD_UNREAD.to_owned(),
            ));
        }
        true if outputs
            .iter()
            .any(|(_, emitted)| emitted.data_type == SnowflakeType::SemiStructured) =>
        {
            let _rule = context.log.rule(DISTINCT);
            let construct = node_construct("distinct");
            items.clear();
            for (name, emitted) in &mut outputs {
                if emitted.data_type == SnowflakeType::SemiStructured {
                    *emitted = canonical_variant(emitted.clone(), &construct, &context)?;
                }
                items.push(projected_sql(
                    Some(name.as_str()).filter(|name| !name.is_empty()),
                    emitted,
                    &context,
                ));
            }
            scope.aliases = outputs.clone();
            "DISTINCT "
        }
        true => {
            let _rule = context.log.rule(DISTINCT);
            "DISTINCT "
        }
        false => "",
    };
    if !distinct.is_empty() && outputs.iter().any(|(_, emitted)| emitted.unordered) {
        items = distinct_checked(&outputs, &context)?;
    }
    let tail = select_tail(select, &scope, &context, (&outputs, &grouped_variants))?;
    Ok((
        format!(
            "{with_sql}SELECT {distinct}{}{from_sql}{tail}",
            items.join(", ")
        ),
        columns,
    ))
}

/// SELECT DISTINCT items where an unordered array of several distinct elements in a repeated row raises.
fn distinct_checked(
    outputs: &[(String, Emitted)],
    context: &Context<'_>,
) -> Result<Vec<String>, Refusal> {
    let windowed = outputs
        .iter()
        .any(|(_, emitted)| emitted.sql.contains(WINDOW_MARKER));
    let partition: Vec<&str> = outputs
        .iter()
        .map(|(_, emitted)| emitted.sql.as_str())
        .collect();
    let repeated = format!("count(*) OVER (PARTITION BY {}) > 1", partition.join(", "));
    let mut items: Vec<String> = Vec::with_capacity(outputs.len());
    for (name, emitted) in outputs {
        let name = Some(name.as_str()).filter(|name| !name.is_empty());
        if !emitted.unordered {
            items.push(projected_sql(name, emitted, context));
            continue;
        }
        let checked = order_checked(emitted.clone(), context)?;
        let value = match windowed {
            true => checked,
            false => Emitted {
                sql: format!(
                    "(CASE WHEN {repeated} THEN {} ELSE {} END)",
                    checked.sql, emitted.sql
                ),
                ..emitted.clone()
            },
        };
        items.push(projected_sql(name, &value, context));
    }
    Ok(items)
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
    (outputs, grouped_variants): (&[(String, Emitted)], &[Regrouped]),
) -> Result<String, Refusal> {
    let regrouped = |mut sql: String| {
        for (converted, grouped) in grouped_variants {
            sql = sql.replace(converted.as_str(), grouped);
        }
        sql
    };
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
        let held: Vec<bool> = outputs
            .iter()
            .map(|(_, emitted)| emitted.is_variant_held())
            .collect();
        sql.push_str(&group_by(select, site, &held)?);
    }
    if let Some(condition) = clause_condition(select, "having") {
        let _rule = context.log.rule(HAVING);
        let site = site.placed(Placement::GROUPED);
        sql.push_str(&format!(
            " HAVING {}",
            regrouped(condition_sql(condition, site)?)
        ));
    }
    if let Some(condition) = clause_condition(select, "qualify") {
        let _rule = context.log.rule(QUALIFY);
        let site = site.placed(Placement::PROJECTION);
        sql.push_str(&format!(
            " QUALIFY {}",
            regrouped(condition_sql(condition, site)?)
        ));
    }
    sql.push_str(&regrouped(order_and_limit(select, site, outputs)?));
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
        require_understood(cte, &["alias", "this", "columns"], &construct)?;
        let name = identifier_name(field(cte, "alias", &construct)?)
            .ok_or_else(|| missing(construct.clone(), "a CTE has no name".to_owned()))?;
        let _muted = fixture(&name).then(|| context.log.muted());
        let (sql, mut columns) = convert_query(
            field(cte, "this", &construct)?,
            outer,
            &Context {
                fixture_body: extended.fixture_body || fixture(&name),
                ..extended.projecting(true)
            },
            true,
        )?;
        let renamed = list(cte, "columns");
        let mut column_list = String::new();
        if !renamed.is_empty() {
            if renamed.len() != columns.len() {
                return Err(unresolved(
                    "a CTE column list does not match its query's columns".to_owned(),
                ));
            }
            let mut quoted: Vec<String> = Vec::with_capacity(renamed.len());
            for (column, alias) in columns.iter_mut().zip(renamed) {
                column.name = identifier_name(alias).ok_or_else(|| {
                    missing(construct.clone(), "a CTE column has no name".to_owned())
                })?;
                quoted.push(quoted_identifier(&column.name, context.log));
            }
            require_distinct_names(&columns)?;
            column_list = format!("({})", quoted.join(", "));
        }
        parts.push(format!(
            "{}{column_list} AS ({sql})",
            quoted_identifier(&name, context.log)
        ));
        let columns = match fixture(&name) {
            true => sourced_columns(columns),
            false => columns,
        };
        extended.ctes.push((name, columns));
    }
    Ok((format!("WITH {} ", parts.join(", ")), extended))
}

/// A case's input fixture stands in for a source table: its VARIANTs are trusted to hold JSON.
fn sourced_columns(columns: Vec<ColumnSpec>) -> Vec<ColumnSpec> {
    columns
        .into_iter()
        .map(|column| ColumnSpec {
            sourced: column.sourced || column.data_type == SnowflakeType::SemiStructured,
            ..column
        })
        .collect()
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
            let emitted = convert_held(field(alias, "this", &construct)?, site)?;
            Ok(vec![(Some(name), emitted)])
        }
        Some(("column", payload)) => {
            let name = payload.get("name").and_then(identifier_name);
            Ok(vec![(name, convert_held(item, site)?)])
        }
        Some((kind, _)) => {
            let emitted = convert_held(item, site)?;
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

/// The columns `*` or `alias.*` names, without those its EXCLUDE list names.
pub(crate) fn star(payload: &Value, site: Site<'_>) -> Result<Vec<Projected>, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(STAR);
    let construct = node_construct("star");
    let star = payload_object(payload, &construct)?;
    require_understood(star, &["table", "except"], &construct)?;
    let qualifier = star.get("table").and_then(identifier_name);
    let excluded: Vec<String> = list(star, "except")
        .iter()
        .map(|name| {
            identifier_name(name).ok_or_else(|| {
                outside_conditions(
                    construct.clone(),
                    "an EXCLUDE entry without a name".to_owned(),
                )
            })
        })
        .collect::<Result<Vec<String>, Refusal>>()?;
    if !excluded.is_empty() && site.context.options.quoted_exclusions {
        return Err(outside_conditions(
            construct,
            "an EXCLUDE list where the source quotes a name, a quoting the parsed tree does not keep"
                .to_owned(),
        ));
    }
    let relations = star_relations(site.scope, qualifier.as_deref(), SNOWFLAKE_TO_DUCKDB)?;
    if relations
        .iter()
        .any(|relation| relation.alias.starts_with(USING_ALIAS_PREFIX))
    {
        return Err(outside_conditions(
            construct.clone(),
            "a star over a FULL JOIN USING, whose merged keys Snowflake lists first".to_owned(),
        ));
    }
    if relations.iter().any(|relation| reads_hidden(relation)) {
        return Err(outside_conditions(
            construct.clone(),
            "a star over a generator or a CONNECT BY walk".to_owned(),
        ));
    }
    let merged = |relation: &ScopeRelation, name: &String| {
        qualifier.is_none() && relation.merged.contains(name)
    };
    if qualifier.is_none() {
        require_leading_keys(&relations, &construct)?;
    }
    let mut read: Vec<&String> = Vec::new();
    for relation in &relations {
        for column in &relation.columns {
            if !merged(relation, &column.name) {
                read.push(&column.name);
            }
        }
    }
    for name in &excluded {
        if !read.contains(&name) {
            return Err(outside_conditions(
                construct.clone(),
                format!("EXCLUDE names `{name}`, which the star does not read"),
            ));
        }
        if read.iter().filter(|column| **column == name).count() > 1 {
            return Err(outside_conditions(
                construct.clone(),
                format!(
                    "EXCLUDE names `{name}`, which several of the star's sources hold: Snowflake \
                     was recorded rejecting it as ambiguous"
                ),
            ));
        }
    }
    let mut expanded: Vec<Projected> = Vec::new();
    for relation in relations {
        for column in relation
            .columns
            .iter()
            .filter(|column| !excluded.contains(&column.name) && !merged(relation, &column.name))
        {
            let sql = read_column(&relation.alias, column, log)?;
            site.context.typed.read((column.untainted, column.sourced));
            let emitted = Emitted {
                held: column.holding,
                untainted: column.untainted,
                sourced: column.sourced,
                utc_offset: column.utc_offset,
                declared: column.declared_offsets.clone(),
                not_null: column.not_null,
                unordered: column.unordered,
                suffixed: column.suffixed,
                ..Emitted::new(sql, column.data_type)
            };
            let emitted =
                match emitted.unordered && (!site.placement.order_free || site.scope.grouped) {
                    true => order_checked(emitted, site.context)?,
                    false => emitted,
                };
            expanded.push((Some(column.name.clone()), emitted));
        }
    }
    Ok(expanded)
}

/// A bare star over USING joins converts only where the keys already lead the first source's columns.
fn require_leading_keys(
    relations: &[&ScopeRelation],
    construct: &Construct,
) -> Result<(), Refusal> {
    let keys: Vec<&String> = relations
        .iter()
        .flat_map(|relation| &relation.merged)
        .collect();
    let leading = relations.first().is_some_and(|first| {
        first.columns.len() >= keys.len()
            && first
                .columns
                .iter()
                .zip(&keys)
                .all(|(column, key)| &&column.name == key)
    });
    match keys.is_empty() || leading {
        true => Ok(()),
        false => Err(outside_conditions(
            construct.clone(),
            "a bare star over a USING join whose keys do not lead the first source".to_owned(),
        )),
    }
}

/// Whether a relation carries a generator's or a CONNECT BY walk's hidden columns.
fn reads_hidden(relation: &ScopeRelation) -> bool {
    relation.columns.iter().any(hidden_column)
}

fn hidden_column(column: &ColumnSpec) -> bool {
    column.name == GENERATOR_SEQUENCE_COLUMN || column.name.starts_with(HIERARCHY_COLUMN_PREFIX)
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
