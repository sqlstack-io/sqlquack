//! Read-only traversal and search over [`Expression`] trees.

#![allow(dead_code)]

use crate::expressions::{Expression, TableRef};
use std::collections::{HashMap, VecDeque};

/// Unique identifier for expression nodes during traversal
pub(crate) type NodeId = usize;

/// Information about a node's parent relationship
#[derive(Debug, Clone)]
pub(crate) struct ParentInfo {
    /// The NodeId of the parent (None for root)
    pub parent_id: Option<NodeId>,
    /// Which argument/field in the parent this node occupies
    pub arg_key: String,
    /// Index if the node is part of a list (e.g., expressions in SELECT)
    pub index: Option<usize>,
}

/// Parent tracking for an expression tree, built on demand since nodes hold no parents.
#[derive(Debug, Default)]
pub(crate) struct TreeContext {
    /// Map from NodeId to parent information
    nodes: HashMap<NodeId, ParentInfo>,
    /// Counter for generating NodeIds
    next_id: NodeId,
    /// Stack for tracking current path during traversal
    path: Vec<(NodeId, String, Option<usize>)>,
}

impl TreeContext {
    /// Create a new empty tree context
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Build context from an expression tree
    pub(crate) fn build(root: &Expression) -> Self {
        let mut ctx = Self::new();
        ctx.visit_expr(root);
        ctx
    }

    /// Visit an expression and record parent information
    fn visit_expr(&mut self, expr: &Expression) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;

        // Record parent info based on current path
        let parent_info = if let Some((parent_id, arg_key, index)) = self.path.last() {
            ParentInfo {
                parent_id: Some(*parent_id),
                arg_key: arg_key.clone(),
                index: *index,
            }
        } else {
            ParentInfo {
                parent_id: None,
                arg_key: String::new(),
                index: None,
            }
        };
        self.nodes.insert(id, parent_info);

        crate::ast_children::for_each_child(expr, |child_path, child| {
            let (key, index) = child_location(child_path);
            self.path.push((id, key, index));
            self.visit_expr(child);
            self.path.pop();
        });

        id
    }

    /// Get parent info for a node
    pub(crate) fn get(&self, id: NodeId) -> Option<&ParentInfo> {
        self.nodes.get(&id)
    }

    /// Get the depth of a node (0 for root)
    pub(crate) fn depth_of(&self, id: NodeId) -> usize {
        let mut depth = 0;
        let mut current = id;
        while let Some(info) = self.nodes.get(&current) {
            if let Some(parent_id) = info.parent_id {
                depth += 1;
                current = parent_id;
            } else {
                break;
            }
        }
        depth
    }

    /// Get ancestors of a node (parent, grandparent, etc.)
    pub(crate) fn ancestors_of(&self, id: NodeId) -> Vec<NodeId> {
        let mut ancestors: Vec<usize> = Vec::new();
        let mut current = id;
        while let Some(info) = self.nodes.get(&current) {
            if let Some(parent_id) = info.parent_id {
                ancestors.push(parent_id);
                current = parent_id;
            } else {
                break;
            }
        }
        ancestors
    }
}

fn child_location(path: &[crate::ast_children::ChildPathSegment]) -> (String, Option<usize>) {
    use crate::ast_children::ChildPathSegment;

    let mut key = String::new();
    let mut index = None;
    for (position, segment) in path.iter().enumerate() {
        match segment {
            ChildPathSegment::Field(field) => {
                if !key.is_empty() {
                    key.push('.');
                }
                key.push_str(field);
            }
            ChildPathSegment::Index(value) if position + 1 == path.len() => {
                index = Some(*value);
            }
            ChildPathSegment::Index(value) => {
                key.push('[');
                key.push_str(&value.to_string());
                key.push(']');
            }
        }
    }
    (key, index)
}

/// Pre-order depth-first iterator over an expression tree.
pub struct DfsIter<'a> {
    stack: Vec<&'a Expression>,
}

impl<'a> DfsIter<'a> {
    /// Create a new DFS iterator starting from the given expression
    pub fn new(root: &'a Expression) -> Self {
        Self { stack: vec![root] }
    }
}

impl<'a> Iterator for DfsIter<'a> {
    type Item = &'a Expression;

    fn next(&mut self) -> Option<Self::Item> {
        let expr = self.stack.pop()?;

        let child_start = self.stack.len();
        crate::ast_children::for_each_child_untracked(expr, |child| self.stack.push(child));
        self.stack[child_start..].reverse();

        Some(expr)
    }
}

