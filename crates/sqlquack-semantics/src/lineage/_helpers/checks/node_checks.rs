//! Each node's type findings: predicates, comparisons, arithmetic, casts and built-in arguments.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::queries::models::Select;
use sqlquack_syntax::expressions::types::models::DataType;

use crate::lineage::_helpers::checks::coercion::{
    Engine, arithmetic, comparable, invalid_literal_for, invalid_temporal_literal, literal_coerces,
    literal_reads_as, predicate, runtime_comparison, runtime_equality, string_literal,
    temporal_argument, temporal_function_argument, temporal_literal_arithmetic,
};
use crate::lineage::_helpers::checks::duckdb_functions::{duckdb_arities, duckdb_function};
use crate::lineage::_helpers::checks::families::{Family, data_type_family, expression_family};
use crate::lineage::_helpers::checks::issues::{issue, type_location};
use crate::lineage::_helpers::checks::relational::{
    in_subquery_findings, range_frame_findings, recursive_findings, row_comparison_findings,
    scalar_query, using_findings,
};
use crate::lineage::_helpers::checks::sets::width;
use crate::lineage::_helpers::checks::signatures::{
    argument_finding, check_call, dispatch_name, has_signature, located, unify,
};
use crate::lineage::_helpers::tracing::expressions::expression_lineage;
use crate::lineage::constants::{
    DUCKDB_KEYWORD_ONLY_FUNCTIONS, NAME_PHASE, RUNTIME_WARNING, SQLSERVER_PROFILE, TYPE_PHASE,
};
use crate::lineage::models::{LineageContext, LineageScope, NameFinding};

/// COALESCE without arguments, which polyglot checks against the catalog under this name.
const COALESCE: &str = "coalesce";

/// BigQuery's runtime-error prefix, whose call takes the bare call's argument types.
const SAFE_PREFIX: &str = "SAFE.";

/// What a node's checks read: its level's scope and the engine.
#[derive(Clone, Copy)]
pub(crate) struct NodeLevel<'s, 'a> {
    pub(crate) scope: &'s LineageScope<'a>,
    pub(crate) context: LineageContext<'a>,
}

impl NodeLevel<'_, '_> {
    fn family(&self, expression: &Expression) -> Family {
        expression_family(expression, self.scope, self.context)
    }

    fn engine(&self) -> Engine {
        Engine::of(self.context)
    }
}

fn finding(code: &'static str, message: String, node: &Expression) -> NameFinding {
    issue((code, TYPE_PHASE), message, None, node)
}

/// The type findings of `node`, as polyglot's type checks report them; none for Databricks.
pub(crate) fn node_findings(node: &Expression, level: NodeLevel<'_, '_>) -> Vec<NameFinding> {
    if level.engine() == Engine::Databricks {
        return Vec::new();
    }
    let sqlite: bool = level.engine() == Engine::Sqlite;
    let mut findings: Vec<NameFinding> = match node {
        Expression::Select(select) if !sqlite => clause_predicates(select, node, level),
        _ => Vec::new(),
    };
    if let Expression::Function(function) = node
        && level.context.schema.known_function(&function.name)
    {
        return findings;
    }
    let family = |expression: &Expression| level.family(expression);
    let mut own: Vec<NameFinding> = match node {
        Expression::Function(function) => {
            let written: &str = function
                .span
                .0
                .and_then(|span| {
                    level
                        .context
                        .sql?
                        .get(span.start..)?
                        .get(..function.name.len())
                })
                .filter(|written| written.eq_ignore_ascii_case(&function.name))
                .unwrap_or(&function.name);
            catalog_arity((&function.name, written, function.args.len()), level, node)
        }
        Expression::Coalesce(function) if function.expressions.is_empty() => {
            catalog_arity((COALESCE, COALESCE, 0), level, node)
        }
        _ => Vec::new(),
    };
    let signatures_abstain: bool = sqlite || level.context.profile.name() == SQLSERVER_PROFILE;
    if signatures_abstain {
        findings.extend(located(own, node));
        return findings;
    }
    let checked = check_call(node, level.engine(), &family);
    let signed: bool = checked.as_ref().is_some_and(|checked| checked.signed);
    let mut call: Vec<NameFinding> = checked.map(|checked| checked.findings).unwrap_or_default();
    if let Expression::Function(function) = node
        && level.engine() == Engine::BigQuery
    {
        let refused: Vec<NameFinding> =
            bigquery_call_findings((&function.name, &function.args), level, node);
        if !refused.is_empty() {
            call.retain(|finding| finding.code != RUNTIME_WARNING);
        }
        call.extend(refused);
    }
    own.extend(call);
    own.extend(expression_checks(node, level));
    if !(signed && !matches!(node, Expression::Function(_))) {
        own.extend(operator_checks(node, level, signed));
    }
    findings.extend(located(own, node));
    if let Expression::Function(function) = node {
        let keyword_only: Vec<NameFinding> = keyword_only_call(function, level.engine(), node);
        if keyword_only.is_empty() {
            findings.extend(unknown_duckdb_call(function, level.engine(), node));
        }
        findings.extend(keyword_only);
    }
    findings
}

