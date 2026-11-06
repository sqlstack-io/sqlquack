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
    Between, BinaryOp, Case, Cast, Exists, In, IsNull, Lateral, Parameter, ParameterStyle,
};
use sqlquack_syntax::expressions::queries::models::{
    Cte, Except, Intersect, Join, JoinKind, JoinedTable, Ordered, Over, Subquery, Union, Values,
    WindowFrame,
};
use sqlquack_syntax::expressions::statements::models::{Command, Delete, Insert, Paren, Update};
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::SourceSpan;

pub(crate) fn column(table: Option<Identifier>, name: Identifier) -> Expression {
    Expression::Column(Box::new(Column {
        name,
        table,
        join_mark: false,
        trailing_comments: Vec::new(),
        span: None,
        inferred_type: None,
    }))
}

pub(crate) fn star(table: Option<Identifier>) -> Expression {
    Expression::Star(Star {
        table,
        ilike: None,
        except: None,
        replace: None,
        rename: None,
        trailing_comments: Vec::new(),
        span: None,
    })
}

pub(crate) fn literal(value: Literal) -> Expression {
    Expression::Literal(Box::new(value))
}

pub(crate) fn boolean(value: bool) -> Expression {
    Expression::Boolean(BooleanLiteral { value })
}

pub(crate) fn null() -> Expression {
    Expression::Null(Null)
}

pub(crate) fn parameter(index: u32) -> Expression {
    Expression::Parameter(Box::new(Parameter {
        name: None,
        index: Some(index),
        style: ParameterStyle::Dollar,
        quoted: false,
        string_quoted: false,
        expression: None,
    }))
}

pub(crate) fn binary(
    kind: fn(Box<BinaryOp>) -> Expression,
    left: Expression,
    right: Expression,
) -> Expression {
    kind(Box::new(BinaryOp::new(left, right)))
}

pub(crate) fn cast(this: Expression, to: DataType, double_colon_syntax: bool) -> Expression {
    Expression::Cast(Box::new(Cast {
        this,
        to,
        trailing_comments: Vec::new(),
        double_colon_syntax,
        format: None,
        default: None,
        inferred_type: None,
        span: SourceSpan(None),
    }))
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

/// `this [NOT] BETWEEN [SYMMETRIC] low AND high`, with `bounds` as `(low, high)`.
pub(crate) fn between(
    this: Expression,
    bounds: (Expression, Expression),
    not: bool,
    symmetric: Option<bool>,
) -> Expression {
    let (low, high) = bounds;
    Expression::Between(Box::new(Between {
        this,
        low,
        high,
        not,
        symmetric,
        span: SourceSpan(None),
    }))
}

pub(crate) fn null_test(this: Expression, not: bool, postfix_form: bool) -> Expression {
    Expression::IsNull(Box::new(IsNull {
        this,
        not,
        postfix_form,
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

pub(crate) fn aggregate(
    function: Function,
    distinct: bool,
    filter: Option<Expression>,
    order_by: Vec<Ordered>,
) -> Expression {
    Expression::AggregateFunction(Box::new(AggregateFunction {
        name: function.name,
        args: function.args,
        distinct,
        filter,
        order_by,
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

/// A table from its qualified name: `name`, `schema.name` or `catalog.schema.name`.
pub(crate) fn table(mut name: Vec<Identifier>) -> TableRef {
    let last: Identifier = name.pop().unwrap_or_else(Identifier::empty);
    let mut table: TableRef = TableRef::new(String::new());
    table.name = last;
    table.schema = name.pop();
    table.catalog = name.pop();
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

pub(crate) fn lateral(
    this: Expression,
    alias: Option<Identifier>,
    columns: Vec<Identifier>,
) -> Expression {
    Expression::Lateral(Box::new(Lateral {
        this: Box::new(this),
        view: None,
        outer: None,
        alias_quoted: alias.as_ref().is_some_and(|alias| alias.quoted),
        alias: alias.map(|alias| alias.name),
        cross_apply: None,
        ordinality: None,
        column_aliases_quoted: columns.iter().map(|column| column.quoted).collect(),
        column_aliases: columns.into_iter().map(|column| column.name).collect(),
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
        span: SourceSpan(None),
    }
}

pub(crate) fn joined_table(
    left: Expression,
    joins: Vec<Join>,
    alias: Option<Identifier>,
) -> Expression {
    Expression::JoinedTable(Box::new(JoinedTable {
        left,
        joins,
        lateral_views: Vec::new(),
        alias,
    }))
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
                span: SourceSpan(None),
            }))
        };
    }
    match operator {
        "intersect" => node!(Intersect, Intersect),
        "except" => node!(Except, Except),
        _ => node!(Union, Union),
    }
}

pub(crate) fn values(rows: Vec<Vec<Expression>>) -> Expression {
    Expression::Values(Box::new(Values {
        expressions: rows
            .into_iter()
            .map(|expressions| Tuple { expressions })
            .collect(),
        alias: None,
        column_aliases: Vec::new(),
    }))
}

pub(crate) fn cte(
    alias: Identifier,
    columns: Vec<Identifier>,
    this: Expression,
    materialized: Option<bool>,
) -> Cte {
    Cte {
        alias,
        this,
        columns,
        materialized,
        key_expressions: Vec::new(),
        alias_first: true,
        comments: Vec::new(),
        span: SourceSpan(None),
    }
}

pub(crate) fn command(text: &str) -> Expression {
    Expression::Command(Box::new(Command {
        this: text.to_owned(),
    }))
}

pub(crate) fn insert(table: TableRef, columns: Vec<Identifier>) -> Insert {
    Insert {
        table,
        columns,
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
        settings: Vec::new(),
    }
}

pub(crate) fn update(table: TableRef) -> Update {
    Update {
        table,
        hint: None,
        extra_tables: Vec::new(),
        table_joins: Vec::new(),
        set: Vec::new(),
        from_clause: None,
        from_joins: Vec::new(),
        where_clause: None,
        returning: Vec::new(),
        output: None,
        with: None,
        leading_comments: Vec::new(),
        limit: None,
        order_by: None,
        from_before_set: false,
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
    }
}
