//! Variance, standard deviation and covariance aggregates, computed the way Snowflake was recorded.

use sqlquack_blocks::blocks::main::exact_decimal_divide::exact_decimal_divide;
use sqlquack_blocks::floats::constants::FLOAT_SUM_ORDER;
use sqlquack_blocks::floats::main::exact_float_sum::exact_float_sum;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_policy::policy::constants::NUMERIC_RANGE;
use sqlquack_semantics::snowflake::typing::constants::MAX_PRECISION;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::casts::cast_to;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Context, Emitted};
use crate::conversion::constants::{COVARIANCE_RULE, VARIANCE_FLOAT_RULE, VARIANCE_RULE};

/// Variances and standard deviations: the name, whether over the population, and whether a root.
const VARIANCE_AGGREGATES: &[(&str, bool, bool)] = &[
    ("VARIANCE", false, false),
    ("VAR_SAMP", false, false),
    ("VAR_POP", true, false),
    ("VARIANCE_POP", true, false),
    ("STDDEV", false, true),
    ("STDDEV_SAMP", false, true),
    ("STDDEV_POP", true, true),
];
/// Covariances, and whether over the population.
const COVARIANCE_AGGREGATES: &[(&str, bool)] = &[("COVAR_SAMP", false), ("COVAR_POP", true)];
/// The variance Snowflake rejects with DISTINCT, though its other spellings accept it.
const NO_DISTINCT_VARIANCE: &str = "VARIANCE";
/// An exact variance's result scale: this base plus twice the input's scale, at most the widest.
const VARIANCE_SCALES: (u8, u8) = (6, 12);
/// The lambda parameter squaring each distinct value of an exact variance.
const SQUARED_PARAMETER: &str = "__sqlquack_squared";
/// The type name statistics cast NUMBER and text arguments to.
const FLOAT_TYPE_NAME: &str = "FLOAT";

/// Whether `name` is a variance, standard deviation or covariance aggregate converted here.
pub(crate) fn is_statistic(name: &str) -> bool {
    VARIANCE_AGGREGATES
        .iter()
        .any(|(known, _, _)| *known == name)
        || COVARIANCE_AGGREGATES
            .iter()
            .any(|(known, _)| *known == name)
}