/// An unqualified DuckDB call to a name that is neither built in nor declared by the project.
fn unknown_duckdb_call(
    function: &sqlquack_syntax::expressions::functions::models::Function,
    engine: Engine,
    node: &Expression,
) -> Vec<NameFinding> {
    match engine == Engine::DuckDB
        && !function.quoted
        && !function.no_parens
        && !function.use_bracket_syntax
        && !function.name.contains('.')
        && !duckdb_function(&function.name)
    {
        true => vec![issue(
            ("E202", NAME_PHASE),
            format!("Unknown function '{}' for dialect DuckDB", function.name),
            function.span.0,
            node,
        )],
        false => Vec::new(),
    }
}

/// A call whose argument count no signature in the catalog accepts, named as `written`.
fn catalog_arity(
    (name, written, count): (&str, &str, usize),
    level: NodeLevel<'_, '_>,
    node: &Expression,
) -> Vec<NameFinding> {
    let canonical: String = dispatch_name(name);
    let recorded: Option<Vec<(usize, Option<usize>)>> = match level.engine() {
        Engine::DuckDB if !name.contains('.') => duckdb_arities(name),
        _ => None,
    };
    let Some(arities) =
        recorded.or_else(|| level.context.schema.function_arities(name, &canonical))
    else {
        return Vec::new();
    };
    if arities
        .iter()
        .any(|(least, most)| count >= *least && most.is_none_or(|most| count <= most))
    {
        return Vec::new();
    }
    let expected: Vec<String> = arities
        .iter()
        .map(|(least, most)| match most {
            Some(most) if most == least => least.to_string(),
            Some(most) => format!("{least}..{most}"),
            None => format!("{least}+"),
        })
        .collect();
    vec![finding(
        "E203",
        format!(
            "Invalid arity for function '{written}': got {count}, expected {}",
            expected.join(", ")
        ),
        node,
    )]
}

/// WHERE, HAVING, QUALIFY and join conditions that are not boolean.
fn clause_predicates(
    select: &Select,
    node: &Expression,
    level: NodeLevel<'_, '_>,
) -> Vec<NameFinding> {
    let mut clauses: Vec<(&str, &Expression)> = Vec::new();
    clauses.extend(select.prewhere.iter().map(|clause| ("PREWHERE", clause)));
    clauses.extend(
        select
            .where_clause
            .iter()
            .map(|clause| ("WHERE", &clause.this)),
    );
    clauses.extend(select.having.iter().map(|clause| ("HAVING", &clause.this)));
    clauses.extend(
        select
            .qualify
            .iter()
            .map(|clause| ("QUALIFY", &clause.this)),
    );
    for join in &select.joins {
        clauses.extend(join.on.iter().map(|on| ("JOIN ON", on)));
        clauses.extend(
            join.match_condition
                .iter()
                .map(|condition| ("JOIN MATCH_CONDITION", condition)),
        );
    }
    clauses
        .into_iter()
        .filter_map(|(clause, condition)| {
            let family: Family = level.family(condition);
            (!clause_predicate(level.engine(), condition, family)).then(|| NameFinding {
                span: type_location(condition),
                ..finding(
                    predicate_code(level.engine()),
                    format!(
                        "{clause} expects a boolean predicate, found {}",
                        family.name()
                    ),
                    node,
                )
            })
        })
        .collect()
}

fn predicate_code(engine: Engine) -> &'static str {
    match engine {
        Engine::DuckDB => "W215",
        Engine::Snowflake
        | Engine::Postgres
        | Engine::Sqlite
        | Engine::BigQuery
        | Engine::Databricks => "E211",
    }
}

