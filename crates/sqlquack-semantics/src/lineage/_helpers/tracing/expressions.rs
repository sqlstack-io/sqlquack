//! The lineage of a value expression in a query level.

use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::names::models::{Identifier, Star};
use sqlquack_syntax::expressions::operators::models::Parameter;
use sqlquack_syntax::{DataType, Expression};

use crate::dialects::main::canonical_type::canonical_type;
use crate::dialects::main::dialect_rules::dialect_rules;
use crate::lineage::_helpers::tracing::lineages::{combined, constant, traced_if, unresolved};
use crate::lineage::_helpers::tracing::queries::{empty_scope, expanded_star, resolved_query};
use crate::lineage::_helpers::tracing::resolution::{column_lineage, key, positional_column};
use crate::lineage::_helpers::tracing::reuse::{alias_read, alias_reads, worked};
use crate::lineage::_helpers::typing::rules::{
    computed_type, literal_type, mapped_timestamp, required_non_null,
};
use crate::lineage::constants::{
    AGGREGATE_FUNCTIONS, BOOLEAN_TYPE, POSITIONAL_COLUMN_BASE, SNOWFLAKE_JSON_HOLDERS,
    SNOWFLAKE_TO_VARIANT, SNOWFLAKE_VARIANT_TYPE,
};
use crate::lineage::main::rendered_type::rendered_type;
use crate::lineage::models::{
    Lineage, LineageContext, LineageScope, Nullability, ResolvedQuery, RowReads, TransformKind,
};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;

