//! Checks over query output families: IN and row subqueries, USING, recursive CTEs, RANGE frames.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::queries::models::{Select, WindowFrameBound, WindowFrameKind};

use crate::lineage::_helpers::checks::coercion::{
    Engine, comparable, invalid_literal_for, projection_literal, runtime_comparison, setop,
};
use crate::lineage::_helpers::checks::families::{Family, expression_family, type_family};
use crate::lineage::_helpers::checks::issues::issue;
use crate::lineage::_helpers::checks::sets::{Branches, families, width};
use crate::lineage::constants::TYPE_PHASE;
use crate::lineage::models::{LineageContext, LineageScope, NameFinding};

fn finding(code: &'static str, message: String, node: &Expression) -> NameFinding {
    issue((code, TYPE_PHASE), message, None, node)
}

/// The output families of a query read inside `scope`, when its projections show them.
pub(crate) fn query_families(
    query: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Option<Vec<Family>> {
    families(
        query,
        Branches {
            ctes: scope.ctes,
            outer: Some(scope),
        },
        context,
    )
}

/// The query a scalar subquery reads, past aliases and parentheses.
pub(crate) fn scalar_query(expression: &Expression) -> Option<&Expression> {
    match expression {
        Expression::Subquery(subquery) => Some(&subquery.this),
        Expression::Alias(alias) => scalar_query(&alias.this),
        Expression::Paren(paren) => scalar_query(&paren.this),
        _ => None,
    }
}

/// An IN subquery of the wrong width, or of a type its operand cannot compare with.
pub(crate) fn in_subquery_findings(
    node: &Expression,
    (this, query): (&Expression, &Expression),
    (scope, context): (&LineageScope<'_>, LineageContext<'_>),
) -> Vec<NameFinding> {
    let Some(types) = query_families(query, scope, context) else {
        return Vec::new();
    };
    let expected: usize = match this {
        Expression::Tuple(tuple) => tuple.expressions.len(),
        _ => 1,
    };
    if types.len() != expected {
        return vec![finding(
            "E216",
            format!(
                "IN subquery returns {} columns, expected {expected}",
                types.len()
            ),
            node,
        )];
    }
    if expected != 1 {
        return Vec::new();
    }
    let engine: Engine = Engine::of(context);
    let left: Family = expression_family(this, scope, context);
    let right: &Expression = projection_literal(query, 0).unwrap_or(query);
    let runtime: bool = engine == Engine::Snowflake && runtime_comparison(left, types[0]);
    match !comparable(engine, (this, right), (left, types[0])) || runtime {
        true => vec![finding(
            if runtime { "W210" } else { "E217" },
            "IN subquery has incompatible output type".to_owned(),
            node,
        )],
        false => Vec::new(),
    }
}

/// A row value compared with a subquery of another width.
pub(crate) fn row_comparison_findings(
    node: &Expression,
    (left, right): (&Expression, &Expression),
) -> Vec<NameFinding> {
    let mut findings: Vec<NameFinding> = Vec::new();
    for (row, query) in [(left, right), (right, left)] {
        if let (Expression::Tuple(row), Some(query)) = (row, scalar_query(query))
            && width(query).is_some_and(|width| width != row.expressions.len())
        {
            findings.push(finding(
                "E216",
                "Row comparison has different column counts".to_owned(),
                node,
            ));
        }
    }
    findings
}

/// The family of the column `name` a FROM item offers, by its SQLQuack type.
fn offered_family(
    source: &crate::lineage::models::ScopeSource,
    name: &str,
    context: LineageContext<'_>,
) -> Family {
    source
        .query
        .columns
        .iter()
        .find(|column| context.profile.same_identifier(&column.key, name))
        .and_then(|column| column.lineage.data_type.as_deref())
        .map_or(Family::Unknown, type_family)
}

/// USING keys of a single two-item join whose sides cannot compare; Snowflake skips conversions.
pub(crate) fn using_findings(
    node: &Expression,
    select: &Select,
    (scope, context): (&LineageScope<'_>, LineageContext<'_>),
) -> Vec<NameFinding> {
    let single_from: bool = select
        .from
        .as_ref()
        .is_some_and(|from| from.expressions.len() == 1);
    let [join] = select.joins.as_slice() else {
        return Vec::new();
    };
    let [left_source, right_source] = scope.sources.as_slice() else {
        return Vec::new();
    };
    if !single_from || join.using.is_empty() {
        return Vec::new();
    }
    let engine: Engine = Engine::of(context);
    let mut findings: Vec<NameFinding> = Vec::new();
    for column in &join.using {
        let name: String = crate::lineage::_helpers::tracing::resolution::key(column, context);
        let left: Family = offered_family(left_source, &name, context);
        let right: Family = offered_family(right_source, &name, context);
        let runtime: bool = runtime_comparison(left, right);
        if runtime && engine == Engine::Snowflake {
            continue;
        }
        if !comparable(engine, (&join.this, &join.this), (left, right)) || runtime {
            findings.push(finding(
                if runtime { "W210" } else { "E217" },
                format!("JOIN USING column '{}' has incompatible types", column.name),
                node,
            ));
        }
    }
    findings
}

/// A recursive CTE whose recursive branch cannot convert to the anchor's column types.
pub(crate) fn recursive_findings(
    node: &Expression,
    select: &Select,
    (scope, context): (&LineageScope<'_>, LineageContext<'_>),
) -> Vec<NameFinding> {
    let engine: Engine = Engine::of(context);
    let Some(with) = select
        .with
        .as_ref()
        .filter(|with| with.recursive && !matches!(engine, Engine::Postgres | Engine::BigQuery))
    else {
        return Vec::new();
    };
    let mut findings: Vec<NameFinding> = Vec::new();
    for cte in &with.ctes {
        let Expression::Union(union) = &cte.this else {
            continue;
        };
        let (Some(anchor), Some(recursive)) = (
            query_families(&union.left, scope, context),
            query_families(&union.right, scope, context),
        ) else {
            continue;
        };
        for (index, (target, source)) in anchor.into_iter().zip(recursive).enumerate() {
            let literal: Option<&Expression> = projection_literal(&union.right, index);
            let invalid: bool = match engine {
                Engine::Snowflake => !setop_compatible(target, source),
                Engine::Sqlite | Engine::Databricks => false,
                Engine::DuckDB | Engine::Postgres | Engine::BigQuery => {
                    literal.is_some_and(|value| invalid_literal_for(value, target))
                        || (source == Family::String
                            && !matches!(target, Family::String | Family::Unknown)
                            && literal.is_none())
                        || !setop(engine, target, source)
                }
            };
            if invalid {
                findings.push(finding(
                    match engine {
                        Engine::Snowflake => "E215",
                        Engine::DuckDB
                        | Engine::Postgres
                        | Engine::Sqlite
                        | Engine::BigQuery
                        | Engine::Databricks => "W214",
                    },
                    "Recursive branch cannot convert to the anchor column type".to_owned(),
                    node,
                ));
            }
        }
    }
    findings
}

/// Whether set-operation branches of these families combine without any conversion.
fn setop_compatible(left: Family, right: Family) -> bool {
    left == Family::Unknown
        || right == Family::Unknown
        || left == right
        || (left.is_numeric() && right.is_numeric())
        || (left.is_temporal() && right.is_temporal())
}

/// A RANGE frame with an offset, ordered by a value that is neither a number nor a time.
pub(crate) fn range_frame_findings(
    node: &Expression,
    (scope, context): (&LineageScope<'_>, LineageContext<'_>),
) -> Vec<NameFinding> {
    let Expression::WindowFunction(window) = node else {
        return Vec::new();
    };
    let Some(frame) = &window.over.frame else {
        return Vec::new();
    };
    let offset = |bound: &WindowFrameBound| {
        matches!(
            bound,
            WindowFrameBound::Preceding(_) | WindowFrameBound::Following(_)
        )
    };
    if frame.kind != WindowFrameKind::Range
        || !(offset(&frame.start) || frame.end.as_ref().is_some_and(offset))
    {
        return Vec::new();
    }
    window
        .over
        .order_by
        .iter()
        .filter(|ordered| {
            let family: Family = expression_family(&ordered.this, scope, context);
            family != Family::Unknown && !family.is_numeric() && !family.is_temporal()
        })
        .map(|_| {
            finding(
                "E232",
                "RANGE offset requires a numeric or temporal ordering expression".to_owned(),
                node,
            )
        })
        .collect()
}