/// Whether a logical operand accepts this value: a predicate family, or a boolean literal.
fn predicate_value(engine: Engine, expression: &Expression, family: Family) -> bool {
    predicate(engine, family)
        || (literal_coerces(engine, expression, Family::Boolean)
            && !invalid_literal_for(expression, Family::Boolean))
}

/// Whether a clause condition accepts this value; Snowflake's conditions take only BOOLEAN.
fn clause_predicate(engine: Engine, expression: &Expression, family: Family) -> bool {
    if engine == Engine::Snowflake && family.fully_modelled() {
        return family == Family::Boolean;
    }
    predicate_value(engine, expression, family)
}

/// Checks of literals, casts, scalar subqueries, conditions and unified values.
fn expression_checks(node: &Expression, level: NodeLevel<'_, '_>) -> Vec<NameFinding> {
    let engine: Engine = level.engine();
    let family = |expression: &Expression| level.family(expression);
    let mut findings: Vec<NameFinding> = scalar_subqueries(node, level);
    let condition = |expression: &Expression| {
        let found: Family = family(expression);
        (!clause_predicate(engine, expression, found)).then(|| {
            finding(
                predicate_code(engine),
                format!("Expected boolean condition, found {}", found.name()),
                node,
            )
        })
    };
    let read = (level.scope, level.context);
    match node {
        Expression::Select(select) => {
            findings.extend(using_findings(node, select, read));
            findings.extend(recursive_findings(node, select, read));
        }
        Expression::WindowFunction(_) => findings.extend(range_frame_findings(node, read)),
        Expression::Interval(interval) if engine == Engine::BigQuery => {
            if let Some(count) = interval.this.as_ref() {
                let found: Family = family(count);
                if !matches!(found, Family::Unknown | Family::Integer) && !string_literal(count) {
                    findings.push(finding(
                        "E213",
                        format!("Interval count expects an integer, found {}", found.name()),
                        node,
                    ));
                }
            }
        }
        Expression::Cast(cast) | Expression::TryCast(cast) | Expression::SafeCast(cast) => {
            findings.extend(unknown_cast_target(&cast.to, level, node));
            if matches!(node, Expression::Cast(_)) {
                findings.extend(impossible_cast(
                    (family(&cast.this), &cast.to),
                    engine,
                    node,
                ));
            }
        }
        Expression::Literal(literal) if engine == Engine::DuckDB => {
            if let Literal::Date(value) | Literal::Timestamp(value) | Literal::Datetime(value) =
                literal.as_ref()
                && invalid_temporal_literal(value)
            {
                findings.push(finding("W213", "Invalid temporal literal".to_owned(), node));
            }
        }
        Expression::Interval(interval) => {
            if let Some(Expression::Literal(literal)) = &interval.this
                && let Literal::String(value) = literal.as_ref()
                && !value.chars().any(|character| character.is_ascii_digit())
            {
                let code: Option<&'static str> = match engine {
                    Engine::Snowflake => Some("E218"),
                    Engine::DuckDB => Some("W213"),
                    Engine::Postgres | Engine::Sqlite | Engine::BigQuery | Engine::Databricks => {
                        None
                    }
                };
                findings.extend(
                    code.map(|code| finding(code, "Invalid interval literal".to_owned(), node)),
                );
            }
        }
        Expression::DateTrunc(function) | Expression::TimestampTrunc(function) => {
            let found: Family = family(&function.this);
            findings.extend(argument_finding(
                ("date_trunc", 1),
                (found, "a temporal argument"),
                temporal_function_argument("date_trunc", engine, (&function.this, found)),
                node,
            ));
        }
        Expression::Extract(function) => {
            let found: Family = family(&function.this);
            findings.extend(argument_finding(
                ("extract", 1),
                (found, "a temporal argument"),
                temporal_function_argument("extract", engine, (&function.this, found)),
                node,
            ));
        }
        Expression::DateDiff(function) => {
            for (index, argument) in [&function.this, &function.expression]
                .into_iter()
                .enumerate()
            {
                let found: Family = family(argument);
                findings.extend(argument_finding(
                    ("date_diff", index),
                    (found, "a temporal argument"),
                    temporal_argument(engine, argument, found),
                    node,
                ));
            }
        }
        Expression::IsTrue(test) | Expression::IsFalse(test) => {
            findings.extend(condition(&test.this));
        }
        Expression::Case(case) => {
            if case.operand.is_none() {
                for (when, _) in &case.whens {
                    findings.extend(condition(when));
                }
            }
            let mut values: Vec<&Expression> = case.whens.iter().map(|(_, value)| value).collect();
            values.extend(case.else_.iter());
            findings.extend(unify(&values, engine, &family, (false, node)));
        }
        Expression::IfFunc(function) => {
            if engine != Engine::Snowflake
                || !predicate_value(engine, &function.condition, family(&function.condition))
            {
                findings.extend(condition(&function.condition));
            }
            let mut values: Vec<&Expression> = vec![&function.true_value];
            values.extend(function.false_value.iter());
            findings.extend(unify(&values, engine, &family, (false, node)));
        }
        Expression::Sum(function)
        | Expression::Avg(function)
        | Expression::Min(function)
        | Expression::Max(function)
        | Expression::Stddev(function)
        | Expression::Variance(function) => {
            findings.extend(function.filter.iter().filter_map(condition));
        }
        Expression::Count(function) => {
            findings.extend(function.filter.iter().filter_map(condition));
        }
        Expression::Neg(unary) => {
            let found: Family = family(&unary.this);
            if found != Family::Unknown && !found.is_numeric() && found != Family::Interval {
                let code: &'static str =
                    match engine == Engine::Snowflake && found == Family::String {
                        true => "W211",
                        false => "E212",
                    };
                findings.push(finding(
                    code,
                    format!("Unary minus cannot operate on {}", found.name()),
                    node,
                ));
            }
        }
        Expression::In(test) => {
            if engine == Engine::DuckDB
                && test
                    .expressions
                    .iter()
                    .any(|value| invalid_literal_for(value, family(&test.this)))
            {
                findings.push(finding(
                    "W213",
                    "IN list contains an invalid coercible literal".to_owned(),
                    node,
                ));
            }
            if let Some(query) = &test.query {
                findings.extend(in_subquery_findings(node, (&test.this, query), read));
            }
        }
        Expression::Eq(comparison)
        | Expression::Neq(comparison)
        | Expression::Lt(comparison)
        | Expression::Gt(comparison)
        | Expression::Lte(comparison)
        | Expression::Gte(comparison) => {
            if engine == Engine::DuckDB
                && (invalid_literal_for(&comparison.left, family(&comparison.right))
                    || invalid_literal_for(&comparison.right, family(&comparison.left)))
            {
                findings.push(finding(
                    "W213",
                    "Comparison contains an invalid coercible literal".to_owned(),
                    node,
                ));
            }
            if let (Expression::Tuple(left), Expression::Tuple(right)) =
                (&comparison.left, &comparison.right)
                && left.expressions.len() != right.expressions.len()
            {
                findings.push(finding(
                    "E216",
                    "Row values have different column counts".to_owned(),
                    node,
                ));
            }
            findings.extend(row_comparison_findings(
                node,
                (&comparison.left, &comparison.right),
            ));
        }
        Expression::Between(between)
            if engine == Engine::DuckDB
                && (invalid_literal_for(&between.low, family(&between.this))
                    || invalid_literal_for(&between.high, family(&between.this))) =>
        {
            findings.push(finding(
                "W213",
                "BETWEEN contains an invalid coercible literal".to_owned(),
                node,
            ));
        }
        _ => {}
    }
    findings
}

