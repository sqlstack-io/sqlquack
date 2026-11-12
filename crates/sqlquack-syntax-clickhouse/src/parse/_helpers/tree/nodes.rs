//! Shared-tree nodes with every field set, so the grammar code names only what it parsed.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::functions::models::{
    AggregateFunction, Function, WindowFunction,
};
use sqlquack_syntax::expressions::names::models::{
    BooleanLiteral, Column, Identifier, Literal, Null, Star, TableRef,
};
use sqlquack_syntax::expressions::operators::models::{
    Alias, Between, BinaryOp, Case, Cast, DotAccess, Exists, In, IsNull, LambdaExpr, Parameter,
    ParameterStyle,
};
use sqlquack_syntax::expressions::queries::models::{
    Cte, Except, Intersect, Join, JoinKind, Ordered, Over, Subquery, Union, WindowFrame,
};
use sqlquack_syntax::expressions::statements::models::{Command, Delete, Insert, Paren, Var};
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::SourceSpan;

/// A column reference, its range running from any qualifier through its name.
pub(crate) fn column(table: Option<Identifier>, name: Identifier) -> Expression {
    let span = match (table.as_ref().and_then(|table| table.span), name.span) {
        (Some(table), Some(name)) => Some(table.through(name)),
        (_, name) => name,
    };
    Expression::Column(Box::new(Column {
        name,
        table,
        join_mark: false,
        trailing_comments: Vec::new(),
        span,
        inferred_type: None,
    }))
}

/// A column reference from its dotted names: `a`, `t.a`, or a field of `s.t.a` as dot access.
pub(crate) fn column_from(parts: Vec<Identifier>) -> Expression {
    let mut parts = parts.into_iter();
    let first: Identifier = parts.next().unwrap_or_else(Identifier::empty);
    let Some(second) = parts.next() else {
        return column(None, first);
    };
    parts.fold(column(Some(first), second), dot)
}

pub(crate) fn dot(this: Expression, field: Identifier) -> Expression {
    Expression::Dot(Box::new(DotAccess {
        this,
        field,
        inferred_type: None,
    }))
}

pub(crate) fn star(table: Option<Identifier>, except: Option<Vec<Identifier>>) -> Star {
    Star {
        table,
        ilike: None,
        except,
        replace: None,
        rename: None,
        trailing_comments: Vec::new(),
        span: SourceSpan::default(),
        qualified: None,
    }
}

/// A multi-part name as one identifier: the part itself, or the parts joined by `.`.
pub(crate) fn joined(mut parts: Vec<Identifier>) -> Identifier {
    match parts.len() {
        1 => parts.remove(0),
        _ => Identifier::new(
            parts
                .iter()
                .map(|part| part.name.as_str())
                .collect::<Vec<&str>>()
                .join("."),
        ),
    }
}

/// `?` (no name) or `:name`.
pub(crate) fn parameter(name: Option<String>) -> Expression {
    let style: ParameterStyle = match name {
        Some(_) => ParameterStyle::Colon,
        None => ParameterStyle::Question,
    };
    Expression::Parameter(Box::new(Parameter {
        name,
        index: None,
        style,
        quoted: false,
        string_quoted: false,
        expression: None,
        span: SourceSpan::default(),
    }))
}

pub(crate) fn lambda(parameters: Vec<Identifier>, body: Expression) -> Expression {
    let parameter_types = vec![None; parameters.len()];
    Expression::Lambda(Box::new(LambdaExpr {
        parameters,
        body,
        colon: false,
        parameter_types,
    }))
}

/// A keyword written as a word in a keyword tuple, such as `DELETE` or `UPDATE`.
pub(crate) fn var(text: &str) -> Expression {
    Expression::Var(Box::new(Var {
        this: text.to_owned(),
    }))
}

pub(crate) fn literal(value: Literal) -> Expression {
    Expression::Literal(Box::new(value))
}

pub(crate) fn string(text: &str) -> Expression {
    literal(Literal::String(text.to_owned()))
}

pub(crate) fn boolean(value: bool) -> Expression {
    Expression::Boolean(BooleanLiteral { value })
}

pub(crate) fn null() -> Expression {
    Expression::Null(Null)
}

pub(crate) fn binary(
    kind: fn(Box<BinaryOp>) -> Expression,
    left: Expression,
    right: Expression,
) -> Expression {
    kind(Box::new(BinaryOp::new(left, right)))
}

pub(crate) fn cast(this: Expression, to: DataType, double_colon_syntax: bool) -> Cast {
    Cast {
        this,
        to,
        trailing_comments: Vec::new(),
        double_colon_syntax,
        typed_literal: false,
        format: None,
        default: None,
        inferred_type: None,
        span: SourceSpan(None),
    }
}

pub(crate) fn case(
    operand: Option<Expression>,
    whens: Vec<(Expression, Expression)>,
    else_: Option<Expression>,
) -> Expression {
    Expression::Case(Box::new(Case {
        operand,
        whens,
        else_,
        comments: Vec::new(),
        inferred_type: None,
        span: SourceSpan(None),
    }))
}

