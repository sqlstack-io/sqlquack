//! The statement a WITH clause prefixes, with the clause attached to it.

use crate::expressions::queries::models::With;
use crate::expressions::tree::models::Expression;
use crate::parser::models::Parser;
use crate::tokens::types::Result;

impl Parser {
    /// The statement after a WITH clause, which carries the clause; a CALL after an anonymous procedure.
    pub(crate) fn parse_with_statement(
        &mut self,
        with_clause: With,
        has_procedure: bool,
    ) -> Result<Expression> {
        if has_procedure {
            return self.parse_call_with(with_clause);
        }
        let mut main_query = self.parse_statement()?;

        loop {
            match main_query {
                Expression::Paren(paren) => {
                    main_query = paren.this;
                }
                Expression::Subquery(ref sub)
                    if sub.alias.is_none()
                        && sub.order_by.is_none()
                        && sub.limit.is_none()
                        && sub.offset.is_none() =>
                {
                    if let Expression::Subquery(sub) = main_query {
                        main_query = sub.this;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        match &mut main_query {
            Expression::Select(ref mut select) => {
                select.with = Some(with_clause);
            }
            Expression::Union(ref mut union) => {
                union.with = Some(with_clause);
            }
            Expression::Intersect(ref mut intersect) => {
                intersect.with = Some(with_clause);
            }
            Expression::Except(ref mut except) => {
                except.with = Some(with_clause);
            }
            Expression::Update(ref mut update) => {
                update.with = Some(with_clause);
            }
            Expression::Insert(ref mut insert) => {
                insert.with = Some(with_clause);
            }
            Expression::Delete(ref mut delete) => {
                delete.with = Some(with_clause);
            }
            Expression::CreateTable(ref mut ct) => {
                ct.with_cte = Some(with_clause);
            }
            Expression::Pivot(ref mut pivot) => {
                pivot.with = Some(with_clause);
            }
            Expression::Merge(ref mut merge) => {
                merge.with_ = Some(Box::new(Expression::With(Box::new(with_clause))));
            }
            _ => {}
        }

        Ok(main_query)
    }
}