/// BigQuery built-in calls whose argument rules the generic signatures do not record.
fn bigquery_call_findings(
    (name, arguments): (&str, &[Expression]),
    level: NodeLevel<'_, '_>,
    node: &Expression,
) -> Vec<NameFinding> {
    let engine: Engine = level.engine();
    let family = |expression: &Expression| level.family(expression);
    let upper: String = name.to_ascii_uppercase();
    let bare: &str = upper.strip_prefix(SAFE_PREFIX).unwrap_or(&upper);
    let called: String = bare.to_ascii_lowercase();
    match (bare, arguments) {
        ("IF", [condition, when_true, when_false]) => {
            let mut found: Vec<NameFinding> = Vec::new();
            let tested: Family = family(condition);
            if tested.converts_to_nothing() {
                found.push(finding(
                    "E213",
                    format!(
                        "Function '{called}' argument 1 expects Boolean, found {}",
                        tested.name()
                    ),
                    node,
                ));
            }
            let unified: Option<NameFinding> =
                unify(&[when_true, when_false], engine, &family, (false, node));
            let (left, right) = (family(when_true), family(when_false));
            let apart: bool = left != right
                && left != Family::Unknown
                && right != Family::Unknown
                && (left.converts_to_nothing() || right.converts_to_nothing());
            found.extend(unified.or_else(|| {
                apart.then(|| {
                    finding(
                        "E213",
                        format!("Cannot unify {} and {}", left.name(), right.name()),
                        node,
                    )
                })
            }));
            found
        }
        ("EXTRACT", [_, source]) => {
            let found: Family = family(source);
            argument_finding(
                ("extract", 1),
                (found, "a temporal argument"),
                found.is_temporal(),
                node,
            )
            .into_iter()
            .collect()
        }
        ("DATE_ADD" | "DATE_SUB", [value, _]) => {
            let found: Family = family(value);
            let refused: bool = match found {
                Family::Time
                | Family::Interval
                | Family::Json
                | Family::Array
                | Family::Map
                | Family::Struct => true,
                Family::String => string_literal(value) && !literal_reads_as(value, Family::Date),
                _ => false,
            };
            refused
                .then(|| {
                    finding(
                        "E213",
                        format!(
                            "Function '{called}' argument 1 expects a date argument, found {}",
                            found.name()
                        ),
                        node,
                    )
                })
                .into_iter()
                .collect()
        }
        ("TRIM" | "LTRIM" | "RTRIM", [value, ..]) if family(value).converts_to_nothing() => {
            vec![finding(
                "E213",
                format!(
                    "Function '{called}' argument 1 expects a string or bytes argument, found {}",
                    family(value).name()
                ),
                node,
            )]
        }
        ("TRIM" | "LTRIM" | "RTRIM", [value]) if family(value) == Family::Binary => {
            vec![finding(
                "E213",
                format!("Function '{called}' of binary needs the bytes to remove"),
                node,
            )]
        }
        _ => Vec::new(),
    }
}

