//! The type and nullability of a value, from its node and what its operands give.

use std::rc::Rc;

use sqlquack_syntax::expressions::names::models::{Column, Identifier, Literal};
use sqlquack_syntax::expressions::queries::models::Select;
use sqlquack_syntax::{DataType, Expression};

use crate::dialects::main::canonical_type::canonical_type;
use crate::lineage::_helpers::tracing::resolution::key;
use crate::lineage::_helpers::tracing::reuse::holds_aggregate;
use crate::lineage::_helpers::typing::bigquery_typing::{bigquery_literal_type, bigquery_type};
use crate::lineage::_helpers::typing::clickhouse_typing::{
    clickhouse_literal_type, clickhouse_type,
};
use crate::lineage::_helpers::typing::duckdb_typing::{duckdb_literal_type, duckdb_type};
use crate::lineage::_helpers::typing::postgres_typing::{postgres_literal_type, postgres_type};
use crate::lineage::_helpers::typing::snowflake_typing::{
    snowflake_literal_type, snowflake_type, snowflake_unified,
};
use crate::lineage::_helpers::typing::sqlite_typing::sqlite_literal_type;
use crate::lineage::constants::{
    ARGUMENT_TYPED_FUNCTIONS, BIGQUERY_PROFILE, BOOLEAN_TYPE, CLICKHOUSE_PROFILE,
    COALESCING_FUNCTIONS, CONDITIONAL_ARGUMENTS, COUNT_TYPE, COUNTING_FUNCTIONS,
    DECIMAL_LITERAL_TYPE, DUCKDB_PROFILE, FIRST_ARGUMENT_NULLABILITY, FLOAT_TYPES,
    FUNCTION_RESULT_TYPES, INTEGER_LITERAL_TYPE, INTEGER_TYPES, OPERAND_TYPED_FUNCTIONS,
    POSTGRES_PROFILE, SNOWFLAKE_PROFILE, SQLITE_PROFILE, TEXT_TYPE, UNMAPPED_TIMESTAMP,
};
use crate::lineage::models::{Lineage, LineageContext, Nullability, NullabilityRule};

/// A value's type and nullability.
pub(crate) type ValueType = (Option<Rc<str>>, Nullability);

/// A cast target's spelling, parameters without spaces, when lineage knows the type.
pub(crate) fn rendered_type(data_type: &DataType) -> Option<String> {
    let text: String = match data_type {
        DataType::Boolean => BOOLEAN_TYPE.to_owned(),
        DataType::Date => "DATE".to_owned(),
        DataType::BigInt { .. } => "BIGINT".to_owned(),
        DataType::Int { .. } => "INT".to_owned(),
        DataType::Float { .. } => "FLOAT".to_owned(),
        DataType::Double { .. } => "DOUBLE".to_owned(),
        DataType::Decimal {
            precision: None, ..
        } => "DECIMAL".to_owned(),
        DataType::Decimal {
            precision: Some(precision),
            scale: None,
        } => format!("DECIMAL({precision})"),
        DataType::Decimal {
            precision: Some(precision),
            scale: Some(scale),
        } => format!("DECIMAL({precision},{scale})"),
        DataType::String { .. } => "STRING".to_owned(),
        DataType::Text => TEXT_TYPE.to_owned(),
        DataType::VarChar { length: None, .. } => TEXT_TYPE.to_owned(),
        DataType::VarChar {
            length: Some(length),
            ..
        } => format!("VARCHAR({length})"),
        DataType::Time { .. } => "TIME".to_owned(),
        DataType::Timestamp {
            timezone: false, ..
        } => "TIMESTAMP".to_owned(),
        DataType::Timestamp { timezone: true, .. } => "TIMESTAMPTZ".to_owned(),
        DataType::Custom { name } => name.replace(", ", ","),
        DataType::Array { element_type, .. } => format!("ARRAY({})", rendered_type(element_type)?),
        _ => return None,
    };
    Some(text)
}

