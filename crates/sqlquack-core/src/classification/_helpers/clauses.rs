//! Clauses, join kinds and set-operation forms, from exhaustively destructured query nodes.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::queries::models::Except;
use sqlquack_syntax::expressions::queries::models::Intersect;
use sqlquack_syntax::expressions::queries::models::Join;
use sqlquack_syntax::expressions::queries::models::Select;
use sqlquack_syntax::expressions::queries::models::Union;

use crate::classification::_helpers::names::snake_case;
use crate::classification::models::{Construct, ConstructFamily};

pub(crate) fn clause_constructs(node: &Expression) -> Vec<Construct> {
    let names = match node {
        Expression::Select(select) => select_clauses(select),
        Expression::Union(union) => union_clauses(union),
        Expression::Intersect(intersect) => intersect_clauses(intersect),
        Expression::Except(except) => except_clauses(except),
        _ => Vec::new(),
    };
    let mut constructs: Vec<Construct> = names
        .into_iter()
        .map(|name| Construct::new(ConstructFamily::Clause, name))
        .collect();
    if let Expression::Select(select) = node {
        constructs.extend(select.joins.iter().flat_map(join_constructs));
    }
    constructs
}

fn present(flags: &[(bool, &str)]) -> Vec<String> {
    flags
        .iter()
        .filter(|(present, _)| *present)
        .map(|(_, name)| (*name).to_owned())
        .collect()
}

fn select_clauses(select: &Select) -> Vec<String> {
    let Select {
        expressions: _,
        from: _,
        joins: _,
        lateral_views,
        prewhere,
        where_clause,
        group_by,
        having,
        qualify,
        order_by,
        distribute_by,
        cluster_by,
        sort_by,
        limit,
        offset,
        limit_by,
        fetch,
        distinct,
        all: _,
        distinct_on,
        top,
        with,
        sample,
        settings,
        format,
        windows,
        hint,
        connect,
        into,
        locks,
        for_xml,
        for_json,
        leading_comments: _,
        post_select_comments: _,
        kind,
        operation_modifiers,
        qualify_after_window: _,
        option,
        exclude,
        vertica,
        result_serialization,
        query_collation,
        query_hints,
        total_rowcount,
        source_dialect: _,
        span: _,
    } = select;
    present(&[
        (with.is_some(), "with"),
        (*distinct, "distinct"),
        (distinct_on.is_some(), "distinct_on"),
        (top.is_some(), "top"),
        (!lateral_views.is_empty(), "lateral_view"),
        (prewhere.is_some(), "prewhere"),
        (where_clause.is_some(), "where"),
        (group_by.is_some(), "group_by"),
        (having.is_some(), "having"),
        (windows.is_some(), "window"),
        (qualify.is_some(), "qualify"),
        (connect.is_some(), "connect_by"),
        (order_by.is_some(), "order_by"),
        (distribute_by.is_some(), "distribute_by"),
        (cluster_by.is_some(), "cluster_by"),
        (sort_by.is_some(), "sort_by"),
        (limit.is_some(), "limit"),
        (limit_by.is_some(), "limit_by"),
        (offset.is_some(), "offset"),
        (fetch.is_some(), "fetch"),
        (sample.is_some(), "sample"),
        (settings.is_some(), "settings"),
        (format.is_some(), "format"),
        (hint.is_some(), "hint"),
        (into.is_some(), "into"),
        (!locks.is_empty(), "lock"),
        (!for_xml.is_empty(), "for_xml"),
        (!for_json.is_empty(), "for_json"),
        (kind.is_some(), "select_kind"),
        (!operation_modifiers.is_empty(), "operation_modifier"),
        (option.is_some(), "option"),
        (exclude.is_some(), "exclude"),
        (vertica.is_some(), "vertica"),
        (result_serialization.is_some(), "result_serialization"),
        (query_collation.is_some(), "query_collation"),
        (!query_hints.is_empty(), "query_hint"),
        (*total_rowcount, "total_rowcount"),
    ])
}

macro_rules! set_operation_clauses {
    ($function:ident, $type:ident, $prefix:literal) => {
        fn $function(operation: &$type) -> Vec<String> {
            let $type {
                left: _,
                right: _,
                all,
                distinct,
                with,
                order_by,
                limit,
                offset,
                distribute_by,
                sort_by,
                cluster_by,
                by_name,
                side,
                kind,
                corresponding,
                strict,
                on_columns,
                locks,
                with_ties,
                span: _,
            } = operation;
            present(&[
                (*all, concat!($prefix, "_all")),
                (*distinct, concat!($prefix, "_distinct")),
                (*by_name, concat!($prefix, "_by_name")),
                (side.is_some(), concat!($prefix, "_side")),
                (kind.is_some(), concat!($prefix, "_kind")),
                (*corresponding, concat!($prefix, "_corresponding")),
                (*strict, concat!($prefix, "_strict")),
                (!on_columns.is_empty(), concat!($prefix, "_on_columns")),
                (with.is_some(), "with"),
                (order_by.is_some(), "order_by"),
                (limit.is_some(), "limit"),
                (offset.is_some(), "offset"),
                (distribute_by.is_some(), "distribute_by"),
                (sort_by.is_some(), "sort_by"),
                (cluster_by.is_some(), "cluster_by"),
                (!locks.is_empty(), "lock"),
                (*with_ties, "with_ties"),
            ])
        }
    };
}

set_operation_clauses!(union_clauses, Union, "union");
set_operation_clauses!(intersect_clauses, Intersect, "intersect");
set_operation_clauses!(except_clauses, Except, "except");

fn join_constructs(join: &Join) -> Vec<Construct> {
    let Join {
        this: _,
        on: _,
        using,
        kind,
        use_inner_keyword: _,
        use_outer_keyword: _,
        deferred_condition: _,
        join_hint,
        match_condition,
        pivots,
        comments: _,
        nesting_group: _,
        directed,
        first_last,
        first_last_alias: _,
        using_alias,
        span: _,
    } = join;
    let mut constructs = vec![Construct::new(
        ConstructFamily::Join,
        snake_case(&format!("{kind:?}")),
    )];
    let names = present(&[
        (!using.is_empty(), "join_using"),
        (join_hint.is_some(), "join_hint"),
        (match_condition.is_some(), "match_condition"),
        (!pivots.is_empty(), "join_pivot"),
        (*directed, "join_directed"),
        (first_last.is_some(), "join_first_last"),
        (using_alias.is_some(), "join_using_alias"),
    ]);
    constructs.extend(
        names
            .into_iter()
            .map(|name| Construct::new(ConstructFamily::Clause, name)),
    );
    constructs
}
