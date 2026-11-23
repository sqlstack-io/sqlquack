//! `values_query`: a VALUES statement as the query that carries a WITH clause.

use crate::expressions::queries::models::{From, Select, Subquery};
use crate::expressions::tree::models::Expression;
use crate::tokens::models::SourceSpan;

/// `values` as `SELECT * FROM (VALUES ...)`, the shape PostgreSQL's parser gives a VALUES statement after WITH.
pub fn values_query(values: Expression) -> Select {
    let mut select = Select::new();
    select.expressions = vec![Expression::star()];
    select.from = Some(From {
        keyword_span: Default::default(),
        comma_spans: Default::default(),
        expressions: vec![Expression::Subquery(Box::new(Subquery {
            row_limit_span: Default::default(),
            this: values,
            alias: None,
            column_aliases: Vec::new(),
            alias_explicit_as: false,
            alias_keyword: None,
            order_by: None,
            limit: None,
            offset: None,
            distribute_by: None,
            sort_by: None,
            cluster_by: None,
            lateral: false,
            modifiers_inside: false,
            trailing_comments: Vec::new(),
            inferred_type: None,
            span: SourceSpan::default(),
            table_sample: None,
            watermark: None,
            ctes_outside: false,
            windows: None,
        }))],
    });
    select
}
