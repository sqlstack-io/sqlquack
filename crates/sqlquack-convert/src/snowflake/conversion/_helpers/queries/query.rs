//! Queries: SELECT with its projection and clauses, and WITH.

use serde_json::{Map, Value, json};
use sqlquack_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_diagnostics::classification::models::{Construct, ConstructFamily};
use sqlquack_diagnostics::diagnostics::constants::{
    REPEATED_RESULT_NAME, UNALIASED_OUTPUT_NAME_CONSTRUCT, UNALIASED_QUOTED_NAME,
    UNALIASED_TEXT_NAME,
};
use sqlquack_diagnostics::provenance::constants::QUERY_CONSTRUCT;
use sqlquack_diagnostics::provenance::models::ShapeArgument;
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::main::unresolved::unresolved;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::snowflake::conversion::_helpers::expressions::ordered_aggregates::order_checked;
use crate::snowflake::conversion::_helpers::expressions::scalar::convert_held;
use crate::snowflake::conversion::_helpers::expressions::windows::canonical_variant;
use crate::snowflake::conversion::_helpers::queries::clauses::{
    clause_condition, condition_sql, group_by, group_positions, order_and_limit, variant_group_keys,
};
use crate::snowflake::conversion::_helpers::queries::group_all::group_all_positions;
use crate::snowflake::conversion::_helpers::queries::hierarchy::hierarchy;
use crate::snowflake::conversion::_helpers::queries::output_names::recorded_name_shape;
use crate::snowflake::conversion::_helpers::queries::set_operations::convert_set_operation;
use crate::snowflake::conversion::_helpers::queries::sources::convert_from;
use crate::snowflake::conversion::_helpers::support::locations::Locator;
use crate::snowflake::conversion::_helpers::support::scope::read_column;
use crate::snowflake::conversion::_helpers::support::scope::{Context, Emitted, Scope};
use crate::snowflake::conversion::_helpers::support::site::{Placement, Site};
use crate::snowflake::conversion::_helpers::support::tree::{
    LITERAL_DATE, LITERAL_NUMBER, LITERAL_STRING, LITERAL_TIMESTAMP, Payload, field, flag,
    function_construct, identifier_name, list, node_construct, node_kind, payload_object,
    require_understood,
};
use crate::snowflake::conversion::constants::UNALIASED_QUOTED_KINDS;
use crate::snowflake::conversion::constants::WINDOW_MARKER;
use crate::snowflake::conversion::constants::{
    ALIAS, COMMENT, COMMON_TABLE, DISTINCT, GENERATOR_SEQUENCE_COLUMN, GROUP_BY, HAVING,
    HIERARCHY_COLUMN_PREFIX, QUALIFY, SELECT, STAR, SYSTEM_TYPEOF_NAME, TYPE_SUFFIX_UNREMOVED,
    USING_ALIAS_PREFIX, VARIANT_HELD_UNREAD, WHERE,
};
use crate::snowflake::conversion::constants::{
    VALUES_COLUMN_PREFIX, VALUES_ROWS, VALUES_ROWS_LIMIT,
};
use sqlquack_semantics::dialects::constants::SNOWFLAKE_TO_DUCKDB;
use sqlquack_semantics::scopes::main::star_relations::star_relations;
use sqlquack_semantics::snowflake::typing::types::ColumnSpec;
use sqlquack_semantics::snowflake::typing::types::ScopeRelation;

/// The most characters Snowflake keeps of an unaliased item's text as its name, as recorded.
const OUTPUT_NAME_MAX_CHARS: usize = 255;
/// Why a name written straight after a number, which Snowflake rejects, is refused.
const GLUED_ALIAS: &str = "a name written straight after a number, such as 0x1F, which Snowflake rejects as a syntax error";
/// Why a repeated name is refused inside a subquery or CTE.
pub(crate) const NESTED_REPEAT: &str =
    "inside a subquery or CTE, where DuckDB renames the second column";
