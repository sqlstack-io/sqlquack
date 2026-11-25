//! UNION, UNION ALL, EXCEPT, MINUS and INTERSECT.

use serde_json::Value;
use sqlquack_blocks::blocks::main::cast::cast;
use sqlquack_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_blocks::held::main::variant_json_branch::variant_json_branch;
use sqlquack_blocks::semi_structured::constants::JSON_FIDELITY;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::main::text_reading::text_reading;
use sqlquack_semantics::snowflake::typing::main::unify::unify;
use sqlquack_semantics::snowflake::typing::models::{CastTarget, SnowflakeType};

use crate::snowflake::conversion::_helpers::expressions::conditionals::read_text;
use crate::snowflake::conversion::_helpers::expressions::semi_structured::{
    held_cast, variant_cast,
};
use crate::snowflake::conversion::_helpers::expressions::temporal::unified_cast;
use crate::snowflake::conversion::_helpers::expressions::windows::canonical_variant;
use crate::snowflake::conversion::_helpers::queries::clauses::order_and_limit;
use crate::snowflake::conversion::_helpers::queries::query::{
    ConvertedQuery, Output, QueryRequest, convert_output, convert_with, integer_item,
    require_distinct_names,
};
use crate::snowflake::conversion::_helpers::support::declared::merged;
use crate::snowflake::conversion::_helpers::support::divergence::{
    diverge, require_widening_range,
};
use crate::snowflake::conversion::_helpers::support::scope::{Context, Emitted, Scope};
use crate::snowflake::conversion::_helpers::support::site::{Placement, Site};
use crate::snowflake::conversion::_helpers::support::tree::{
    Payload, field, flag, identifier_name, list, node_construct, node_kind, payload_object,
    require_understood,
};
use crate::snowflake::conversion::constants::DUPLICATE_VARIANTS;
use crate::snowflake::conversion::constants::{
    NULL_BRANCHES, SEMI_STRUCTURED_HOLDING, SET_OPERATION, TEXT_BRANCHES, UNION_BY_NAME,
    VARCHAR_NAME, VARIANT_NAME, VARIANT_TEXT_BRANCHES,
};
use sqlquack_semantics::schemas::models::Holding;
use sqlquack_semantics::snowflake::typing::types::ColumnSpec;
use sqlquack_semantics::snowflake::typing::types::ScopeRelation;

/// The relation name for the result of a set operation that is then ordered or limited.
const SET_RESULT: &str = "_SET";
/// Why a repeated name in a set operation's result is refused.
const SET_REPEAT: &str =
    "in a set operation the converted SQL wraps, where DuckDB renames the second column";
/// The relation name for a branch cast to the unified column types.
const BRANCH: &str = "_BRANCH";

/// The node kind of a UNION.
const UNION_KIND: &str = "union";
/// The keyword of a UNION that keeps duplicates.
const UNION_ALL: &str = "UNION ALL";

