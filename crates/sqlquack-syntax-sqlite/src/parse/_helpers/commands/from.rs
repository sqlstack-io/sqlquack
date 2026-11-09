//! `seltablist`, `joinop` and `on_using` (parse.y): the `FROM` clause.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{From, Join, JoinKind, Select};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_ids, is_nm};
use crate::parse::_helpers::tree::nodes::{
    JoinConstraint, TableAlias, function, join, joined_table, subquery, table,
};
use crate::parse::constants::{
    JOIN_WORDS, JT_CROSS, JT_INNER, JT_LEFT, JT_NATURAL, JT_OUTER, JT_RIGHT, MAX_JOIN_WORDS,
};
use crate::scan::models::Token;

/// How an item joins the items before it: by a comma, or by a join of a kind.
#[derive(Clone, Copy)]
enum Joiner {
    Comma,
    Join {
        kind: JoinKind,
        inner: bool,
        outer: bool,
    },
}

/// One `seltablist` item: how it joins, the item, and its `ON` or `USING`.
struct Item {
    joiner: Joiner,
    this: Expression,
    constraint: JoinConstraint,
}

/// The shared tree's join kind for `sqlite3JoinType`'s bits.
fn join_kind(bits: u8) -> JoinKind {
    let natural: bool = bits & JT_NATURAL != 0;
    let (left, right) = (bits & JT_LEFT != 0, bits & JT_RIGHT != 0);
    match (natural, left, right) {
        (true, true, true) => JoinKind::NaturalFull,
        (true, true, false) => JoinKind::NaturalLeft,
        (true, false, true) => JoinKind::NaturalRight,
        (true, false, false) => JoinKind::Natural,
        (false, true, true) => JoinKind::Full,
        (false, true, false) => JoinKind::Left,
        (false, false, true) => JoinKind::Right,
        (false, false, false) if bits & JT_CROSS != 0 => JoinKind::Cross,
        (false, false, false) => JoinKind::Inner,
    }
}

/// `item` under `alias`, as the shared tree keeps an aliased item of each kind.
fn aliased(item: Expression, alias: Option<TableAlias>) -> Expression {
    let Some(alias) = alias else {
        return item;
    };
    match item {
        Expression::Table(mut relation) => {
            relation.alias = Some(alias.name);
            relation.alias_explicit_as = alias.explicit;
            Expression::Table(relation)
        }
        Expression::Subquery(mut node) => {
            node.alias = Some(alias.name);
            node.alias_explicit_as = alias.explicit;
            node.alias_keyword = alias.explicit.then(|| "AS".to_owned());
            Expression::Subquery(node)
        }
        Expression::JoinedTable(mut node) => {
            node.alias = Some(alias.name);
            Expression::JoinedTable(node)
        }
        Expression::Alias(mut node) => {
            node.alias = alias.name;
            node.alias_explicit_as = alias.explicit;
            node.alias_keyword = alias.explicit.then(|| "AS".to_owned());
            Expression::Alias(node)
        }
        other => {
            let mut node: Alias = Alias::new(other, alias.name);
            node.alias_explicit_as = alias.explicit;
            node.alias_keyword = alias.explicit.then(|| "AS".to_owned());
            Expression::Alias(Box::new(node))
        }
    }
}

