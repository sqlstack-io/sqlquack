//! FROM: tables, CTEs, subqueries and joins.

use serde_json::{Value, json};
use sqlquack_core::classification::models::{Construct, ConstructFamily};
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::main::unresolved::unresolved;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::project_functions::{exact_arguments, held_type};
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::semi_structured::guarded_column;
use crate::conversion::_helpers::queries::clauses::condition_sql;
use crate::conversion::_helpers::queries::flatten::lateral_flatten;
use crate::conversion::_helpers::queries::query::{convert_query, require_distinct_names};
use crate::conversion::_helpers::support::collecting::{collecting, events, opaque, record};
use crate::conversion::_helpers::support::scope::{Context, Scope};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, field, function_construct, identifier_name, list, node_construct, node_kind,
    payload_object, require_understood,
};
use crate::conversion::constants::{
    ASOF_JOIN, ASOF_KIND, FROM_RELATION, FROM_SUBQUERY, JOIN_CROSS, JOIN_ON, JOIN_USING,
    LATERAL_KEY, LATERAL_SUBQUERY, OPAQUE_ALIAS_PREFIX, PROJECT_TABLE_FUNCTION, TABLE_WRAPPER,
    UNALIASED_SUBQUERY_PREFIX,
};
use sqlquack_semantics::lineage::main::stored_columns::stored_columns;
use sqlquack_semantics::scopes::models::ScopeRelation;
use sqlquack_semantics::types::models::FunctionReturns;

/// The FROM clause with its joins, and the relations it brings into scope.
pub(crate) fn convert_from(
    select: &Payload,
    outer: Option<&Scope<'_>>,
    context: &Context<'_>,
) -> Result<(String, Vec<ScopeRelation>), Refusal> {
    let construct = node_construct("from");
    let Some(from) = select.get("from").filter(|from| !from.is_null()) else {
        if !list(select, "joins").is_empty() {
            return Err(missing(construct, "joins without FROM".to_owned()));
        }
        return Ok((String::new(), Vec::new()));
    };
    let from = payload_object(from, &construct)?;
    require_understood(from, &["expressions"], &construct)?;
    let mut relations: Vec<ScopeRelation> = Vec::new();
    let sources = list(from, "expressions");
    let separator = match sources
        .iter()
        .any(|source| source.get(LATERAL_KEY).is_some())
    {
        true => " CROSS JOIN ",
        false => ", ",
    };
    let mut sql = String::from(" FROM ");
    for (position, source) in sources.iter().enumerate() {
        let before = events();
        if let Some(lateral) = source.get(LATERAL_KEY) {
            let flattened = match position {
                0 => Err(missing(
                    node_construct(LATERAL_KEY),
                    "LATERAL as the first FROM source".to_owned(),
                )),
                _ => lateral_flatten(lateral, &relations, outer, context)
                    .map(|flattened| (flattened.sql, flattened.relation)),
            }
            .map_err(|refusal| context.locator.placed(refusal, source));
            let (flattened_sql, relation) =
                recovered(flattened, source, (before, relations.len()))?;
            sql.push_str(&flattened_sql);
            relations.push(relation);
            continue;
        }
        let converted = match position == 0 && lateral_subquery(source) {
            true => Err(missing(
                node_construct(LATERAL_KEY),
                "LATERAL as the first FROM source".to_owned(),
            )),
            false => convert_source(source, (&relations, outer), context),
        };
        let (source_sql, relation) = recovered(converted, source, (before, relations.len()))?;
        if position > 0 {
            sql.push_str(separator);
        }
        sql.push_str(&source_sql);
        relations.push(relation);
    }
    for join in list(select, "joins") {
        let before = events();
        let joined = convert_join(join, &relations, outer, context)
            .map_err(|refusal| context.locator.placed(refusal, join));
        let source = join.get("this").unwrap_or(&Value::Null);
        let (join_sql, relation) = recovered(joined, source, (before, relations.len()))?;
        sql.push_str(&join_sql);
        relations.push(relation);
    }
    require_distinct_aliases(&relations)?;
    Ok((sql, relations))
}

fn require_distinct_aliases(relations: &[ScopeRelation]) -> Result<(), Refusal> {
    for (index, relation) in relations.iter().enumerate() {
        if relations[..index]
            .iter()
            .any(|earlier| earlier.alias.eq_ignore_ascii_case(&relation.alias))
        {
            return Err(unresolved(format!(
                "the relation name `{}` is used twice in one FROM clause",
                relation.alias
            )));
        }
    }
    Ok(())
}