/// A set operation, whose branches are cast to their unified column types where they differ.
pub(crate) fn convert_set_operation(
    kind: &str,
    payload: &Value,
    request: QueryRequest<'_>,
) -> Result<ConvertedQuery, Refusal> {
    let QueryRequest {
        outer,
        context,
        output,
    } = request;
    let construct = node_construct(kind);
    let operation = payload_object(payload, &construct)?;
    require_understood(
        operation,
        &[
            "left",
            "right",
            "all",
            "distinct",
            "order_by",
            "limit",
            "offset",
            "with",
            "by_name",
            "corresponding",
            "strict",
        ],
        &construct,
    )?;
    let _rule = context.log.rule(SET_OPERATION);
    let by_name = flag(operation, "by_name");
    if flag(operation, "corresponding")
        || flag(operation, "strict")
        || (by_name && kind != UNION_KIND)
    {
        return Err(missing(
            construct,
            "CORRESPONDING, STRICT and BY NAME outside UNION".to_owned(),
        ));
    }
    let _by_name_rule = by_name.then(|| context.log.rule(UNION_BY_NAME));
    let keyword = match (kind, flag(operation, "all")) {
        ("union", true) => UNION_ALL,
        ("union", false) => "UNION",
        ("except", false) => "EXCEPT",
        ("intersect", false) => "INTERSECT",
        _ => {
            return Err(missing(
                construct,
                format!("{} ALL", kind.to_ascii_uppercase()),
            ));
        }
    };
    let context = &context.projecting(context.free_projections && keyword == UNION_ALL);
    let (with_sql, context) = convert_with(operation.get("with"), outer, context)?;
    let ordered = ["order_by", "limit", "offset"]
        .iter()
        .any(|clause| operation.get(*clause).is_some_and(|value| !value.is_null()));
    let (left_node, right_node) = (
        field(operation, "left", &construct)?,
        field(operation, "right", &construct)?,
    );
    let chain = keyword == UNION_ALL && chain_member(operation);
    let (bare, root) = match (chain, output.bare) {
        (true, 0) => {
            let bare = hoisted(operation);
            (bare, bare != 0)
        }
        (true, bare) => (bare, false),
        (false, _) => (0, false),
    };
    let left = convert_output(
        left_node,
        outer,
        &context,
        Output {
            written: output.written || by_name || ordered || root,
            bare,
            ..output
        },
    )?;
    let right = convert_output(
        right_node,
        outer,
        &context,
        Output {
            named: false,
            written: by_name,
            bare,
        },
    )?;
    let (left_columns, right_columns) = (left.1.clone(), right.1.clone());
    let left_written = output.written || ordered || root;
    let (left_sql, right_sql, columns) = match by_name {
        true => {
            let columns = named_columns((&left.1, &right.1), &construct, &context)?;
            (
                conform_by_name(left, &columns, left_written, &construct, &context)?,
                conform_by_name(right, &columns, false, &construct, &context)?,
                columns,
            )
        }
        false => {
            let columns = unified_columns((&left.1, &right.1), &construct, &context)?;
            let left_plain = plain_select(left_node) || chained(left_node, kind, operation);
            (
                conform(left, &columns, (left_written, left_plain, bare), &construct, &context)?,
                conform(right, &columns, (false, plain_select(right_node), bare), &construct, &context)?,
                columns,
            )
        }
    };
    let columns = proven_columns(columns, (&left_columns, &right_columns), by_name);
    let (left_sql, right_sql) = match keyword != UNION_ALL
        && columns
            .iter()
            .any(|column| column.data_type == SnowflakeType::SemiStructured)
    {
        true => (
            canonical_branch(left_sql, (&left_columns, &columns), left_written, &construct, &context)?,
            canonical_branch(right_sql, (&right_columns, &columns), false, &construct, &context)?,
        ),
        false => (left_sql, right_sql),
    };
    let sql = format!("{with_sql}{left_sql} {keyword} {right_sql}");
    let sql = match root {
        true => cast_once(&sql, &columns, bare, &construct, &context)?,
        false => sql,
    };
    let wrapped = ordered
        || columns.iter().any(|column| {
            matches!(
                column.data_type,
                SnowflakeType::SemiStructured | SnowflakeType::Null
            )
        });
    if wrapped {
        require_distinct_names(&columns, Some(SET_REPEAT))?;
    }
    if !ordered {
        return Ok((sql, columns));
    }
    let log = context.log;
    let scope = Scope::new(
        vec![ScopeRelation {
            alias: SET_RESULT.to_owned(),
            columns: columns.clone(),
            merged: Vec::new(),
        }],
        outer,
    );
    let outputs: Vec<(String, Emitted)> = columns
        .iter()
        .map(|column| {
            let sql = quoted_identifier(&column.name, log);
            let emitted = Emitted {
                suffixed: column.suffixed,
                ..Emitted::new(sql, column.data_type)
            };
            (column.name.clone(), emitted)
        })
        .collect();
    let site = Site {
        scope: &scope,
        context: &context,
        placement: Placement::PROJECTION,
    };
    let tail = order_and_limit(operation, site, &outputs)?;
    Ok((
        format!(
            "SELECT * FROM ({sql}) AS {}{tail}",
            quoted_identifier(SET_RESULT, log)
        ),
        columns,
    ))
}