impl Parser<'_> {
    /// `FROM seltablist`: comma-joined items before any join are sources, the rest are joins.
    pub(crate) fn select_from(&mut self, mut select: Box<Select>) -> Parsing<Box<Select>> {
        let (first, items) = self.table_list()?;
        let mut sources: Vec<Expression> = vec![first];
        for item in items {
            match (item.joiner, select.joins.is_empty()) {
                (Joiner::Comma, true) => sources.push(item.this),
                (joiner, _) => select
                    .joins
                    .push(Self::joined(item.this, joiner, item.constraint)),
            }
        }
        select.from = Some(From {
            expressions: sources,
        });
        Ok(select)
    }

    /// `this` joined by `joiner` under `constraint`.
    fn joined(this: Expression, joiner: Joiner, constraint: JoinConstraint) -> Join {
        match joiner {
            Joiner::Comma => join(this, JoinKind::Implicit, constraint, (false, false)),
            Joiner::Join { kind, inner, outer } => join(this, kind, constraint, (inner, outer)),
        }
    }

    /// `seltablist`: the first item, which may not take `ON` or `USING`, and the joined items.
    fn table_list(&mut self) -> Parsing<(Expression, Vec<Item>)> {
        let (first, constraint) = self.table_item()?;
        let clause: Option<&str> = match &constraint {
            JoinConstraint::On(_) => Some("ON"),
            JoinConstraint::Using(_) => Some("USING"),
            JoinConstraint::None => None,
        };
        if let Some(clause) = clause {
            let message: String = format!("a JOIN clause is required before {clause}");
            self.defer(self.error_with(&message, self.offset(), false));
        }
        let mut items: Vec<Item> = Vec::new();
        while let Some(joiner) = self.join_operator()? {
            let (this, constraint) = self.table_item()?;
            items.push(Item {
                joiner,
                this,
                constraint,
            });
        }
        Ok((first, items))
    }

    /// `joinop`: `,`, `JOIN`, or one to three join words read as `sqlite3JoinType` reads them.
    fn join_operator(&mut self) -> Parsing<Option<Joiner>> {
        if self.eat_op("COMMA") {
            return Ok(Some(Joiner::Comma));
        }
        if self.eat_kw("JOIN") {
            return Ok(Some(Joiner::Join {
                kind: JoinKind::Inner,
                inner: false,
                outer: false,
            }));
        }
        if !self.kw("JOIN_KW") {
            return Ok(None);
        }
        let mut words: Vec<Token> = vec![self.advance()?];
        while words.len() < MAX_JOIN_WORDS && !self.kw("JOIN") {
            words.push(self.expect_token(is_nm)?);
        }
        self.expect_kw("JOIN")?;
        let written: Vec<&str> = words
            .iter()
            .map(|word| &self.sql[word.start..word.end])
            .collect();
        let mut bits: u8 = 0;
        let mut known: bool = true;
        for word in &written {
            match JOIN_WORDS
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(word))
            {
                Some((_, code)) => bits |= code,
                None => {
                    known = false;
                    break;
                }
            }
        }
        let invalid: bool = !known
            || bits & (JT_INNER | JT_OUTER) == (JT_INNER | JT_OUTER)
            || bits & (JT_OUTER | JT_LEFT | JT_RIGHT) == JT_OUTER;
        if invalid {
            let message: String = format!("unknown join type: {}", written.join(" "));
            self.defer(self.error_with(&message, words[0].start, true));
        }
        let spelled = |name: &str| written.iter().any(|word| word.eq_ignore_ascii_case(name));
        Ok(Some(Joiner::Join {
            kind: join_kind(bits),
            inner: spelled("INNER"),
            outer: spelled("OUTER"),
        }))
    }

    /// One `seltablist` item with its alias, and its `ON` or `USING`.
    fn table_item(&mut self) -> Parsing<(Expression, JoinConstraint)> {
        let first: usize = self.at;
        let start: usize = self.offset();
        let item: Expression = match self.op("LP") {
            true if self.starts_select_at(1) => {
                self.at += 1;
                let query: Expression = self.select()?;
                self.expect_op("RP")?;
                let alias: Option<TableAlias> = self.table_alias()?;
                aliased(subquery(query, None, false), alias)
            }
            true => {
                self.at += 1;
                let (first, items) = self.deeper(|parser| parser.table_list())?;
                self.expect_op("RP")?;
                let alias: Option<TableAlias> = self.table_alias()?;
                aliased(Self::nested(first, items), alias)
            }
            false => {
                let mut name: Vec<Identifier> = vec![self.name(is_nm)?];
                if self.eat_op("DOT") {
                    name.push(self.name(is_nm)?);
                }
                if self.eat_op("LP") {
                    let args: Vec<Expression> = self.expression_list("RP")?;
                    self.expect_op("RP")?;
                    let call = Expression::Function(Box::new(function(name, args)));
                    let alias: Option<TableAlias> = self.table_alias()?;
                    aliased(self.spanned(first, call), alias)
                } else {
                    let relation: TableRef = table(name);
                    let alias: Option<TableAlias> = self.table_alias()?;
                    let item: Expression = aliased(Expression::Table(Box::new(relation)), alias);
                    if self.kw("INDEXED") {
                        self.at += 1;
                        self.expect_kw("BY")?;
                        self.expect_token(is_nm)?;
                        self.gap("indexed_by", start);
                    } else if self.eat_kw("NOT") {
                        self.expect_kw("INDEXED")?;
                        self.gap("indexed_by", start);
                    }
                    item
                }
            }
        };
        let constraint: JoinConstraint = self.join_constraint()?;
        Ok((item, constraint))
    }

    /// A parenthesized `seltablist`: its one item, or the items as one joined table.
    fn nested(first: Expression, items: Vec<Item>) -> Expression {
        let joins: Vec<Join> = items
            .into_iter()
            .map(|item| Self::joined(item.this, item.joiner, item.constraint))
            .collect();
        match joins.is_empty() {
            true => first,
            false => joined_table(first, joins, None),
        }
    }

    /// `as`: `AS nm`, a bare `ids`, or nothing.
    fn table_alias(&mut self) -> Parsing<Option<TableAlias>> {
        let explicit: bool = self.eat_kw("AS");
        let name: Identifier = match explicit {
            true => self.name(is_nm)?,
            false if self.is(is_ids) => self.name(is_ids)?,
            false => return Ok(None),
        };
        Ok(Some(TableAlias {
            name,
            columns: Vec::new(),
            explicit,
        }))
    }

    /// `on_using`: `ON expr`, `USING ( idlist )`, or nothing.
    fn join_constraint(&mut self) -> Parsing<JoinConstraint> {
        if self.eat_kw("ON") {
            return Ok(JoinConstraint::On(Box::new(self.expression()?)));
        }
        if self.eat_kw("USING") {
            self.expect_op("LP")?;
            let columns: Vec<Identifier> = self.comma_list(|parser| parser.name(is_nm))?;
            self.expect_op("RP")?;
            return Ok(JoinConstraint::Using(columns));
        }
        Ok(JoinConstraint::None)
    }
}