/// Why a repeated name is refused where a clause could read it.
const CLAUSE_REPEAT: &str = "where ORDER BY, GROUP BY, HAVING or QUALIFY could read either column, which no recording shows";
/// Why a repeated name is refused where the converted SQL wraps the result.
const WRAPPED_REPEAT: &str = "where the converted SQL wraps the result to write VARIANT or NULL columns, and DuckDB renames the second column";
/// The SELECT clauses that could read a repeated result name.
const NAME_READING_CLAUSES: [&str; 4] = ["order_by", "group_by", "having", "qualify"];
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
    "top",
    "fetch",
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
    let group_positions = group_positions(select, &scope);
    let mut names: Vec<Option<String>> = Vec::new();
    let mut outputs: Vec<(String, Emitted)> = Vec::new();
    let mut suffixed_item: Option<&Value> = None;
    let mut origins: Vec<&Value> = Vec::new();
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
            if emitted.suffixed {
                suffixed_item.get_or_insert(item);
            }
            outputs.push((name.clone().unwrap_or_default(), emitted));
            names.push(name);
            origins.push(item);
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
        let refused_because = match !context.free_projections && outer.is_none() {
            false => Some(NESTED_REPEAT),
            true if NAME_READING_CLAUSES
                .iter()
                .any(|clause| select.get(*clause).is_some_and(|value| !value.is_null())) =>
            {
                Some(CLAUSE_REPEAT)
            }
            true if columns.iter().any(|column| {
                matches!(
                    column.data_type,
                    SnowflakeType::SemiStructured | SnowflakeType::Null
                )
            }) =>
            {
                Some(WRAPPED_REPEAT)
            }
            true => None,
        };
        require_distinct_items(&columns, &origins, refused_because, &context)?;
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
    if let Some(item) = suffixed_item.filter(|_| suffix_read) {
        return Err(context.locator.placed(
            outside_conditions(
                function_construct(SYSTEM_TYPEOF_NAME),
                TYPE_SUFFIX_UNREMOVED.to_owned(),
            ),
            item,
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
            if let Some((index, refusal)) = repeated_name(&columns, Some(NESTED_REPEAT)) {
                return Err(context.locator.placed(refusal, &renamed[index]));
            }
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
                .map_err(|refusal| site.context.locator.placed(refusal, item))
        }
        Some(("alias", payload)) => {
            let _rule = site.context.log.rule(ALIAS);
            let construct = node_construct("alias");
            let alias = payload_object(payload, &construct)?;
            require_understood(alias, &["this", "alias"], &construct)?;
            let name = identifier_name(field(alias, "alias", &construct)?)
                .ok_or_else(|| missing(construct.clone(), "an alias has no name".to_owned()))?;
            let this = field(alias, "this", &construct)?;
            if glued_to_number(site.context.locator.source_text(item).as_deref(), this) {
                return Err(site
                    .context
                    .locator
                    .placed(outside_conditions(construct, GLUED_ALIAS.to_owned()), item));
            }
            let emitted = convert_held(this, site)?;
            Ok(vec![(Some(name), emitted)])
        }
        Some(("column", payload)) => {
            let name = payload.get("name").and_then(identifier_name);
            Ok(vec![(name, convert_held(item, site)?)])
        }
        Some((kind, _)) => {
            let emitted = convert_held(item, site)?;
            if !named {
                return Ok(vec![(None, emitted)]);
            }
            let written = site
                .context
                .locator
                .source_text(item)
                .or_else(|| literal_text(item, site.context.locator));
            let text_of = |node: &Value| site.context.locator.source_text(node);
            match written
                .filter(|_| recorded_name_shape(item, &text_of))
                .and_then(output_name)
            {
                Some(name) => {
                    let _rule = site.context.log.rule(ALIAS);
                    Ok(vec![(Some(name), emitted)])
                }
                None => Err(Refusal {
                    location: site.context.locator.locate_written(item),
                    ..missing(
                        Construct::new(ConstructFamily::Syntax, UNALIASED_OUTPUT_NAME_CONSTRUCT),
                        match UNALIASED_QUOTED_KINDS.contains(&kind) {
                            true => UNALIASED_QUOTED_NAME.to_owned(),
                            false => UNALIASED_TEXT_NAME.to_owned(),
                        },
                    )
                }),
            }
        }
        None => Err(missing(
            node_construct("unknown"),
            "a select item has an unexpected shape".to_owned(),
        )),
    }
}

