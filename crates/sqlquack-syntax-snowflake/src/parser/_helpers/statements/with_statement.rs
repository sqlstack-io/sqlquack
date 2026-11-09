//! The statement a WITH clause prefixes, with the clause attached to it.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::queries::models::Cte;
use sqlquack_syntax::expressions::queries::models::With;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

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

    /// Parse a WITH clause (CTEs)
    pub(crate) fn parse_with(&mut self) -> Result<Expression> {
        let with_token = self.expect_token(TokenType::With)?;
        let leading_comments = with_token.comments;

        let recursive = self.match_token(TokenType::Recursive);
        let mut ctes: Vec<Cte> = Vec::new();
        let mut has_procedure = false;

        loop {
            let cte_start = self.current;
            let name = self.expect_identifier_or_alias_keyword_with_quoted()?;

            let columns = if self.check(TokenType::LParen) && !self.check_next(TokenType::Select) {
                self.skip();
                let cols = self.parse_identifier_list()?;
                self.expect_token(TokenType::RParen)?;
                cols
            } else {
                Vec::new()
            };

            let key_expressions = if self.match_keywords(&[TokenType::Using, TokenType::Key]) {
                self.expect_token(TokenType::LParen)?;
                let keys = self.parse_identifier_list()?;
                self.expect_token(TokenType::RParen)?;
                keys
            } else {
                Vec::new()
            };

            let has_as = self.match_token(TokenType::As);
            let cte_comments = if has_as {
                self.previous_trailing_comments().to_vec()
            } else {
                Vec::new()
            };
            if matches!(
                self.config.dialect,
                Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
            ) {
                self.reject_reserved_name_part(&name)?;
                if !has_as {
                    return Err(self.parse_error("Expected AS before the CTE body"));
                }
            }
            if has_as && columns.is_empty() && self.at_anonymous_procedure() {
                let procedure = self.parse_anonymous_procedure(&name)?;
                has_procedure = true;
                ctes.push(Cte {
                    search_cycle: Vec::new(),
                    alias: name,
                    this: procedure,
                    columns,
                    materialized: None,
                    key_expressions,
                    alias_first: true,
                    comments: cte_comments,
                    span: self.source_range(cte_start),
                    key_items: Vec::new(),
                });
                if self.match_token(TokenType::Comma) {
                    continue;
                }
                break;
            }

            let materialized = if self.match_token(TokenType::Materialized) {
                Some(true)
            } else if self.match_token(TokenType::Not) {
                self.expect_token(TokenType::Materialized)?;
                Some(false)
            } else {
                None
            };

            self.expect_token(TokenType::LParen)?;
            if matches!(
                self.config.dialect,
                Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
            ) && self.check(TokenType::Values)
            {
                return Err(self.parse_error(format!(
                    "Snowflake does not accept a bare VALUES list as the body of CTE '{}'; \
                     select from it instead, as SELECT * FROM (VALUES ...) AS t(column, ...)",
                    name.name
                )));
            }
            let query_start = self.current;
            let query = self.parse_cte_body(has_procedure)?;
            let query = self.spanned(query_start, query);
            self.expect_token(TokenType::RParen)?;

            ctes.push(Cte {
                search_cycle: Vec::new(),
                alias: name,
                this: query,
                columns,
                materialized,
                key_expressions,
                alias_first: true,
                comments: cte_comments,
                span: self.source_range(cte_start),
                key_items: Vec::new(),
            });

            if !self.match_token(TokenType::Comma) {
                if self.check(TokenType::With) {
                    self.skip();
                    self.match_token(TokenType::Recursive);
                    continue;
                }
                break;
            }

            self.match_token(TokenType::With);
        }

        let search = self.parse_recursive_with_search()?;

        self.parse_with_statement(
            With {
                ctes,
                recursive,
                leading_comments,
                search,
            },
            has_procedure,
        )
    }
}