/// Level-order breadth-first iterator over an expression tree.
pub struct BfsIter<'a> {
    queue: VecDeque<&'a Expression>,
}

impl<'a> BfsIter<'a> {
    /// Create a new BFS iterator starting from the given expression
    pub fn new(root: &'a Expression) -> Self {
        let mut queue: VecDeque<&Expression> = VecDeque::new();
        queue.push_back(root);
        Self { queue }
    }
}

impl<'a> Iterator for BfsIter<'a> {
    type Item = &'a Expression;

    fn next(&mut self) -> Option<Self::Item> {
        let expr = self.queue.pop_front()?;

        crate::ast_children::for_each_child_untracked(expr, |child| self.queue.push_back(child));

        Some(expr)
    }
}

/// Traversal and search methods on [`Expression`].
pub trait ExpressionWalk {
    /// A pre-order depth-first iterator over this expression and its descendants.
    fn dfs(&self) -> DfsIter<'_>;

    /// A level-order breadth-first iterator over this expression and its descendants.
    fn bfs(&self) -> BfsIter<'_>;

    /// The first expression matching `predicate`, depth first, this node included.
    fn find<F>(&self, predicate: F) -> Option<&Expression>
    where
        F: Fn(&Expression) -> bool;

    /// Every expression matching `predicate`, depth first.
    fn find_all<F>(&self, predicate: F) -> Vec<&Expression>
    where
        F: Fn(&Expression) -> bool;

    /// Returns `true` if this node or any descendant matches `predicate`.
    fn contains<F>(&self, predicate: F) -> bool
    where
        F: Fn(&Expression) -> bool;

    /// Counts how many nodes (including this one) match `predicate`.
    fn count<F>(&self, predicate: F) -> usize
    where
        F: Fn(&Expression) -> bool;

    /// This node's direct child expressions; leaves have none.
    fn children(&self) -> Vec<&Expression>;

    /// The depth of the tree rooted here; a leaf has depth 0.
    fn tree_depth(&self) -> usize;
}

impl ExpressionWalk for Expression {
    fn dfs(&self) -> DfsIter<'_> {
        DfsIter::new(self)
    }

    fn bfs(&self) -> BfsIter<'_> {
        BfsIter::new(self)
    }

    fn find<F>(&self, predicate: F) -> Option<&Expression>
    where
        F: Fn(&Expression) -> bool,
    {
        self.dfs().find(|e| predicate(e))
    }

    fn find_all<F>(&self, predicate: F) -> Vec<&Expression>
    where
        F: Fn(&Expression) -> bool,
    {
        self.dfs().filter(|e| predicate(e)).collect()
    }

    fn contains<F>(&self, predicate: F) -> bool
    where
        F: Fn(&Expression) -> bool,
    {
        self.dfs().any(predicate)
    }

    fn count<F>(&self, predicate: F) -> usize
    where
        F: Fn(&Expression) -> bool,
    {
        self.dfs().filter(|e| predicate(e)).count()
    }

    fn children(&self) -> Vec<&Expression> {
        let mut result: Vec<&Expression> = Vec::new();
        crate::ast_children::for_each_child(self, |_, child| result.push(child));
        result
    }

    fn tree_depth(&self) -> usize {
        let mut max_depth = 0usize;
        let mut stack = vec![(self, 0usize)];
        while let Some((node, depth)) = stack.pop() {
            max_depth = max_depth.max(depth);
            crate::ast_children::for_each_child(node, |_, child| {
                stack.push((child, depth + 1));
            });
        }
        max_depth
    }
}

// Shared stack engine. Dialect rewrites retain their selective dispatch and
// wrapper semantics; general AST rewrites use the generated visitor everywhere.

// ---------------------------------------------------------------------------
// Common expression predicates
// ---------------------------------------------------------------------------
// These free functions are intended for use with the search methods on
// `ExpressionWalk` (e.g., `expr.find(is_column)`, `expr.contains(is_aggregate)`).

/// Returns `true` if `expr` is a column reference ([`Expression::Column`]).
pub(crate) fn is_column(expr: &Expression) -> bool {
    matches!(expr, Expression::Column(_))
}

/// Returns `true` if `expr` is a literal value (number, string, boolean, or NULL).
pub(crate) fn is_literal(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Literal(_) | Expression::Boolean(_) | Expression::Null(_)
    )
}

