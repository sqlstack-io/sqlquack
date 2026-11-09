//! Set-operation branches that cannot combine: DuckDB's families, Snowflake's directional binder.

use sqlquack_syntax::Expression;

use crate::dialects::main::dialect_rules::dialect_rules;
use crate::lineage::_helpers::checks::coercion::{literal_coerces, projection_literal, setop};
use crate::lineage::_helpers::checks::families::expression_family;
use crate::lineage::_helpers::checks::issues::{issue, type_location};
use crate::lineage::_helpers::tracing::expressions::expression_lineage;
use crate::lineage::_helpers::tracing::queries::{empty_scope, level_scope, unannotated};
use crate::lineage::constants::TYPE_PHASE;
use crate::lineage::main::type_family::type_family;
use crate::lineage::models::Engine;
use crate::lineage::models::Family;
use crate::lineage::models::{LineageContext, LineageScope, NameFinding, NamedQuery};

/// Snowflake's binder kinds, which decide how set-operation branches combine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Unknown,
    Null,
    Boolean,
    Number,
    String,
    Date,
    TimestampLtz,
    TimestampNtz,
    TimestampTz,
    Time,
    Variant,
    Array,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Clean,
    Runtime,
    Rejected,
}

impl Kind {
    fn temporal(self) -> bool {
        matches!(
            self,
            Self::Date | Self::TimestampLtz | Self::TimestampNtz | Self::TimestampTz | Self::Time
        )
    }

    fn of_family(family: Family) -> Self {
        match family {
            Family::Boolean => Self::Boolean,
            Family::Integer | Family::Numeric => Self::Number,
            Family::String => Self::String,
            Family::Date => Self::Date,
            Family::Timestamp => Self::TimestampNtz,
            Family::Time => Self::Time,
            Family::Json => Self::Variant,
            Family::Array => Self::Array,
            _ => Self::Unknown,
        }
    }

    fn of_spelling(spelling: &str) -> Self {
        let base: String = spelling
            .split('(')
            .next()
            .unwrap_or(spelling)
            .trim()
            .replace('_', "")
            .to_ascii_uppercase();
        match base.as_str() {
            "TIMESTAMPNTZ" => Self::TimestampNtz,
            "TIMESTAMPTZ" => Self::TimestampTz,
            "TIMESTAMPLTZ" => Self::TimestampLtz,
            _ => Self::of_family(type_family(spelling)),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Null => "NULL",
            Self::Boolean => "BOOLEAN",
            Self::Number => "NUMBER",
            Self::String => "VARCHAR",
            Self::Date => "DATE",
            Self::TimestampLtz => "TIMESTAMP_LTZ",
            Self::TimestampNtz => "TIMESTAMP_NTZ",
            Self::TimestampTz => "TIMESTAMP_TZ",
            Self::Time => "TIME",
            Self::Variant => "VARIANT",
            Self::Array => "ARRAY",
        }
    }

    /// The kind a later branch's `later` leaves after this accumulated one.
    fn fold(self, later: Self) -> Self {
        use Kind::{
            Date, Null, Number, String, TimestampLtz, TimestampNtz, TimestampTz, Unknown, Variant,
        };
        if self == Unknown || later == Unknown || verdict(self, later) == Verdict::Rejected {
            return Unknown;
        }
        match (self, later) {
            (Null, _) => later,
            (_, Null) | (Number, Number | String) => self,
            (String, Number) | (Number, Variant) => later,
            (String, kind) if kind.temporal() => later,
            (Date, TimestampLtz | TimestampNtz | TimestampTz) => later,
            _ => self,
        }
    }
}

/// How Snowflake binds an accumulated kind (`first`) with a later branch's.
fn verdict(first: Kind, later: Kind) -> Verdict {
    use Kind::{
        Array, Boolean, Date, Null, Number, String, Time, TimestampLtz, TimestampNtz, TimestampTz,
        Unknown, Variant,
    };
    use Verdict::{Clean, Rejected, Runtime};
    if matches!(first, Unknown | Null) || matches!(later, Unknown | Null) || first == later {
        return Clean;
    }
    match (first, later) {
        (Boolean, Number)
        | (String, Boolean | Variant)
        | (Number | TimestampLtz | TimestampNtz | TimestampTz, Variant)
        | (Variant, Boolean | Number | Array)
        | (Array, Variant)
        | (Date, TimestampLtz | TimestampNtz | TimestampTz)
        | (TimestampLtz | TimestampNtz | TimestampTz, Date)
        | (TimestampTz, TimestampNtz)
        | (TimestampLtz, TimestampNtz | TimestampTz)
        | (TimestampNtz | TimestampTz, TimestampLtz) => Clean,
        (Boolean | Number, String) | (String, Number) | (Boolean | Date | Time, Variant) => Runtime,
        (String, value) if value.temporal() => Runtime,
        (value, String) if value.temporal() => Runtime,
        _ => Rejected,
    }
}