/// The branches' common column types.
fn unified_columns(
    (left, right): (&[ColumnSpec], &[ColumnSpec]),
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Vec<ColumnSpec>, Refusal> {
    if left.len() != right.len() {
        return Err(outside_conditions(
            construct.clone(),
            "set operation branches differ in column count".to_owned(),
        ));
    }
    let mut columns: Vec<ColumnSpec> = Vec::new();
    for (left_column, right_column) in left.iter().zip(right) {
        let holding = shared_holding([Some(left_column), Some(right_column)], construct, context)?;
        let variant = SnowflakeType::SemiStructured;
        let data_type = match (left_column.data_type, right_column.data_type) {
            (left_type, right_type) if left_type == variant && right_type == variant => variant,
            (left_type, right_type) => {
                set_operation_type((left_type, right_type), construct, context)?
            }
        };
        columns.push(ColumnSpec {
            holding,
            unordered: left_column.unordered || right_column.unordered,
            suffixed: left_column.suffixed || right_column.suffixed,
            ..ColumnSpec::new(left_column.name.clone(), data_type)
        });
    }
    Ok(columns)
}

/// A branch cast to the `target` column types, its names `written`; `plain` when it can stand unparenthesised.
fn conform(
    (sql, actual): ConvertedQuery,
    target: &[ColumnSpec],
    (written, plain, bare): (bool, bool, u64),
    construct: &Construct,
    context: &Context<'_>,
) -> Result<String, Refusal> {
    let is_bare = |index: usize| index < 64 && bare >> index & 1 == 1;
    let same = actual.iter().zip(target).enumerate().all(|(index, (actual, target))| {
        is_bare(index)
            || (actual.data_type == target.data_type
                && target.data_type != SnowflakeType::Null
                && !joins_held(actual, target))
    });
    if same {
        return Ok(match plain {
            true => sql,
            false => format!("({sql})"),
        });
    }
    let log = context.log;
    let names: Vec<String> = (1..=actual.len())
        .map(|index| format!("C{index}"))
        .collect();
    let mut items: Vec<String> = Vec::new();
    for (index, ((name, column), own)) in names.iter().zip(target).zip(&actual).enumerate() {
        let value = match (column.data_type, own.data_type) {
            _ if is_bare(index) => quoted_identifier(name, log),
            (SnowflakeType::SemiStructured, _) if joins_held(own, column) => {
                variant_json_branch(&quoted_identifier(name, log), log)
            }
            (SnowflakeType::SemiStructured, _) => quoted_identifier(name, log),
            (SnowflakeType::Varchar, SnowflakeType::SemiStructured) => {
                variant_text(quoted_identifier(name, log), own, context)?
            }
            (target_type, SnowflakeType::Varchar)
                if target_type != SnowflakeType::Varchar && target_type != SnowflakeType::Null =>
            {
                let reading = text_reading(&[SnowflakeType::Varchar, target_type], log)
                    .ok_or_else(|| {
                        outside_conditions(
                            construct.clone(),
                            format!("text beside {target_type} in a set operation"),
                        )
                    })?;
                let read = read_text(
                    &Emitted::new(quoted_identifier(name, log), SnowflakeType::Varchar),
                    (reading, target_type),
                    construct,
                    context,
                )?;
                cast(&read.sql, target_type, log)
                    .map_err(|reason| outside_conditions(construct.clone(), reason))?
            }
            (SnowflakeType::Null, _) => {
                let _rule = log.rule(NULL_BRANCHES);
                cast(&quoted_identifier(name, log), SnowflakeType::Varchar, log)
                    .map_err(|reason| outside_conditions(construct.clone(), reason))?
            }
            (data_type, own_type) => unified_cast(
                &quoted_identifier(name, log),
                (own_type, data_type),
                (context, construct),
            )
            .map_err(|reason| outside_conditions(construct.clone(), reason))?,
        };
        items.push(named_item(value, &column.name, written, log));
    }
    let column_list: Vec<String> = names
        .iter()
        .map(|name| quoted_identifier(name, log))
        .collect();
    Ok(format!(
        "SELECT {} FROM ({sql}) AS {}({})",
        items.join(", "),
        quoted_identifier(BRANCH, log),
        column_list.join(", ")
    ))
}

/// Whether a UNION ALL is one link of a chain: not BY NAME, not ordered or limited and without WITH.
fn chain_member(operation: &Payload) -> bool {
    !flag(operation, "by_name")
        && ["with", "order_by", "limit", "offset"]
            .iter()
            .all(|clause| operation.get(*clause).is_none_or(Value::is_null))
}

/// The positions, as bits, a UNION ALL chain writes as bare integer literals and casts once around itself.
///
/// Every branch must be a plain SELECT holding an integer literal there, its first branch naming
/// every column apart, and casting once must save more bytes than its wrapper costs.
fn hoisted(operation: &Payload) -> u64 {
    let mut branches: Vec<&Payload> = Vec::new();
    if !chain_branches(operation, &mut branches) {
        return 0;
    }
    let mut bare = u64::MAX;
    for branch in &branches {
        let items = list(branch, "expressions");
        if items.len() > 64 || flag(branch, "distinct") || branch.get("connect").is_some_and(|connect| !connect.is_null()) {
            return 0;
        }
        let mut literals = 0_u64;
        for (index, item) in items.iter().enumerate() {
            if integer_item(item).is_some() {
                literals |= 1 << index;
            }
        }
        bare &= literals;
    }
    let Some(names) = branches.first().and_then(|first| aliases(first)) else {
        return 0;
    };
    let positions = (0..names.len()).filter(|index| bare >> index & 1 == 1);
    let cost: usize = positions
        .clone()
        .map(|index| 2 * names[index].len() + CAST_ONCE_ITEM)
        .sum::<usize>()
        + CAST_ONCE_WRAPPER;
    let saved = branches.len() * positions.count() * LITERAL_CAST;
    match saved > cost {
        true => bare,
        false => 0,
    }
}

/// The bytes a cast of an integer literal adds at the least: `CAST(`, ` AS DECIMAL(` and `1,0))`.
const LITERAL_CAST: usize = 22;
/// The most bytes one cast written once adds beside its name twice, quotes doubled included.
const CAST_ONCE_ITEM: usize = 40;
/// The bytes of `SELECT * REPLACE (`, `) FROM (` and `)`.
const CAST_ONCE_WRAPPER: usize = 27;

/// The plain SELECT branches of a UNION ALL chain, in order; false when a branch is anything else.
fn chain_branches<'a>(operation: &'a Payload, branches: &mut Vec<&'a Payload>) -> bool {
    ["left", "right"].iter().all(|side| {
        let Some(node) = operation.get(*side) else {
            return false;
        };
        match node_kind(node).map(|(kind, payload)| (kind, payload.as_object())) {
            Some(("union", Some(inner))) if flag(inner, "all") && chain_member(inner) => {
                chain_branches(inner, branches)
            }
            Some(("select", Some(select))) if plain_select(node) => {
                branches.push(select);
                true
            }
            _ => false,
        }
    })
}