/// Returns `true` if `expr` is a function call (regular or aggregate).
pub(crate) fn is_function(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Function(_) | Expression::AggregateFunction(_) | Expression::Hierarchy(_)
    )
}

/// Returns `true` if `expr` is a subquery ([`Expression::Subquery`]).
pub(crate) fn is_subquery(expr: &Expression) -> bool {
    matches!(expr, Expression::Subquery(_))
}

/// Returns `true` if `expr` is a SELECT statement ([`Expression::Select`]).
pub(crate) fn is_select(expr: &Expression) -> bool {
    matches!(expr, Expression::Select(_))
}

/// Whether `expr` is an aggregate function, the crate's canonical classifier.
pub(crate) fn is_aggregate(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::AggregateFunction(_)
            | Expression::Count(_)
            | Expression::Sum(_)
            | Expression::Avg(_)
            | Expression::Min(_)
            | Expression::Max(_)
            | Expression::GroupConcat(_)
            | Expression::StringAgg(_)
            | Expression::ListAgg(_)
            | Expression::ArrayAgg(_)
            | Expression::CountIf(_)
            | Expression::SumIf(_)
            | Expression::Stddev(_)
            | Expression::StddevPop(_)
            | Expression::StddevSamp(_)
            | Expression::Variance(_)
            | Expression::VarPop(_)
            | Expression::VarSamp(_)
            | Expression::Median(_)
            | Expression::Mode(_)
            | Expression::First(_)
            | Expression::Last(_)
            | Expression::AnyValue(_)
            | Expression::ApproxDistinct(_)
            | Expression::ApproxCountDistinct(_)
            | Expression::ApproxPercentile(_)
            | Expression::Percentile(_)
            | Expression::PercentileCont(_)
            | Expression::PercentileDisc(_)
            | Expression::LogicalAnd(_)
            | Expression::LogicalOr(_)
            | Expression::Skewness(_)
            | Expression::ArrayConcatAgg(_)
            | Expression::ArrayUniqueAgg(_)
            | Expression::BoolXorAgg(_)
            | Expression::BitwiseAndAgg(_)
            | Expression::BitwiseOrAgg(_)
            | Expression::BitwiseXorAgg(_)
            | Expression::JsonArrayAgg(_)
            | Expression::JsonObjectAgg(_)
            | Expression::JSONArrayAgg(_)
            | Expression::JSONObjectAgg(_)
            | Expression::JSONBObjectAgg(_)
            | Expression::ParameterizedAgg(_)
            | Expression::ArgMax(_)
            | Expression::ArgMin(_)
            | Expression::ApproxTopK(_)
            | Expression::ApproxTopKAccumulate(_)
            | Expression::ApproxTopKCombine(_)
            | Expression::ApproxTopSum(_)
            | Expression::ApproxQuantiles(_)
            | Expression::Grouping(_)
            | Expression::GroupingId(_)
            | Expression::AnonymousAggFunc(_)
            | Expression::CombinedAggFunc(_)
            | Expression::CombinedParameterizedAgg(_)
            | Expression::HashAgg(_)
            | Expression::Hll(_)
            | Expression::Minhash(_)
            | Expression::ObjectAgg(_)
            | Expression::AIAgg(_)
            | Expression::Quantile(_)
            | Expression::ApproxQuantile(_)
            | Expression::Corr(_)
            | Expression::CovarPop(_)
            | Expression::CovarSamp(_)
            | Expression::RegrValx(_)
            | Expression::RegrValy(_)
            | Expression::RegrAvgx(_)
            | Expression::RegrAvgy(_)
            | Expression::RegrCount(_)
            | Expression::RegrIntercept(_)
            | Expression::RegrR2(_)
            | Expression::RegrSxx(_)
            | Expression::RegrSxy(_)
            | Expression::RegrSyy(_)
            | Expression::RegrSlope(_)
    )
}

/// Returns `true` if `expr` is a window function ([`Expression::WindowFunction`]).
pub(crate) fn is_window_function(expr: &Expression) -> bool {
    matches!(expr, Expression::WindowFunction(_))
}

/// Every column reference in the tree, depth first.
pub(crate) fn get_columns(expr: &Expression) -> Vec<&Expression> {
    expr.find_all(is_column)
}