/// BigQuery arithmetic its types refuse though their families combine.
fn bigquery_arithmetic_refusal(
    node: &Expression,
    (left, right): (&Expression, &Expression),
    level: NodeLevel<'_, '_>,
) -> Option<NameFinding> {
    let spelt = |expression: &Expression| {
        expression_lineage(expression, level.scope, level.context)
            .data_type
            .as_deref()
            .and_then(|spelling| spelling.split(['(', '<']).next())
            .map(|base| base.trim().to_ascii_uppercase())
    };
    let (left_spelt, right_spelt) = (spelt(left)?, spelt(right)?);
    let (left_type, right_type) = (left_spelt.as_str(), right_spelt.as_str());
    let refused: bool = match node {
        Expression::Sub(_) => matches!(
            (left_type, right_type),
            ("DATE" | "DATETIME", "TIMESTAMP") | ("TIMESTAMP", "DATE" | "DATETIME")
        ),
        Expression::Mul(_) => matches!(
            (left_type, right_type),
            ("INTERVAL", "NUMERIC" | "BIGNUMERIC") | ("NUMERIC" | "BIGNUMERIC", "INTERVAL")
        ),
        Expression::Div(_) => matches!(
            (left_type, right_type),
            ("INTERVAL", "NUMERIC" | "BIGNUMERIC" | "FLOAT64")
        ),
        _ => false,
    };
    refused.then(|| {
        finding(
            "E212",
            format!(
                "Arithmetic operation expects numeric-compatible operands, found {} and {}",
                left_spelt.to_ascii_lowercase(),
                right_spelt.to_ascii_lowercase()
            ),
            node,
        )
    })
}