/// A select's item names, each an alias and none repeated in any case; `None` otherwise.
fn aliases(select: &Payload) -> Option<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    for item in list(select, "expressions") {
        let ("alias", alias) = node_kind(item)? else {
            return None;
        };
        let name = alias.get("alias").and_then(identifier_name)?;
        if name.is_empty() || names.iter().any(|seen| seen.eq_ignore_ascii_case(&name)) {
            return None;
        }
        names.push(name);
    }
    Some(names)
}

/// A UNION ALL chain whose `bare` columns are cast once to their unified types.
fn cast_once(
    sql: &str,
    columns: &[ColumnSpec],
    bare: u64,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<String, Refusal> {
    let log = context.log;
    let mut items: Vec<String> = Vec::new();
    for (index, column) in columns.iter().enumerate() {
        if index < 64 && bare >> index & 1 == 1 {
            let name = quoted_identifier(&column.name, log);
            let cast = cast(&name, column.data_type, log)
                .map_err(|reason| outside_conditions(construct.clone(), reason))?;
            items.push(format!("{cast} AS {name}"));
        }
    }
    Ok(format!("SELECT * REPLACE ({}) FROM ({sql})", items.join(", ")))
}

/// A select item, `AS name` when `written`.
fn named_item(value: String, name: &str, written: bool, log: &ProvenanceLog) -> String {
    match written {
        true => format!("{value} AS {}", quoted_identifier(name, log)),
        false => value,
    }
}

/// Whether a branch is one SELECT without WITH, ORDER BY or a row limit, which stands unparenthesised.
fn plain_select(node: &Value) -> bool {
    matches!(
        node_kind(node),
        Some(("select", select)) if ["with", "order_by", "limit", "top", "fetch", "offset"]
            .iter()
            .all(|clause| select.get(*clause).is_none_or(Value::is_null))
    )
}

/// Whether a left branch is the same set operation, unordered and without WITH, which DuckDB groups the same unparenthesised.
fn chained(node: &Value, kind: &str, operation: &Payload) -> bool {
    let Some((inner, Some(branch))) = node_kind(node).map(|(inner, branch)| (inner, branch.as_object())) else {
        return false;
    };
    inner == kind
        && flag(branch, "all") == flag(operation, "all")
        && !flag(branch, "by_name")
        && ["with", "order_by", "limit", "offset"]
            .iter()
            .all(|clause| branch.get(*clause).is_none_or(Value::is_null))
}

/// A deduplicated branch, its JSON-held VARIANTs written as Snowflake writes them, keys sorted.
fn canonical_branch(
    sql: String,
    (own, target): (&[ColumnSpec], &[ColumnSpec]),
    written: bool,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<String, Refusal> {
    let log = context.log;
    let mut items: Vec<String> = Vec::with_capacity(target.len());
    let names: Vec<String> = (1..=target.len())
        .map(|index| format!("C{index}"))
        .collect();
    for ((name, column), own) in names.iter().zip(target).zip(own) {
        let value = quoted_identifier(name, log);
        let value = match column.data_type {
            SnowflakeType::SemiStructured if column.holding != Holding::Json => {
                return Err(outside_conditions(
                    construct.clone(),
                    format!("a set operation {DUPLICATE_VARIANTS}"),
                ));
            }
            SnowflakeType::SemiStructured => {
                let emitted = Emitted {
                    untainted: own.untainted,
                    sourced: own.sourced,
                    ..Emitted::new(value, SnowflakeType::SemiStructured)
                };
                canonical_variant(emitted, construct, context)?.sql
            }
            _ => value,
        };
        items.push(named_item(value, &column.name, written, log));
    }
    let column_list: Vec<String> = names
        .iter()
        .map(|name| quoted_identifier(name, log))
        .collect();
    Ok(format!(
        "(SELECT {} FROM ({sql}) AS {}({}))",
        items.join(", "),
        quoted_identifier(BRANCH, log),
        column_list.join(", ")
    ))
}

/// UNION BY NAME's columns: the left branch's, then the right's new ones, NULL-filled where absent.
fn named_columns<'a>(
    (left, right): (&'a [ColumnSpec], &'a [ColumnSpec]),
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Vec<ColumnSpec>, Refusal> {
    let mut names: Vec<&str> = left.iter().map(|column| column.name.as_str()).collect();
    for (position, column) in right.iter().enumerate() {
        let repeated = right[..position]
            .iter()
            .any(|earlier| earlier.name.eq_ignore_ascii_case(&column.name));
        if repeated {
            return Err(outside_conditions(
                construct.clone(),
                format!(
                    "UNION BY NAME columns `{}` repeat or differ only in case",
                    column.name
                ),
            ));
        }
        if !names.contains(&column.name.as_str()) {
            names.push(column.name.as_str());
        }
    }
    let mut columns: Vec<ColumnSpec> = Vec::new();
    for name in names {
        if columns
            .iter()
            .any(|column| column.name.eq_ignore_ascii_case(name))
        {
            return Err(outside_conditions(
                construct.clone(),
                format!("UNION BY NAME columns `{name}` repeat or differ only in case"),
            ));
        }
        let found = |branch: &'a [ColumnSpec]| branch.iter().find(|column| column.name == name);
        let typed = |branch: &'a [ColumnSpec]| {
            found(branch).map_or(SnowflakeType::Null, |column| column.data_type)
        };
        let holding = shared_holding([found(left), found(right)], construct, context)?;
        let data_type = set_operation_type((typed(left), typed(right)), construct, context)?;
        columns.push(ColumnSpec {
            holding,
            unordered: [found(left), found(right)]
                .into_iter()
                .flatten()
                .any(|column| column.unordered),
            suffixed: [found(left), found(right)]
                .into_iter()
                .flatten()
                .any(|column| column.suffixed),
            ..ColumnSpec::new(name, data_type)
        });
    }
    Ok(columns)
}

/// A branch projected onto UNION BY NAME's columns, NULL where it lacks one.
fn conform_by_name(
    (sql, actual): ConvertedQuery,
    target: &[ColumnSpec],
    written: bool,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<String, Refusal> {
    let log = context.log;
    let mut items: Vec<String> = Vec::new();
    for column in target {
        let own = actual.iter().find(|own| own.name == column.name);
        let value = match own {
            Some(_) => quoted_identifier(&column.name, log),
            None => "NULL".to_owned(),
        };
        let value = match (
            column.data_type,
            column.holding,
            own.map(|own| own.data_type),
        ) {
            (SnowflakeType::Varchar, _, Some(SnowflakeType::SemiStructured)) => {
                variant_text(value, own.unwrap_or(column), context)?
            }
            (
                SnowflakeType::SemiStructured,
                Holding::Variant,
                Some(SnowflakeType::SemiStructured),
            ) if own.is_some_and(|own| joins_held(own, column)) => variant_json_branch(&value, log),
            (SnowflakeType::SemiStructured, Holding::Variant, _) => {
                format!("CAST({value} AS {VARIANT_NAME})")
            }
            (SnowflakeType::SemiStructured, _, _) => {
                format!("CAST({value} AS {SEMI_STRUCTURED_HOLDING})")
            }
            (SnowflakeType::Null, _, _) => {
                let _rule = log.rule(NULL_BRANCHES);
                cast(&value, SnowflakeType::Varchar, log)
                    .map_err(|reason| outside_conditions(construct.clone(), reason))?
            }
            (data_type, _, own_type) => unified_cast(
                &value,
                (own_type.unwrap_or(SnowflakeType::Null), data_type),
                (context, construct),
            )
            .map_err(|reason| outside_conditions(construct.clone(), reason))?,
        };
        items.push(named_item(value, &column.name, written, log));
    }
    Ok(format!(
        "(SELECT {} FROM ({sql}) AS {})",
        items.join(", "),
        quoted_identifier(BRANCH, log)
    ))
}

/// A branch's VARIANT column read as text, as Snowflake reads it beside text in a set operation.
fn variant_text(
    sql: String,
    column: &ColumnSpec,
    context: &Context<'_>,
) -> Result<String, Refusal> {
    let _rule = context.log.rule(VARIANT_TEXT_BRANCHES);
    let value = Emitted {
        held: column.holding,
        untainted: column.untainted,
        sourced: column.sourced,
        ..Emitted::new(sql, SnowflakeType::SemiStructured)
    };
    let target = CastTarget {
        data_type: SnowflakeType::Varchar,
        max_length: None,
    };
    let construct = &node_construct("cast");
    match value.is_variant_held() {
        true => Ok(held_cast(value, (VARCHAR_NAME, target), false, (context, construct))?.sql),
        false => variant_cast(&value, (VARCHAR_NAME, target), false, (context, construct))
            .map_err(|reason| outside_conditions(construct.clone(), reason)),
    }
}

/// The common type of two branches' columns: text beside a VARIANT is text, otherwise as unified.
fn set_operation_type(
    (left, right): (SnowflakeType, SnowflakeType),
    construct: &Construct,
    context: &Context<'_>,
) -> Result<SnowflakeType, Refusal> {
    match (left, right) {
        (SnowflakeType::Varchar, SnowflakeType::SemiStructured)
        | (SnowflakeType::SemiStructured, SnowflakeType::Varchar) => Ok(SnowflakeType::Varchar),
        (SnowflakeType::Varchar, other @ (SnowflakeType::Number { .. } | SnowflakeType::Float))
        | (other @ (SnowflakeType::Number { .. } | SnowflakeType::Float), SnowflakeType::Varchar) =>
        {
            let _rule = context.log.rule(TEXT_BRANCHES);
            let reading = text_reading(&[SnowflakeType::Varchar, other], context.log)
                .unwrap_or(SnowflakeType::Varchar);
            require_widening_range(&[reading, other], construct, context)?;
            unify(&[reading, other], context.log)
                .map_err(|reason| outside_conditions(construct.clone(), reason))
        }
        _ => {
            require_widening_range(&[left, right], construct, context)?;
            unify(&[left, right], context.log)
                .map_err(|reason| outside_conditions(construct.clone(), reason))
        }
    }
}

/// How a result column's VARIANTs are held: as both branches do, or as a VARIANT if one is JSON.
fn shared_holding(
    branches: [Option<&ColumnSpec>; 2],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Holding, Refusal> {
    let holdings: Vec<Holding> = branches
        .into_iter()
        .flatten()
        .filter(|column| column.data_type == SnowflakeType::SemiStructured)
        .map(|column| column.holding)
        .collect();
    match holdings.as_slice() {
        [] => Ok(Holding::Json),
        [only] => Ok(*only),
        [first, second] if first == second => Ok(*first),
        [Holding::Json, Holding::Variant] | [Holding::Variant, Holding::Json] => {
            diverge(context, JSON_FIDELITY, construct)?;
            Ok(Holding::Variant)
        }
        _ => Err(outside_conditions(
            construct.clone(),
            "a set operation over a VARIANT built from a FLOAT, which DuckDB holds apart from JSON"
                .to_owned(),
        )),
    }
}

/// Whether a branch's JSON-held VARIANT `own` joins a result column held as a DuckDB VARIANT.
fn joins_held(own: &ColumnSpec, target: &ColumnSpec) -> bool {
    own.data_type == SnowflakeType::SemiStructured
        && own.holding == Holding::Json
        && target.holding == Holding::Variant
}

/// The result `columns`: proven free of typed leaves and UTC-offset where both branches are, sourced where either is.
fn proven_columns(
    columns: Vec<ColumnSpec>,
    (left, right): (&[ColumnSpec], &[ColumnSpec]),
    by_name: bool,
) -> Vec<ColumnSpec> {
    let mut proven: Vec<ColumnSpec> = Vec::with_capacity(columns.len());
    for (index, column) in columns.into_iter().enumerate() {
        let mut untainted = true;
        let mut sourced = false;
        let mut utc_offset = true;
        let mut declared: Vec<String> = Vec::new();
        for branch in [left, right] {
            let found = match by_name {
                true => branch.iter().find(|found| found.name == column.name),
                false => branch.get(index),
            };
            utc_offset &= found.is_some_and(|found| found.utc_offset);
            if let Some(found) = found {
                declared.extend(found.declared_offsets.iter().cloned());
                untainted &= found.untainted;
                sourced |= found.sourced;
            }
        }
        proven.push(ColumnSpec {
            untainted,
            sourced,
            utc_offset,
            declared_offsets: match utc_offset {
                true => merged(declared),
                false => Vec::new(),
            },
            ..column
        });
    }
    proven
}