/// Every table reference reachable by traversal, depth first.
pub(crate) fn get_tables(expr: &Expression) -> Vec<&Expression> {
    expr.find_all(|e| matches!(e, Expression::Table(_)))
}

/// Every referenced table, DML targets held as `TableRef` fields included.
pub(crate) fn get_all_tables(expr: &Expression) -> Vec<Expression> {
    use std::collections::HashSet;

    let mut seen: HashSet<String> = HashSet::new();
    let mut result: Vec<Expression> = Vec::new();

    // First: collect all Expression::Table nodes found via DFS.
    for node in expr.dfs() {
        if let Expression::Table(t) = node {
            let qname = table_ref_qualified_name(t);
            if seen.insert(qname) {
                result.push(node.clone());
            }
        }

        // Also extract DML target TableRef fields not reachable via iter_children.
        let refs: Vec<&TableRef> = match node {
            Expression::Insert(ins) => vec![&ins.table],
            Expression::Update(upd) => {
                let mut v = vec![&upd.table];
                v.extend(upd.extra_tables.iter());
                v
            }
            Expression::Delete(del) => {
                let mut v = vec![&del.table];
                v.extend(del.using.iter());
                v
            }
            _ => continue,
        };
        for tref in refs {
            if tref.name.name.is_empty() {
                continue;
            }
            let qname = table_ref_qualified_name(tref);
            if seen.insert(qname) {
                result.push(Expression::Table(Box::new(tref.clone())));
            }
        }
    }

    result
}

/// Build a qualified name string from a TableRef for deduplication purposes.
fn table_ref_qualified_name(t: &TableRef) -> String {
    let mut name = String::new();
    if let Some(ref cat) = t.catalog {
        name.push_str(&cat.name);
        name.push('.');
    }
    if let Some(ref schema) = t.schema {
        name.push_str(&schema.name);
        name.push('.');
    }
    name.push_str(&t.name.name);
    name
}

/// The table under a MERGE field: a bare table, an aliased one or an identifier.
fn unwrap_merge_table(expr: &Expression) -> Option<&Expression> {
    match expr {
        Expression::Table(_) => Some(expr),
        Expression::Alias(alias) => match &alias.this {
            Expression::Table(_) => Some(&alias.this),
            _ => None,
        },
        _ => None,
    }
}

/// The target table of a MERGE, without its alias.
pub(crate) fn get_merge_target(expr: &Expression) -> Option<&Expression> {
    match expr {
        Expression::Merge(m) => unwrap_merge_table(&m.this),
        _ => None,
    }
}

/// The source table of a MERGE, without its alias.
pub(crate) fn get_merge_source(expr: &Expression) -> Option<&Expression> {
    match expr {
        Expression::Merge(m) => unwrap_merge_table(&m.using),
        _ => None,
    }
}

/// Returns `true` if the expression tree contains any aggregate function calls.
pub(crate) fn contains_aggregate(expr: &Expression) -> bool {
    expr.contains(is_aggregate)
}

/// Returns `true` if the expression tree contains any window function calls.
pub(crate) fn contains_window_function(expr: &Expression) -> bool {
    expr.contains(is_window_function)
}

/// Returns `true` if the expression tree contains any subquery nodes.
pub(crate) fn contains_subquery(expr: &Expression) -> bool {
    expr.contains(is_subquery)
}

// ---------------------------------------------------------------------------
// Extended type predicates
// ---------------------------------------------------------------------------

/// Macro for generating simple type-predicate functions.
macro_rules! is_type {
    ($name:ident, $($variant:pat),+ $(,)?) => {
        /// Returns `true` if `expr` matches the expected AST variant(s).
        pub(crate) fn $name(expr: &Expression) -> bool {
            matches!(expr, $($variant)|+)
        }
    };
}

// Query
is_type!(is_insert, Expression::Insert(_));
is_type!(is_update, Expression::Update(_));
is_type!(is_delete, Expression::Delete(_));
is_type!(is_merge, Expression::Merge(_));
is_type!(is_union, Expression::Union(_));
is_type!(is_intersect, Expression::Intersect(_));
is_type!(is_except, Expression::Except(_));

// Identifiers & literals
is_type!(is_boolean, Expression::Boolean(_));
is_type!(is_null_literal, Expression::Null(_));
is_type!(is_star, Expression::Star(_));
is_type!(is_identifier, Expression::Identifier(_));
is_type!(is_table, Expression::Table(_));