/// Where branches are read: the CTEs in reach and the enclosing level.
#[derive(Clone, Copy)]
pub(crate) struct Branches<'s, 'a> {
    pub(crate) ctes: &'s [NamedQuery],
    pub(crate) outer: Option<&'s LineageScope<'a>>,
}

/// A set operation's operator and operands; BY NAME pairs columns by name, so it has none here.
fn operands(query: &Expression) -> Option<(&'static str, &Expression, &Expression)> {
    match query {
        Expression::Union(set) if !set.by_name => Some(("UNION", &set.left, &set.right)),
        Expression::Intersect(set) if !set.by_name => Some(("INTERSECT", &set.left, &set.right)),
        Expression::Except(set) if !set.by_name => Some(("EXCEPT", &set.left, &set.right)),
        _ => None,
    }
}

/// The findings of set operation `query` whose branches do not combine.
pub(crate) fn set_findings(
    query: &Expression,
    branches: Branches<'_, '_>,
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let Some((operator, left, right)) = operands(query) else {
        return Vec::new();
    };
    let counts_only: bool = dialect_rules(context.profile).checks_set_widths_only();
    let findings: Vec<NameFinding> = match Engine::of(context) {
        Engine::Snowflake if !counts_only => snowflake_findings(query, branches, context),
        Engine::Sqlite | Engine::Snowflake => {
            width_findings((operator, query), (left, right), branches, context)
                .into_iter()
                .collect()
        }
        Engine::Databricks => Vec::new(),
        engine @ (Engine::DuckDB | Engine::Postgres | Engine::BigQuery) => {
            binder_findings((engine, query, operator), (left, right), branches, context)
        }
    };
    let span = type_location(query);
    findings
        .into_iter()
        .map(|finding| NameFinding { span, ..finding })
        .collect()
}

/// The finding of branches whose column counts differ, which every engine refuses.
fn width_finding(
    operator: &str,
    (left, right): (&[Family], &[Family]),
    query: &Expression,
) -> Option<NameFinding> {
    (left.len() != right.len()).then(|| {
        issue(
            ("E216", TYPE_PHASE),
            format!(
                "{operator} operands return different column counts: left {}, right {}",
                left.len(),
                right.len()
            ),
            None,
            query,
        )
    })
}

/// SQLite's findings: column counts only, as its coercions are not recorded.
fn width_findings(
    (operator, query): (&str, &Expression),
    (left, right): (&Expression, &Expression),
    branches: Branches<'_, '_>,
    context: LineageContext<'_>,
) -> Option<NameFinding> {
    let left_families: Vec<Family> = families(left, branches, context)?;
    let right_families: Vec<Family> = families(right, branches, context)?;
    width_finding(operator, (&left_families, &right_families), query)
}

/// DuckDB's and PostgreSQL's findings: column counts, then each column's families.
fn binder_findings(
    (engine, query, operator): (Engine, &Expression, &str),
    (left, right): (&Expression, &Expression),
    branches: Branches<'_, '_>,
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let (Some(left_families), Some(right_families)) = (
        families(left, branches, context),
        families(right, branches, context),
    ) else {
        return Vec::new();
    };
    if let Some(finding) = width_finding(operator, (&left_families, &right_families), query) {
        return vec![finding];
    }
    let literal = |branch: &Expression, index: usize, target: Family| {
        projection_literal(branch, index)
            .is_some_and(|value| literal_coerces(engine, value, target))
    };
    left_families
        .into_iter()
        .zip(right_families)
        .enumerate()
        .filter(|(index, (left_family, right_family))| {
            !setop(engine, *left_family, *right_family)
                && !literal(left, *index, *right_family)
                && !literal(right, *index, *left_family)
        })
        .map(|(index, (left_family, right_family))| {
            issue(
                (
                    match engine {
                        Engine::DuckDB => "W214",
                        _ => "E215",
                    },
                    TYPE_PHASE,
                ),
                format!(
                    "{operator} column {} has incompatible types: {} vs {}",
                    index + 1,
                    left_family.name(),
                    right_family.name()
                ),
                None,
                query,
            )
        })
        .collect()
}

