//! Parser queries: joins.

use crate::expressions::names::models::Identifier;
use crate::expressions::operators::models::Alias;
use crate::expressions::queries::models::Join;
use crate::expressions::queries::models::JoinKind;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::Tuple;
use crate::parser::models::JoinKindParts;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parses a table expression and its joins, as in `(t1 CROSS JOIN t2)`.
    pub(crate) fn parse_table_expression_with_joins(&mut self) -> Result<(Expression, Vec<Join>)> {
        let left = self.parse_table_expression()?;

        let joins = self.parse_joins()?;

        Ok((left, joins))
    }

    /// Parses joins; chained ON/USING clauses bind right to left, as `a JOIN b JOIN c ON x ON y`.
    pub(crate) fn parse_joins(&mut self) -> Result<Vec<Join>> {
        let mut joins = Vec::with_capacity(2);
        let mut nesting_group: usize = 0;

        loop {
            let joins_before = joins.len();

            loop {
                let pos_before_join_kind = self.current;
                let join_kind_result = self.try_parse_join_kind();
                let Some(JoinKindParts {
                    kind,
                    needs_join_keyword,
                    use_inner_keyword,
                    use_outer_keyword,
                    hint: join_hint,
                }) = join_kind_result
                else {
                    break;
                };
                let mut join_comments: Vec<String> = Vec::new();
                if pos_before_join_kind < self.tokens.len() {
                    join_comments
                        .extend(self.tokens[pos_before_join_kind].comments.iter().cloned());
                }
                for i in pos_before_join_kind..self.current {
                    if i < self.tokens.len() {
                        join_comments.extend(self.tokens[i].trailing_comments.iter().cloned());
                    }
                }
                let directed = if needs_join_keyword && self.check_identifier("DIRECTED") {
                    self.skip();
                    true
                } else {
                    false
                };
                if needs_join_keyword {
                    self.expect_token(TokenType::Join)?;
                }

                let table = self.parse_join_table(&kind)?;

                let table = self.parse_join_table_suffix(&kind, table)?;

                let has_match_condition = self.check_identifier("MATCH_CONDITION");
                let has_inline_condition = self.check(TokenType::On)
                    || self.check(TokenType::Using)
                    || has_match_condition;
                let next_is_join = self.check_join_keyword();

                let match_condition = if has_match_condition && !next_is_join {
                    if self.match_identifier("MATCH_CONDITION") {
                        self.expect_token(TokenType::LParen)?;
                        let condition = self.parse_expression()?;
                        self.expect_token(TokenType::RParen)?;
                        Some(condition)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let (on, using) = if (has_inline_condition || match_condition.is_some())
                    && !self.check_join_keyword()
                {
                    if self.match_token(TokenType::On) {
                        (Some(self.parse_expression()?), Vec::new())
                    } else if self.match_token(TokenType::Using) {
                        let has_parens = self.match_token(TokenType::LParen);
                        let cols = self.parse_using_column_list()?;
                        if has_parens {
                            self.expect_token(TokenType::RParen)?;
                        }
                        (None, cols)
                    } else {
                        (None, Vec::new())
                    }
                } else {
                    (None, Vec::new())
                };

                joins.push(Join {
                    this: table,
                    on,
                    using,
                    kind,
                    use_inner_keyword,
                    use_outer_keyword,
                    deferred_condition: false,
                    join_hint,
                    match_condition,
                    pivots: Vec::new(),
                    comments: join_comments,
                    nesting_group,
                    directed,
                });
            }

            joins = self.parse_deferred_join_conditions(joins, joins_before)?;

            if joins.len() == joins_before {
                break;
            }

            if !self.check_join_keyword() {
                break;
            }
            nesting_group += 1;
        }

        Ok(joins)
    }

    fn parse_join_table(&mut self, kind: &JoinKind) -> Result<Expression> {
        Ok(if matches!(kind, JoinKind::Array | JoinKind::LeftArray) {
            let mut items: Vec<Expression> = Vec::new();
            if !self.is_at_end()
                && !self.check(TokenType::Semicolon)
                && !self.check(TokenType::RParen)
            {
                loop {
                    let expr = self.parse_expression()?;
                    let item = if self.match_token(TokenType::As) {
                        let alias = self.expect_identifier_or_safe_keyword_with_quoted()?;
                        Expression::Alias(Box::new(Alias {
                            this: expr,
                            alias,
                            column_aliases: Vec::new(),
                            alias_explicit_as: false,
                            alias_keyword: None,
                            pre_alias_comments: Vec::new(),
                            trailing_comments: Vec::new(),
                            inferred_type: None,
                        }))
                    } else {
                        expr
                    };
                    items.push(item);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            if items.len() == 1 {
                items.pop().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?
            } else if items.is_empty() {
                Expression::Tuple(Box::new(Tuple {
                    expressions: Vec::new(),
                }))
            } else {
                Expression::Tuple(Box::new(Tuple { expressions: items }))
            }
        } else {
            self.parse_table_expression()?
        })
    }

    fn parse_join_table_suffix(
        &mut self,
        kind: &JoinKind,
        table: Expression,
    ) -> Result<Expression> {
        Ok(
            if matches!(
                kind,
                JoinKind::AsOf | JoinKind::AsOfLeft | JoinKind::AsOfRight
            ) && (self.check(TokenType::Offset) || self.check(TokenType::Limit))
                && self
                    .peek_nth(1)
                    .map(|t| t.text.eq_ignore_ascii_case("MATCH_CONDITION"))
                    == Some(true)
            {
                let alias_name = self.advance_text()?;
                Expression::Alias(Box::new(Alias {
                    this: table,
                    alias: Identifier::new(alias_name),
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    pre_alias_comments: Vec::new(),
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                }))
            } else {
                table
            },
        )
    }

    fn parse_deferred_join_conditions(
        &mut self,
        mut joins: Vec<Join>,
        joins_before: usize,
    ) -> Result<Vec<Join>> {
        let unconditioned: Vec<usize> = joins[joins_before..]
            .iter()
            .enumerate()
            .filter(|(_, j)| j.on.is_none() && j.using.is_empty())
            .map(|(i, _)| joins_before + i)
            .collect();

        let mut idx = unconditioned.len();
        while idx > 0 {
            if self.match_token(TokenType::On) {
                idx -= 1;
                let join_idx = unconditioned[idx];
                joins[join_idx].on = Some(self.parse_expression()?);
                joins[join_idx].deferred_condition = true;
            } else if self.match_token(TokenType::Using) {
                idx -= 1;
                let join_idx = unconditioned[idx];
                let has_parens = self.match_token(TokenType::LParen);
                let cols = if has_parens && self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_using_column_list()?
                };
                joins[join_idx].using = cols;
                if has_parens {
                    self.expect_token(TokenType::RParen)?;
                }
                joins[join_idx].deferred_condition = true;
            } else {
                break;
            }
        }
        Ok(joins)
    }

    /// Check if the current token starts a JOIN clause
    pub(crate) fn check_join_keyword(&self) -> bool {
        self.check(TokenType::Join)
            || self.check(TokenType::Inner)
            || self.check(TokenType::Left)
            || self.check(TokenType::Right)
            || self.check(TokenType::Full)
            || self.check(TokenType::Cross)
            || self.check(TokenType::Natural)
            || self.check(TokenType::Outer)
    }

    /// Parses a join kind with its keyword flags and hint.
    pub(crate) fn try_parse_join_kind(&mut self) -> Option<JoinKindParts> {
        if self.match_token(TokenType::AsOf) {
            if self.match_token(TokenType::Left) {
                let use_outer = self.match_token(TokenType::Outer);
                Some(JoinKindParts {
                    kind: JoinKind::AsOfLeft,
                    needs_join_keyword: true,
                    use_inner_keyword: false,
                    use_outer_keyword: use_outer,
                    hint: None,
                })
            } else if self.match_token(TokenType::Right) {
                let use_outer = self.match_token(TokenType::Outer);
                Some(JoinKindParts {
                    kind: JoinKind::AsOfRight,
                    needs_join_keyword: true,
                    use_inner_keyword: false,
                    use_outer_keyword: use_outer,
                    hint: None,
                })
            } else if self.match_token(TokenType::Inner) {
                Some(JoinKindParts {
                    kind: JoinKind::AsOf,
                    needs_join_keyword: true,
                    use_inner_keyword: true,
                    use_outer_keyword: false,
                    hint: None,
                })
            } else {
                Some(JoinKindParts {
                    kind: JoinKind::AsOf,
                    needs_join_keyword: true,
                    use_inner_keyword: false,
                    use_outer_keyword: false,
                    hint: None,
                })
            }
        } else if self.check(TokenType::Inner) {
            let saved = self.current;
            self.skip();
            if self.check(TokenType::Union)
                || self.check(TokenType::Intersect)
                || self.check(TokenType::Except)
            {
                self.current = saved;
                return None;
            }
            let join_hint = self.parse_tsql_join_hint();
            Some(JoinKindParts {
                kind: JoinKind::Inner,
                needs_join_keyword: true,
                use_inner_keyword: true,
                use_outer_keyword: false,
                hint: join_hint,
            })
        } else if self.check(TokenType::Left) {
            self.parse_left_join_kind()
        } else if self.check(TokenType::Right) {
            self.parse_right_join_kind()
        } else if self.check(TokenType::Full) {
            self.parse_full_join_kind()
        } else if self.match_token(TokenType::Cross) {
            if self.match_token(TokenType::Apply) {
                Some(JoinKindParts {
                    kind: JoinKind::CrossApply,
                    needs_join_keyword: false,
                    use_inner_keyword: false,
                    use_outer_keyword: false,
                    hint: None,
                })
            } else {
                Some(JoinKindParts {
                    kind: JoinKind::Cross,
                    needs_join_keyword: true,
                    use_inner_keyword: false,
                    use_outer_keyword: false,
                    hint: None,
                })
            }
        } else if self.match_token(TokenType::Natural) {
            self.parse_natural_join_kind()
        } else if self.match_token(TokenType::Outer) {
            if self.match_token(TokenType::Apply) {
                Some(JoinKindParts {
                    kind: JoinKind::OuterApply,
                    needs_join_keyword: false,
                    use_inner_keyword: false,
                    use_outer_keyword: true,
                    hint: None,
                })
            } else {
                Some(JoinKindParts {
                    kind: JoinKind::Outer,
                    needs_join_keyword: true,
                    use_inner_keyword: false,
                    use_outer_keyword: true,
                    hint: None,
                })
            }
        } else if self.check(TokenType::Lateral) {
            if self.current + 1 < self.tokens.len()
                && self.tokens[self.current + 1].token_type == TokenType::View
            {
                None
            } else {
                self.skip();
                Some(JoinKindParts {
                    kind: JoinKind::Lateral,
                    needs_join_keyword: true,
                    use_inner_keyword: false,
                    use_outer_keyword: false,
                    hint: None,
                })
            }
        } else if self.match_token(TokenType::Semi) {
            Some(JoinKindParts {
                kind: JoinKind::Semi,
                needs_join_keyword: true,
                use_inner_keyword: false,
                use_outer_keyword: false,
                hint: None,
            })
        } else if self.match_token(TokenType::Anti) {
            Some(JoinKindParts {
                kind: JoinKind::Anti,
                needs_join_keyword: true,
                use_inner_keyword: false,
                use_outer_keyword: false,
                hint: None,
            })
        } else if self.check_identifier("POSITIONAL") && self.check_next(TokenType::Join) {
            self.skip();
            Some(JoinKindParts {
                kind: JoinKind::Positional,
                needs_join_keyword: true,
                use_inner_keyword: false,
                use_outer_keyword: false,
                hint: None,
            })
        } else if self.match_token(TokenType::StraightJoin) {
            Some(JoinKindParts {
                kind: JoinKind::Straight,
                needs_join_keyword: false,
                use_inner_keyword: false,
                use_outer_keyword: false,
                hint: None,
            })
        } else if self.check(TokenType::Join) {
            Some(JoinKindParts {
                kind: JoinKind::Inner,
                needs_join_keyword: true,
                use_inner_keyword: false,
                use_outer_keyword: false,
                hint: None,
            })
        } else if self.match_token(TokenType::Comma) {
            Some(JoinKindParts {
                kind: JoinKind::Implicit,
                needs_join_keyword: false,
                use_inner_keyword: false,
                use_outer_keyword: false,
                hint: None,
            })
        } else {
            None
        }
    }

    fn parse_left_join_kind(&mut self) -> Option<JoinKindParts> {
        let saved = self.current;
        self.skip();
        let at_set_op = self.check(TokenType::Union)
            || self.check(TokenType::Intersect)
            || self.check(TokenType::Except);
        let at_inner_set_op = self.check(TokenType::Inner) && {
            let saved2 = self.current;
            self.skip();
            let is_setop = self.check(TokenType::Union)
                || self.check(TokenType::Intersect)
                || self.check(TokenType::Except);
            self.current = saved2;
            is_setop
        };
        if at_set_op || at_inner_set_op {
            self.current = saved;
            return None;
        }
        self.current = saved;
        self.match_token(TokenType::Left);
        let use_outer = self.match_token(TokenType::Outer);
        let use_inner = self.match_token(TokenType::Inner);
        let join_hint = self.parse_tsql_join_hint();
        if self.match_token(TokenType::Semi) {
            Some(JoinKindParts {
                kind: JoinKind::LeftSemi,
                needs_join_keyword: true,
                use_inner_keyword: use_inner,
                use_outer_keyword: use_outer,
                hint: join_hint,
            })
        } else if self.match_token(TokenType::Anti) {
            Some(JoinKindParts {
                kind: JoinKind::LeftAnti,
                needs_join_keyword: true,
                use_inner_keyword: use_inner,
                use_outer_keyword: use_outer,
                hint: join_hint,
            })
        } else if self.match_token(TokenType::Lateral) {
            Some(JoinKindParts {
                kind: JoinKind::LeftLateral,
                needs_join_keyword: true,
                use_inner_keyword: use_inner,
                use_outer_keyword: use_outer,
                hint: join_hint,
            })
        } else {
            Some(JoinKindParts {
                kind: JoinKind::Left,
                needs_join_keyword: true,
                use_inner_keyword: use_inner,
                use_outer_keyword: use_outer,
                hint: join_hint,
            })
        }
    }

    fn parse_right_join_kind(&mut self) -> Option<JoinKindParts> {
        let saved = self.current;
        self.skip();
        let at_set_op = self.check(TokenType::Union)
            || self.check(TokenType::Intersect)
            || self.check(TokenType::Except);
        let at_inner_set_op = self.check(TokenType::Inner) && {
            let saved2 = self.current;
            self.skip();
            let is_setop = self.check(TokenType::Union)
                || self.check(TokenType::Intersect)
                || self.check(TokenType::Except);
            self.current = saved2;
            is_setop
        };
        if at_set_op || at_inner_set_op {
            self.current = saved;
            return None;
        }
        self.current = saved;
        self.match_token(TokenType::Right);
        let use_outer = self.match_token(TokenType::Outer);
        let use_inner = self.match_token(TokenType::Inner);
        let join_hint = self.parse_tsql_join_hint();
        if self.match_token(TokenType::Semi) {
            Some(JoinKindParts {
                kind: JoinKind::RightSemi,
                needs_join_keyword: true,
                use_inner_keyword: use_inner,
                use_outer_keyword: use_outer,
                hint: join_hint,
            })
        } else if self.match_token(TokenType::Anti) {
            Some(JoinKindParts {
                kind: JoinKind::RightAnti,
                needs_join_keyword: true,
                use_inner_keyword: use_inner,
                use_outer_keyword: use_outer,
                hint: join_hint,
            })
        } else {
            Some(JoinKindParts {
                kind: JoinKind::Right,
                needs_join_keyword: true,
                use_inner_keyword: use_inner,
                use_outer_keyword: use_outer,
                hint: join_hint,
            })
        }
    }

    fn parse_full_join_kind(&mut self) -> Option<JoinKindParts> {
        let saved = self.current;
        self.skip();
        let at_set_op = self.check(TokenType::Union)
            || self.check(TokenType::Intersect)
            || self.check(TokenType::Except);
        let at_inner_set_op = self.check(TokenType::Inner) && {
            let saved2 = self.current;
            self.skip();
            let is_setop = self.check(TokenType::Union)
                || self.check(TokenType::Intersect)
                || self.check(TokenType::Except);
            self.current = saved2;
            is_setop
        };
        if at_set_op || at_inner_set_op {
            self.current = saved;
            return None;
        }
        self.current = saved;
        self.match_token(TokenType::Full);
        let use_outer = self.match_token(TokenType::Outer);
        let join_hint = self.parse_tsql_join_hint();
        Some(JoinKindParts {
            kind: JoinKind::Full,
            needs_join_keyword: true,
            use_inner_keyword: false,
            use_outer_keyword: use_outer,
            hint: join_hint,
        })
    }

    fn parse_natural_join_kind(&mut self) -> Option<JoinKindParts> {
        if self.match_token(TokenType::Left) {
            let use_outer = self.match_token(TokenType::Outer);
            Some(JoinKindParts {
                kind: JoinKind::NaturalLeft,
                needs_join_keyword: true,
                use_inner_keyword: false,
                use_outer_keyword: use_outer,
                hint: None,
            })
        } else if self.match_token(TokenType::Right) {
            let use_outer = self.match_token(TokenType::Outer);
            Some(JoinKindParts {
                kind: JoinKind::NaturalRight,
                needs_join_keyword: true,
                use_inner_keyword: false,
                use_outer_keyword: use_outer,
                hint: None,
            })
        } else if self.match_token(TokenType::Full) {
            let use_outer = self.match_token(TokenType::Outer);
            Some(JoinKindParts {
                kind: JoinKind::NaturalFull,
                needs_join_keyword: true,
                use_inner_keyword: false,
                use_outer_keyword: use_outer,
                hint: None,
            })
        } else {
            if self.match_token(TokenType::Inner) {
                Some(JoinKindParts {
                    kind: JoinKind::Natural,
                    needs_join_keyword: true,
                    use_inner_keyword: true,
                    use_outer_keyword: false,
                    hint: None,
                })
            } else {
                Some(JoinKindParts {
                    kind: JoinKind::Natural,
                    needs_join_keyword: true,
                    use_inner_keyword: false,
                    use_outer_keyword: false,
                    hint: None,
                })
            }
        }
    }

    /// Parse TSQL join hints: LOOP, HASH, MERGE, REMOTE
    pub(crate) fn parse_tsql_join_hint(&mut self) -> Option<String> {
        if self.check_identifier("LOOP") {
            self.skip();
            Some("LOOP".to_string())
        } else if self.check_identifier("HASH") {
            self.skip();
            Some("HASH".to_string())
        } else if self.check_identifier("REMOTE") {
            self.skip();
            Some("REMOTE".to_string())
        } else if self.check(TokenType::Merge) && {
            let next_pos = self.current + 1;
            next_pos < self.tokens.len() && self.tokens[next_pos].token_type == TokenType::Join
        } {
            self.skip();
            Some("MERGE".to_string())
        } else {
            None
        }
    }
}
