//! Expressions: what a query's structure says, for readers of the located tree.

use crate::expressions::queries::models::Join;
use crate::expressions::queries::models::JoinKind;
use crate::expressions::queries::models::Select;
use crate::expressions::queries::models::With;
use crate::expressions::tree::models::Expression;
use crate::tokens::models::SourceSpan;
use crate::traversal::models::{CommaItem, DfsIter, FromItemKind, SetOperation, SetOperator};

/// Snowflake's wrapper of a table function call in FROM.
const TABLE_CALL: &str = "TABLE";
/// The call that expands an array into rows.
const UNNEST_CALL: &str = "UNNEST";
/// BigQuery's wrapper of an item that numbers its rows.
const WITH_OFFSET_CALL: &str = "WITH OFFSET";
/// DuckDB's call that stands for several columns.
const COLUMNS_CALL: &str = "COLUMNS";

/// A wildcard kept as a name.
const WILDCARD: &str = "*";

impl Expression {
    /// SQLQuack: the WITH this node owns; `None` when it has none.
    pub fn with_clause(&self) -> Option<&With> {
        match self {
            Self::Select(node) => node.with.as_ref(),
            Self::Union(node) => node.with.as_ref(),
            Self::Intersect(node) => node.with.as_ref(),
            Self::Except(node) => node.with.as_ref(),
            Self::Pivot(node) => node.with.as_ref(),
            Self::Insert(node) => node.with.as_ref(),
            Self::Update(node) => node.with.as_ref(),
            Self::Delete(node) => node.with.as_ref(),
            Self::Call(node) => node.with.as_ref(),
            Self::Merge(node) => match node.with_.as_deref() {
                Some(Self::With(with)) => Some(with),
                _ => None,
            },
            _ => None,
        }
    }

    /// SQLQuack: this node as a set operation; `None` for any other node.
    pub fn set_operation(&self) -> Option<SetOperation<'_>> {
        macro_rules! operation {
            ($node:expr, $operator:ident) => {
                SetOperation {
                    operator: SetOperator::$operator,
                    left: &$node.left,
                    right: &$node.right,
                    by_name: $node.by_name || $node.corresponding,
                    all: $node.all,
                    distinct: $node.distinct,
                    operator_span: $node.operator_span,
                }
            };
        }
        match self {
            Self::Union(node) => Some(operation!(node, Union)),
            Self::Intersect(node) => Some(operation!(node, Intersect)),
            Self::Except(node) => Some(operation!(node, Except)),
            _ => None,
        }
    }

    /// SQLQuack: how many columns a query or VALUES returns; `None` when only a schema can say.
    pub fn column_count(&self) -> Option<usize> {
        let mut query: &Self = self;
        loop {
            query = match query {
                Self::Annotated(node) => &node.this,
                Self::Paren(node) => &node.this,
                Self::Subquery(node) => &node.this,
                _ => match query.set_operation() {
                    Some(set) if set.by_name => return None,
                    Some(set) => set.left,
                    None => break,
                },
            };
        }
        match query {
            Self::Select(select) => {
                if let ([Self::Star(_)], Some(from), []) = (
                    select.expressions.as_slice(),
                    &select.from,
                    select.joins.as_slice(),
                ) {
                    if let [Self::Values(rows)] = from.expressions.as_slice() {
                        return rows.expressions.first().map(|row| row.expressions.len());
                    }
                }
                let countable: bool = !select.expressions.is_empty()
                    && !select.expressions.iter().any(Self::expands_to_columns);
                countable.then_some(select.expressions.len())
            }
            Self::Values(values) => values.expressions.first().map(|row| row.expressions.len()),
            _ => None,
        }
    }

    /// SQLQuack: the first query of a chain of set operations, with parentheses opened.
    pub fn first_branch(&self) -> &Self {
        self.edge_branch(|set| set.left)
    }

    /// SQLQuack: the last query of a chain of set operations, with parentheses opened.
    pub fn last_branch(&self) -> &Self {
        self.edge_branch(|set| set.right)
    }

    /// The branch `side` leads to from this query.
    fn edge_branch<'a>(&'a self, side: impl Fn(SetOperation<'a>) -> &'a Self) -> &'a Self {
        let mut query: &Self = self;
        loop {
            query = match query {
                Self::Annotated(node) => &node.this,
                Self::Paren(node) => &node.this,
                Self::Subquery(node) => &node.this,
                _ => match query.set_operation() {
                    Some(set) => side(set),
                    None => return query,
                },
            };
        }
    }

    /// SQLQuack: whether this SELECT projects `*` or `name.*`; multiplication is no wildcard.
    pub fn projects_wildcard(&self) -> bool {
        match self {
            Self::Select(select) => {
                let rows_only: bool = match &select.from {
                    Some(from) => matches!(from.expressions.as_slice(), [Self::Values(_)]),
                    None => false,
                };
                let parsers_wrapper: bool = rows_only && select.span.is_absent();
                !parsers_wrapper && select.expressions.iter().any(Self::is_wildcard)
            }
            _ => false,
        }
    }

    /// Whether a projection is written as a wildcard.
    fn is_wildcard(&self) -> bool {
        match self {
            Self::Star(_) => true,
            Self::Alias(node) => node.this.is_wildcard(),
            Self::Annotated(node) => node.this.is_wildcard(),
            Self::Column(column) => column.name.name == WILDCARD && !column.name.quoted,
            Self::Dot(dot) => dot.field.name == WILDCARD && !dot.field.quoted,
            _ => false,
        }
    }

    /// Whether a projection stands for a number of columns only a schema knows.
    fn expands_to_columns(&self) -> bool {
        match self {
            Self::Star(_) | Self::Columns(_) => true,
            Self::Alias(node) => node.this.expands_to_columns(),
            Self::Annotated(node) => node.this.expands_to_columns(),
            Self::Function(call) => call.name.eq_ignore_ascii_case(COLUMNS_CALL),
            Self::Column(column) => column.name.name == WILDCARD && !column.name.quoted,
            Self::Dot(dot) => dot.field.name == WILDCARD && !dot.field.quoted,
            _ => false,
        }
    }

    /// The name a FROM item answers to, when it has one.
    fn relation_name(&self) -> Option<&str> {
        match self {
            Self::Table(table) => Some(&table.alias.as_ref().unwrap_or(&table.name).name),
            Self::Subquery(query) => query.alias.as_ref().map(|alias| alias.name.as_str()),
            Self::Alias(alias) => Some(&alias.alias.name),
            Self::Annotated(node) => node.this.relation_name(),
            _ => None,
        }
    }

    /// What this FROM item is, after the items named `earlier`.
    fn item_kind(&self, earlier: &[&str]) -> FromItemKind {
        let mut item: &Self = self;
        loop {
            item = match item {
                Self::Annotated(node) => &node.this,
                Self::Alias(node) => &node.this,
                Self::JoinedTable(node) => &node.left,
                Self::Pivot(node) => &node.this,
                Self::Function(call) if call.name == WITH_OFFSET_CALL => match call.args.first() {
                    Some(inner) => inner,
                    None => break,
                },
                _ => break,
            };
        }
        let named: &str = match item {
            Self::Function(call) => &call.name,
            _ => "",
        };
        match item {
            Self::Lateral(_) => FromItemKind::Lateral,
            Self::Subquery(query) if query.lateral => FromItemKind::Lateral,
            Self::Subquery(_) | Self::Values(_) => FromItemKind::Derived,
            Self::Table(table) => {
                let root: Option<&str> = table
                    .catalog
                    .as_ref()
                    .or(table.schema.as_ref())
                    .map(|root| root.name.as_str());
                match root {
                    Some(root) if earlier.iter().any(|name| name.eq_ignore_ascii_case(root)) => {
                        FromItemKind::ArrayPath
                    }
                    _ => FromItemKind::Relation,
                }
            }
            Self::Unnest(_) => item.unnest_kind(),
            Self::Function(_)
                if named
                    .split(' ')
                    .next()
                    .is_some_and(|word| word.eq_ignore_ascii_case(UNNEST_CALL)) =>
            {
                item.unnest_kind()
            }
            Self::Function(call) if named.eq_ignore_ascii_case(TABLE_CALL) => {
                let names_a_table: bool = matches!(
                    call.args.as_slice(),
                    [Self::Literal(_) | Self::Column(_) | Self::Identifier(_) | Self::Table(_)]
                );
                match names_a_table {
                    true => FromItemKind::Relation,
                    false => FromItemKind::TableFunction,
                }
            }
            Self::Function(_)
            | Self::AggregateFunction(_)
            | Self::WindowFunction(_)
            | Self::MethodCall(_)
            | Self::Explode(_)
            | Self::ExplodeOuter(_) => FromItemKind::TableFunction,
            _ => FromItemKind::Relation,
        }
    }

    /// An UNNEST item's kind: correlated when a column is read in its arguments.
    fn unnest_kind(&self) -> FromItemKind {
        let correlated: bool =
            DfsIter::new(self).any(|node| matches!(node, Self::Column(_) | Self::Dot(_)));
        FromItemKind::Unnest { correlated }
    }
}