// Comparison
is_type!(is_eq, Expression::Eq(_));
is_type!(is_neq, Expression::Neq(_));
is_type!(is_lt, Expression::Lt(_));
is_type!(is_lte, Expression::Lte(_));
is_type!(is_gt, Expression::Gt(_));
is_type!(is_gte, Expression::Gte(_));
is_type!(is_like, Expression::Like(_));
is_type!(is_ilike, Expression::ILike(_));

// Arithmetic
is_type!(is_add, Expression::Add(_));
is_type!(is_sub, Expression::Sub(_));
is_type!(is_mul, Expression::Mul(_));
is_type!(is_div, Expression::Div(_));
is_type!(is_mod, Expression::Mod(_));
is_type!(is_concat, Expression::Concat(_));

// Logical
is_type!(is_and, Expression::And(_));
is_type!(is_or, Expression::Or(_));
is_type!(is_not, Expression::Not(_));

// Predicates
is_type!(is_in, Expression::In(_));
is_type!(is_between, Expression::Between(_));
is_type!(is_is_null, Expression::IsNull(_));
is_type!(is_exists, Expression::Exists(_));

// Functions
is_type!(is_count, Expression::Count(_));
is_type!(is_sum, Expression::Sum(_));
is_type!(is_avg, Expression::Avg(_));
is_type!(is_min_func, Expression::Min(_));
is_type!(is_max_func, Expression::Max(_));
is_type!(is_coalesce, Expression::Coalesce(_));
is_type!(is_null_if, Expression::NullIf(_));
is_type!(is_cast, Expression::Cast(_));
is_type!(is_try_cast, Expression::TryCast(_));
is_type!(is_safe_cast, Expression::SafeCast(_));
is_type!(is_case, Expression::Case(_));

// Clauses
is_type!(is_from, Expression::From(_));
is_type!(is_join, Expression::Join(_));
is_type!(is_where, Expression::Where(_));
is_type!(is_group_by, Expression::GroupBy(_));
is_type!(is_having, Expression::Having(_));
is_type!(is_order_by, Expression::OrderBy(_));
is_type!(is_limit, Expression::Limit(_));
is_type!(is_offset, Expression::Offset(_));
is_type!(is_with, Expression::With(_));
is_type!(is_cte, Expression::Cte(_));
is_type!(is_alias, Expression::Alias(_));
is_type!(is_paren, Expression::Paren(_));
is_type!(is_ordered, Expression::Ordered(_));

// DDL
is_type!(is_create_table, Expression::CreateTable(_));
is_type!(is_drop_table, Expression::DropTable(_));
is_type!(is_alter_table, Expression::AlterTable(_));
is_type!(is_create_index, Expression::CreateIndex(_));
is_type!(is_drop_index, Expression::DropIndex(_));
is_type!(is_create_view, Expression::CreateView(_));
is_type!(is_drop_view, Expression::DropView(_));

// ---------------------------------------------------------------------------
// Composite predicates
// ---------------------------------------------------------------------------

/// Returns `true` if `expr` is a query statement (SELECT, INSERT, UPDATE, DELETE, or MERGE).
pub(crate) fn is_query(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Select(_)
            | Expression::Insert(_)
            | Expression::Update(_)
            | Expression::Delete(_)
            | Expression::Merge(_)
    )
}

/// Returns `true` if `expr` is a set operation (UNION, INTERSECT, or EXCEPT).
pub(crate) fn is_set_operation(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Union(_) | Expression::Intersect(_) | Expression::Except(_)
    )
}

/// Returns `true` if `expr` is a comparison operator.
pub(crate) fn is_comparison(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Eq(_)
            | Expression::Neq(_)
            | Expression::Lt(_)
            | Expression::Lte(_)
            | Expression::Gt(_)
            | Expression::Gte(_)
            | Expression::Like(_)
            | Expression::ILike(_)
    )
}

/// Returns `true` if `expr` is an arithmetic operator.
pub(crate) fn is_arithmetic(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Add(_)
            | Expression::Sub(_)
            | Expression::Mul(_)
            | Expression::Div(_)
            | Expression::Mod(_)
    )
}

/// Returns `true` if `expr` is a logical operator (AND, OR, NOT).
pub(crate) fn is_logical(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::And(_) | Expression::Or(_) | Expression::Not(_)
    )
}