pub(crate) fn in_list(
    this: Expression,
    expressions: Vec<Expression>,
    query: Option<Expression>,
    not: bool,
) -> Expression {
    Expression::In(Box::new(In {
        this,
        expressions,
        query,
        not,
        global: false,
        unnest: None,
        is_field: false,
        span: SourceSpan(None),
    }))
}

pub(crate) fn between(
    this: Expression,
    low: Expression,
    high: Expression,
    not: bool,
) -> Expression {
    Expression::Between(Box::new(Between {
        this,
        low,
        high,
        not,
        symmetric: None,
        span: SourceSpan(None),
    }))
}

pub(crate) fn null_test(this: Expression, not: bool) -> Expression {
    Expression::IsNull(Box::new(IsNull {
        this,
        not,
        postfix_form: false,
        span: SourceSpan(None),
    }))
}

pub(crate) fn exists(this: Expression) -> Expression {
    Expression::Exists(Box::new(Exists {
        this,
        not: false,
        span: SourceSpan(None),
    }))
}

pub(crate) fn paren(this: Expression) -> Expression {
    Expression::Paren(Box::new(Paren {
        this,
        trailing_comments: Vec::new(),
        span: SourceSpan(None),
    }))
}

pub(crate) fn tuple(expressions: Vec<Expression>) -> Expression {
    Expression::Tuple(Box::new(Tuple { expressions }))
}

pub(crate) fn alias(this: Expression, name: Identifier, explicit: bool) -> Expression {
    let mut node: Alias = Alias::new(this, name);
    node.alias_explicit_as = explicit;
    Expression::Alias(Box::new(node))
}

/// A function call; a qualified name keeps its parts and joins them into `name`.
pub(crate) fn function(name: Vec<Identifier>, args: Vec<Expression>) -> Function {
    let joined: String = name
        .iter()
        .map(|part| part.name.as_str())
        .collect::<Vec<&str>>()
        .join(".");
    let quoted: bool = name.iter().any(|part| part.quoted);
    Function {
        qualified_name: match name.len() {
            1 => Vec::new(),
            _ => name,
        },
        name: joined,
        args,
        quoted,
        ..Function::default()
    }
}

/// A call of a built-in written without a name list, such as `CURRENT_DATE`.
pub(crate) fn call(name: &str, args: Vec<Expression>) -> Expression {
    Expression::Function(Box::new(function(vec![Identifier::new(name)], args)))
}

pub(crate) fn aggregate(
    function: Function,
    distinct: bool,
    filter: Option<Expression>,
    ignore_nulls: Option<bool>,
) -> Expression {
    let name: String = match function.qualified_name.is_empty() {
        true => function.name,
        false => function
            .qualified_name
            .iter()
            .map(|part| part.name.as_str())
            .collect::<Vec<&str>>()
            .join("."),
    };
    Expression::AggregateFunction(Box::new(AggregateFunction {
        name,
        args: function.args,
        distinct,
        filter,
        ignore_nulls,
        ..AggregateFunction::default()
    }))
}

pub(crate) fn window_function(this: Expression, over: Over) -> Expression {
    Expression::WindowFunction(Box::new(WindowFunction {
        this,
        over,
        keep: None,
        inferred_type: None,
        span: SourceSpan(None),
    }))
}

pub(crate) fn over(
    window_name: Option<Identifier>,
    partition_by: Vec<Expression>,
    order_by: Vec<Ordered>,
    frame: Option<WindowFrame>,
) -> Over {
    Over {
        window_name,
        partition_by,
        order_by,
        frame,
        alias: None,
    }
}

pub(crate) fn ordered(
    this: Expression,
    desc: bool,
    explicit_asc: bool,
    nulls_first: Option<bool>,
) -> Ordered {
    Ordered {
        nulls_auto: false,
        this,
        desc,
        nulls_first,
        explicit_asc,
        with_fill: None,
    }
}

/// A table from its qualified name; parts before `catalog.schema.name` join the catalog as one dotted name.
pub(crate) fn table(mut name: Vec<Identifier>) -> TableRef {
    let last: Identifier = name.pop().unwrap_or_else(Identifier::empty);
    let span = last.span;
    let mut table: TableRef = TableRef::new(String::new());
    table.name = last;
    table.span = span;
    table.schema = name.pop();
    let catalog: Option<Identifier> = name.pop();
    table.catalog = match (catalog, name.is_empty()) {
        (Some(catalog), false) => {
            name.push(catalog);
            let joined: String = name
                .iter()
                .map(|part| part.name.as_str())
                .collect::<Vec<&str>>()
                .join(".");
            Some(Identifier::new(joined))
        }
        (catalog, _) => catalog,
    };
    table
}

/// A table alias: its name, its column names, and whether `AS` was written.
pub(crate) struct TableAlias {
    pub(crate) name: Identifier,
    pub(crate) columns: Vec<Identifier>,
    pub(crate) explicit: bool,
}