/// A literal's type and nullability.
pub(crate) fn literal_type(expression: &Expression, context: LineageContext<'_>) -> ValueType {
    let (data_type, nullability): ValueType = generic_literal_type(expression);
    if context.profile.name() == CLICKHOUSE_PROFILE {
        return (
            clickhouse_literal_type(expression).map(Rc::from),
            nullability,
        );
    }
    if context.profile.name() == BIGQUERY_PROFILE {
        return (bigquery_literal_type(expression).map(Rc::from), nullability);
    }
    match expression {
        Expression::Literal(literal) if context.profile.name() == DUCKDB_PROFILE => (
            duckdb_literal_type(literal).map(|data_type| Rc::from(data_type.spelling())),
            nullability,
        ),
        Expression::Literal(literal) if context.profile.name() == POSTGRES_PROFILE => (
            postgres_literal_type(literal).map(Rc::from).or(data_type),
            nullability,
        ),
        Expression::Literal(literal) if context.profile.name() == SQLITE_PROFILE => {
            (sqlite_literal_type(literal).map(Rc::from), nullability)
        }
        _ if context.profile.name() == SQLITE_PROFILE => (None, nullability),
        Expression::Literal(literal) if context.profile.name() == SNOWFLAKE_PROFILE => {
            match snowflake_literal_type(literal) {
                Some(recorded) => (recorded, nullability),
                None => (data_type, nullability),
            }
        }
        _ => (data_type, nullability),
    }
}

fn generic_literal_type(expression: &Expression) -> ValueType {
    let data_type: Option<&str> = match expression {
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::Number(text) if text.contains(['.', 'e', 'E']) => Some(DECIMAL_LITERAL_TYPE),
            Literal::Number(_) => Some(INTEGER_LITERAL_TYPE),
            Literal::String(_) => Some(TEXT_TYPE),
            _ => None,
        },
        Expression::Boolean(_) => Some(BOOLEAN_TYPE),
        _ => None,
    };
    let nullability: Nullability = match expression {
        Expression::Null(_) => Nullability::Nullable,
        _ => Nullability::NonNull,
    };
    (data_type.map(Rc::from), nullability)
}

/// The type and nullability of `expression` from its children's `parts`; untyped in SQLite.
pub(crate) fn computed_type(
    expression: &Expression,
    parts: &[(&Expression, Lineage)],
    context: LineageContext<'_>,
) -> ValueType {
    let (data_type, nullability): ValueType = generic_type(expression, parts, context);
    let ruled: Option<Option<Rc<str>>> = match context.profile.name() {
        CLICKHOUSE_PROFILE => Some(clickhouse_type(expression, parts, context)),
        BIGQUERY_PROFILE => Some(bigquery_type(expression, parts, context)),
        DUCKDB_PROFILE => duckdb_type(expression, parts),
        SNOWFLAKE_PROFILE => snowflake_type(expression, parts),
        POSTGRES_PROFILE => postgres_type(expression, parts),
        SQLITE_PROFILE => {
            let nullability: Nullability = match nullability {
                Nullability::Nullable => Nullability::Nullable,
                _ => Nullability::Unknown,
            };
            return (None, nullability);
        }
        _ => None,
    };
    let data_type: Option<Rc<str>> = ruled.unwrap_or(data_type);
    (
        data_type.map(|data_type| mapped_timestamp(data_type, context)),
        nullability,
    )
}

/// A timestamp spelled without its kind, as the profile's TIMESTAMP_TYPE_MAPPING maps it.
pub(crate) fn mapped_timestamp(data_type: Rc<str>, context: LineageContext<'_>) -> Rc<str> {
    match data_type.eq_ignore_ascii_case(UNMAPPED_TIMESTAMP) {
        true => Rc::from(context.profile.timestamp_type()),
        false => data_type,
    }
}