/// One statistic of converted `arguments`; `over` is its OVER clause, or empty.
pub(crate) fn statistic(
    name: &str,
    arguments: &[Emitted],
    (distinct, over): (bool, &str),
    (construct, context): (&Construct, &Context<'_>),
) -> Result<Emitted, Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    if distinct && name == NO_DISTINCT_VARIANCE {
        return Err(refuse(format!(
            "{name}(DISTINCT), which Snowflake rejects as an unsupported feature"
        )));
    }
    let variance = VARIANCE_AGGREGATES
        .iter()
        .find(|(known, _, _)| *known == name);
    let covariance = COVARIANCE_AGGREGATES
        .iter()
        .find(|(known, _)| *known == name);
    match (variance, covariance, arguments) {
        (Some((_, population, root)), _, [value]) => {
            let shape = Shape {
                population: *population,
                distinct,
                over,
            };
            let sql = match value.data_type {
                SnowflakeType::Number { scale, .. } => {
                    exact_variance(&value.sql, scale, shape, (construct, context))?
                }
                SnowflakeType::Float | SnowflakeType::Varchar if !distinct => {
                    let float = as_float(value, (construct, context))?;
                    float_variance(&float, shape, (construct, context))?
                }
                other => {
                    return Err(refuse(format!(
                        "{name}{} of {other}, which no recording shows",
                        distinct_text(distinct)
                    )));
                }
            };
            Ok(match (root, value.data_type) {
                (true, SnowflakeType::Number { .. }) => {
                    Emitted::new(format!("sqrt(CAST({sql} AS DOUBLE))"), SnowflakeType::Float)
                }
                (true, _) => Emitted::new(format!("sqrt({sql})"), SnowflakeType::Float),
                (false, SnowflakeType::Number { scale, .. }) => Emitted::new(
                    sql,
                    SnowflakeType::Number {
                        precision: MAX_PRECISION,
                        scale: variance_scale(scale),
                    },
                ),
                (false, _) => Emitted::new(sql, SnowflakeType::Float),
            })
        }
        (_, Some((_, population)), [y, x]) if !distinct && numeric(y) && numeric(x) => {
            let shape = Shape {
                population: *population,
                distinct,
                over,
            };
            let pair = (
                as_float(y, (construct, context))?,
                as_float(x, (construct, context))?,
            );
            Ok(Emitted::new(
                float_covariance((&y.sql, &x.sql), pair, shape, (construct, context))?,
                SnowflakeType::Float,
            ))
        }
        (_, _, arguments) => Err(refuse(format!(
            "{name}{} of {}, which no recording shows",
            distinct_text(distinct),
            arguments
                .iter()
                .map(|argument| argument.data_type.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// How a statistic aggregates: over the population or a sample, its distinct values, its window.
#[derive(Clone, Copy)]
struct Shape<'a> {
    population: bool,
    distinct: bool,
    over: &'a str,
}

impl Shape<'_> {
    /// The fewest values with a result: one for a population, two for a sample.
    fn fewest(self) -> u8 {
        match self.population {
            true => 1,
            false => 2,
        }
    }
}

/// The recorded result scale of the variance of a NUMBER of `scale`.
fn variance_scale(scale: u8) -> u8 {
    let (base, widest) = VARIANCE_SCALES;
    (base + 2 * scale).min(widest)
}

/// The exact variance of a NUMBER of `scale`, rounded half up to its recorded scale.
fn exact_variance(
    value: &str,
    scale: u8,
    shape: Shape<'_>,
    (construct, context): (&Construct, &Context<'_>),
) -> Result<String, Refusal> {
    let log = context.log;
    let _rule = log.rule(VARIANCE_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let (_, widest) = VARIANCE_SCALES;
    if scale > widest {
        return Err(refuse(format!(
            "the variance of a NUMBER of scale {scale}, above the widest recorded ({widest})"
        )));
    }
    diverge(context, NUMERIC_RANGE, construct)?;
    let over = shape.over;
    let squares = 2 * scale;
    let decimal = |sql: &str, scale: u8| format!("CAST({sql} AS DECIMAL(38,{scale}))");
    let (count, total, sum_of_squares) = match shape.distinct {
        true => (
            format!("count(DISTINCT {value}){over}"),
            format!("sum(DISTINCT {value}){over}"),
            format!(
                "list_sum(list_transform(list(DISTINCT {value}){over}, \
                 lambda {SQUARED_PARAMETER}: {} * {}))",
                decimal(SQUARED_PARAMETER, scale),
                decimal(SQUARED_PARAMETER, scale)
            ),
        ),
        false => (
            format!("count({value}){over}"),
            format!("sum({value}){over}"),
            format!(
                "sum({} * {}){over}",
                decimal(value, scale),
                decimal(value, scale)
            ),
        ),
    };
    let numerator = decimal(
        &format!(
            "{} * {} - {} * {}",
            decimal(&count, 0),
            decimal(&sum_of_squares, squares),
            decimal(&total, scale),
            decimal(&total, scale)
        ),
        squares,
    );
    let result = variance_scale(scale);
    let divisor_scale = squares.saturating_sub(result);
    let pairs = match shape.population {
        true => format!("{count} * {count}"),
        false => format!("{count} * ({count} - 1)"),
    };
    let (divided, _) = exact_decimal_divide(
        (&numerator, MAX_PRECISION, squares),
        (
            &decimal(&pairs, divisor_scale),
            MAX_PRECISION,
            divisor_scale,
        ),
        (
            SnowflakeType::Number {
                precision: MAX_PRECISION,
                scale: result,
            },
            false,
            None,
        ),
        log,
    )
    .map_err(refuse)?;
    Ok(format!(
        "(CASE WHEN {count} < {} THEN NULL ELSE {divided} END)",
        shape.fewest()
    ))
}

/// The variance of FLOAT values `value`, as (n·Σx² − (Σx)²) / (n·d) with exact sums.
fn float_variance(
    value: &str,
    shape: Shape<'_>,
    (construct, context): (&Construct, &Context<'_>),
) -> Result<String, Refusal> {
    let log = context.log;
    let _rule = log.rule(VARIANCE_FLOAT_RULE);
    diverge(context, FLOAT_SUM_ORDER, construct)?;
    let over = shape.over;
    let count = format!("count({value}){over}");
    let values = format!("CAST({count} AS DOUBLE)");
    let total = exact_float_sum(value, "", over, log);
    let squares = exact_float_sum(&format!("({value}) * ({value})"), "", over, log);
    Ok(format!(
        "(CASE WHEN {count} < {} THEN NULL ELSE ({values} * {squares} - {total} * {total}) / \
         ({values} * {}) END)",
        shape.fewest(),
        degrees(&values, shape)
    ))
}

/// The covariance of FLOAT pairs, as (Σxy − Σx·Σy / n) / d with exact sums over complete pairs.
fn float_covariance(
    (y_sql, x_sql): (&str, &str),
    (y, x): (String, String),
    shape: Shape<'_>,
    (construct, context): (&Construct, &Context<'_>),
) -> Result<String, Refusal> {
    let log = context.log;
    let _rule = log.rule(COVARIANCE_RULE);
    diverge(context, FLOAT_SUM_ORDER, construct)?;
    let over = shape.over;
    let complete = format!("{y_sql} IS NOT NULL AND {x_sql} IS NOT NULL");
    let kept = |sql: &str| format!("CASE WHEN {complete} THEN {sql} END");
    let count = format!("count({}){over}", kept("1"));
    let values = format!("CAST({count} AS DOUBLE)");
    let sum = |sql: &str| exact_float_sum(&kept(sql), "", over, log);
    Ok(format!(
        "(CASE WHEN {count} < {} THEN NULL ELSE ({} - {} * {} / {values}) / {} END)",
        shape.fewest(),
        sum(&format!("({y}) * ({x})")),
        sum(&x),
        sum(&y),
        degrees(&values, shape)
    ))
}

/// The divisor's degrees of freedom: n for a population, n − 1 for a sample.
fn degrees(values: &str, shape: Shape<'_>) -> String {
    match shape.population {
        true => values.to_owned(),
        false => format!("({values} - 1)"),
    }
}

/// `value` as a FLOAT: as it is, or cast as Snowflake casts a NUMBER or text argument.
fn as_float(
    value: &Emitted,
    (construct, context): (&Construct, &Context<'_>),
) -> Result<String, Refusal> {
    match value.data_type {
        SnowflakeType::Float => Ok(value.sql.clone()),
        _ => cast_to(value, FLOAT_TYPE_NAME, false, (context, construct))
            .map_err(|reason| outside_conditions(construct.clone(), reason)),
    }
}

/// Whether a covariance argument is of a type its recordings show: NUMBER or FLOAT.
fn numeric(value: &Emitted) -> bool {
    matches!(
        value.data_type,
        SnowflakeType::Number { .. } | SnowflakeType::Float
    )
}

/// " (DISTINCT)" when `distinct`, for refusal reasons.
fn distinct_text(distinct: bool) -> &'static str {
    match distinct {
        true => "(DISTINCT)",
        false => "",
    }
}