/// Scalar subqueries in a scalar position that return more than one column.
fn scalar_subqueries(node: &Expression, level: NodeLevel<'_, '_>) -> Vec<NameFinding> {
    let children: Vec<&Expression> = match node {
        Expression::Select(select) => {
            let mut children: Vec<&Expression> = select.expressions.iter().collect();
            children.extend(select.where_clause.iter().map(|clause| &clause.this));
            children.extend(select.having.iter().map(|clause| &clause.this));
            children.extend(select.qualify.iter().map(|clause| &clause.this));
            children.extend(select.joins.iter().filter_map(|join| join.on.as_ref()));
            children
        }
        Expression::Eq(_)
        | Expression::Neq(_)
        | Expression::Lt(_)
        | Expression::Lte(_)
        | Expression::Gt(_)
        | Expression::Gte(_)
        | Expression::Add(_)
        | Expression::Sub(_)
        | Expression::Mul(_)
        | Expression::Div(_)
        | Expression::Mod(_)
        | Expression::And(_)
        | Expression::Or(_)
        | Expression::Not(_)
        | Expression::Neg(_)
        | Expression::Pos(_)
        | Expression::Like(_)
        | Expression::ILike(_)
        | Expression::Cast(_)
        | Expression::TryCast(_)
        | Expression::SafeCast(_)
        | Expression::IfFunc(_) => sqlquack_syntax::ExpressionWalk::children(node),
        Expression::Function(function)
            if has_signature(level.engine(), &dispatch_name(&function.name)) =>
        {
            sqlquack_syntax::ExpressionWalk::children(node)
        }
        _ if has_signature(level.engine(), node.variant_name()) => {
            sqlquack_syntax::ExpressionWalk::children(node)
        }
        _ => Vec::new(),
    };
    children
        .into_iter()
        .filter(|child| {
            scalar_query(child).is_some_and(|query| width(query).is_some_and(|width| width > 1))
        })
        .map(|_| {
            finding(
                "E216",
                "Scalar subquery must return one column".to_owned(),
                node,
            )
        })
        .collect()
}

/// A cast to a named type no engine family and no project declaration knows.
fn unknown_cast_target(
    target: &DataType,
    level: NodeLevel<'_, '_>,
    node: &Expression,
) -> Option<NameFinding> {
    let DataType::Custom { name } = target else {
        return None;
    };
    (data_type_family(target) == Family::Unknown && !level.context.schema.known_type(name))
        .then(|| finding("W213", format!("Unknown cast target type '{name}'"), node))
}

/// A built-in conversion no engine performs: TIMESTAMP to a number or BOOLEAN, or the reverse.
fn impossible_cast(
    (source, target): (Family, &DataType),
    engine: Engine,
    node: &Expression,
) -> Option<NameFinding> {
    let target: Family = data_type_family(target);
    let impossible: bool = (source == Family::Timestamp
        && matches!(target, Family::Boolean | Family::Integer | Family::Numeric))
        || (source == Family::Boolean && target == Family::Timestamp);
    impossible.then(|| {
        finding(
            match engine {
                Engine::Snowflake
                | Engine::Postgres
                | Engine::Sqlite
                | Engine::BigQuery
                | Engine::Databricks => "E218",
                Engine::DuckDB => "W213",
            },
            format!("Cannot cast {} to {}", source.name(), target.name()),
            node,
        )
    })
}