fn generic_type(
    expression: &Expression,
    parts: &[(&Expression, Lineage)],
    context: LineageContext<'_>,
) -> ValueType {
    match expression {
        Expression::Eq(_)
        | Expression::Neq(_)
        | Expression::Lt(_)
        | Expression::Lte(_)
        | Expression::Gt(_)
        | Expression::Gte(_)
        | Expression::And(_)
        | Expression::Or(_)
        | Expression::Not(_)
        | Expression::In(_)
        | Expression::Between(_)
        | Expression::Like(_)
        | Expression::ILike(_)
        | Expression::Is(_)
        | Expression::Exists(_)
        | Expression::RegexpLike(_) => (Some(Rc::from(BOOLEAN_TYPE)), Nullability::Unknown),
        Expression::IsNull(_) => (Some(Rc::from(BOOLEAN_TYPE)), Nullability::NonNull),
        Expression::ListAgg(_) | Expression::StringAgg(_) | Expression::GroupConcat(_) => {
            (Some(Rc::from(TEXT_TYPE)), Nullability::Unknown)
        }
        Expression::Neg(_) | Expression::Pos(_) => (first_type(parts), Nullability::Unknown),
        Expression::Upper(_) | Expression::Lower(_) => (
            Some(Rc::from(TEXT_TYPE)),
            ruled(NullabilityRule::FirstArgument, parts),
        ),
        Expression::Concat(_) | Expression::DPipe(_) => {
            (Some(Rc::from(TEXT_TYPE)), Nullability::Unknown)
        }
        Expression::Count(_) => (Some(Rc::from(COUNT_TYPE)), Nullability::NonNull),
        Expression::CountIf(_) => (None, Nullability::NonNull),
        Expression::Coalesce(_) => {
            let operands: Vec<&Expression> = parts.iter().map(|(child, _)| *child).collect();
            (common_type(parts, context, &operands), coalesced(parts))
        }
        Expression::Min(_) | Expression::Max(_) => (first_type(parts), Nullability::Unknown),
        Expression::Case(case) => {
            let mut results: Vec<&Expression> =
                case.whens.iter().map(|(_, result)| result).collect();
            results.extend(case.else_.iter());
            (common_type(parts, context, &results), Nullability::Unknown)
        }
        Expression::Function(function) => function_type(&function.name, parts, context),
        Expression::AggregateFunction(function) => function_type(&function.name, parts, context),
        Expression::IfFunc(_) => {
            let results: Vec<&Expression> = parts.iter().skip(1).map(|(child, _)| *child).collect();
            (
                common_type(parts, context, &results),
                ruled(NullabilityRule::ConditionalResult, parts),
            )
        }
        Expression::WithinGroup(group) => parts
            .iter()
            .find(|(child, _)| std::ptr::eq(*child, &group.this))
            .map_or((None, Nullability::Unknown), |(_, part)| {
                (part.data_type.clone(), part.nullability)
            }),
        Expression::Add(_) | Expression::Sub(_) | Expression::Mul(_) => (
            float_arithmetic(parts).or_else(|| integer_arithmetic(parts, context)),
            Nullability::Unknown,
        ),
        Expression::Div(_) | Expression::Mod(_) => (float_arithmetic(parts), Nullability::Unknown),
        Expression::WindowFunction(window) => parts
            .iter()
            .find(|(child, _)| std::ptr::eq(*child, &window.this))
            .map_or((None, Nullability::Unknown), |(_, part)| {
                (part.data_type.clone(), Nullability::Unknown)
            }),
        _ => (None, Nullability::Unknown),
    }
}

fn function_type(
    name: &str,
    parts: &[(&Expression, Lineage)],
    context: LineageContext<'_>,
) -> ValueType {
    let (data_type, nullability): ValueType = built_in_function_type(name, parts, context);
    let data_type: Option<Rc<str>> = context
        .schema
        .function_type(name)
        .map(Rc::from)
        .or(data_type);
    let built_in: Option<NullabilityRule> = FIRST_ARGUMENT_NULLABILITY
        .iter()
        .any(|function| function.eq_ignore_ascii_case(name))
        .then_some(NullabilityRule::FirstArgument);
    let nullability: Nullability = match context.schema.function_nullability(name).or(built_in) {
        Some(rule) if nullability == Nullability::Unknown => ruled(rule, parts),
        _ => nullability,
    };
    (data_type, nullability)
}

