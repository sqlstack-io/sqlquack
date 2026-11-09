//! Data models of functions dispatch.

/// Parser dispatch behavior for phase-4 registry-driven typed-function handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserDispatchBehavior {
    ExprListFunction,
    OptionalExprListFunction,
    FunctionArgumentsFunction,
    ZeroArgFunction,
    ExprListMaybeAggregateByFilter,
    ExprListMaybeAggregateByAggSuffix,
    HashLike,
    HllAggregate,
    PercentileAggregate,
    ExprListAggregate,
    UnaryAggregate,
    TranslateNonTeradata,
}

/// Parser dispatch metadata entry keyed by uppercased function name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParserDispatchSpec {
    pub name: &'static str,
    pub behavior: ParserDispatchBehavior,
}