fn convert_join(
    join: &Value,
    earlier: &[ScopeRelation],
    outer: Option<&Scope<'_>>,
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let join_construct = node_construct("join");
    let join = payload_object(join, &join_construct)?;
    require_understood(
        join,
        &["this", "on", "kind", "using", "match_condition"],
        &join_construct,
    )?;
    let kind = join.get("kind").and_then(Value::as_str).unwrap_or_default();
    let source = field(join, "this", &join_construct)?;
    let on = join.get("on").filter(|on| !on.is_null());
    let using = list(join, "using");
    let matching = join
        .get("match_condition")
        .filter(|matching| !matching.is_null());
    let before = events();
    if let (Some(lateral), "Implicit" | "Cross", None) = (source.get(LATERAL_KEY), kind, on) {
        let flattened = lateral_flatten(lateral, earlier, outer, context)
            .map(|flattened| (flattened.sql, flattened.relation));
        return recovered(flattened, source, (before, earlier.len()));
    }
    let converted = match lateral_subquery(source) {
        true => Err(missing(
            node_construct("subquery"),
            "a LATERAL subquery after JOIN ... ON clauses".to_owned(),
        )),
        false => convert_source(source, (earlier, outer), context),
    };
    let (source_sql, relation) = recovered(converted, source, (before, earlier.len()))?;
    if !using.is_empty() {
        return using_join(
            (kind, on.is_some(), using),
            (source_sql, relation),
            earlier,
            context,
        );
    }
    if let Some(matching) = matching {
        return asof_join(
            (kind, on, matching),
            (source_sql, relation),
            (earlier, outer),
            context,
        );
    }
    let keyword = match (kind, on) {
        ("Inner", Some(_)) => "JOIN",
        ("Left", Some(_)) => "LEFT JOIN",
        ("Right", Some(_)) => "RIGHT JOIN",
        ("Full", Some(_)) => "FULL JOIN",
        ("Cross", None) => "CROSS JOIN",
        ("Implicit", None) => ",",
        _ => {
            return Err(missing(
                Construct::new(ConstructFamily::Join, kind.to_ascii_lowercase()),
                format!("no rule converts a {kind} join of this shape"),
            ));
        }
    };
    let Some(on) = on else {
        let _rule = context.log.rule(JOIN_CROSS);
        let separator = match keyword {
            "," => ",",
            _ => " CROSS JOIN",
        };
        return Ok((format!("{separator} {source_sql}"), relation));
    };
    let _rule = context.log.rule(JOIN_ON);
    let mut visible = earlier.to_vec();
    visible.push(relation.clone());
    let scope = Scope::new(visible, outer);
    let site = Site {
        scope: &scope,
        context,
        placement: Placement::ROW,
    };
    let condition = condition_sql(on, site)?;
    Ok((format!(" {keyword} {source_sql} ON {condition}"), relation))
}

/// `JOIN ... USING (keys)`: each key equal on both sides, read bare as the earlier side's column.
fn using_join(
    (kind, has_on, keys): (&str, bool, &[Value]),
    (source_sql, mut relation): (String, ScopeRelation),
    earlier: &[ScopeRelation],
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let construct = node_construct("join");
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let keyword = match (kind, has_on) {
        ("Inner", false) => "JOIN",
        ("Left", false) => "LEFT JOIN",
        _ => return Err(refuse(format!("a {kind} join with USING"))),
    };
    let _rule = context.log.rule(JOIN_USING);
    let mut equalities: Vec<Value> = Vec::with_capacity(keys.len());
    for key in keys {
        let name = identifier_name(key)
            .ok_or_else(|| missing(construct.clone(), "a USING key has no name".to_owned()))?;
        let owners: Vec<&ScopeRelation> = earlier
            .iter()
            .filter(|owner| readable(owner, &name))
            .collect();
        let [owner] = owners.as_slice() else {
            return Err(unresolved(format!(
                "the USING key `{name}` is not one column of the earlier sources"
            )));
        };
        let Some(joined) = relation.columns.iter().find(|column| column.name == name) else {
            return Err(unresolved(format!(
                "the USING key `{name}` is not a column of the joined source"
            )));
        };
        let earlier_type = owner
            .columns
            .iter()
            .find(|column| column.name == name)
            .map(|column| column.data_type);
        if earlier_type != Some(joined.data_type) {
            return Err(refuse(format!(
                "the USING key `{name}` of different types on its two sides, whose merged \
                 value no recording shows"
            )));
        }
        let side =
            |alias: &str| json!({"column": {"name": {"name": name}, "table": {"name": alias}}});
        equalities
            .push(json!({"eq": {"left": side(&owner.alias), "right": side(&relation.alias)}}));
        relation.merged.push(name);
    }
    let condition = equalities
        .into_iter()
        .reduce(|left, right| json!({"and": {"left": left, "right": right}}))
        .ok_or_else(|| missing(construct.clone(), "USING without keys".to_owned()))?;
    let mut visible = earlier.to_vec();
    visible.push(relation.clone());
    let scope = Scope::new(visible, None);
    let site = Site {
        scope: &scope,
        context,
        placement: Placement::ROW,
    };
    let condition = condition_sql(&condition, site)?;
    Ok((format!(" {keyword} {source_sql} ON {condition}"), relation))
}