/// A function's nullability under `rule`, from its arguments'.
fn ruled(rule: NullabilityRule, parts: &[(&Expression, Lineage)]) -> Nullability {
    let argument = |index: usize| {
        parts
            .get(index)
            .map_or(Nullability::Unknown, |(_, part)| part.nullability)
    };
    match rule {
        NullabilityRule::FirstArgument => argument(0),
        NullabilityRule::SafeCast => match argument(0) {
            Nullability::Nullable => Nullability::Nullable,
            _ => Nullability::Unknown,
        },
        NullabilityRule::ConditionalResult => match (argument(1), argument(2)) {
            _ if parts.len() < CONDITIONAL_ARGUMENTS => Nullability::Unknown,
            (Nullability::NonNull, Nullability::NonNull) => Nullability::NonNull,
            (Nullability::Nullable, _) | (_, Nullability::Nullable) => Nullability::Nullable,
            _ => Nullability::Unknown,
        },
    }
}

/// What SQLQuack knows of a function by name, whatever the catalog says.
fn built_in_function_type(
    name: &str,
    parts: &[(&Expression, Lineage)],
    context: LineageContext<'_>,
) -> ValueType {
    if COUNTING_FUNCTIONS
        .iter()
        .any(|function| function.eq_ignore_ascii_case(name))
    {
        let count = name.eq_ignore_ascii_case("COUNT");
        let data_type: Option<Rc<str>> = count.then(|| Rc::from(COUNT_TYPE));
        let nullability = match count {
            true => Nullability::NonNull,
            false => Nullability::Unknown,
        };
        return (data_type, nullability);
    }
    if let Some((_, data_type)) = FUNCTION_RESULT_TYPES
        .iter()
        .find(|(function, _)| function.eq_ignore_ascii_case(name))
    {
        return (Some(Rc::from(*data_type)), Nullability::Unknown);
    }
    if let Some((_, position)) = ARGUMENT_TYPED_FUNCTIONS
        .iter()
        .find(|(function, _)| function.eq_ignore_ascii_case(name))
    {
        let data_type: Option<Rc<str>> = parts
            .get(*position)
            .and_then(|(_, part)| part.data_type.clone());
        return (data_type, Nullability::Unknown);
    }
    if let Some((_, first)) = OPERAND_TYPED_FUNCTIONS
        .iter()
        .find(|(function, _)| function.eq_ignore_ascii_case(name))
    {
        let operands: Vec<&Expression> =
            parts.iter().skip(*first).map(|(child, _)| *child).collect();
        let nullability: Nullability = match COALESCING_FUNCTIONS
            .iter()
            .any(|function| function.eq_ignore_ascii_case(name))
        {
            true => coalesced(parts),
            false => Nullability::Unknown,
        };
        return (common_type(parts, context, &operands), nullability);
    }
    (None, Nullability::Unknown)
}

/// A COALESCE's nullability: never NULL when an operand never is, NULL when all may be.
fn coalesced(parts: &[(&Expression, Lineage)]) -> Nullability {
    if parts
        .iter()
        .any(|(_, part)| part.nullability == Nullability::NonNull)
    {
        return Nullability::NonNull;
    }
    match !parts.is_empty()
        && parts
            .iter()
            .all(|(_, part)| part.nullability == Nullability::Nullable)
    {
        true => Nullability::Nullable,
        false => Nullability::Unknown,
    }
}

/// A float operand's type when arithmetic has one: the rest is computed in floating point.
fn float_arithmetic(parts: &[(&Expression, Lineage)]) -> Option<Rc<str>> {
    parts
        .iter()
        .filter_map(|(_, part)| part.data_type.clone())
        .find(|data_type| FLOAT_TYPES.contains(&canonical_type(data_type).as_str()))
}

