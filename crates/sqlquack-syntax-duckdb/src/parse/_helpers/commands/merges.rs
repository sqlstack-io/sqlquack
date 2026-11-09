//! `MergeIntoStatement` (merge_into.gram): `MERGE INTO` with its `WHEN` clauses as a tree.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::operators::models::Returning;
use sqlquack_syntax::expressions::queries::models::With;
use sqlquack_syntax::expressions::statements::models::{Merge, When, Whens};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    JoinConstraint, binary, boolean, column_from, joined_table, star, tuple, var,
};

/// The clause kinds whose unconditional clause makes a later one of the same kind unreachable.
const CLAUSE_KINDS: [&str; 3] = ["MATCHED", "NOT MATCHED", "NOT MATCHED BY SOURCE"];

/// One `WHEN` clause's kind, as an index into `CLAUSE_KINDS`, and whether it has a condition.
struct ClauseKind {
    index: usize,
    conditional: bool,
}

impl Parser<'_> {
    /// `MergeIntoStatement`: `MERGE INTO target [alias] USING source qualifier WHEN ... [RETURNING]`.
    pub(crate) fn merge_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("MERGE")?;
        self.expect_kw("INTO")?;
        let mut target: TableRef = self.base_table_name()?;
        if let Some((name, explicit)) = self.target_alias()? {
            target.alias = Some(name);
            target.alias_explicit_as = explicit;
        }
        self.expect_kw("USING")?;
        let (source, joins) = self.table_ref()?;
        let source: Expression = match joins.is_empty() {
            true => source,
            false => joined_table(source, joins, None),
        };
        let (on, using_cond) = match self.join_qualifier()? {
            JoinConstraint::On(condition) => (Some(condition), None),
            JoinConstraint::Using(columns) => (None, Some(Box::new(name_tuple(columns)))),
            JoinConstraint::None => (None, None),
        };
        let mut whens: Vec<Expression> = Vec::new();
        let mut unconditional: [bool; 3] = [false; 3];
        loop {
            let start: usize = self.offset();
            let (when, kind) = self.merge_match()?;
            if unconditional[kind.index] {
                let name: &str = CLAUSE_KINDS[kind.index];
                let message: String = format!(
                    "Unconditional WHEN {name} clause was already defined - any following WHEN \
                     {name} clause would be unreachable"
                );
                self.refuse_later(&message, start);
            }
            unconditional[kind.index] |= !kind.conditional;
            whens.push(when);
            if !self.kw("WHEN") {
                break;
            }
        }
        let returning: Vec<Expression> = self.returning()?;
        Ok(Expression::Merge(Box::new(Merge {
            this: Box::new(Expression::Table(Box::new(target))),
            using: Box::new(source),
            on,
            using_cond,
            whens: Some(Box::new(Expression::Whens(Box::new(Whens {
                expressions: whens,
            })))),
            with_: with.map(|with| Box::new(Expression::With(Box::new(with)))),
            returning: match returning.is_empty() {
                true => None,
                false => Some(Box::new(Expression::Returning(Box::new(Returning {
                    expressions: returning,
                    into: None,
                })))),
            },
            returning_aliases: Vec::new(),
        })))
    }

    /// `MergeMatch`: `WHEN [NOT] MATCHED [BY SOURCE|TARGET] [AND condition] THEN action`.
    fn merge_match(&mut self) -> Parsing<(Expression, ClauseKind)> {
        self.expect_kw("WHEN")?;
        let not: bool = self.eat_kw("NOT");
        self.expect_kw("MATCHED")?;
        let by_source: bool = not && self.eat_kws(&["BY", "SOURCE"]);
        if not && !by_source {
            self.eat_kws(&["BY", "TARGET"]);
        }
        let condition: Option<Box<Expression>> = match self.eat_kw("AND") {
            true => Some(Box::new(self.expression()?)),
            false => None,
        };
        self.expect_kw("THEN")?;
        let then: Expression = self.merge_action()?;
        let kind: ClauseKind = ClauseKind {
            index: usize::from(not) + usize::from(by_source),
            conditional: condition.is_some(),
        };
        let when: Expression = Expression::When(Box::new(When {
            matched: Some(Box::new(boolean(!not))),
            source: by_source.then(|| Box::new(boolean(true))),
            condition,
            then: Box::new(then),
        }));
        Ok((when, kind))
    }

    /// `MatchedClauseAction`, as its verb alone or a tuple of the verb and its parts.
    fn merge_action(&mut self) -> Parsing<Expression> {
        if self.eat_kws(&["DO", "NOTHING"]) {
            return Ok(var("DO NOTHING"));
        }
        let word: String = self.word_at(0).unwrap_or_default();
        match word.as_str() {
            "DELETE" => {
                self.at += 1;
                Ok(var("DELETE"))
            }
            "UPDATE" => {
                self.at += 1;
                self.merge_update()
            }
            "INSERT" => {
                self.at += 1;
                self.merge_insert()
            }
            "ERROR" => {
                self.at += 1;
                let ends: bool = self.terminal_at(0) || self.kw("WHEN") || self.kw("RETURNING");
                Ok(match ends {
                    true => var("ERROR"),
                    false => tuple(vec![var("ERROR"), self.expression()?]),
                })
            }
            _ => Err(self.error()),
        }
    }

    /// `UpdateMatchInfo?`: `SET assignments`, `SET *` or `BY NAME|POSITION`.
    fn merge_update(&mut self) -> Parsing<Expression> {
        if self.eat_kw("SET") {
            if self.eat_char('*') {
                return Ok(tuple(vec![var("UPDATE"), star_all()]));
            }
            let assignments: Vec<Expression> = self
                .set_clauses()?
                .into_iter()
                .map(|(target, value)| {
                    binary(Expression::Eq, Expression::Identifier(target), value)
                })
                .collect();
            return Ok(tuple(vec![var("UPDATE"), tuple(assignments)]));
        }
        Ok(match self.by_name_or_position() {
            Some(order) => tuple(vec![var("UPDATE"), var(order)]),
            None => var("UPDATE"),
        })
    }

    /// `InsertMatchInfo?`: `[(columns)] VALUES (values)`, `DEFAULT VALUES`, or `[BY ...] [*]`.
    fn merge_insert(&mut self) -> Parsing<Expression> {
        let mut parts: Vec<Expression> = vec![var("INSERT")];
        if self.is_char('(') || self.kw("VALUES") {
            if self.is_char('(') {
                let columns: Vec<Identifier> =
                    self.parenthesized(|parser| parser.name(is_identifier))?;
                parts.push(name_tuple(columns));
            }
            self.expect_kw("VALUES")?;
            self.expect_char('(')?;
            parts.push(tuple(self.expression_list(')')?));
            self.expect_char(')')?;
            return Ok(tuple(parts));
        }
        if self.eat_kws(&["DEFAULT", "VALUES"]) {
            parts.push(var("DEFAULT VALUES"));
            return Ok(tuple(parts));
        }
        if let Some(order) = self.by_name_or_position() {
            parts.push(var(order));
        }
        if self.eat_char('*') {
            parts.push(star_all());
        }
        Ok(match parts.len() {
            1 => var("INSERT"),
            _ => tuple(parts),
        })
    }

    /// `ByNameOrPosition?`: `BY NAME` or `BY POSITION`, when present.
    fn by_name_or_position(&mut self) -> Option<&'static str> {
        if self.eat_kws(&["BY", "NAME"]) {
            return Some("BY NAME");
        }
        self.eat_kws(&["BY", "POSITION"]).then_some("BY POSITION")
    }
}

/// `*`, every column.
fn star_all() -> Expression {
    Expression::Star(Box::new(star(None, None)))
}

/// Names as a tuple of columns.
fn name_tuple(names: Vec<Identifier>) -> Expression {
    tuple(
        names
            .into_iter()
            .map(|name| column_from(vec![name]))
            .collect(),
    )
}
