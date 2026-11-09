//! FROM: tables, CTEs, subqueries and joins.

use std::cell::RefCell;

use serde_json::{Value, json};
use sqlquack_core::classification::models::{Construct, ConstructFamily};
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::main::unresolved::unresolved;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_duckdb_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_duckdb_blocks::checking::main::quoted_relation::quoted_relation;
use sqlquack_duckdb_blocks::semi_structured::models::PathStep;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::project_functions::{exact_arguments, held_type};
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::semi_structured::{guarded_column, path_steps};
use crate::conversion::_helpers::queries::clauses::condition_sql;
use crate::conversion::_helpers::queries::flatten::{FLATTEN_NAME, lateral_flatten};
use crate::conversion::_helpers::queries::pivot::pivot_subquery;
use crate::conversion::_helpers::queries::query::{NESTED_REPEAT, convert_query, repeated_name};
use crate::conversion::_helpers::support::collecting::{
    collecting, events, opaque, record, recorded, replace_since, suspended, unresolved_since,
};
use crate::conversion::_helpers::support::scope::{Context, Scope};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, field, function_construct, identifier_name, integer_literal, is_inert, list,
    node_construct, node_kind, payload_object, require_understood,
};
use crate::conversion::constants::{
    ASOF_JOIN, ASOF_KIND, CORRELATED_DERIVED, FROM_RELATION, FROM_SUBQUERY, GENERATOR_MAX_ROWS,
    GENERATOR_NAME, GENERATOR_RULE, GENERATOR_SEQUENCE_COLUMN, JOIN_CROSS, JOIN_ON, JOIN_USING,
    LATERAL_KEY, LATERAL_SUBQUERY, OPAQUE_ALIAS_PREFIX, PIVOT_RULE, PROJECT_TABLE_FUNCTION,
    TABLE_WRAPPER, UNALIASED_SUBQUERY_PREFIX, USING_ALIAS_PREFIX,
};
use crate::conversion::constants::{FULL_JOIN_KEYWORD, INNER_JOIN_KEYWORD, NEGATE_KIND, NULL_KIND};

/// How the parsed tree spells a NATURAL join's kind, lower-cased.
const NATURAL_JOIN_PREFIX: &str = "natural";
use sqlquack_semantics::lineage::main::stored_columns::stored_columns;
use sqlquack_semantics::names::constants::RELATION_KEY_SEPARATOR;
use sqlquack_semantics::names::main::relation_key::relation_key;
use sqlquack_semantics::scopes::models::ScopeRelation;
use sqlquack_semantics::types::models::{ColumnSpec, FunctionReturns};