/// The one integer type of both operands, which adding, subtracting or multiplying keeps.
fn integer_arithmetic(
    parts: &[(&Expression, Lineage)],
    context: LineageContext<'_>,
) -> Option<Rc<str>> {
    let [(_, left), (_, right)] = parts else {
        return None;
    };
    let (left, right) = (left.data_type.clone()?, right.data_type.clone()?);
    (INTEGER_TYPES.contains(&canonical_type(&left).as_str())
        && context.profile.same_type(&left, &right))
    .then_some(left)
}

/// The first operand's type.
fn first_type(parts: &[(&Expression, Lineage)]) -> Option<Rc<str>> {
    parts.first().and_then(|(_, part)| part.data_type.clone())
}

/// The one type every typed operand among `operands` has, NULL operands aside.
fn common_type(
    parts: &[(&Expression, Lineage)],
    context: LineageContext<'_>,
    operands: &[&Expression],
) -> Option<Rc<str>> {
    let mut found: Option<Rc<str>> = None;
    let mut typed: Vec<&Lineage> = Vec::new();
    let mut differ: bool = false;
    for (child, part) in parts {
        if !operands
            .iter()
            .any(|operand| std::ptr::eq(*operand, *child))
        {
            continue;
        }
        if matches!(child, Expression::Null(_)) {
            continue;
        }
        let data_type: Rc<str> = part.data_type.clone()?;
        typed.push(part);
        match &found {
            Some(existing) if !context.profile.same_type(existing, &data_type) => differ = true,
            Some(_) => {}
            None => found = Some(data_type),
        }
    }
    match (differ, context.profile.name()) {
        (false, _) => found,
        (true, SNOWFLAKE_PROFILE) => snowflake_unified(&typed),
        (true, _) => None,
    }
}

/// The columns a level's WHERE requires `IS NOT NULL`, unless it aggregates without grouping.
pub(crate) fn filtered_non_null(select: &Select) -> Vec<&Column> {
    let aggregates_all: bool =
        select.group_by.is_none() && select.expressions.iter().any(holds_aggregate);
    match &select.where_clause {
        Some(filter) if !aggregates_all => non_null_conjuncts(&filter.this),
        _ => Vec::new(),
    }
}

/// Whether `column` is one of the columns the level requires `IS NOT NULL`.
pub(crate) fn required_non_null(
    column: &Column,
    required: &[&Column],
    context: LineageContext<'_>,
) -> bool {
    required
        .iter()
        .any(|filtered| same_column(column, filtered, context))
}

/// The columns `IS NOT NULL` conjuncts of `filter` test.
fn non_null_conjuncts(filter: &Expression) -> Vec<&Column> {
    match filter {
        Expression::Paren(paren) => non_null_conjuncts(&paren.this),
        Expression::And(and) => {
            let mut columns: Vec<&Column> = non_null_conjuncts(&and.left);
            columns.extend(non_null_conjuncts(&and.right));
            columns
        }
        Expression::IsNull(test) if test.not => bare_column(&test.this).into_iter().collect(),
        Expression::Not(not) => match unparenthesized(&not.this) {
            Expression::IsNull(test) if !test.not => bare_column(&test.this).into_iter().collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn bare_column(expression: &Expression) -> Option<&Column> {
    match unparenthesized(expression) {
        Expression::Column(column) => Some(column),
        _ => None,
    }
}

fn unparenthesized(mut expression: &Expression) -> &Expression {
    while let Expression::Paren(paren) = expression {
        expression = &paren.this;
    }
    expression
}

/// Whether two column references name the same column, qualified alike.
fn same_column(left: &Column, right: &Column, context: LineageContext<'_>) -> bool {
    let same = |a: &Identifier, b: &Identifier| key(a, context) == key(b, context);
    same(&left.name, &right.name)
        && match (&left.table, &right.table) {
            (Some(left), Some(right)) => same(left, right),
            (None, None) => true,
            _ => false,
        }
}