/// Whether a relation has a column `name` a bare name reads, not one a USING merged away.
fn readable(relation: &ScopeRelation, name: &str) -> bool {
    !relation.merged.iter().any(|merged| merged == name)
        && relation.columns.iter().any(|column| column.name == name)
}

/// `ASOF JOIN ... MATCH_CONDITION (inequality) ON equalities`: each left row with its nearest match.
fn asof_join(
    (kind, on, matching): (&str, Option<&Value>, &Value),
    (source_sql, relation): (String, ScopeRelation),
    (earlier, outer): Earlier<'_, '_>,
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let construct = Construct::new(ConstructFamily::Join, "asof".to_owned());
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    if kind != ASOF_KIND {
        return Err(refuse("a MATCH_CONDITION on other than an ASOF join"));
    }
    if !matches!(node_kind(matching), Some(("gt" | "gte" | "lt" | "lte", _))) {
        return Err(refuse("a MATCH_CONDITION other than one inequality"));
    }
    if on.is_some_and(|on| !equalities_only(on)) {
        return Err(refuse(
            "an ASOF ON condition other than equalities joined by AND",
        ));
    }
    let _rule = context.log.rule(ASOF_JOIN);
    let mut visible = earlier.to_vec();
    visible.push(relation.clone());
    let scope = Scope::new(visible, outer);
    let site = Site {
        scope: &scope,
        context,
        placement: Placement::ROW,
    };
    if let Some((_, operands)) = node_kind(matching) {
        for side in ["left", "right"] {
            let operand = field(payload_object(operands, &construct)?, side, &construct)?;
            if !matches!(node_kind(operand), Some(("column", _))) {
                return Err(refuse("a MATCH_CONDITION over other than two columns"));
            }
            let converted = convert_expression(operand, site)?;
            if !matches!(
                converted.data_type,
                SnowflakeType::Number { .. }
                    | SnowflakeType::Float
                    | SnowflakeType::Date
                    | SnowflakeType::TimestampNtz { .. }
            ) {
                return Err(refuse(&format!(
                    "a MATCH_CONDITION over {}, which no recording shows",
                    converted.data_type
                )));
            }
        }
    }
    let mut conditions = vec![condition_sql(matching, site)?];
    if let Some(on) = on {
        conditions.insert(0, condition_sql(on, site)?);
    }
    Ok((
        format!(
            " ASOF LEFT JOIN {source_sql} ON {}",
            conditions.join(" AND ")
        ),
        relation,
    ))
}

/// Whether a condition is equalities joined by AND, as an ASOF join's ON must be.
fn equalities_only(condition: &Value) -> bool {
    match node_kind(condition) {
        Some(("and", payload)) => ["left", "right"]
            .iter()
            .all(|side| payload.get(*side).is_some_and(equalities_only)),
        Some(("paren", payload)) => payload.get("this").is_some_and(equalities_only),
        Some(("eq", _)) => true,
        _ => false,
    }
}

