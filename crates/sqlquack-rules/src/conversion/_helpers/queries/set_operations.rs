//! UNION, UNION ALL, EXCEPT, MINUS and INTERSECT.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_snowflake::typing::main::unify::unify;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::queries::clauses::order_and_limit;
use crate::conversion::_helpers::queries::query::{
    ConvertedQuery, QueryRequest, convert_query, convert_with,
};
use crate::conversion::_helpers::support::divergence::require_widening_range;
use crate::conversion::_helpers::support::scope::{Context, Emitted, Scope, ScopeRelation};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    field, flag, missing, node_construct, outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{SEMI_STRUCTURED_HOLDING, SET_OPERATION, UNION_BY_NAME};
use crate::conversion::models::ColumnSpec;

/// The relation name for the result of a set operation that is then ordered or limited.
const SET_RESULT: &str = "_SET";
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
        named,
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
    let (with_sql, context) = convert_with(operation.get("with"), outer, context)?;
    let left = convert_query(
        field(operation, "left", &construct)?,
        outer,
        &context,
        named,
    )?;
    let right = convert_query(
        field(operation, "right", &construct)?,
        outer,
        &context,
        false,
    )?;
    let (left_sql, right_sql, columns) = match by_name {
        true => {
            let columns = named_columns(
                (&left.1, &right.1),
                keyword == UNION_ALL,
                &construct,
                &context,
            )?;
            (
                conform_by_name(left, &columns, &construct, &context)?,
                conform_by_name(right, &columns, &construct, &context)?,
                columns,
            )
        }
        false => {
            let columns = unified_columns(
                (&left.1, &right.1),
                keyword == UNION_ALL,
                &construct,
                &context,
            )?;
            (
                conform(left, &columns, &construct, &context)?,
                conform(right, &columns, &construct, &context)?,
                columns,
            )
        }
    };
    let sql = format!("{with_sql}{left_sql} {keyword} {right_sql}");
    let ordered = ["order_by", "limit", "offset"]
        .iter()
        .any(|clause| operation.get(*clause).is_some_and(|value| !value.is_null()));
    if !ordered {
        return Ok((sql, columns));
    }
    let log = context.log;
    let scope = Scope::new(
        vec![ScopeRelation {
            alias: SET_RESULT.to_owned(),
            columns: columns.clone(),
        }],
        outer,
    );
    let outputs: Vec<(String, Emitted)> = columns
        .iter()
        .map(|column| {
            let sql = quoted_identifier(&column.name, log);
            (column.name.clone(), Emitted::new(sql, column.data_type))
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

/// The branches' common column types; VARIANT only under UNION ALL, which compares nothing.
fn unified_columns(
    (left, right): (&[ColumnSpec], &[ColumnSpec]),
    keeps_all: bool,
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
        let variant = SnowflakeType::SemiStructured;
        let data_type = match (left_column.data_type, right_column.data_type) {
            (left_type, right_type)
                if keeps_all && left_type == variant && right_type == variant =>
            {
                variant
            }
            (left_type, right_type) if left_type == variant && right_type == variant => {
                return Err(outside_conditions(
                    construct.clone(),
                    "a set operation removing duplicate VARIANT values, whose equality no \
                     recording shows"
                        .to_owned(),
                ));
            }
            (left_type, right_type) => {
                require_widening_range(&[left_type, right_type], construct, context)?;
                unify(&[left_type, right_type], context.log)
                    .map_err(|reason| outside_conditions(construct.clone(), reason))?
            }
        };
        columns.push(ColumnSpec {
            name: left_column.name.clone(),
            data_type,
        });
    }
    Ok(columns)
}

fn conform(
    (sql, actual): ConvertedQuery,
    target: &[ColumnSpec],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<String, Refusal> {
    let same = actual
        .iter()
        .zip(target)
        .all(|(actual, target)| actual.data_type == target.data_type);
    if same {
        return Ok(format!("({sql})"));
    }
    let log = context.log;
    let names: Vec<String> = (1..=actual.len())
        .map(|index| format!("C{index}"))
        .collect();
    let mut items: Vec<String> = Vec::new();
    for (name, column) in names.iter().zip(target) {
        let value = match column.data_type {
            SnowflakeType::SemiStructured => quoted_identifier(name, log),
            data_type => cast(&quoted_identifier(name, log), data_type, log)
                .map_err(|reason| outside_conditions(construct.clone(), reason))?,
        };
        items.push(format!(
            "{value} AS {}",
            quoted_identifier(&column.name, log)
        ));
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
fn named_columns(
    (left, right): (&[ColumnSpec], &[ColumnSpec]),
    keeps_all: bool,
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
        let typed = |branch: &[ColumnSpec]| {
            branch
                .iter()
                .find(|column| column.name == name)
                .map_or(SnowflakeType::Null, |column| column.data_type)
        };
        if !keeps_all
            && typed(left) == SnowflakeType::SemiStructured
            && typed(right) == SnowflakeType::SemiStructured
        {
            return Err(outside_conditions(
                construct.clone(),
                "a set operation removing duplicate VARIANT values, whose equality no recording \
                 shows"
                    .to_owned(),
            ));
        }
        require_widening_range(&[typed(left), typed(right)], construct, context)?;
        let data_type = unify(&[typed(left), typed(right)], context.log)
            .map_err(|reason| outside_conditions(construct.clone(), reason))?;
        columns.push(ColumnSpec {
            name: name.to_owned(),
            data_type,
        });
    }
    Ok(columns)
}

/// A branch projected onto UNION BY NAME's columns, NULL where it lacks one.
fn conform_by_name(
    (sql, actual): ConvertedQuery,
    target: &[ColumnSpec],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<String, Refusal> {
    let log = context.log;
    let mut items: Vec<String> = Vec::new();
    for column in target {
        let value = match actual.iter().find(|own| own.name == column.name) {
            Some(_) => quoted_identifier(&column.name, log),
            None => "NULL".to_owned(),
        };
        let value = match column.data_type {
            SnowflakeType::SemiStructured => format!("CAST({value} AS {SEMI_STRUCTURED_HOLDING})"),
            data_type => cast(&value, data_type, log)
                .map_err(|reason| outside_conditions(construct.clone(), reason))?,
        };
        items.push(format!(
            "{value} AS {}",
            quoted_identifier(&column.name, log)
        ));
    }
    Ok(format!(
        "(SELECT {} FROM ({sql}) AS {})",
        items.join(", "),
        quoted_identifier(BRANCH, log)
    ))
}