/// The lineage of `expression` read in `scope`.
pub(crate) fn expression_lineage(
    expression: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Lineage {
    let node: usize = std::ptr::from_ref(expression) as usize;
    if let Some((traced, by_alias)) = scope.traced.borrow().get(&node) {
        if *by_alias {
            alias_read();
        }
        return traced.clone();
    }
    let shared =
        |traces: &Rc<RefCell<FxHashMap<usize, Lineage>>>| traces.borrow().get(&node).cloned();
    if let Some(found) = scope.shared.as_ref().and_then(shared) {
        scope
            .traced
            .borrow_mut()
            .insert(node, (found.clone(), false));
        return found;
    }
    worked();
    let (lineage, by_alias): (Lineage, bool) = alias_reads(|| {
        crate::lineage::_helpers::tracing::stack::deeper(|| {
            expression_lineage_level(expression, scope, context)
        })
    });
    if let (Some(traces), false) = (&scope.shared, by_alias) {
        traces.borrow_mut().insert(node, lineage.clone());
    }
    scope
        .traced
        .borrow_mut()
        .insert(node, (lineage.clone(), by_alias));
    lineage
}

fn expression_lineage_level(
    expression: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Lineage {
    match expression {
        Expression::Column(column) => {
            let read: Lineage = column_lineage(column.table.as_ref(), &column.name, scope, context);
            match required_non_null(column, &scope.non_null, context) {
                true => Lineage {
                    nullability: Nullability::NonNull,
                    ..read
                },
                false => read,
            }
        }
        Expression::Alias(alias) => expression_lineage(&alias.this, scope, context),
        Expression::Paren(paren) => expression_lineage(&paren.this, scope, context),
        Expression::Annotated(annotated) => {
            let inner: Lineage = expression_lineage(&annotated.this, scope, context);
            if inner.data_type.is_some() {
                return inner;
            }
            let asserted: Option<Rc<str>> = match (&annotated.type_assertion, &annotated.type_like)
            {
                (Some(asserted), _) => rendered_type(asserted).map(Rc::from),
                (None, Some(like)) => expression_lineage(like, scope, context).data_type,
                (None, None) => None,
            };
            Lineage {
                data_type: asserted,
                ..inner
            }
        }
        Expression::Star(star) => star_lineage(star, scope, context),
        Expression::Parameter(parameter) => positional_lineage(parameter, scope),
        Expression::ConnectByRoot(root) => match &root.this {
            Expression::Var(var) => {
                column_lineage(None, &Identifier::new(var.this.clone()), scope, context)
            }
            Expression::Dot(dot) => match &dot.this {
                Expression::Var(table) => column_lineage(
                    Some(&Identifier::new(table.this.clone())),
                    &dot.field,
                    scope,
                    context,
                ),
                other => expression_lineage(other, scope, context),
            },
            other => expression_lineage(other, scope, context),
        },
        Expression::Cast(cast) | Expression::TryCast(cast) | Expression::SafeCast(cast) => {
            let inner: Lineage = expression_lineage(&cast.this, scope, context);
            let target: Option<String> = dialect_rules(context.profile).cast_spelling(&cast.to);
            let target: Option<Rc<str>> =
                target.map(|target| mapped_timestamp(Rc::from(target), context));
            let nullability: Nullability = match expression {
                Expression::Cast(_) if is_text_type(&cast.to) && inner.json_null_free => {
                    inner.nullability
                }
                Expression::Cast(_) => cast_nullability(&inner, target.as_deref(), context),
                _ => Nullability::Unknown,
            };
            typed(
                combined(vec![inner], TransformKind::Cast),
                (target, nullability),
            )
        }
        Expression::Literal(_) | Expression::Null(_) | Expression::Boolean(_) => {
            typed(constant(), literal_type(expression, context))
        }
        Expression::Identifier(_) | Expression::Var(_) => constant(),
        Expression::Exists(exists) => {
            let query: ResolvedQuery =
                resolved_query(&exists.this, scope.ctes, Some(scope), context);
            let reads: RowReads = query_reads(query);
            Lineage {
                rows: Rc::new(reads.columns),
                rows_lost: reads.lost,
                resolved: false,
                data_type: Some(Rc::from(BOOLEAN_TYPE)),
                ..constant()
            }
        }
        Expression::Subquery(subquery) => query_value(&subquery.this, scope, context),
        Expression::Select(_)
        | Expression::Union(_)
        | Expression::Intersect(_)
        | Expression::Except(_) => query_value(expression, scope, context),
        Expression::Lambda(lambda) => {
            let names: Vec<String> = lambda
                .parameters
                .iter()
                .map(|parameter| key(parameter, context))
                .collect();
            let inner = LineageScope {
                lambda_names: names,
                ..empty_scope(scope.ctes, Some(scope))
            };
            expression_lineage(&lambda.body, &inner, context)
        }
        Expression::WindowFunction(window) => {
            let mut parts: Vec<Lineage> = vec![children_lineage(
                expression,
                scope,
                context,
                TransformKind::Expression,
            )];
            if let Some(name) = &window.over.window_name {
                let name: String = key(name, context);
                let named: Option<&Lineage> = scope
                    .windows
                    .iter()
                    .find(|(window, _)| context.profile.same_identifier(window, &name))
                    .map(|(_, lineage)| lineage);
                parts.push(named.cloned().unwrap_or_else(unresolved));
            }
            let floor: TransformKind = parts[0].kind.max(TransformKind::Expression);
            let value_type: ValueType = (parts[0].data_type.clone(), parts[0].nullability);
            typed(combined(parts, floor), value_type)
        }
        _ if is_aggregate(expression) => {
            children_lineage(expression, scope, context, TransformKind::Aggregation)
        }
        _ => children_lineage(expression, scope, context, TransformKind::Expression),
    }
}

fn children_lineage(
    expression: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
    floor: TransformKind,
) -> Lineage {
    let parts: Vec<(&Expression, Lineage)> = expression
        .children()
        .into_iter()
        .map(|child| match (child, floor) {
            (Expression::Star(_), TransformKind::Aggregation) => (child, constant()),
            _ if date_part_argument(expression, child, context) => (child, constant()),
            _ => (child, expression_lineage(child, scope, context)),
        })
        .collect();
    let (data_type, nullability): ValueType = computed_type(expression, &parts, context);
    let json_null_free: bool = json_null_free_variant(expression, &parts, context);
    let nullability: Nullability = match (json_null_free, parts.as_slice()) {
        (true, [(_, argument)]) => argument.nullability,
        _ => nullability,
    };
    let lineage: Lineage = combined(parts.into_iter().map(|(_, part)| part).collect(), floor);
    Lineage {
        json_null_free,
        ..typed(lineage, (data_type, nullability))
    }
}

/// Whether `child` is the date part keyword a function like DATEADD or LAST_DAY takes.
pub(crate) fn date_part_argument(
    function: &Expression,
    child: &Expression,
    context: LineageContext<'_>,
) -> bool {
    let Expression::Function(function) = function else {
        return false;
    };
    let rules: &dyn DialectRules = dialect_rules(context.profile);
    let functions: &[(&str, usize)] = rules.date_part_functions();
    let part: bool = match child {
        Expression::Column(column) => column.table.is_none() && !column.name.quoted,
        Expression::Function(weekday) => {
            rules
                .weekday_date_parts()
                .iter()
                .any(|name| weekday.name.eq_ignore_ascii_case(name))
                && matches!(weekday.args.as_slice(), [Expression::Column(day)] if day.table.is_none() && !day.name.quoted)
        }
        _ => false,
    };
    part && functions.iter().any(|(name, position)| {
        function.name.eq_ignore_ascii_case(name)
            && function
                .args
                .get(*position)
                .is_some_and(|argument| std::ptr::eq(argument, child))
    })
}

/// `lineage` with the value's type and nullability.
fn typed(lineage: Lineage, (data_type, nullability): ValueType) -> Lineage {
    Lineage {
        data_type,
        nullability,
        ..lineage
    }
}

/// `$n`: the n-th column of the level's one FROM item, untraced when that is not known.
fn positional_lineage(parameter: &Parameter, scope: &LineageScope<'_>) -> Lineage {
    let (Some(index), [only]) = (parameter.index, scope.sources.as_slice()) else {
        return unresolved();
    };
    let position: Option<usize> = match usize::try_from(index) {
        Ok(index) => index.checked_sub(POSITIONAL_COLUMN_BASE),
        Err(_) => None,
    };
    position.map_or_else(unresolved, |position| positional_column(only, position))
}

/// A query used as a value: what its outputs read, its filters as row dependencies.
fn query_value(
    query: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Lineage {
    let resolved = resolved_query(query, scope.ctes, Some(scope), context);
    let open: bool = resolved.open;
    let rows: RowReads = resolved.rows.clone();
    let single_type: Option<Rc<str>> = match resolved.columns.as_slice() {
        [column] => column.lineage.data_type.clone(),
        _ => None,
    };
    let value: Lineage = combined(
        Rc::unwrap_or_clone(resolved.columns)
            .into_iter()
            .map(|column| column.lineage)
            .collect(),
        TransformKind::Expression,
    );
    let mut value: Lineage = traced_if(value, !open);
    value.data_type = single_type;
    Rc::make_mut(&mut value.rows).extend(rows.columns);
    value.rows_lost |= rows.lost;
    value
}

/// Every column a query reads, values and row dependencies together.
fn query_reads(query: ResolvedQuery) -> RowReads {
    let mut read: RowReads = query.rows;
    for column in query.columns.iter() {
        read.read(&column.lineage);
    }
    read
}

/// Whether `data_type` is a text type, the cast of a TO_VARIANT that keeps its value's text.
fn is_text_type(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Char { .. }
            | DataType::VarChar { .. }
            | DataType::String { .. }
            | DataType::Text
            | DataType::TextWithLength { .. }
    )
}

/// Whether `expression` is Snowflake's TO_VARIANT of a value whose type no JSON null can hold.
fn json_null_free_variant(
    expression: &Expression,
    parts: &[(&Expression, Lineage)],
    context: LineageContext<'_>,
) -> bool {
    let (Expression::Function(function), [(_, argument)]) = (expression, parts) else {
        return false;
    };
    dialect_rules(context.profile).variants_hold_json_null()
        && !function.quoted
        && function.name.eq_ignore_ascii_case(SNOWFLAKE_TO_VARIANT)
        && argument.data_type.as_deref().is_some_and(|spelling| {
            !SNOWFLAKE_JSON_HOLDERS.contains(&canonical_type(spelling).as_str())
        })
}

/// A cast keeps its value's nullability, unless a Snowflake JSON null may become SQL NULL.
fn cast_nullability(
    inner: &Lineage,
    target: Option<&str>,
    context: LineageContext<'_>,
) -> Nullability {
    let semi_structured = |spelling: Option<&str>| {
        spelling.is_none_or(|spelling| {
            SNOWFLAKE_JSON_HOLDERS.contains(&canonical_type(spelling).as_str())
        })
    };
    match dialect_rules(context.profile).variants_hold_json_null() {
        true if inner.nullability != Nullability::Nullable
            && semi_structured(inner.data_type.as_deref())
            && !target.is_some_and(|target| canonical_type(target) == SNOWFLAKE_VARIANT_TYPE) =>
        {
            match inner.data_type {
                Some(_) => Nullability::Nullable,
                None => Nullability::Unknown,
            }
        }
        _ => inner.nullability,
    }
}

pub(crate) fn is_aggregate(expression: &Expression) -> bool {
    match expression {
        Expression::AggregateFunction(_)
        | Expression::Count(_)
        | Expression::Sum(_)
        | Expression::Avg(_)
        | Expression::Min(_)
        | Expression::Max(_)
        | Expression::GroupConcat(_)
        | Expression::StringAgg(_)
        | Expression::ListAgg(_)
        | Expression::ArrayAgg(_)
        | Expression::CountIf(_)
        | Expression::SumIf(_)
        | Expression::Stddev(_)
        | Expression::StddevPop(_)
        | Expression::StddevSamp(_)
        | Expression::Variance(_)
        | Expression::VarPop(_)
        | Expression::VarSamp(_)
        | Expression::Median(_)
        | Expression::Mode(_)
        | Expression::First(_)
        | Expression::Last(_)
        | Expression::AnyValue(_)
        | Expression::ApproxDistinct(_)
        | Expression::ApproxCountDistinct(_)
        | Expression::ApproxPercentile(_)
        | Expression::Percentile(_)
        | Expression::PercentileCont(_)
        | Expression::PercentileDisc(_)
        | Expression::LogicalAnd(_)
        | Expression::LogicalOr(_)
        | Expression::Skewness(_)
        | Expression::BoolXorAgg(_)
        | Expression::BitwiseAndAgg(_)
        | Expression::BitwiseOrAgg(_)
        | Expression::BitwiseXorAgg(_)
        | Expression::ArgMax(_)
        | Expression::ArgMin(_) => true,
        Expression::Function(function) => AGGREGATE_FUNCTIONS
            .iter()
            .any(|name| function.name.eq_ignore_ascii_case(name)),
        _ => false,
    }
}

/// A star read as a value, as in `OBJECT_CONSTRUCT(*)`: everything its columns read.
fn star_lineage(star: &Star, scope: &LineageScope<'_>, context: LineageContext<'_>) -> Lineage {
    if star.table.is_none() && scope.sources.is_empty() {
        return constant();
    }
    let expanded = expanded_star(star, scope, context);
    let open: bool = expanded.open;
    let read = combined(
        Rc::unwrap_or_clone(expanded.columns)
            .into_iter()
            .map(|column| column.lineage)
            .collect(),
        TransformKind::Expression,
    );
    traced_if(read, !open)
}