/// GENERATOR's named arguments.
const ROWCOUNT_ARGUMENT: &str = "ROWCOUNT";
const TIMELIMIT_ARGUMENT: &str = "TIMELIMIT";
/// The node kinds a constant ROWCOUNT is written with.
const PAREN_KIND: &str = "paren";
const ADD_KIND: &str = "add";
const SUB_KIND: &str = "sub";
const MUL_KIND: &str = "mul";

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
        .any(|source| lateral_payload(source).is_some())
    {
        true => " CROSS JOIN ",
        false => ", ",
    };
    let mut sql = String::from(" FROM ");
    for (position, source) in sources.iter().enumerate() {
        let before = events();
        if let Some(lateral) = lateral_payload(source) {
            let flattened = match position {
                0 => Err(missing(
                    node_construct(LATERAL_KEY),
                    "LATERAL as the first FROM source".to_owned(),
                )),
                _ => lateral_flatten(&lateral, &relations, outer, context)
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
        let merging = RefCell::new(None);
        let joined = convert_join(join, &relations, (outer, &merging), context)
            .map_err(|refusal| context.locator.placed(refusal, join));
        let source = join.get("this").unwrap_or(&Value::Null);
        let (join_sql, relation) = recovered(joined, source, (before, relations.len()))?;
        sql.push_str(&join_sql);
        relations.push(relation);
        if let Some(Merging { keys, relation }) = merging.into_inner() {
            for (owner, name) in keys {
                if let Some(owner) = relations.iter_mut().find(|earlier| earlier.alias == owner) {
                    owner.merged.push(name);
                }
            }
            relations.extend(relation);
        }
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

/// What a FULL JOIN USING adds: its merged keys, hidden on both sides, read through `relation`.
struct Merging {
    /// The earlier relation's alias and the key name, for each key a FULL JOIN merges.
    keys: Vec<(String, String)>,
    /// For a FULL JOIN, the merged keys, each the first non-NULL side's value.
    relation: Option<ScopeRelation>,
}

fn convert_join(
    join: &Value,
    earlier: &[ScopeRelation],
    (outer, merging): (Option<&Scope<'_>>, &RefCell<Option<Merging>>),
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
    if let (Some(lateral), "Implicit" | "Cross", None) = (lateral_payload(source), kind, on) {
        let flattened = lateral_flatten(&lateral, earlier, outer, context)
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
            (earlier, merging),
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
    (earlier, merging): (&[ScopeRelation], &RefCell<Option<Merging>>),
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let construct = node_construct("join");
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let keyword = match (kind, has_on) {
        ("Inner", false) => INNER_JOIN_KEYWORD,
        ("Left", false) => "LEFT JOIN",
        ("Full", false) => FULL_JOIN_KEYWORD,
        _ => return Err(refuse(format!("a {kind} join with USING"))),
    };
    let _rule = context.log.rule(JOIN_USING);
    let log = context.log;
    let mut merged_keys: Vec<(String, String)> = Vec::new();
    let mut merged_items: Vec<String> = Vec::new();
    let mut merged_columns: Vec<ColumnSpec> = Vec::new();
    let mut equalities: Vec<Value> = Vec::with_capacity(keys.len());
    let unwidened: Vec<ColumnSpec> = relation.columns.clone();
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
        let merged_type = match (earlier_type, joined.data_type) {
            (Some(left), right) if left == right => left,
            (
                Some(SnowflakeType::Number {
                    precision: left,
                    scale,
                }),
                SnowflakeType::Number {
                    precision: right,
                    scale: joined_scale,
                },
            ) if scale == joined_scale => SnowflakeType::Number {
                precision: match keyword {
                    FULL_JOIN_KEYWORD => left.max(right),
                    _ => left,
                },
                scale,
            },
            _ => {
                return Err(refuse(format!(
                    "the USING key `{name}` of different types on its two sides, whose merged \
                     value no recording shows"
                )));
            }
        };
        let read = |alias: &str| {
            format!(
                "{}.{}",
                quoted_identifier(alias, log),
                quoted_identifier(&name, log)
            )
        };
        merged_items.push(format!(
            "coalesce({}, {}) AS {}",
            read(&owner.alias),
            read(&relation.alias),
            quoted_identifier(&name, log)
        ));
        merged_keys.push((owner.alias.clone(), name.clone()));
        merged_columns.push(ColumnSpec {
            not_null: false,
            widened: false,
            data_type: merged_type,
            ..joined.clone()
        });
        if let Some(joined) = relation
            .columns
            .iter_mut()
            .find(|column| column.name == name)
            && joined.data_type != merged_type
            && keyword == INNER_JOIN_KEYWORD
        {
            joined.data_type = merged_type;
            joined.widened = true;
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
    visible.push(ScopeRelation {
        columns: unwidened,
        ..relation.clone()
    });
    let scope = Scope::new(visible, None);
    let site = Site {
        scope: &scope,
        context,
        placement: Placement::ROW,
    };
    let condition = condition_sql(&condition, site)?;
    if keyword != FULL_JOIN_KEYWORD {
        merging.replace(Some(Merging {
            keys: Vec::new(),
            relation: None,
        }));
        return Ok((format!(" {keyword} {source_sql} ON {condition}"), relation));
    }
    let alias = format!("{USING_ALIAS_PREFIX}{}", earlier.len());
    let sql = format!(
        " {keyword} {source_sql} ON {condition} CROSS JOIN LATERAL (SELECT {}) AS {}",
        merged_items.join(", "),
        quoted_identifier(&alias, log)
    );
    merging.replace(Some(Merging {
        keys: merged_keys,
        relation: Some(ScopeRelation {
            alias,
            columns: merged_columns,
            merged: Vec::new(),
        }),
    }));
    Ok((sql, relation))
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

/// The LATERAL payload of `LATERAL f(...) alias` or `TABLE(FLATTEN(...)) alias`, correlated alike.
fn lateral_payload(source: &Value) -> Option<Value> {
    if let Some(lateral) = source.get(LATERAL_KEY) {
        return Some(lateral.clone());
    }
    let ("alias", aliased) = node_kind(source)? else {
        return None;
    };
    let call = table_call(aliased.get("this")?)?;
    let flatten = call
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| name.eq_ignore_ascii_case(FLATTEN_NAME));
    let alias = identifier_name(aliased.get("alias")?)?;
    let unlisted = aliased
        .get("column_aliases")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty);
    (flatten && unlisted && alias == alias.to_ascii_uppercase())
        .then(|| json!({ "this": { "function": call }, "alias": alias }))
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
        Some(("values", payload)) => {
            let mut rows = payload.as_object().cloned().unwrap_or_default();
            let mut wrapped = serde_json::Map::new();
            for name in ["alias", "column_aliases"] {
                if let Some(value) = rows.remove(name) {
                    wrapped.insert(name.to_owned(), value);
                }
            }
            wrapped.insert("this".to_owned(), json!({ "values": rows }));
            let wrapped = Value::Object(wrapped);
            subquery_source(&wrapped, earlier, context)
        }
        Some(("function", _)) => table_function_source(source, None, context),
        Some(("alias", payload)) => aliased_source(payload, context),
        Some(("pivot", payload)) => pivot_source(payload, earlier, context),
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
    require_understood(table, &["name", "alias", "catalog", "schema"], &construct)?;
    let name = identifier_name(field(table, "name", &construct)?)
        .ok_or_else(|| missing(construct.clone(), "a table has no name".to_owned()))?;
    let mut parts: Vec<String> = ["catalog", "schema"]
        .iter()
        .filter_map(|part| table.get(*part).and_then(identifier_name))
        .collect();
    parts.push(name.clone());
    let written = parts.join(RELATION_KEY_SEPARATOR);
    let is_cte = parts.len() == 1 && context.ctes.iter().any(|(cte, _)| cte == &name);
    let options = context.options;
    if let (1, Some(schema)) = (parts.len(), &options.current_schema) {
        parts.insert(0, schema.clone());
    }
    if let (2, Some(database)) = (parts.len(), &options.current_database) {
        parts.insert(0, database.clone());
    }
    let completed = parts.join(RELATION_KEY_SEPARATOR);
    let key = match is_cte {
        true => name.clone(),
        false => relation_key(&parts, context.schema)?.unwrap_or_else(|| written.clone()),
    };
    if !is_cte && context.ctes.iter().any(|(cte, _)| cte == &key) {
        return Err(unresolved(format!(
            "`{written}` reads the schema relation `{key}`, which a CTE of the same name hides"
        )));
    }
    let columns = context.relation(&key)?;
    let alias = table
        .get("alias")
        .and_then(identifier_name)
        .unwrap_or_else(|| name.clone());
    if !is_cte && let Some(earlier) = context.qualifiers.read(&key, &completed) {
        return Err(unresolved(format!(
            "`{earlier}` and `{completed}` both read the schema relation `{key}`; declare each \
             under its qualified name, or set the session's DATABASE and SCHEMA, to tell them apart"
        )));
    }
    let relation_name = match is_cte {
        true => quoted_identifier(&key, log),
        false => quoted_relation(&key, log),
    };
    let mut guarded: Vec<String> = Vec::new();
    for column in columns
        .iter()
        .filter(|column| !is_cte && column.data_type == SnowflakeType::SemiStructured)
        .filter(|column| may_read(context.locator.tree(), &column.name))
    {
        let quoted = quoted_identifier(&column.name, log);
        let reads = literal_reads(context.locator.tree(), &column.name);
        guarded.push(format!(
            "{} AS {quoted}",
            guarded_column(&quoted, reads.as_deref(), context)?
        ));
    }
    let relation = match guarded.is_empty() {
        true => relation_name,
        false => format!(
            "(SELECT * REPLACE ({}) FROM {relation_name})",
            guarded.join(", ")
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
    if name == GENERATOR_NAME && !context.schema.functions.contains_key(&name) {
        return generator_source(call, alias, context);
    }
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

/// `TABLE(GENERATOR(ROWCOUNT => n))` for a literal n: n rows numbered from 0 for SEQ4 and SEQ8.
fn generator_source(
    call: &Payload,
    alias: Option<String>,
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let construct = function_construct(GENERATOR_NAME);
    let log = context.log;
    let _rule = log.rule(GENERATOR_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let rows = generator_rows(list(call, "args")).map_err(refuse)?;
    let alias = alias.unwrap_or_else(|| GENERATOR_NAME.to_owned());
    let column = quoted_identifier(GENERATOR_SEQUENCE_COLUMN, log);
    Ok((
        format!(
            "(SELECT CAST(range AS BIGINT) AS {column} FROM range({rows})) AS {}",
            quoted_identifier(&alias, log)
        ),
        ScopeRelation {
            alias,
            columns: vec![ColumnSpec::new(
                GENERATOR_SEQUENCE_COLUMN,
                SnowflakeType::Number {
                    precision: 19,
                    scale: 0,
                },
            )],
            merged: Vec::new(),
        },
    ))
}

/// GENERATOR's row count: ROWCOUNT (beside a TIMELIMIT of 1s or more), or 0 when both are absent.
fn generator_rows(arguments: &[Value]) -> Result<u64, String> {
    let mut rowcount: Option<&Value> = None;
    let mut timelimit: Option<&Value> = None;
    for argument in arguments {
        let named = argument
            .get("named_argument")
            .and_then(Value::as_object)
            .ok_or_else(|| "GENERATOR with a positional argument".to_owned())?;
        let value = named
            .get("value")
            .filter(|value| node_kind(value).is_some_and(|(kind, _)| kind != NULL_KIND));
        match named.get("name").and_then(identifier_name) {
            Some(name) if name.eq_ignore_ascii_case(ROWCOUNT_ARGUMENT) => rowcount = value,
            Some(name) if name.eq_ignore_ascii_case(TIMELIMIT_ARGUMENT) => timelimit = value,
            _ => {
                return Err(
                    "GENERATOR with an argument other than ROWCOUNT or TIMELIMIT".to_owned(),
                );
            }
        }
    }
    let rows = match rowcount {
        None => None,
        Some(value) => match constant_integer(value) {
            Some(rows) if rows < 0 => {
                return Err(
                    "a negative ROWCOUNT, which Snowflake refuses when compiling".to_owned(),
                );
            }
            Some(rows) if rows > i128::from(GENERATOR_MAX_ROWS) => {
                return Err(format!("a ROWCOUNT above {GENERATOR_MAX_ROWS}"));
            }
            Some(rows) => Some(u64::try_from(rows).map_err(|error| error.to_string())?),
            None => {
                return Err("a ROWCOUNT other than a whole number written with literals".to_owned());
            }
        },
    };
    match (rows, timelimit) {
        (rows, None) => Ok(rows.unwrap_or(0)),
        (Some(rows), Some(limit)) if integer_literal(limit).is_some_and(|seconds| seconds >= 1) => {
            Ok(rows)
        }
        (Some(_), Some(_)) => Err("ROWCOUNT beside a TIMELIMIT under one whole second".to_owned()),
        (None, Some(_)) => Err(
            "GENERATOR with only a TIMELIMIT, whose row count depends on how fast rows are made"
                .to_owned(),
        ),
    }
}

/// A whole number written with integer literals, signs, parentheses, `+`, `-` and `*`.
fn constant_integer(node: &Value) -> Option<i128> {
    let operands = |payload: &Value| {
        Some((
            constant_integer(payload.get("left")?)?,
            constant_integer(payload.get("right")?)?,
        ))
    };
    match node_kind(node)? {
        (NEGATE_KIND, payload) => constant_integer(payload.get("this")?)?.checked_neg(),
        (PAREN_KIND, payload) => constant_integer(payload.get("this")?),
        (ADD_KIND, payload) => operands(payload).and_then(|(left, right)| left.checked_add(right)),
        (SUB_KIND, payload) => operands(payload).and_then(|(left, right)| left.checked_sub(right)),
        (MUL_KIND, payload) => operands(payload).and_then(|(left, right)| left.checked_mul(right)),
        _ => integer_literal(node).map(i128::from),
    }
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

/// `source PIVOT(...) AS alias(columns)`, as the grouped subquery it equals over the source.
fn pivot_source(
    payload: &Value,
    earlier: Earlier<'_, '_>,
    context: &Context<'_>,
) -> Result<(String, ScopeRelation), Refusal> {
    let construct = node_construct("pivot");
    let _rule = context.log.rule(PIVOT_RULE);
    let pivot = payload_object(payload, &construct)?;
    let source = field(pivot, "this", &construct)?;
    let columns = {
        let _muted = context.log.muted();
        suspended(|| convert_source_kind(source, earlier, context))?
            .1
            .columns
    };
    subquery_source(&pivot_subquery(pivot, &columns)?, earlier, context)
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
    let query = field(subquery, "this", &construct)?;
    let before = recorded();
    let converted = convert_query(
        query,
        lateral.then_some(&scope),
        &context.projecting(true),
        true,
    );
    let unresolved_inside = match &converted {
        Ok(_) => unresolved_since(before),
        Err(refusal) => refusal.category == RefusalCategory::UnresolvedName,
    };
    let correlated = match (outer, lateral, unresolved_inside) {
        (Some(outer), false, true) => reads_enclosing(query, outer, context),
        _ => false,
    };
    let (sql, mut columns) = match (correlated, converted) {
        (false, converted) => converted?,
        (true, converted) => {
            let refusal = context.locator.placed(
                outside_conditions(node_construct("subquery"), CORRELATED_DERIVED.to_owned()),
                query,
            );
            match (collecting(), converted) {
                (true, Ok(converted)) => {
                    replace_since(before, refusal);
                    converted
                }
                _ => return Err(refusal),
            }
        }
    };
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
        if let Some((index, refusal)) = repeated_name(&columns, Some(NESTED_REPEAT)) {
            return Err(context.locator.placed(refusal, &renamed[index]));
        }
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

/// Whether a FROM subquery's names resolve once it may read the enclosing query's columns.
fn reads_enclosing(query: &Value, outer: &Scope<'_>, context: &Context<'_>) -> bool {
    let _muted = context.log.muted();
    let enclosing = Scope::new(Vec::new(), Some(outer));
    let converted =
        suspended(|| convert_query(query, Some(&enclosing), &context.projecting(true), true));
    match converted {
        Ok(_) => true,
        Err(refusal) => refusal.reason == CORRELATED_DERIVED,
    }
}

/// Whether `tree` names column `name`, or reads columns unnamed: `*`, `$n`, PIVOT or NATURAL.
fn may_read(tree: &Value, name: &str) -> bool {
    let plain = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$');
    !plain || reads_unnamed_or(tree, name)
}

/// The walk behind `may_read`.
fn reads_unnamed_or(tree: &Value, name: &str) -> bool {
    match tree {
        Value::String(text) => {
            text.eq_ignore_ascii_case(name)
                || text.to_ascii_lowercase().starts_with(NATURAL_JOIN_PREFIX)
        }
        Value::Array(items) => items.iter().any(|item| reads_unnamed_or(item, name)),
        Value::Object(fields) => fields.iter().any(|(key, value)| match key.as_str() {
            "star" | "parameter" | "pivot" | "unpivot" | "columns" => true,
            "count" | "aggregate_function" if counts_rows(value) => false,
            _ => reads_unnamed_or(value, name),
        }),
        _ => false,
    }
}

/// The literal paths `tree` reads column `name` through; `None` when it may read it otherwise.
fn literal_reads(tree: &Value, name: &str) -> Option<Vec<Vec<PathStep>>> {
    if let Some(("j_s_o_n_extract", payload)) = node_kind(tree)
        && let Some(steps) = literal_path(payload, name)
    {
        return Some(vec![steps]);
    }
    match tree {
        Value::String(text) => (!text.eq_ignore_ascii_case(name)
            && !text.to_ascii_lowercase().starts_with(NATURAL_JOIN_PREFIX))
        .then(Vec::new),
        Value::Array(items) => items
            .iter()
            .map(|item| literal_reads(item, name))
            .collect::<Option<Vec<_>>>()
            .map(|reads| reads.concat()),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| match key.as_str() {
                "star" | "parameter" | "pivot" | "unpivot" | "columns" => None,
                "count" | "aggregate_function" if counts_rows(value) => Some(Vec::new()),
                _ => literal_reads(value, name),
            })
            .collect::<Option<Vec<_>>>()
            .map(|reads| reads.concat()),
        _ => Some(Vec::new()),
    }
}

/// The steps of `column:path` when the path is a literal and its base is column `name`.
fn literal_path(payload: &Value, name: &str) -> Option<Vec<PathStep>> {
    let ("column", column) = node_kind(payload.get("this")?)? else {
        return None;
    };
    let named = column.get("name").and_then(identifier_name)?;
    if !named.eq_ignore_ascii_case(name) {
        return None;
    }
    let text = payload
        .get("expression")?
        .get("literal")?
        .get("value")?
        .as_str()?;
    path_steps(text)
}

/// Whether a COUNT payload is COUNT(*) alone, which reads no column.
fn counts_rows(payload: &Value) -> bool {
    let Some(fields) = payload.as_object() else {
        return false;
    };
    let star =
        |value: &Value| matches!(node_kind(value), Some(("star", star)) if is_inert_star(star));
    let named_count = fields
        .get("name")
        .and_then(Value::as_str)
        .is_none_or(|called| called.eq_ignore_ascii_case("COUNT"));
    named_count
        && fields.iter().all(|(key, value)| match key.as_str() {
            "this" => star(value),
            "args" => value
                .as_array()
                .is_some_and(|items| items.len() == 1 && star(&items[0])),
            "name" => true,
            _ => is_inert(value),
        })
}

/// A bare `*`, without a qualifier, ILIKE, EXCLUDE, REPLACE or RENAME.
fn is_inert_star(star: &Value) -> bool {
    star.as_object()
        .is_none_or(|fields| fields.values().all(is_inert))
}
