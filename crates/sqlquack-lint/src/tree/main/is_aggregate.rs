//! `is_aggregate`: whether a node is an aggregate call, as polyglot classified them.

use crate::lint::models::LintDialect;
use sqlquack_syntax::Expression;

use crate::tree::constants::{
    POSTGRES_AGGREGATE_CALLS, POSTGRES_FOREIGN_AGGREGATE_CALLS,
    POSTGRES_SINGLE_ARGUMENT_AGGREGATES, WINDOW_ONLY_CALLS,
};
use crate::tree::main::call_name::call_name;

/// Whether `expression` is an aggregate call; window-only calls and scalar helpers are not.
pub(crate) fn is_aggregate(expression: &Expression, dialect: LintDialect) -> bool {
    match expression {
        Expression::Function(_) | Expression::AggregateFunction(_)
            if dialect == LintDialect::PostgreSQL =>
        {
            call_name(expression)
                .is_some_and(|name| is_postgres_aggregate(name, call_arguments(expression)))
        }
        Expression::AggregateFunction(function) => !WINDOW_ONLY_CALLS
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&function.name)),
        _ => matches!(
            expression,
            Expression::Count(_)
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
                | Expression::ArrayConcatAgg(_)
                | Expression::ArrayUniqueAgg(_)
                | Expression::BoolXorAgg(_)
                | Expression::BitwiseAndAgg(_)
                | Expression::BitwiseOrAgg(_)
                | Expression::BitwiseXorAgg(_)
                | Expression::JsonArrayAgg(_)
                | Expression::JsonObjectAgg(_)
                | Expression::JSONArrayAgg(_)
                | Expression::JSONObjectAgg(_)
                | Expression::JSONBObjectAgg(_)
                | Expression::ParameterizedAgg(_)
                | Expression::ArgMax(_)
                | Expression::ArgMin(_)
                | Expression::ApproxTopK(_)
                | Expression::ApproxTopKAccumulate(_)
                | Expression::ApproxTopKCombine(_)
                | Expression::ApproxTopSum(_)
                | Expression::ApproxQuantiles(_)
                | Expression::Grouping(_)
                | Expression::GroupingId(_)
                | Expression::AnonymousAggFunc(_)
                | Expression::CombinedAggFunc(_)
                | Expression::CombinedParameterizedAgg(_)
                | Expression::HashAgg(_)
                | Expression::Hll(_)
                | Expression::Minhash(_)
                | Expression::ObjectAgg(_)
                | Expression::AIAgg(_)
                | Expression::Quantile(_)
                | Expression::ApproxQuantile(_)
                | Expression::Corr(_)
                | Expression::CovarPop(_)
                | Expression::CovarSamp(_)
                | Expression::RegrValx(_)
                | Expression::RegrValy(_)
                | Expression::RegrAvgx(_)
                | Expression::RegrAvgy(_)
                | Expression::RegrCount(_)
                | Expression::RegrIntercept(_)
                | Expression::RegrR2(_)
                | Expression::RegrSxx(_)
                | Expression::RegrSxy(_)
                | Expression::RegrSyy(_)
                | Expression::RegrSlope(_)
        ),
    }
}

/// Whether a call of `name` with `arguments` arguments is an aggregate, as polyglot read PostgreSQL.
fn is_postgres_aggregate(name: &str, arguments: usize) -> bool {
    let named = |names: &[&str]| names.iter().any(|known| known.eq_ignore_ascii_case(name));
    (named(&POSTGRES_AGGREGATE_CALLS) || named(&POSTGRES_FOREIGN_AGGREGATE_CALLS))
        && (arguments <= 1 || !named(&POSTGRES_SINGLE_ARGUMENT_AGGREGATES))
}

/// How many arguments a call passes.
fn call_arguments(expression: &Expression) -> usize {
    match expression {
        Expression::Function(function) => function.args.len(),
        Expression::AggregateFunction(function) => function.args.len(),
        _ => 0,
    }
}