/// Whether the alias item written as `written` follows its number `this` with no space, as `0x1F`.
fn glued_to_number(written: Option<&str>, this: &Value) -> bool {
    let Some(("literal", payload)) = node_kind(this) else {
        return false;
    };
    let value = payload
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    payload.get("literal_type").and_then(Value::as_str) == Some(LITERAL_NUMBER)
        && !value.is_empty()
        && written
            .and_then(|written| written.strip_prefix(value))
            .and_then(|rest| rest.chars().next())
            .is_some_and(|next| next.is_ascii_alphabetic() || next == '_')
}

/// Snowflake's name for an unaliased item written as `text`: it upper-cased, cut at 255 characters.
fn output_name(text: String) -> Option<String> {
    let recorded = !text.is_empty()
        && text.is_ascii()
        && text.trim() == text
        && !text.contains("--")
        && !text.contains('\r')
        && !text.contains(" (")
        && text.matches('\n').count() <= 1;
    recorded.then(|| {
        text.to_ascii_uppercase()
            .chars()
            .take(OUTPUT_NAME_MAX_CHARS)
            .collect()
    })
}

/// A literal's written text: a number as parsed, a plain string quoted, NULL, TRUE or FALSE.
fn literal_text(item: &Value, locator: &Locator<'_>) -> Option<String> {
    match node_kind(item)? {
        ("literal", payload) => {
            let value = payload.get("value")?.as_str()?;
            let keyword = match payload.get("literal_type")?.as_str()? {
                LITERAL_NUMBER => return Some(value.to_owned()),
                LITERAL_STRING if !value.contains(['\'', '\\']) => {
                    return Some(format!("'{value}'"));
                }
                LITERAL_DATE => "DATE",
                LITERAL_TIMESTAMP => "TIMESTAMP",
                _ => return None,
            };
            spelled_once(item, locator, value).then(|| format!("{keyword} '{value}'"))
        }
        ("boolean", payload) => Some(
            match payload.get("value")?.as_bool()? {
                true => "TRUE",
                false => "FALSE",
            }
            .to_owned(),
        ),
        ("null", _) => Some("NULL".to_owned()),
        _ => None,
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

/// Refuses repeated result names, exact repeats only for `refused_because`, which says why.
pub(crate) fn require_distinct_names(
    columns: &[ColumnSpec],
    refused_because: Option<&str>,
) -> Result<(), Refusal> {
    repeated_name(columns, refused_because).map_or(Ok(()), |(_, refusal)| Err(refusal))
}

/// Refuses repeated result names as `require_distinct_names` does, placed at the repeating item.
fn require_distinct_items(
    columns: &[ColumnSpec],
    items: &[&Value],
    refused_because: Option<&str>,
    context: &Context<'_>,
) -> Result<(), Refusal> {
    match repeated_name(columns, refused_because) {
        Some((index, refusal)) => Err(Refusal {
            location: context.locator.locate_written(items[index]),
            ..refusal
        }),
        None => Ok(()),
    }
}

/// The first column whose name `require_distinct_names` refuses, by index, with the refusal.
pub(crate) fn repeated_name(
    columns: &[ColumnSpec],
    refused_because: Option<&str>,
) -> Option<(usize, Refusal)> {
    for (index, column) in columns.iter().enumerate() {
        let Some(earlier) = columns[..index]
            .iter()
            .find(|earlier| earlier.name.eq_ignore_ascii_case(&column.name))
        else {
            continue;
        };
        let reason = match (earlier.name == column.name, refused_because) {
            (true, None) => continue,
            (true, Some(reason)) => {
                format!(
                    "{REPEATED_RESULT_NAME} {reason}; Snowflake keeps both names, so alias one of them"
                )
            }
            (false, _) => format!(
                "{REPEATED_RESULT_NAME} in another case, which DuckDB treats as the same name"
            ),
        };
        let refusal = unresolved(format!("the result column name `{}` {reason}", column.name));
        return Some((index, refusal));
    }
    None
}

/// Whether the source writes the typed literal `item` as `keyword 'value'` with one space, everywhere.
fn spelled_once(item: &Value, locator: &Locator<'_>, value: &str) -> bool {
    !value.contains(['\'', '\\'])
        && locator
            .written_literal(item)
            .is_some_and(|found| found.iter().all(|(_, gap)| *gap == Some(" ")))
}