/// The FROM sources before a source and the enclosing scope, which a LATERAL subquery reads.
type Earlier<'s, 'a> = (&'s [ScopeRelation], Option<&'s Scope<'a>>);

/// When collecting, a refused source becomes a relation of unknown columns, so later clauses convert.
fn recovered(
    converted: Result<(String, ScopeRelation), Refusal>,
    source: &Value,
    (before, position): (usize, usize),
) -> Result<(String, ScopeRelation), Refusal> {
    match converted {
        Err(refusal) if collecting() => {
            record(refusal, before);
            let alias =
                source_alias(source).unwrap_or_else(|| format!("{OPAQUE_ALIAS_PREFIX}{position}"));
            opaque(&alias);
            Ok((
                String::new(),
                ScopeRelation {
                    alias,
                    columns: Vec::new(),
                    merged: Vec::new(),
                },
            ))
        }
        converted => converted,
    }
}

/// The name a FROM source is read by, where its tree says.
fn source_alias(source: &Value) -> Option<String> {
    if let Some(lateral) = source.get(LATERAL_KEY) {
        return lateral
            .get("alias")
            .and_then(Value::as_str)
            .map(str::to_ascii_uppercase);
    }
    let (kind, payload) = node_kind(source)?;
    let alias = payload.get("alias").and_then(identifier_name);
    match kind {
        "table" => alias.or_else(|| payload.get("name").and_then(identifier_name)),
        _ => alias,
    }
}

fn lateral_subquery(source: &Value) -> bool {
    matches!(node_kind(source), Some(("subquery", payload))
        if payload.get(LATERAL_KEY).and_then(Value::as_bool) == Some(true))
}

fn convert_source(
    source: &Value,
    earlier: Earlier<'_, '_>,
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let (sql, relation) = convert_source_kind(source, earlier, context)
        .map_err(|refusal| context.locator.placed(refusal, source))?;
    Ok((context.locator.identified(&sql, source), relation))
}

fn convert_source_kind(
    source: &Value,
    earlier: Earlier<'_, '_>,
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    match node_kind(source) {
        Some(("table", payload)) => table_source(payload, context),
        Some(("subquery", payload)) => subquery_source(payload, earlier, context),
        Some(("values", _)) => {
            let wrapped = json!({ "this": source });
            subquery_source(&wrapped, earlier, context)
        }
        Some(("function", _)) => table_function_source(source, None, context),
        Some(("alias", payload)) => aliased_source(payload, context),
        Some((other, _)) => Err(missing(
            node_construct(other),
            format!("no rule converts node:{other} in FROM"),
        )),
        None => Err(missing(
            node_construct("unknown"),
            "a FROM source has an unexpected shape".to_owned(),
        )),
    }
}

fn table_source(
    payload: &Value,
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let construct = node_construct("table");
    let log = context.log;
    let _rule = log.rule(FROM_RELATION);
    let table = payload_object(payload, &construct)?;
    require_understood(table, &["name", "alias"], &construct)?;
    let name = identifier_name(field(table, "name", &construct)?)
        .ok_or_else(|| missing(construct.clone(), "a table has no name".to_owned()))?;
    let columns = context.relation(&name)?;
    let alias = table
        .get("alias")
        .and_then(identifier_name)
        .unwrap_or_else(|| name.clone());
    let is_cte = context.ctes.iter().any(|(cte, _)| cte == &name);
    let mut guarded: Vec<String> = Vec::new();
    for column in columns
        .iter()
        .filter(|column| !is_cte && column.data_type == SnowflakeType::SemiStructured)
    {
        let quoted = quoted_identifier(&column.name, log);
        guarded.push(format!("{} AS {quoted}", guarded_column(&quoted, context)?));
    }
    let relation = match guarded.is_empty() {
        true => quoted_identifier(&name, log),
        false => format!(
            "(SELECT * REPLACE ({}) FROM {})",
            guarded.join(", "),
            quoted_identifier(&name, log)
        ),
    };
    let columns = match is_cte {
        true => columns,
        false => stored_columns(columns),
    };
    Ok((
        format!("{relation} AS {}", quoted_identifier(&alias, log)),
        ScopeRelation {
            alias,
            columns,
            merged: Vec::new(),
        },
    ))
}

/// `TABLE(f(...))` of a project table function, arguments of exactly the declared types.
fn table_function_source(
    table: &Value,
    alias: Option<String>,
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let construct = node_construct("table_function");
    let log = context.log;
    let call = table_call(table)
        .ok_or_else(|| missing(construct.clone(), "TABLE() of other than a call".to_owned()))?;
    let name = call
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_uppercase();
    let Some((resolved, spec)) = context.schema.functions.get_key_value(&name) else {
        return Err(missing(
            function_construct(&name),
            format!("no rule converts TABLE({name}(...))"),
        ));
    };
    let _rule = log.rule(PROJECT_TABLE_FUNCTION);
    let FunctionReturns::Table(columns) = &spec.returns else {
        return Err(outside_conditions(
            construct,
            "TABLE() of a scalar function".to_owned(),
        ));
    };
    let scope = Scope::new(Vec::new(), None);
    let site = Site {
        scope: &scope,
        context,
        placement: Placement::ROW,
    };
    let arguments = exact_arguments(spec, list(call, "args"), &construct, site)?;
    let mut projected: Vec<String> = Vec::with_capacity(columns.len());
    for column in columns {
        let held = held_type(column.data_type, log)
            .map_err(|reason| outside_conditions(construct.clone(), reason))?;
        let quoted = quoted_identifier(&column.name, log);
        projected.push(format!("CAST({quoted} AS {held}) AS {quoted}"));
    }
    let alias = alias.unwrap_or_else(|| resolved.clone());
    Ok((
        format!(
            "(SELECT {} FROM {}({})) AS {}",
            projected.join(", "),
            quoted_identifier(resolved, log),
            arguments.join(", "),
            quoted_identifier(&alias, log)
        ),
        ScopeRelation {
            alias,
            columns: columns.clone(),
            merged: Vec::new(),
        },
    ))
}

/// The call inside `TABLE(call)`, a function node.
fn table_call(table: &Value) -> Option<&Payload> {
    let wrapper = table.get("function")?.as_object()?;
    let is_table = wrapper
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| name.eq_ignore_ascii_case(TABLE_WRAPPER));
    match (is_table, list(wrapper, "args")) {
        (true, [inner]) => inner.get("function")?.as_object(),
        _ => None,
    }
}

