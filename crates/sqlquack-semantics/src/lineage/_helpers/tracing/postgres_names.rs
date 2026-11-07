//! PostgreSQL's names for unaliased outputs, as its `FigureColname` gives them.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::typed_functions::models::TrimPosition;
use sqlquack_syntax::expressions::types::models::DataType;

use crate::lineage::_helpers::checks::signatures::call;

/// The name PostgreSQL gives every output it cannot otherwise name.
const UNNAMED: &str = "?column?";

/// How sure a name is: a type name yields to any operand name, as in `FigureColnameInternal`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Strength {
    None,
    TypeName,
    Named,
}

/// The output name PostgreSQL 18 gives the unaliased projection `expression`.
pub(crate) fn postgres_name(expression: &Expression) -> String {
    match figured(expression) {
        (Strength::None, _) => UNNAMED.to_owned(),
        (_, name) => name,
    }
}

/// The internal type name a cast target carries, as gram.y names built-in types.
fn type_name(data_type: &DataType) -> Option<String> {
    let name: &str = match data_type {
        DataType::Boolean => "bool",
        DataType::SmallInt { .. } => "int2",
        DataType::Int { .. } => "int4",
        DataType::BigInt { .. } => "int8",
        DataType::Float { .. } => "float4",
        DataType::Double { .. } => "float8",
        DataType::Decimal { .. } => "numeric",
        DataType::Char { .. } => "bpchar",
        DataType::VarChar { .. } => "varchar",
        DataType::Text => "text",
        DataType::Bit { .. } => "bit",
        DataType::VarBit { .. } => "varbit",
        DataType::Date => "date",
        DataType::Time {
            timezone: false, ..
        } => "time",
        DataType::Time { timezone: true, .. } => "timetz",
        DataType::Timestamp {
            timezone: false, ..
        } => "timestamp",
        DataType::Timestamp { timezone: true, .. } => "timestamptz",
        DataType::Interval { .. } => "interval",
        DataType::Json => "json",
        DataType::JsonB => "jsonb",
        DataType::Uuid => "uuid",
        DataType::Array { element_type, .. } => return type_name(element_type),
        DataType::Custom { name } => return name.rsplit('.').next().map(str::to_owned),
        _ => return None,
    };
    Some(name.to_owned())
}

fn named(name: &str) -> (Strength, String) {
    (Strength::Named, name.to_owned())
}

/// A function's own name: its last qualified part, unquoted names as gram.y folds them.
fn function_name(name: &str, quoted: bool) -> String {
    let last: &str = name.rsplit('.').next().unwrap_or(name);
    match quoted {
        true => last.to_owned(),
        false => last.to_lowercase(),
    }
}

/// The name of a query's first output, as an expression sublink takes it.
fn first_output(query: &Expression) -> (Strength, String) {
    match query {
        Expression::Select(select) => match select.expressions.first() {
            Some(Expression::Alias(alias)) => named(&alias.alias.name),
            Some(first) => (Strength::Named, postgres_name(first)),
            None => (Strength::None, String::new()),
        },
        Expression::Union(node) => first_output(&node.left),
        Expression::Intersect(node) => first_output(&node.left),
        Expression::Except(node) => first_output(&node.left),
        Expression::Subquery(node) => first_output(&node.this),
        Expression::Paren(node) => first_output(&node.this),
        _ => (Strength::None, String::new()),
    }
}

/// `FigureColnameInternal`: a name and how sure it is.
fn figured(expression: &Expression) -> (Strength, String) {
    match expression {
        Expression::Column(column) => named(&column.name.name),
        Expression::Identifier(identifier) => named(&identifier.name),
        Expression::Dot(dot) => named(&dot.field.name),
        Expression::Paren(paren) => figured(&paren.this),
        Expression::Annotated(annotated) => figured(&annotated.this),
        Expression::Collation(collation) => figured(&collation.this),
        Expression::Subscript(subscript) => figured(&subscript.this),
        Expression::WindowFunction(window) => figured(&window.this),
        Expression::Function(function) => (
            Strength::Named,
            function_name(&function.name, function.quoted),
        ),
        Expression::AggregateFunction(function) => {
            (Strength::Named, function_name(&function.name, false))
        }
        Expression::Cast(cast) => match figured(&cast.this) {
            found @ (Strength::Named, _) => found,
            other => type_name(&cast.to).map_or(other, |name| (Strength::TypeName, name)),
        },
        Expression::Case(case) => match case.else_.as_ref().map(figured) {
            Some(found @ (Strength::Named, _)) => found,
            _ => (Strength::TypeName, "case".to_owned()),
        },
        Expression::Subquery(subquery) => first_output(&subquery.this),
        Expression::Exists(_) => named("exists"),
        Expression::ArrayFunc(_) => named("array"),
        Expression::Tuple(_) => named("row"),
        _ => typed(expression),
    }
}

/// The names of the typed nodes a parse makes of built-in calls and literals.
fn typed(expression: &Expression) -> (Strength, String) {
    match expression {
        Expression::Trim(trim) => named(match trim.position {
            TrimPosition::Both => "btrim",
            TrimPosition::Leading => "ltrim",
            TrimPosition::Trailing => "rtrim",
        }),
        Expression::StrPosition(_) => named("position"),
        Expression::Extract(_) => named("extract"),
        Expression::Substring(_) => named("substring"),
        Expression::Count(_) => named("count"),
        Expression::RowNumber(_) => named("row_number"),
        Expression::Rank(_) => named("rank"),
        Expression::DenseRank(_) => named("dense_rank"),
        Expression::PercentRank(_) => named("percent_rank"),
        Expression::CumeDist(_) => named("cume_dist"),
        Expression::NTile(_) => named("ntile"),
        Expression::FirstValue(_) => named("first_value"),
        Expression::LastValue(_) => named("last_value"),
        Expression::NthValue(_) => named("nth_value"),
        Expression::Interval(_) => (Strength::TypeName, "interval".to_owned()),
        Expression::MethodCall(method) => named(&method.method.name),
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::Date(_) => (Strength::TypeName, "date".to_owned()),
            Literal::Time(_) => (Strength::TypeName, "time".to_owned()),
            Literal::Timestamp(_) => (Strength::TypeName, "timestamp".to_owned()),
            _ => (Strength::None, String::new()),
        },
        _ => match call(expression) {
            Some((name, _)) => (Strength::Named, function_name(&name, false)),
            None => (Strength::None, String::new()),
        },
    }
}