/// A branch's output families: a SELECT's projections, a VALUES row's, or a nested set's.
pub(crate) fn families(
    query: &Expression,
    branches: Branches<'_, '_>,
    context: LineageContext<'_>,
) -> Option<Vec<Family>> {
    match unannotated(query) {
        Expression::Select(select) => {
            if select
                .expressions
                .iter()
                .any(|projection| matches!(unannotated(projection), Expression::Star(_)))
            {
                return None;
            }
            let scope: LineageScope<'_> =
                level_scope(select, branches.ctes, branches.outer, context);
            Some(
                select
                    .expressions
                    .iter()
                    .map(|projection| expression_family(projection, &scope, context))
                    .collect(),
            )
        }
        Expression::Subquery(subquery) => families(&subquery.this, branches, context),
        Expression::Paren(paren) => families(&paren.this, branches, context),
        _ => None,
    }
}

/// How many families `families` gives `query`, without typing a projection.
pub(crate) fn width(query: &Expression) -> Option<usize> {
    match unannotated(query) {
        Expression::Select(select) => (!select
            .expressions
            .iter()
            .any(|projection| matches!(unannotated(projection), Expression::Star(_))))
        .then_some(select.expressions.len()),
        Expression::Subquery(subquery) => width(&subquery.this),
        Expression::Paren(paren) => width(&paren.this),
        _ => None,
    }
}

fn snowflake_findings(
    query: &Expression,
    branches: Branches<'_, '_>,
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let Some((_, left, right)) = operands(query) else {
        return Vec::new();
    };
    let (Some(first), Some(later)) = (
        kinds(left, branches, context),
        kinds(right, branches, context),
    ) else {
        return Vec::new();
    };
    if first.len() != later.len() {
        return vec![issue(
            ("E216", TYPE_PHASE),
            format!(
                "Set-operation operands return different column counts: left {}, right {}",
                first.len(),
                later.len()
            ),
            None,
            query,
        )];
    }
    first
        .into_iter()
        .zip(later)
        .enumerate()
        .filter_map(|(index, (first, later))| {
            let verdict: Verdict = verdict(first, later);
            let (code, words) = match verdict {
                Verdict::Clean => return None,
                Verdict::Rejected => ("E215", "has incompatible types"),
                Verdict::Runtime => ("W214", "may fail during runtime conversion"),
            };
            Some(issue(
                (code, TYPE_PHASE),
                format!(
                    "Set-operation column {} {words}: accumulated type {}, next type {}",
                    index + 1,
                    first.name(),
                    later.name()
                ),
                None,
                query,
            ))
        })
        .collect()
}

/// A branch's output kinds: a SELECT's projections, a VALUES row's, or a nested set's fold.
fn kinds(
    query: &Expression,
    branches: Branches<'_, '_>,
    context: LineageContext<'_>,
) -> Option<Vec<Kind>> {
    if let Some((_, left, right)) = operands(query) {
        let (left, right) = (
            kinds(left, branches, context)?,
            kinds(right, branches, context)?,
        );
        return (left.len() == right.len()).then(|| {
            left.into_iter()
                .zip(right)
                .map(|(first, later)| first.fold(later))
                .collect()
        });
    }
    match unannotated(query) {
        Expression::Select(select) => {
            if select
                .expressions
                .iter()
                .any(|projection| matches!(projection, Expression::Star(_)))
            {
                return None;
            }
            let scope: LineageScope<'_> =
                level_scope(select, branches.ctes, branches.outer, context);
            Some(
                select
                    .expressions
                    .iter()
                    .map(|projection| kind(projection, &scope, context))
                    .collect(),
            )
        }
        Expression::Subquery(subquery) => kinds(&subquery.this, branches, context),
        Expression::Paren(paren) => kinds(&paren.this, branches, context),
        Expression::Values(values) => {
            let scope: LineageScope<'_> = empty_scope(branches.ctes, None);
            let row = values.expressions.first()?;
            Some(
                row.expressions
                    .iter()
                    .map(|value| kind(value, &scope, context))
                    .collect(),
            )
        }
        _ => None,
    }
}

/// A projection's kind: NULL, its SQLQuack type, else its family.
fn kind(projection: &Expression, scope: &LineageScope<'_>, context: LineageContext<'_>) -> Kind {
    match projection {
        Expression::Alias(alias) => kind(&alias.this, scope, context),
        Expression::Annotated(annotated) => kind(&annotated.this, scope, context),
        Expression::Paren(paren) => kind(&paren.this, scope, context),
        Expression::Null(_) => Kind::Null,
        _ => expression_lineage(projection, scope, context)
            .data_type
            .as_deref()
            .map(Kind::of_spelling)
            .filter(|kind| *kind != Kind::Unknown)
            .unwrap_or_else(|| Kind::of_family(expression_family(projection, scope, context))),
    }
}