/// Operator checks: logical operands, comparisons, LIKE, BETWEEN, IN, arithmetic and functions.
fn operator_checks(node: &Expression, level: NodeLevel<'_, '_>, signed: bool) -> Vec<NameFinding> {
    let engine: Engine = level.engine();
    let family = |expression: &Expression| level.family(expression);
    let logical = |expression: &Expression, words: String| {
        let found: Family = family(expression);
        (!predicate_value(engine, expression, found)).then(|| {
            finding(
                predicate_code(engine),
                format!("{words}{}", found.name()),
                node,
            )
        })
    };
    match node {
        Expression::And(operation) | Expression::Or(operation) => {
            [("left", &operation.left), ("right", &operation.right)]
                .into_iter()
                .filter_map(|(side, operand)| {
                    logical(
                        operand,
                        format!("Logical {side} operand expects boolean, found "),
                    )
                })
                .collect()
        }
        Expression::Not(unary) => logical(&unary.this, "NOT expects boolean, found ".to_owned())
            .into_iter()
            .collect(),
        Expression::Eq(comparison)
        | Expression::Neq(comparison)
        | Expression::Lt(comparison)
        | Expression::Lte(comparison)
        | Expression::Gt(comparison)
        | Expression::Gte(comparison) => {
            let (left, right) = (family(&comparison.left), family(&comparison.right));
            let runtime: bool = (engine == Engine::Snowflake && runtime_comparison(left, right))
                || (matches!(node, Expression::Eq(_) | Expression::Neq(_))
                    && runtime_equality(engine, left, right));
            let mismatch: bool =
                !comparable(engine, (&comparison.left, &comparison.right), (left, right))
                    || (runtime
                        && !string_literal(&comparison.left)
                        && !string_literal(&comparison.right));
            mismatch
                .then(|| {
                    finding(
                        if runtime { "W210" } else { "E217" },
                        format!(
                            "Incompatible comparison between {} and {}",
                            left.name(),
                            right.name()
                        ),
                        node,
                    )
                })
                .into_iter()
                .collect()
        }
        Expression::Like(operation) | Expression::ILike(operation) => {
            let (left, right) = (family(&operation.left), family(&operation.right));
            (left != Family::Unknown
                && right != Family::Unknown
                && (left != Family::String || right != Family::String)
                && engine != Engine::Snowflake)
                .then(|| {
                    finding(
                        "E217",
                        format!(
                            "LIKE/ILIKE expects string operands, found {} and {}",
                            left.name(),
                            right.name()
                        ),
                        node,
                    )
                })
                .into_iter()
                .collect()
        }
        Expression::Between(between) => {
            let this: Family = family(&between.this);
            let (low, high) = (family(&between.low), family(&between.high));
            (!comparable(engine, (&between.this, &between.low), (this, low))
                || !comparable(engine, (&between.this, &between.high), (this, high)))
            .then(|| {
                finding(
                    "E217",
                    format!(
                        "BETWEEN bounds are incompatible with {} (found {} and {})",
                        this.name(),
                        low.name(),
                        high.name()
                    ),
                    node,
                )
            })
            .into_iter()
            .collect()
        }
        Expression::In(test) => {
            let this: Family = family(&test.this);
            test.expressions
                .iter()
                .find_map(|value| {
                    let item: Family = family(value);
                    (!comparable(engine, (&test.this, value), (this, item))).then(|| {
                        finding(
                            if engine != Engine::BigQuery && runtime_comparison(this, item) {
                                "W210"
                            } else {
                                "E217"
                            },
                            format!(
                                "IN item type {} is incompatible with {}",
                                item.name(),
                                this.name()
                            ),
                            node,
                        )
                    })
                })
                .into_iter()
                .collect()
        }
        Expression::Add(operation)
        | Expression::Sub(operation)
        | Expression::Mul(operation)
        | Expression::Div(operation)
        | Expression::Mod(operation) => {
            arithmetic_findings(node, (&operation.left, &operation.right), level)
        }
        Expression::Function(function) if !signed => generic_function(function, level, node),
        Expression::Function(_) => Vec::new(),
        _ => typed_function_checks(node, level),
    }
}

/// Arithmetic over families the engine has no overload for, or a literal that will not convert.
fn arithmetic_findings(
    node: &Expression,
    (left, right): (&Expression, &Expression),
    level: NodeLevel<'_, '_>,
) -> Vec<NameFinding> {
    let engine: Engine = level.engine();
    let (left_family, right_family) = (level.family(left), level.family(right));
    if left_family == Family::Unknown || right_family == Family::Unknown {
        return Vec::new();
    }
    if engine == Engine::BigQuery
        && let Some(refusal) = bigquery_arithmetic_refusal(node, (left, right), level)
    {
        return vec![refusal];
    }
    let literal_numeric: bool = !(engine == Engine::DuckDB && matches!(node, Expression::Add(_)))
        && ((left_family.is_numeric() && literal_coerces(engine, right, left_family))
            || (right_family.is_numeric() && literal_coerces(engine, left, right_family)));
    let mut findings: Vec<NameFinding> = Vec::new();
    if literal_numeric
        && (invalid_literal_for(left, right_family) || invalid_literal_for(right, left_family))
    {
        findings.push(finding(
            "W211",
            "Arithmetic literal conversion may fail at execution".to_owned(),
            node,
        ));
    }
    if !literal_numeric
        && !arithmetic(engine, node, (left_family, right_family))
        && !temporal_literal_arithmetic(engine, node, (left, right), (left_family, right_family))
    {
        findings.push(finding(
            "E212",
            format!(
                "Arithmetic operation expects numeric-compatible operands, found {} and {}",
                left_family.name(),
                right_family.name()
            ),
            node,
        ));
    }
    findings
}

/// DuckDB's keyword-only functions called with parentheses, which DuckDB does not accept.
fn keyword_only_call(
    function: &sqlquack_syntax::expressions::functions::models::Function,
    engine: Engine,
    node: &Expression,
) -> Vec<NameFinding> {
    let base: &str = function
        .name
        .rsplit('.')
        .next()
        .unwrap_or(&function.name)
        .trim();
    match engine == Engine::DuckDB
        && !function.no_parens
        && DUCKDB_KEYWORD_ONLY_FUNCTIONS
            .iter()
            .any(|keyword| base.eq_ignore_ascii_case(keyword))
    {
        true => vec![issue(
            ("E202", NAME_PHASE),
            format!("Unknown function '{}' for dialect DuckDB", function.name),
            None,
            node,
        )],
        false => Vec::new(),
    }
}