impl Join {
    /// SQLQuack: whether the joined item may read the items before it, as a LATERAL one does.
    pub fn may_read_earlier(&self) -> bool {
        matches!(self.kind, JoinKind::Lateral | JoinKind::LeftLateral)
            || matches!(
                self.this.item_kind(&[]),
                FromItemKind::Lateral
                    | FromItemKind::TableFunction
                    | FromItemKind::Unnest { correlated: true }
            )
    }
}

impl Select {
    /// SQLQuack: the FROM items written after commas, whether kept in FROM or as implicit joins.
    pub fn items_after_commas(&self) -> Vec<CommaItem<'_>> {
        let mut found: Vec<CommaItem<'_>> = Vec::new();
        let mut earlier: Vec<&str> = Vec::new();
        if let Some(from) = &self.from {
            for (index, item) in from.expressions.iter().enumerate() {
                if let Some(before) = index.checked_sub(1) {
                    let comma = SourceSpan(from.comma_spans.0.get(before).copied());
                    let kind = item.item_kind(&earlier);
                    found.push(CommaItem { item, kind, comma });
                }
                if let Expression::JoinedTable(joined) = item {
                    earlier.extend(joined.left.relation_name());
                    earlier.extend(
                        joined
                            .joins
                            .iter()
                            .filter_map(|join| join.this.relation_name()),
                    );
                }
                earlier.extend(item.relation_name());
            }
        }
        for join in &self.joins {
            let comma_cross: bool = join.kind == JoinKind::Cross
                && join.kind_span.0.is_some()
                && join.kind_span.0 == join.keyword_span.0;
            if join.kind == JoinKind::Implicit || comma_cross {
                found.push(CommaItem {
                    item: &join.this,
                    kind: join.this.item_kind(&earlier),
                    comma: join.keyword_span,
                });
            }
            earlier.extend(join.this.relation_name());
        }
        found
    }
}