/// Returns `true` if `expr` is a DDL statement.
pub(crate) fn is_ddl(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::CreateTable(_)
            | Expression::DropTable(_)
            | Expression::Undrop(_)
            | Expression::AlterTable(_)
            | Expression::CreateIndex(_)
            | Expression::DropIndex(_)
            | Expression::CreateView(_)
            | Expression::DropView(_)
            | Expression::AlterView(_)
            | Expression::CreateSchema(_)
            | Expression::DropSchema(_)
            | Expression::CreateDatabase(_)
            | Expression::DropDatabase(_)
            | Expression::CreateFunction(_)
            | Expression::DropFunction(_)
            | Expression::CreateProcedure(_)
            | Expression::DropProcedure(_)
            | Expression::CreateSequence(_)
            | Expression::CreateSynonym(_)
            | Expression::DropSequence(_)
            | Expression::AlterSequence(_)
            | Expression::CreateTrigger(_)
            | Expression::DropTrigger(_)
            | Expression::CreateType(_)
            | Expression::DropType(_)
    )
}

/// The parent of `target` in the tree at `root`, found by pointer identity.
pub(crate) fn find_parent<'a>(root: &'a Expression, target: &Expression) -> Option<&'a Expression> {
    fn search(node: &Expression, target: *const Expression) -> Option<&Expression> {
        let mut result = None;
        crate::ast_children::for_each_child(node, |_, child| {
            if result.is_none() {
                if std::ptr::eq(child, target) {
                    result = Some(node);
                } else {
                    result = search(child, target);
                }
            }
        });
        result
    }

    search(root, target as *const Expression)
}