/// A function the signatures do not record, whose name still shows what its arguments take.
fn generic_function(
    function: &sqlquack_syntax::expressions::functions::models::Function,
    level: NodeLevel<'_, '_>,
    node: &Expression,
) -> Vec<NameFinding> {
    const NUMERIC: &str = "a numeric argument";
    const STRING: &str = "a string argument";
    let name: String = dispatch_name(&function.name);
    let positions: &[(usize, &str)] = match name.as_str() {
        "abs" | "sqrt" | "cbrt" | "ln" | "exp" => &[(0, NUMERIC)],
        "round" | "floor" | "ceil" | "ceiling" | "power" | "pow" => &[(0, NUMERIC), (1, NUMERIC)],
        "length" | "char_length" | "character_length" => &[(0, "a string or binary argument")],
        "upper" | "lower" | "trim" | "ltrim" | "rtrim" | "reverse" => &[(0, STRING)],
        "substring" | "substr" => &[(0, STRING), (1, NUMERIC), (2, NUMERIC)],
        "replace" => &[(0, STRING), (1, STRING), (2, STRING)],
        "left" | "right" | "repeat" => &[(0, STRING), (1, NUMERIC)],
        "lpad" | "rpad" => &[(0, STRING), (1, NUMERIC), (2, STRING)],
        _ => &[],
    };
    positions
        .iter()
        .filter_map(|(index, expected)| {
            let argument: &Expression = function.args.get(*index)?;
            let found: Family = level.family(argument);
            let valid: bool = match *expected {
                NUMERIC => found.is_numeric(),
                STRING => found == Family::String,
                _ => matches!(found, Family::String | Family::Binary),
            };
            argument_finding((&name, *index), (found, expected), valid, node)
        })
        .collect()
}

/// One argument a typed function's checks read: function, position, value and what it takes.
struct TypedArgument<'e> {
    function: &'static str,
    index: usize,
    argument: &'e Expression,
    expected: &'static str,
}

fn typed<'e>(
    (function, index): (&'static str, usize),
    argument: &'e Expression,
    expected: &'static str,
) -> TypedArgument<'e> {
    TypedArgument {
        function,
        index,
        argument,
        expected,
    }
}

/// Typed string and numeric functions the signatures do not record.
fn typed_function_checks(node: &Expression, level: NodeLevel<'_, '_>) -> Vec<NameFinding> {
    const NUMERIC: &str = "a numeric argument";
    const STRING: &str = "a string argument";
    let arguments: Vec<TypedArgument<'_>> = match node {
        Expression::Reverse(function) => {
            vec![typed(("string_function", 0), &function.this, STRING)]
        }
        Expression::Repeat(function) => vec![
            typed(("repeat", 0), &function.this, STRING),
            typed(("repeat", 1), &function.times, NUMERIC),
        ],
        Expression::Lpad(function) | Expression::Rpad(function) => {
            let mut arguments: Vec<TypedArgument<'_>> = vec![
                typed(("pad", 0), &function.this, STRING),
                typed(("pad", 1), &function.length, NUMERIC),
            ];
            arguments.extend(
                function
                    .fill
                    .iter()
                    .map(|fill| typed(("pad", 2), fill, STRING)),
            );
            arguments
        }
        Expression::Cbrt(function) | Expression::Ln(function) | Expression::Exp(function) => {
            vec![typed(("numeric_function", 0), &function.this, NUMERIC)]
        }
        Expression::Log(function) => {
            let mut arguments: Vec<TypedArgument<'_>> =
                vec![typed(("log", 0), &function.this, NUMERIC)];
            arguments.extend(
                function
                    .base
                    .iter()
                    .map(|base| typed(("log", 1), base, NUMERIC)),
            );
            arguments
        }
        _ => Vec::new(),
    };
    arguments
        .into_iter()
        .filter_map(|checked| {
            let found: Family = level.family(checked.argument);
            let valid: bool = match checked.expected {
                NUMERIC => found.is_numeric(),
                _ => found == Family::String,
            };
            argument_finding(
                (checked.function, checked.index),
                (found, checked.expected),
                valid,
                node,
            )
        })
        .collect()
}