/// What follows a join's right side: nothing, `ON condition`, or `USING (columns)`.
pub(crate) enum JoinConstraint {
    None,
    On(Box<Expression>),
    Using(Vec<Identifier>),
}

pub(crate) fn subquery(this: Expression, alias: Option<TableAlias>, lateral: bool) -> Expression {
    let (alias, column_aliases, alias_explicit_as) = match alias {
        Some(alias) => (Some(alias.name), alias.columns, alias.explicit),
        None => (None, Vec::new(), false),
    };
    Expression::Subquery(Box::new(Subquery {
        this,
        alias,
        column_aliases,
        alias_explicit_as,
        alias_keyword: None,
        order_by: None,
        limit: None,
        offset: None,
        distribute_by: None,
        sort_by: None,
        cluster_by: None,
        lateral,
        modifiers_inside: false,
        trailing_comments: Vec::new(),
        inferred_type: None,
        span: SourceSpan(None),
    }))
}

/// A join of `kind`, with `keywords` as whether `INNER` and `OUTER` were written.
pub(crate) fn join(
    this: Expression,
    kind: JoinKind,
    constraint: JoinConstraint,
    keywords: (bool, bool),
) -> Join {
    let (on, using) = match constraint {
        JoinConstraint::None => (None, Vec::new()),
        JoinConstraint::On(condition) => (Some(*condition), Vec::new()),
        JoinConstraint::Using(columns) => (None, columns),
    };
    Join {
        this,
        on,
        using,
        kind,
        use_inner_keyword: keywords.0,
        use_outer_keyword: keywords.1,
        deferred_condition: false,
        join_hint: None,
        match_condition: None,
        pivots: Vec::new(),
        comments: Vec::new(),
        nesting_group: 0,
        directed: false,
        first_last: None,
        first_last_alias: None,
        using_alias: None,
        span: SourceSpan(None),
    }
}

/// UNION, INTERSECT or EXCEPT of `left` and `right`, with `quantifier` as (`ALL`, `DISTINCT`).
pub(crate) fn set_operation(
    operator: &str,
    left: Expression,
    right: Expression,
    quantifier: (bool, bool),
) -> Expression {
    let (all, distinct) = quantifier;
    macro_rules! node {
        ($variant:ident, $node:ident) => {
            Expression::$variant(Box::new($node {
                left,
                right,
                all,
                distinct,
                with: None,
                order_by: None,
                limit: None,
                offset: None,
                distribute_by: None,
                sort_by: None,
                cluster_by: None,
                by_name: false,
                side: None,
                kind: None,
                corresponding: false,
                strict: false,
                on_columns: Vec::new(),
                locks: Vec::new(),
                with_ties: false,
                span: SourceSpan(None),
            }))
        };
    }
    match operator {
        "INTERSECT" => node!(Intersect, Intersect),
        "EXCEPT" | "MINUS" => node!(Except, Except),
        _ => node!(Union, Union),
    }
}

pub(crate) fn cte(alias: Identifier, columns: Vec<Identifier>, this: Expression) -> Cte {
    Cte {
        alias,
        this,
        columns,
        materialized: None,
        key_expressions: Vec::new(),
        alias_first: true,
        comments: Vec::new(),
        search_cycle: Vec::new(),
        span: SourceSpan(None),
        key_items: Vec::new(),
    }
}

pub(crate) fn command(text: &str) -> Expression {
    Expression::Command(Box::new(Command {
        this: text.to_owned(),
    }))
}

pub(crate) fn insert(table: TableRef) -> Insert {
    Insert {
        table,
        columns: Vec::new(),
        values: Vec::new(),
        query: None,
        overwrite: false,
        partition: Vec::new(),
        directory: None,
        returning: Vec::new(),
        output: None,
        on_conflict: None,
        leading_comments: Vec::new(),
        if_exists: false,
        with: None,
        ignore: false,
        source_alias: None,
        alias: None,
        alias_explicit_as: false,
        default_values: false,
        by_name: false,
        conflict_action: None,
        is_replace: false,
        hint: None,
        replace_where: None,
        source: None,
        function_target: None,
        partition_by: None,
        overriding: None,
        returning_aliases: Vec::new(),
        column_targets: Vec::new(),
        settings: Vec::new(),
    }
}

pub(crate) fn delete(table: TableRef) -> Delete {
    Delete {
        table,
        hint: None,
        on_cluster: None,
        alias: None,
        alias_explicit_as: false,
        using: Vec::new(),
        where_clause: None,
        output: None,
        leading_comments: Vec::new(),
        with: None,
        limit: None,
        order_by: None,
        returning: Vec::new(),
        tables: Vec::new(),
        tables_from_using: false,
        joins: Vec::new(),
        force_index: None,
        no_from: false,
        current_of: None,
        returning_aliases: Vec::new(),
        using_items: Vec::new(),
        using_joins: Vec::new(),
        settings: Vec::new(),
    }
}