/// The nearest ancestor of `target` matching `predicate`, found by pointer identity.
pub(crate) fn find_ancestor<'a, F>(
    root: &'a Expression,
    target: &Expression,
    predicate: F,
) -> Option<&'a Expression>
where
    F: Fn(&Expression) -> bool,
{
    // Build path from root to target
    fn build_path<'a>(
        node: &'a Expression,
        target: *const Expression,
        path: &mut Vec<&'a Expression>,
    ) -> bool {
        if std::ptr::eq(node, target) {
            return true;
        }
        path.push(node);
        let mut found = false;
        crate::ast_children::for_each_child(node, |_, child| {
            if !found {
                found = build_path(child, target, path);
            }
        });
        if found {
            return true;
        }
        path.pop();
        false
    }

    let mut path: Vec<&Expression> = Vec::new();
    if !build_path(root, target as *const Expression, &mut path) {
        return None;
    }

    // Walk path in reverse (parent first, then grandparent, etc.)
    path.iter()
        .rev()
        .find(|&ancestor| predicate(ancestor))
        .map(|v| v as _)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expressions::{BinaryOp, Column, Identifier, LikeOp, Literal, TableRef};

    fn make_column(name: &str) -> Expression {
        Expression::boxed_column(Column {
            name: Identifier {
                name: name.to_string(),
                quoted: false,
                trailing_comments: vec![],
                span: None,
            },
            table: None,
            join_mark: false,
            trailing_comments: vec![],
            span: None,
            inferred_type: None,
        })
    }

    fn make_literal(value: i64) -> Expression {
        Expression::Literal(Box::new(Literal::Number(value.to_string())))
    }

    #[test]
    fn test_dfs_simple() {
        let left = make_column("a");
        let right = make_literal(1);
        let expr = Expression::Eq(Box::new(BinaryOp {
            left,
            right,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));

        let nodes: Vec<_> = expr.dfs().collect();
        assert_eq!(nodes.len(), 3); // Eq, Column, Literal
        assert!(matches!(nodes[0], Expression::Eq(_)));
        assert!(matches!(nodes[1], Expression::Column(_)));
        assert!(matches!(nodes[2], Expression::Literal(_)));
    }

    #[test]
    fn test_find() {
        let left = make_column("a");
        let right = make_literal(1);
        let expr = Expression::Eq(Box::new(BinaryOp {
            left,
            right,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));

        let column = expr.find(is_column);
        assert!(column.is_some());
        assert!(matches!(column.unwrap(), Expression::Column(_)));

        let literal = expr.find(is_literal);
        assert!(literal.is_some());
        assert!(matches!(literal.unwrap(), Expression::Literal(_)));
    }

    #[test]
    fn test_find_all() {
        let col1 = make_column("a");
        let col2 = make_column("b");
        let expr = Expression::And(Box::new(BinaryOp {
            left: col1,
            right: col2,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));

        let columns = expr.find_all(is_column);
        assert_eq!(columns.len(), 2);
    }

    #[test]
    fn test_contains() {
        let col = make_column("a");
        let lit = make_literal(1);
        let expr = Expression::Eq(Box::new(BinaryOp {
            left: col,
            right: lit,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));

        assert!(expr.contains(is_column));
        assert!(expr.contains(is_literal));
        assert!(!expr.contains(is_subquery));
    }

    #[test]
    fn test_count() {
        let col1 = make_column("a");
        let col2 = make_column("b");
        let lit = make_literal(1);

        let inner = Expression::Add(Box::new(BinaryOp {
            left: col2,
            right: lit,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));

        let expr = Expression::Eq(Box::new(BinaryOp {
            left: col1,
            right: inner,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));

        assert_eq!(expr.count(is_column), 2);
        assert_eq!(expr.count(is_literal), 1);
    }

    #[test]
    fn test_tree_depth() {
        // Single node
        let lit = make_literal(1);
        assert_eq!(lit.tree_depth(), 0);

        // One level
        let col = make_column("a");
        let expr = Expression::Eq(Box::new(BinaryOp {
            left: col,
            right: lit.clone(),
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));
        assert_eq!(expr.tree_depth(), 1);

        // Two levels
        let inner = Expression::Add(Box::new(BinaryOp {
            left: make_column("b"),
            right: lit,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));
        let outer = Expression::Eq(Box::new(BinaryOp {
            left: make_column("a"),
            right: inner,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));
        assert_eq!(outer.tree_depth(), 2);
    }

    #[test]
    fn test_tree_context() {
        let col = make_column("a");
        let lit = make_literal(1);
        let expr = Expression::Eq(Box::new(BinaryOp {
            left: col,
            right: lit,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));

        let ctx = TreeContext::build(&expr);

        // Root has no parent
        let root_info = ctx.get(0).unwrap();
        assert!(root_info.parent_id.is_none());

        // Children have root as parent
        let left_info = ctx.get(1).unwrap();
        assert_eq!(left_info.parent_id, Some(0));
        assert_eq!(left_info.arg_key, "left");

        let right_info = ctx.get(2).unwrap();
        assert_eq!(right_info.parent_id, Some(0));
        assert_eq!(right_info.arg_key, "right");
    }

    // -- Step 8: transform / transform_map tests --

    // -- children() tests --

    #[test]
    fn test_children_leaf() {
        let lit = make_literal(1);
        assert_eq!(lit.children().len(), 0);
    }

    #[test]
    fn test_children_binary_op() {
        let left = make_column("a");
        let right = make_literal(1);
        let expr = Expression::Eq(Box::new(BinaryOp {
            left,
            right,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));
        let children = expr.children();
        assert_eq!(children.len(), 2);
        assert!(matches!(children[0], Expression::Column(_)));
        assert!(matches!(children[1], Expression::Literal(_)));
    }

    #[test]
    fn test_children_select() {
        let ast = crate::parser::Parser::parse_sql("SELECT a, b FROM t").unwrap();
        let expr = &ast[0];
        let children = expr.children();
        // Should include select list items (a, b)
        assert!(children.len() >= 2);
    }

    #[test]
    fn test_children_follow_ast_field_order() {
        let ast = crate::parser::Parser::parse_sql("SELECT a, b FROM t").unwrap();
        let children = ast[0].children();

        assert!(matches!(children[0], Expression::Column(column) if column.name.name == "a"));
        assert!(matches!(children[1], Expression::Column(column) if column.name.name == "b"));
        assert!(matches!(children[2], Expression::Table(table) if table.name.name == "t"));
    }

    #[test]
    fn test_traversal_covers_previously_omitted_expression_fields() {
        let like = Expression::Like(Box::new(LikeOp {
            negated: false,
            left: make_column("name"),
            right: Expression::Literal(Box::new(Literal::String("x%".to_string()))),
            escape: Some(Expression::Literal(Box::new(Literal::String(
                "!".to_string(),
            )))),
            quantifier: None,
            inferred_type: None,
        }));
        let nodes: Vec<_> = like.dfs().collect();
        assert_eq!(nodes.len(), 4);
        assert!(matches!(
            nodes[3],
            Expression::Literal(literal)
                if matches!(literal.as_ref(), Literal::String(value) if value == "!")
        ));

        let mut table = TableRef::new("events");
        table.hints.push(make_column("table_hint"));
        table.identifier_func = Some(Box::new(Expression::identifier("dynamic_table")));
        let table = Expression::Table(Box::new(table));
        let children = table.children();
        assert_eq!(children.len(), 2);
        assert!(
            matches!(children[0], Expression::Column(column) if column.name.name == "table_hint")
        );
        assert!(
            matches!(children[1], Expression::Identifier(identifier) if identifier.name == "dynamic_table")
        );
    }

    #[test]
    fn test_children_select_includes_from_and_join_sources() {
        let ast = crate::parser::Parser::parse_sql(
            "SELECT u.id FROM users u JOIN orders o ON u.id = o.user_id",
        )
        .unwrap();
        let expr = &ast[0];
        let children = expr.children();

        let table_names: Vec<&str> = children
            .iter()
            .filter_map(|e| match e {
                Expression::Table(t) => Some(t.name.name.as_str()),
                _ => None,
            })
            .collect();

        assert!(table_names.contains(&"users"));
        assert!(table_names.contains(&"orders"));
    }

    #[test]
    fn test_get_tables_includes_insert_query_sources() {
        let ast = crate::parser::Parser::parse_sql(
            "INSERT INTO dst (id) SELECT s.id FROM src s JOIN dim d ON s.id = d.id",
        )
        .unwrap();
        let expr = &ast[0];
        let tables = get_tables(expr);
        let names: Vec<&str> = tables
            .iter()
            .filter_map(|e| match e {
                Expression::Table(t) => Some(t.name.name.as_str()),
                _ => None,
            })
            .collect();

        assert!(names.contains(&"src"));
        assert!(names.contains(&"dim"));
    }

    // -- find_parent() tests --

    #[test]
    fn test_find_parent_binary() {
        let left = make_column("a");
        let right = make_literal(1);
        let expr = Expression::Eq(Box::new(BinaryOp {
            left,
            right,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));

        // Find the column child and get its parent
        let col = expr.find(is_column).unwrap();
        let parent = super::find_parent(&expr, col);
        assert!(parent.is_some());
        assert!(matches!(parent.unwrap(), Expression::Eq(_)));
    }

    #[test]
    fn test_find_parent_root_has_none() {
        let lit = make_literal(1);
        let parent = super::find_parent(&lit, &lit);
        assert!(parent.is_none());
    }

    // -- find_ancestor() tests --

    #[test]
    fn test_find_ancestor_select() {
        let ast = crate::parser::Parser::parse_sql("SELECT a FROM t WHERE a > 1").unwrap();
        let expr = &ast[0];

        // Find a column inside the WHERE clause
        let where_col = expr.dfs().find(|e| {
            if let Expression::Column(c) = e {
                c.name.name == "a"
            } else {
                false
            }
        });
        assert!(where_col.is_some());

        // Find Select ancestor of that column
        let ancestor = super::find_ancestor(expr, where_col.unwrap(), is_select);
        assert!(ancestor.is_some());
        assert!(matches!(ancestor.unwrap(), Expression::Select(_)));
    }

    #[test]
    fn test_find_ancestor_no_match() {
        let left = make_column("a");
        let right = make_literal(1);
        let expr = Expression::Eq(Box::new(BinaryOp {
            left,
            right,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));

        let col = expr.find(is_column).unwrap();
        let ancestor = super::find_ancestor(&expr, col, is_select);
        assert!(ancestor.is_none());
    }

    #[test]
    fn test_ancestors() {
        let col = make_column("a");
        let lit = make_literal(1);
        let inner = Expression::Add(Box::new(BinaryOp {
            left: col,
            right: lit,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));
        let outer = Expression::Eq(Box::new(BinaryOp {
            left: make_column("b"),
            right: inner,
            left_comments: vec![],
            operator_comments: vec![],
            trailing_comments: vec![],
            inferred_type: None,
        }));

        let ctx = TreeContext::build(&outer);

        // The inner Add's left child (column "a") should have ancestors
        // Node 0: Eq
        // Node 1: Column "b" (left of Eq)
        // Node 2: Add (right of Eq)
        // Node 3: Column "a" (left of Add)
        // Node 4: Literal (right of Add)

        let ancestors = ctx.ancestors_of(3);
        assert_eq!(ancestors, vec![2, 0]); // Add, then Eq
    }
}