/// `TABLE(f(...)) AS alias`, the alias without column renames.
fn aliased_source(
    payload: &Value,
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let construct = node_construct("alias");
    let aliased = payload_object(payload, &construct)?;
    require_understood(
        aliased,
        &["this", "alias", "alias_explicit_as", "alias_keyword"],
        &construct,
    )?;
    let alias = aliased.get("alias").and_then(identifier_name);
    table_function_source(field(aliased, "this", &construct)?, alias, context)
}

fn subquery_source(
    payload: &Value,
    (relations, outer): Earlier<'_, '_>,
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let construct = node_construct("subquery");
    let log = context.log;
    let _rule = log.rule(FROM_SUBQUERY);
    let subquery = payload_object(payload, &construct)?;
    require_understood(
        subquery,
        &["this", "alias", "column_aliases", LATERAL_KEY],
        &construct,
    )?;
    let lateral = subquery.get(LATERAL_KEY).and_then(Value::as_bool) == Some(true);
    let _lateral_rule = lateral.then(|| log.rule(LATERAL_SUBQUERY));
    let scope = Scope::new(relations.to_vec(), outer);
    let (sql, mut columns) = convert_query(
        field(subquery, "this", &construct)?,
        lateral.then_some(&scope),
        context,
        true,
    )?;
    let renamed = list(subquery, "column_aliases");
    if !renamed.is_empty() {
        if renamed.len() != columns.len() {
            return Err(unresolved(
                "a column alias list does not match the subquery's columns".to_owned(),
            ));
        }
        for (column, alias) in columns.iter_mut().zip(renamed) {
            column.name = identifier_name(alias).ok_or_else(|| {
                missing(construct.clone(), "a column alias has no name".to_owned())
            })?;
        }
        require_distinct_names(&columns)?;
    }
    let alias = subquery
        .get("alias")
        .and_then(identifier_name)
        .unwrap_or_else(|| format!("{UNALIASED_SUBQUERY_PREFIX}{}", relations.len()));
    let keyword = match lateral {
        true => "LATERAL ",
        false => "",
    };
    let column_list: Vec<String> = columns
        .iter()
        .map(|column| quoted_identifier(&column.name, log))
        .collect();
    Ok((
        format!(
            "{keyword}({sql}) AS {}({})",
            quoted_identifier(&alias, log),
            column_list.join(", ")
        ),
        ScopeRelation {
            alias,
            columns,
            merged: Vec::new(),
        },
    ))
}
