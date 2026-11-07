//! Parser queries: clauses.

use crate::expressions::functions::models::Function;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Null;
use crate::expressions::operators::main::maybe_rebalance_boolean_chain::maybe_rebalance_boolean_chain;
use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::operators::models::Cube;
use crate::expressions::operators::models::ResultSerialization;
use crate::expressions::operators::models::Rollup;
use crate::expressions::operators::models::WithFill;
use crate::expressions::queries::models::ClusterBy;
use crate::expressions::queries::models::Connect;
use crate::expressions::queries::models::ConnectByRoot;
use crate::expressions::queries::models::DistributeBy;
use crate::expressions::queries::models::Fetch;
use crate::expressions::queries::models::From;
use crate::expressions::queries::models::GroupBy;
use crate::expressions::queries::models::Hint;
use crate::expressions::queries::models::HintExpression;
use crate::expressions::queries::models::LateralView;
use crate::expressions::queries::models::Limit;
use crate::expressions::queries::models::MatchRecognize;
use crate::expressions::queries::models::MatchRecognizeAfter;
use crate::expressions::queries::models::MatchRecognizeMeasure;
use crate::expressions::queries::models::MatchRecognizeRows;
use crate::expressions::queries::models::MatchRecognizeSemantics;
use crate::expressions::queries::models::NamedWindow;
use crate::expressions::queries::models::Offset;
use crate::expressions::queries::models::OrderBy;
use crate::expressions::queries::models::OrderByAll;
use crate::expressions::queries::models::Ordered;
use crate::expressions::queries::models::Over;
use crate::expressions::queries::models::Prior;
use crate::expressions::queries::models::Select;
use crate::expressions::queries::models::SortBy;
use crate::expressions::queries::models::Subquery;
use crate::expressions::statements::models::Lock;
use crate::expressions::statements::models::Var;
use crate::expressions::tables::models::QueryOption;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::Tuple;
use crate::parser::models::Parser;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parse GROUP BY clause
    pub(crate) fn parse_group_by(&mut self) -> Result<GroupBy> {
        let all = if self.match_token(TokenType::All) {
            Some(true)
        } else if self.match_token(TokenType::Distinct) {
            Some(false)
        } else {
            None
        };

        let mut expressions: Vec<Expression> = Vec::new();

        if all.is_some() && self.is_at_query_modifier_or_end() {
            return Ok(GroupBy {
                expressions,
                all,
                totals: false,
                comments: Vec::new(),
            });
        }

        if all.is_some()
            && self.check(TokenType::With)
            && (self.check_next(TokenType::Cube)
                || self.check_next(TokenType::Rollup)
                || self.check_next_identifier("TOTALS"))
        {
            let mut totals = false;
            if self.check_next(TokenType::Cube) || self.check_next(TokenType::Rollup) {
                self.skip();
                if self.match_token(TokenType::Cube) {
                    expressions.push(Expression::Cube(Box::new(Cube {
                        options: None,

                        expressions: Vec::new(),
                    })));
                } else if self.match_token(TokenType::Rollup) {
                    expressions.push(Expression::Rollup(Box::new(Rollup {
                        options: None,

                        expressions: Vec::new(),
                    })));
                }
            }
            if self.check(TokenType::With) && self.check_next_identifier("TOTALS") {
                self.skip();
                self.skip();
                totals = true;
            }
            return Ok(GroupBy {
                expressions,
                all,
                totals,
                comments: Vec::new(),
            });
        }

        loop {
            let expr = {
                if self.check_identifier("GROUPING")
                    && self
                        .peek_nth(1)
                        .is_some_and(|t| t.text.eq_ignore_ascii_case("SETS"))
                    && {
                        self.skip();
                        self.skip();
                        true
                    }
                {
                    self.expect_token(TokenType::LParen)?;
                    let args = self.parse_grouping_sets_args()?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: "GROUPING SETS".to_string(),
                        args,
                        distinct: false,
                        trailing_comments: Vec::new(),
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                    }))
                } else if self.match_token(TokenType::Cube) {
                    self.expect_token(TokenType::LParen)?;
                    let args = self.parse_expression_list()?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: "CUBE".to_string(),
                        args,
                        distinct: false,
                        trailing_comments: Vec::new(),
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                    }))
                } else if self.match_token(TokenType::Rollup) {
                    self.expect_token(TokenType::LParen)?;
                    let args = self.parse_expression_list()?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: "ROLLUP".to_string(),
                        args,
                        distinct: false,
                        trailing_comments: Vec::new(),
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                    }))
                } else {
                    self.parse_expression()?
                }
            };

            let expr = { expr };

            expressions.push(expr);

            if !self.match_token(TokenType::Comma) {
                if self.check(TokenType::Cube)
                    || self.check(TokenType::Rollup)
                    || self.check_identifier("GROUPING")
                        && self
                            .peek_nth(1)
                            .is_some_and(|t| t.text.eq_ignore_ascii_case("SETS"))
                {
                    continue;
                }
                break;
            }
        }

        if self.check(TokenType::With)
            && (self.check_next(TokenType::Cube) || self.check_next(TokenType::Rollup))
        {
            self.skip();
            if self.match_token(TokenType::Cube) {
                expressions.push(Expression::Cube(Box::new(Cube {
                    options: None,

                    expressions: Vec::new(),
                })));
            } else if self.match_token(TokenType::Rollup) {
                expressions.push(Expression::Rollup(Box::new(Rollup {
                    options: None,

                    expressions: Vec::new(),
                })));
            }
        }

        let totals = if self.check(TokenType::With) && self.check_next_identifier("TOTALS") {
            self.skip();
            self.skip();
            true
        } else {
            false
        };

        Ok(GroupBy {
            expressions,
            all,
            totals,
            comments: Vec::new(),
        })
    }

    /// Parse GROUPING SETS arguments which can include tuples like (x, y), nested GROUPING SETS, CUBE, ROLLUP
    pub(crate) fn parse_grouping_sets_args(&mut self) -> Result<Vec<Expression>> {
        self.with_parser_depth(|parser| parser.parse_grouping_sets_args_inner())
    }

    #[inline(never)]
    pub(crate) fn parse_grouping_sets_args_inner(&mut self) -> Result<Vec<Expression>> {
        let mut args: Vec<Expression> = Vec::new();

        loop {
            let expr = {
                if self.check_identifier("GROUPING")
                    && self
                        .peek_nth(1)
                        .is_some_and(|t| t.text.eq_ignore_ascii_case("SETS"))
                    && {
                        self.skip();
                        self.skip();
                        true
                    }
                {
                    self.expect_token(TokenType::LParen)?;
                    let inner_args = self.parse_grouping_sets_args()?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: "GROUPING SETS".to_string(),
                        args: inner_args,
                        distinct: false,
                        trailing_comments: Vec::new(),
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                    }))
                } else if self.match_token(TokenType::Cube) {
                    self.expect_token(TokenType::LParen)?;
                    let inner_args = self.parse_expression_list()?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: "CUBE".to_string(),
                        args: inner_args,
                        distinct: false,
                        trailing_comments: Vec::new(),
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                    }))
                } else if self.match_token(TokenType::Rollup) {
                    self.expect_token(TokenType::LParen)?;
                    let inner_args = self.parse_expression_list()?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: "ROLLUP".to_string(),
                        args: inner_args,
                        distinct: false,
                        trailing_comments: Vec::new(),
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                    }))
                } else if self.check(TokenType::LParen) {
                    self.skip();
                    if self.check(TokenType::RParen) {
                        self.skip();
                        Expression::Tuple(Box::new(Tuple {
                            expressions: Vec::new(),
                        }))
                    } else {
                        let inner = self.parse_expression_list()?;
                        self.expect_token(TokenType::RParen)?;
                        Expression::Tuple(Box::new(Tuple { expressions: inner }))
                    }
                } else {
                    self.parse_expression()?
                }
            };

            args.push(expr);

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(args)
    }

    /// Parse ORDER BY clause
    pub(crate) fn parse_order_by(&mut self) -> Result<OrderBy> {
        self.parse_order_by_with_siblings(false)
    }

    pub(crate) fn maybe_parse_clickhouse_order_collate(
        &mut self,
        expr: Expression,
    ) -> Result<Expression> {
        Ok(expr)
    }

    /// Parse ORDER BY clause with optional siblings flag (Oracle ORDER SIBLINGS BY)
    pub(crate) fn parse_order_by_with_siblings(&mut self, siblings: bool) -> Result<OrderBy> {
        let mut expressions: Vec<Ordered> = Vec::new();

        loop {
            let expr = self.parse_expression()?;

            let expr = { expr };

            let (desc, explicit_asc) = if self.match_token(TokenType::Desc) {
                (true, false)
            } else if self.match_token(TokenType::Asc) {
                (false, true)
            } else {
                (false, false)
            };

            let expr = self.maybe_parse_clickhouse_order_collate(expr)?;

            let nulls_auto = false;
            let nulls_first = if self.match_token(TokenType::Nulls) {
                if self.match_token(TokenType::First) {
                    Some(true)
                } else if self.match_token(TokenType::Last) {
                    Some(false)
                } else {
                    return Err(self.parse_error("Expected FIRST, LAST or AUTO after NULLS"));
                }
            } else {
                None
            };

            let with_fill = if self.match_text_seq(&["WITH", "FILL"]) {
                let from_ = if self.match_token(TokenType::From) {
                    Some(Box::new(self.parse_or()?))
                } else {
                    None
                };
                let to = if self.match_text_seq(&["TO"]) {
                    Some(Box::new(self.parse_or()?))
                } else {
                    None
                };
                let step = if self.match_text_seq(&["STEP"]) {
                    Some(Box::new(self.parse_or()?))
                } else {
                    None
                };
                let staleness = if self.match_text_seq(&["STALENESS"]) {
                    Some(Box::new(self.parse_or()?))
                } else {
                    None
                };
                let interpolate = if self.match_text_seq(&["INTERPOLATE"]) {
                    if self.match_token(TokenType::LParen) {
                        let mut items: Vec<Expression> = Vec::new();
                        loop {
                            if self.check(TokenType::RParen) {
                                break;
                            }
                            let quoted = self.check(TokenType::QuotedIdentifier);
                            let name_text = self.expect_identifier_or_safe_keyword()?;
                            let name_id = Identifier {
                                name: name_text,
                                quoted,
                                trailing_comments: Vec::new(),
                                span: None,
                            };
                            let item = if self.match_token(TokenType::As) {
                                let expr = self.parse_expression()?;
                                Expression::Alias(Box::new(Alias {
                                    this: expr,
                                    alias: name_id,
                                    column_aliases: Vec::new(),
                                    alias_explicit_as: false,
                                    alias_keyword: None,
                                    pre_alias_comments: Vec::new(),
                                    trailing_comments: Vec::new(),
                                    inferred_type: None,
                                    span: SourceSpan::default(),
                                }))
                            } else {
                                Expression::Identifier(name_id)
                            };
                            items.push(item);
                            if !self.match_token(TokenType::Comma) {
                                break;
                            }
                        }
                        self.expect_token(TokenType::RParen)?;
                        if items.len() == 1 {
                            Some(Box::new(items.into_iter().next().ok_or_else(|| {
                                crate::tokens::errors::Error::internal(
                                    "a value the parser checked is missing",
                                )
                            })?))
                        } else {
                            Some(Box::new(Expression::Tuple(Box::new(
                                crate::expressions::types::models::Tuple { expressions: items },
                            ))))
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                Some(Box::new(WithFill {
                    from_,
                    to,
                    step,
                    staleness,
                    interpolate,
                }))
            } else {
                None
            };

            expressions.push(Ordered {
                nulls_auto,
                this: expr,
                desc,
                nulls_first,
                explicit_asc,
                with_fill,
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }

            if self.is_at_end() || self.check(TokenType::Semicolon) {
                break;
            }
        }

        let (expressions, all) = self.take_order_by_all(expressions);
        Ok(OrderBy {
            expressions,
            siblings,
            comments: Vec::new(),
            all,
        })
    }

    /// Takes a lone unquoted `ALL` sort key as the ORDER BY ALL keyword, not an identifier.
    pub(crate) fn take_order_by_all(
        &self,
        mut expressions: Vec<Ordered>,
    ) -> (Vec<Ordered>, Option<OrderByAll>) {
        let supported = self
            .config
            .dialect
            .is_some_and(|d| d.supports_order_by_all());
        let lone_all = matches!(expressions.as_slice(), [ordered] if matches!(&ordered.this,
            Expression::Column(c)
                if c.table.is_none() && !c.name.quoted && c.name.name.eq_ignore_ascii_case("all")));
        if !(supported && lone_all) {
            return (expressions, None);
        }
        let all = expressions.pop().map(|ordered| OrderByAll {
            desc: ordered.desc,
            nulls_first: ordered.nulls_first,
            explicit_asc: ordered.explicit_asc,
        });
        (expressions, all)
    }

    /// Parse a LIMIT count without mistaking a trailing percent sign for modulo.
    pub(crate) fn parse_limit_value(&mut self) -> Result<(Expression, bool)> {
        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) && (self.is_at_end() || self.check(TokenType::Semicolon))
        {
            return Ok((Expression::Null(Null), false));
        }
        if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
            && (self.check(TokenType::String) || self.check(TokenType::DollarString))
            && !self.peek().text.is_empty()
        {
            return Err(
                self.parse_error("Expected a row count; only an empty string means no limit")
            );
        }
        let saved_pos = self.current;
        if let Ok(expr) = self.parse_unary() {
            if self.check(TokenType::Percent) && self.is_percent_modifier() {
                self.skip();
                return Ok((expr, true));
            }
        }
        self.current = saved_pos;
        let expr = self.parse_expression()?;
        let percent = self.check(TokenType::Percent) && self.is_percent_modifier();
        if percent {
            self.skip();
        }
        Ok((expr, percent))
    }

    /// Parses a query-level LIMIT/OFFSET/FETCH tail with SELECT's count parser.
    pub(crate) fn parse_row_limit_modifiers(
        &mut self,
    ) -> Result<(Option<Limit>, Option<Offset>, Option<Fetch>)> {
        let (mut limit, mut offset, mut fetch) = (None, None, None);
        loop {
            if self.check(TokenType::Limit) {
                if limit.is_some() || fetch.is_some() {
                    return Err(self.parse_error("Multiple LIMIT/FETCH clauses in the same query"));
                }
                let comments = self.current_leading_comments().to_vec();
                self.skip();
                let (mut this, percent) = self.parse_limit_value()?;
                if self.match_token(TokenType::Comma) {
                    if offset.is_some() || percent {
                        return Err(self.parse_error("Invalid LIMIT offset/count combination"));
                    }
                    offset = Some(Offset { this, rows: None });
                    this = self.parse_expression()?;
                }
                limit = Some(Limit {
                    this,
                    percent,
                    comments,
                });
            } else if self.match_token(TokenType::Offset) {
                if offset.is_some() {
                    return Err(self.parse_error("Multiple OFFSET clauses in the same query"));
                }
                let this = self.parse_expression()?;
                let rows = (self.match_token(TokenType::Row) || self.match_token(TokenType::Rows))
                    .then_some(true);
                offset = Some(Offset { this, rows });
            } else if self.match_token(TokenType::Fetch) {
                if limit.is_some() || fetch.is_some() {
                    return Err(self.parse_error("Multiple LIMIT/FETCH clauses in the same query"));
                }
                fetch = Some(self.parse_fetch()?);
            } else {
                break;
            }
        }
        Ok((limit, offset, fetch))
    }

    /// Parse modifiers following a parenthesized query without changing its inner scope.
    pub(crate) fn parse_query_modifiers(&mut self, inner: Expression) -> Result<Expression> {
        let distribute_by = if self.match_keywords(&[TokenType::Distribute, TokenType::By]) {
            let exprs = self.parse_expression_list()?;
            Some(DistributeBy { expressions: exprs })
        } else {
            None
        };

        let (sort_by, cluster_by) = if self.match_keywords(&[TokenType::Sort, TokenType::By]) {
            let mut orders: Vec<Ordered> = Vec::new();
            loop {
                if let Some(ordered) = self.parse_ordered_item()? {
                    orders.push(ordered);
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            (
                Some(SortBy {
                    expressions: orders,
                }),
                None,
            )
        } else if self.match_keywords(&[TokenType::Cluster, TokenType::By]) {
            let mut orders: Vec<Ordered> = Vec::new();
            loop {
                if let Some(ordered) = self.parse_ordered_item()? {
                    orders.push(ordered);
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            (
                None,
                Some(ClusterBy {
                    expressions: orders,
                }),
            )
        } else {
            (None, None)
        };

        let order_by = if self.match_keywords(&[TokenType::Order, TokenType::By]) {
            Some(self.parse_order_by()?)
        } else {
            None
        };

        let (limit, offset, fetch) = self.parse_row_limit_modifiers()?;
        if fetch.is_some() || limit.as_ref().is_some_and(|limit| limit.percent) {
            let mut subquery = match inner {
                Expression::Subquery(subquery) => *subquery,
                this => Subquery {
                    this,
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
                },
            };
            subquery
                .alias
                .get_or_insert_with(|| Identifier::new("_l_0"));
            let mut select = Select::new();
            select.expressions = vec![Expression::star()];
            select.from = Some(From {
                expressions: vec![Expression::Subquery(Box::new(subquery))],
            });
            select.order_by = order_by;
            select.offset = offset;
            select.limit = limit;
            select.fetch = fetch;
            select.distribute_by = distribute_by;
            select.sort_by = sort_by;
            select.cluster_by = cluster_by;
            return Ok(Expression::Select(Box::new(select)));
        }

        if order_by.is_some()
            || limit.is_some()
            || offset.is_some()
            || distribute_by.is_some()
            || sort_by.is_some()
            || cluster_by.is_some()
        {
            if let Expression::Subquery(mut subq) = inner {
                subq.order_by = order_by;
                subq.limit = limit;
                subq.offset = offset;
                subq.distribute_by = distribute_by;
                subq.sort_by = sort_by;
                subq.cluster_by = cluster_by;
                Ok(Expression::Subquery(subq))
            } else if let Expression::Paren(paren) = inner {
                Ok(Expression::Subquery(Box::new(Subquery {
                    this: Expression::Paren(paren),
                    alias: None,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    order_by,
                    limit,
                    offset,
                    distribute_by,
                    sort_by,
                    cluster_by,
                    lateral: false,
                    modifiers_inside: false,
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                    span: SourceSpan::default(),
                })))
            } else {
                Ok(Expression::Subquery(Box::new(Subquery {
                    this: inner,
                    alias: None,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    order_by,
                    limit,
                    offset,
                    distribute_by,
                    sort_by,
                    cluster_by,
                    lateral: false,
                    modifiers_inside: false,
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                    span: SourceSpan::default(),
                })))
            }
        } else {
            Ok(inner)
        }
    }

    /// Parses ORDER BY items inside aggregate calls.
    pub(crate) fn parse_order_by_list(&mut self) -> Result<Vec<Ordered>> {
        let mut expressions: Vec<Ordered> = Vec::new();

        loop {
            let expr = self.parse_expression()?;

            let (desc, explicit_asc) = if self.match_token(TokenType::Desc) {
                (true, false)
            } else if self.match_token(TokenType::Asc) {
                (false, true)
            } else {
                (false, false)
            };

            let nulls_auto = false;
            let nulls_first = if self.match_token(TokenType::Nulls) {
                if self.match_token(TokenType::First) {
                    Some(true)
                } else if self.match_token(TokenType::Last) {
                    Some(false)
                } else {
                    return Err(self.parse_error("Expected FIRST, LAST or AUTO after NULLS"));
                }
            } else {
                None
            };

            expressions.push(Ordered {
                nulls_auto,
                this: expr,
                desc,
                nulls_first,
                explicit_asc,
                with_fill: None,
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(expressions)
    }

    /// Parse DISTRIBUTE BY clause (Hive/Spark)
    pub(crate) fn parse_distribute_by(&mut self) -> Result<DistributeBy> {
        let mut expressions: Vec<Expression> = Vec::new();

        loop {
            expressions.push(self.parse_expression()?);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(DistributeBy { expressions })
    }

    /// Parse CLUSTER BY clause (Hive/Spark)
    pub(crate) fn parse_cluster_by(&mut self) -> Result<ClusterBy> {
        let mut expressions: Vec<Ordered> = Vec::new();

        loop {
            let expr = self.parse_expression()?;

            let (desc, explicit_asc) = if self.match_token(TokenType::Desc) {
                (true, false)
            } else if self.match_token(TokenType::Asc) {
                (false, true)
            } else {
                (false, false)
            };

            expressions.push(Ordered {
                nulls_auto: false,
                this: expr,
                desc,
                nulls_first: None,
                explicit_asc,
                with_fill: None,
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(ClusterBy { expressions })
    }

    /// Parse SORT BY clause (Hive/Spark)
    pub(crate) fn parse_sort_by(&mut self) -> Result<SortBy> {
        let mut expressions: Vec<Ordered> = Vec::new();

        loop {
            let expr = self.parse_expression()?;

            let (desc, explicit_asc) = if self.match_token(TokenType::Desc) {
                (true, false)
            } else if self.match_token(TokenType::Asc) {
                (false, true)
            } else {
                (false, false)
            };

            let nulls_auto = false;
            let nulls_first = if self.match_token(TokenType::Nulls) {
                if self.match_token(TokenType::First) {
                    Some(true)
                } else if self.match_token(TokenType::Last) {
                    Some(false)
                } else {
                    return Err(self.parse_error("Expected FIRST, LAST or AUTO after NULLS"));
                }
            } else {
                None
            };

            expressions.push(Ordered {
                nulls_auto,
                this: expr,
                desc,
                nulls_first,
                explicit_asc,
                with_fill: None,
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(SortBy { expressions })
    }

    /// Parses FOR UPDATE/SHARE locking clauses, MySQL LOCK IN SHARE MODE and TSQL FOR XML/JSON.
    pub(crate) fn parse_locks_and_for_xml(
        &mut self,
    ) -> Result<(Vec<Lock>, Vec<Expression>, Vec<Expression>)> {
        let mut locks: Vec<Lock> = Vec::new();
        let mut for_xml: Vec<Expression> = Vec::new();
        let mut for_json: Vec<Expression> = Vec::new();

        loop {
            let (update, key) = if self.match_keywords(&[TokenType::For, TokenType::Update]) {
                (
                    Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: true,
                    }))),
                    None,
                )
            } else if self.check(TokenType::For) && self.check_next_identifier("XML") {
                self.skip();
                self.skip();
                for_xml = self.parse_for_xml_options()?;
                break;
            } else if self.check(TokenType::For) && self.check_next_identifier("JSON") {
                self.skip();
                self.skip();
                for_json = self.parse_for_json_options()?;
                break;
            } else if self.check(TokenType::For) && self.check_next_identifier("SHARE") {
                self.skip();
                self.skip();

                (None, None)
            } else if self.check_identifier("LOCK") && self.check_next(TokenType::In) {
                self.skip();
                self.skip();
                if self.match_identifier("SHARE") {
                    let _ = self.match_identifier("MODE");
                }
                (None, None)
            } else if self.check(TokenType::For) && self.check_next(TokenType::Key) {
                self.skip();
                self.skip();
                if !self.match_identifier("SHARE") {
                    break;
                }
                (
                    None,
                    Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: true,
                    }))),
                )
            } else if self.check(TokenType::For) && self.check_next(TokenType::No) {
                self.skip();
                self.skip();
                if !self.match_identifier("KEY") || !self.match_token(TokenType::Update) {
                    break;
                }
                (
                    Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: true,
                    }))),
                    Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: true,
                    }))),
                )
            } else {
                break;
            };

            let expressions = if self.match_token(TokenType::Of) {
                let mut tables: Vec<Expression> = Vec::new();
                loop {
                    let table = self.parse_table_ref()?;
                    tables.push(Expression::Table(Box::new(table)));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                tables
            } else {
                Vec::new()
            };

            let wait = if self.match_identifier("NOWAIT") {
                Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                })))
            } else if self.match_identifier("WAIT") {
                Some(Box::new(self.parse_primary()?))
            } else if self.match_identifier("SKIP") && self.match_identifier("LOCKED") {
                Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: false,
                })))
            } else {
                None
            };

            let ignore_locked = false;
            locks.push(Lock {
                ignore_locked,
                update,
                expressions,
                wait,
                key,
            });
        }

        Ok((locks, for_xml, for_json))
    }

    pub(crate) fn parse_hana_select_options(
        &mut self,
    ) -> Result<(
        Option<ResultSerialization>,
        Option<Identifier>,
        Vec<Expression>,
    )> {
        Ok((None, None, Vec::new()))
    }

    /// Parses TSQL FOR XML PATH|RAW|AUTO|EXPLICIT and its options.
    pub(crate) fn parse_for_xml_options(&mut self) -> Result<Vec<Expression>> {
        let mut options: Vec<Expression> = Vec::new();

        loop {
            if let Some(opt) = self.parse_for_xml_single_option()? {
                options.push(opt);
            } else {
                break;
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(options)
    }

    /// Parse a single FOR XML option
    pub(crate) fn parse_for_xml_single_option(&mut self) -> Result<Option<Expression>> {
        if self.match_identifier("PATH") {
            let expression = if self.match_token(TokenType::LParen) {
                let expr = self.parse_string()?;
                self.expect_token(TokenType::RParen)?;
                expr
            } else {
                None
            };
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "PATH".to_string(),
                }))),
                expression: expression.map(Box::new),
            }))));
        }

        if self.match_identifier("RAW") {
            let expression = if self.match_token(TokenType::LParen) {
                let expr = self.parse_string()?;
                self.expect_token(TokenType::RParen)?;
                expr
            } else {
                None
            };
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "RAW".to_string(),
                }))),
                expression: expression.map(Box::new),
            }))));
        }

        if self.match_identifier("AUTO") {
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "AUTO".to_string(),
                }))),
                expression: None,
            }))));
        }

        if self.match_identifier("EXPLICIT") {
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "EXPLICIT".to_string(),
                }))),
                expression: None,
            }))));
        }

        if self.match_identifier("TYPE") || self.match_token(TokenType::Type) {
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "TYPE".to_string(),
                }))),
                expression: None,
            }))));
        }

        if self.match_identifier("BINARY") {
            if self.match_identifier("BASE64") {
                return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                    this: Box::new(Expression::Var(Box::new(Var {
                        this: "BINARY BASE64".to_string(),
                    }))),
                    expression: None,
                }))));
            } else {
                return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                    this: Box::new(Expression::Var(Box::new(Var {
                        this: "BINARY".to_string(),
                    }))),
                    expression: None,
                }))));
            }
        }

        if self.match_identifier("ELEMENTS") {
            let suboption = if self.match_identifier("XSINIL") {
                Some("XSINIL".to_string())
            } else if self.match_identifier("ABSENT") {
                Some("ABSENT".to_string())
            } else {
                None
            };
            let option_name = match &suboption {
                Some(sub) => format!("ELEMENTS {}", sub),
                None => "ELEMENTS".to_string(),
            };
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var { this: option_name }))),
                expression: None,
            }))));
        }

        if self.match_identifier("ROOT") {
            let expression = if self.match_token(TokenType::LParen) {
                let expr = self.parse_string()?;
                self.expect_token(TokenType::RParen)?;
                expr
            } else {
                None
            };
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "ROOT".to_string(),
                }))),
                expression: expression.map(Box::new),
            }))));
        }

        Ok(None)
    }

    /// Parses TSQL FOR JSON PATH|AUTO and its options.
    pub(crate) fn parse_for_json_options(&mut self) -> Result<Vec<Expression>> {
        let mut options: Vec<Expression> = Vec::new();

        loop {
            if let Some(opt) = self.parse_for_json_single_option()? {
                options.push(opt);
            } else {
                break;
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(options)
    }

    /// Parse a single FOR JSON option
    pub(crate) fn parse_for_json_single_option(&mut self) -> Result<Option<Expression>> {
        if self.match_identifier("PATH") {
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "PATH".to_string(),
                }))),
                expression: None,
            }))));
        }

        if self.match_identifier("AUTO") {
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "AUTO".to_string(),
                }))),
                expression: None,
            }))));
        }

        if self.match_identifier("ROOT") {
            let expression = if self.match_token(TokenType::LParen) {
                let expr = self.parse_string()?;
                self.expect_token(TokenType::RParen)?;
                expr
            } else {
                None
            };
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "ROOT".to_string(),
                }))),
                expression: expression.map(Box::new),
            }))));
        }

        if self.match_identifier("INCLUDE_NULL_VALUES") {
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "INCLUDE_NULL_VALUES".to_string(),
                }))),
                expression: None,
            }))));
        }

        if self.match_identifier("WITHOUT_ARRAY_WRAPPER") {
            return Ok(Some(Expression::QueryOption(Box::new(QueryOption {
                this: Box::new(Expression::Var(Box::new(Var {
                    this: "WITHOUT_ARRAY_WRAPPER".to_string(),
                }))),
                expression: None,
            }))));
        }

        Ok(None)
    }

    /// Parses Oracle [START WITH cond] CONNECT BY [NOCYCLE] cond, START WITH before or after.
    pub(crate) fn parse_connect(&mut self) -> Result<Option<Connect>> {
        let start_before = if self.match_keywords(&[TokenType::Start, TokenType::With]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        if !self.match_keywords(&[TokenType::Connect, TokenType::By]) {
            if start_before.is_some() {
                return Err(self.parse_error("START WITH without CONNECT BY"));
            }
            return Ok(None);
        }

        let nocycle = self.match_token(TokenType::NoCycle);

        let connect = self.parse_connect_expression()?;

        let start = if start_before.is_some() {
            start_before
        } else if self.match_keywords(&[TokenType::Start, TokenType::With]) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Some(Connect {
            start,
            connect,
            nocycle,
        }))
    }

    /// Parse expression in CONNECT BY context, treating PRIOR as prefix operator
    pub(crate) fn parse_connect_expression(&mut self) -> Result<Expression> {
        self.parse_connect_or()
    }

    /// Parse OR expression in CONNECT BY context
    pub(crate) fn parse_connect_or(&mut self) -> Result<Expression> {
        let mut left = self.parse_connect_and()?;

        while self.match_token(TokenType::Or) {
            let right = self.parse_connect_and()?;
            left = Expression::Or(Box::new(BinaryOp::new(left, right)));
        }

        Ok(maybe_rebalance_boolean_chain(left, false))
    }

    /// Parse AND expression in CONNECT BY context
    pub(crate) fn parse_connect_and(&mut self) -> Result<Expression> {
        let mut left = self.parse_connect_comparison()?;

        while self.match_token(TokenType::And) {
            let right = self.parse_connect_comparison()?;
            left = Expression::And(Box::new(BinaryOp::new(left, right)));
        }

        Ok(maybe_rebalance_boolean_chain(left, true))
    }

    /// Parse comparison in CONNECT BY context
    pub(crate) fn parse_connect_comparison(&mut self) -> Result<Expression> {
        let left = self.parse_connect_primary()?;

        if self.match_token(TokenType::Eq) {
            let right = self.parse_connect_primary()?;
            return Ok(Expression::Eq(Box::new(BinaryOp::new(left, right))));
        }
        if self.match_token(TokenType::Neq) {
            let right = self.parse_connect_primary()?;
            return Ok(Expression::Neq(Box::new(BinaryOp::new(left, right))));
        }
        if self.match_token(TokenType::Lt) {
            let right = self.parse_connect_primary()?;
            return Ok(Expression::Lt(Box::new(BinaryOp::new(left, right))));
        }
        if self.match_token(TokenType::Lte) {
            let right = self.parse_connect_primary()?;
            return Ok(Expression::Lte(Box::new(BinaryOp::new(left, right))));
        }
        if self.match_token(TokenType::Gt) {
            let right = self.parse_connect_primary()?;
            return Ok(Expression::Gt(Box::new(BinaryOp::new(left, right))));
        }
        if self.match_token(TokenType::Gte) {
            let right = self.parse_connect_primary()?;
            return Ok(Expression::Gte(Box::new(BinaryOp::new(left, right))));
        }

        Ok(left)
    }

    /// Parse primary in CONNECT BY context with PRIOR support
    pub(crate) fn parse_connect_primary(&mut self) -> Result<Expression> {
        if self.match_token(TokenType::Prior) {
            let expr = self.parse_primary()?;
            return Ok(Expression::Prior(Box::new(Prior { this: expr })));
        }

        if let Some(connect_by_root) = self.try_parse_connect_by_root_expression()? {
            return Ok(connect_by_root);
        }

        self.parse_primary()
    }

    /// Parses Oracle CONNECT_BY_ROOT col or CONNECT_BY_ROOT(col).
    pub(crate) fn try_parse_connect_by_root_expression(&mut self) -> Result<Option<Expression>> {
        if !(self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("CONNECT_BY_ROOT"))
        {
            return Ok(None);
        }

        self.skip();

        let this = if self.match_token(TokenType::LParen) {
            let expr = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            expr
        } else {
            self.parse_column()?.ok_or_else(|| {
                self.parse_error("Expected expression or column after CONNECT_BY_ROOT")
            })?
        };

        Ok(Some(Expression::ConnectByRoot(Box::new(ConnectByRoot {
            this,
        }))))
    }

    /// Parses MATCH_RECOGNIZE (PARTITION BY, ORDER BY, MEASURES, PATTERN, DEFINE, ...).
    pub(crate) fn parse_match_recognize(
        &mut self,
        source: Option<Expression>,
    ) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;

        let partition_by = if self.match_keywords(&[TokenType::Partition, TokenType::By]) {
            Some(self.parse_expression_list()?)
        } else {
            None
        };

        let order_by = if self.match_keywords(&[TokenType::Order, TokenType::By]) {
            Some(self.parse_order_by()?.expressions)
        } else {
            None
        };

        let measures = if self.match_token(TokenType::Measures) {
            Some(self.parse_match_recognize_measures()?)
        } else {
            None
        };

        let rows = self.parse_match_recognize_rows()?;

        let after = self.parse_match_recognize_after()?;

        let pattern = if self.match_token(TokenType::Pattern) {
            Some(self.parse_match_recognize_pattern()?)
        } else {
            None
        };

        let define = if self.match_token(TokenType::Define) {
            Some(self.parse_match_recognize_define()?)
        } else {
            None
        };

        self.expect_token(TokenType::RParen)?;

        Ok(Expression::MatchRecognize(Box::new(MatchRecognize {
            this: source.map(Box::new),
            partition_by,
            order_by,
            measures,
            rows,
            after,
            pattern,
            define,
            alias: None,
            alias_explicit_as: false,
            options: Vec::new(),
        })))
    }

    /// Parse MEASURES clause in MATCH_RECOGNIZE
    pub(crate) fn parse_match_recognize_measures(&mut self) -> Result<Vec<MatchRecognizeMeasure>> {
        let mut measures: Vec<MatchRecognizeMeasure> = Vec::new();

        loop {
            let window_frame = if self.match_token(TokenType::Running) {
                Some(MatchRecognizeSemantics::Running)
            } else if self.match_token(TokenType::Final) {
                Some(MatchRecognizeSemantics::Final)
            } else {
                None
            };

            let mut expr = self.parse_expression()?;

            if self.match_token(TokenType::As) {
                let alias = self.parse_unquoted_identifier(Self::expect_identifier)?;
                expr = Expression::Alias(Box::new(Alias::new(expr, alias)));
            }

            measures.push(MatchRecognizeMeasure {
                this: expr,
                window_frame,
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(measures)
    }

    /// Parse row semantics in MATCH_RECOGNIZE
    pub(crate) fn parse_match_recognize_rows(&mut self) -> Result<Option<MatchRecognizeRows>> {
        if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("ONE") {
            self.skip();
            if !self.match_token(TokenType::Row) {
                return Err(self.parse_error("Expected ROW after ONE"));
            }
            if !(self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("PER")) {
                return Err(self.parse_error("Expected PER after ONE ROW"));
            }
            self.skip();
            if !self.match_token(TokenType::Match) {
                return Err(self.parse_error("Expected MATCH after ONE ROW PER"));
            }
            return Ok(Some(MatchRecognizeRows::OneRowPerMatch));
        }

        if self.match_token(TokenType::All) {
            if !self.match_token(TokenType::Rows) {
                return Err(self.parse_error("Expected ROWS after ALL"));
            }
            if !(self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("PER")) {
                return Err(self.parse_error("Expected PER after ALL ROWS"));
            }
            self.skip();
            if !self.match_token(TokenType::Match) {
                return Err(self.parse_error("Expected MATCH after ALL ROWS PER"));
            }

            if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("SHOW") {
                self.skip();
                if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("EMPTY") {
                    self.skip();
                    if self.check(TokenType::Var)
                        && self.peek_text().eq_ignore_ascii_case("MATCHES")
                    {
                        self.skip();
                        return Ok(Some(MatchRecognizeRows::AllRowsPerMatchShowEmptyMatches));
                    }
                }
                return Err(self.parse_error("Expected EMPTY MATCHES after SHOW"));
            }

            if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("OMIT") {
                self.skip();
                if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("EMPTY") {
                    self.skip();
                    if self.check(TokenType::Var)
                        && self.peek_text().eq_ignore_ascii_case("MATCHES")
                    {
                        self.skip();
                        return Ok(Some(MatchRecognizeRows::AllRowsPerMatchOmitEmptyMatches));
                    }
                }
                return Err(self.parse_error("Expected EMPTY MATCHES after OMIT"));
            }

            if self.match_token(TokenType::With) {
                if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("UNMATCHED")
                {
                    self.skip();
                    if self.match_token(TokenType::Rows) {
                        return Ok(Some(MatchRecognizeRows::AllRowsPerMatchWithUnmatchedRows));
                    }
                }
                return Err(self.parse_error("Expected UNMATCHED ROWS after WITH"));
            }

            return Ok(Some(MatchRecognizeRows::AllRowsPerMatch));
        }

        Ok(None)
    }

    /// Parse AFTER MATCH SKIP clause in MATCH_RECOGNIZE
    pub(crate) fn parse_match_recognize_after(&mut self) -> Result<Option<MatchRecognizeAfter>> {
        if !self.match_token(TokenType::After) {
            return Ok(None);
        }

        if !self.match_token(TokenType::Match) {
            return Err(self.parse_error("Expected MATCH after AFTER"));
        }

        if !(self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("SKIP")) {
            return Err(self.parse_error("Expected SKIP after AFTER MATCH"));
        }
        self.skip();

        if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("PAST") {
            self.skip();
            if self.match_token(TokenType::Last) && self.match_token(TokenType::Row) {
                return Ok(Some(MatchRecognizeAfter::PastLastRow));
            }
            return Err(self.parse_error("Expected LAST ROW after PAST"));
        }

        if self.match_token(TokenType::To) {
            if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("NEXT") {
                self.skip();
                if self.match_token(TokenType::Row) {
                    return Ok(Some(MatchRecognizeAfter::ToNextRow));
                }
                return Err(self.parse_error("Expected ROW after NEXT"));
            }

            if self.match_token(TokenType::First) {
                let name = self.expect_identifier()?;
                return Ok(Some(MatchRecognizeAfter::ToFirst(Identifier::new(name))));
            }

            if self.match_token(TokenType::Last) {
                let name = self.expect_identifier()?;
                return Ok(Some(MatchRecognizeAfter::ToLast(Identifier::new(name))));
            }

            return Err(self.parse_error("Expected NEXT ROW, FIRST x, or LAST x after TO"));
        }

        Err(self.parse_error("Expected PAST LAST ROW or TO ... after AFTER MATCH SKIP"))
    }

    /// Parse PATTERN clause in MATCH_RECOGNIZE using bracket counting
    pub(crate) fn parse_match_recognize_pattern(&mut self) -> Result<String> {
        self.expect_token(TokenType::LParen)?;

        let mut depth = 1;
        let mut pattern = String::new();

        while depth > 0 && !self.is_at_end() {
            let token = self.advance()?;
            match token.token_type {
                TokenType::LParen => {
                    depth += 1;
                    pattern.push('(');
                }
                TokenType::RParen => {
                    depth -= 1;
                    if depth > 0 {
                        pattern.push(')');
                    }
                }
                _ => {
                    let is_quantifier = matches!(token.text.as_str(), "+" | "*" | "?")
                        || token.text.starts_with('{');

                    if !pattern.is_empty()
                        && !pattern.ends_with('(')
                        && !pattern.ends_with(' ')
                        && !is_quantifier
                    {
                        pattern.push(' ');
                    }
                    pattern.push_str(&token.text);
                }
            }
        }

        if depth > 0 {
            return Err(self.parse_error("Unclosed parenthesis in PATTERN clause"));
        }

        Ok(pattern.trim().to_string())
    }

    /// Parse DEFINE clause in MATCH_RECOGNIZE
    pub(crate) fn parse_match_recognize_define(&mut self) -> Result<Vec<(Identifier, Expression)>> {
        let mut definitions: Vec<(Identifier, Expression)> = Vec::new();

        loop {
            let name = self.parse_unquoted_identifier(Self::expect_identifier)?;
            self.expect_token(TokenType::As)?;
            let expr = self.parse_expression()?;

            definitions.push((name, expr));

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(definitions)
    }

    /// Parses Hive/Spark LATERAL VIEW [OUTER] generator(args) alias AS col [, ...].
    pub(crate) fn parse_lateral_views(&mut self) -> Result<Vec<LateralView>> {
        let mut views: Vec<LateralView> = Vec::new();

        while self.match_keywords(&[TokenType::Lateral, TokenType::View]) {
            let outer = self.match_token(TokenType::Outer);

            let this = self.parse_primary()?;

            let table_alias = if self.check(TokenType::Var) && !self.check_keyword() {
                Some(self.parse_unquoted_identifier(Self::expect_identifier)?)
            } else {
                None
            };

            let column_aliases = if self.match_token(TokenType::As) {
                let mut aliases: Vec<Identifier> = Vec::new();
                if self.match_token(TokenType::LParen) {
                    loop {
                        aliases.push(
                            self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?,
                        );
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                } else {
                    loop {
                        aliases.push(
                            self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?,
                        );
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                        if !self.is_identifier_or_keyword_token() {
                            break;
                        }
                        if self.peek().token_type == TokenType::Lateral
                            || self.peek().token_type == TokenType::Where
                            || self.peek().token_type == TokenType::Group
                            || self.peek().token_type == TokenType::Having
                            || self.peek().token_type == TokenType::Order
                            || self.peek().token_type == TokenType::Limit
                        {
                            break;
                        }
                    }
                }
                aliases
            } else {
                Vec::new()
            };

            views.push(LateralView {
                this,
                table_alias,
                column_aliases,
                outer,
            });
        }

        Ok(views)
    }

    /// Parse named windows (WINDOW w AS (...), ...)
    pub(crate) fn parse_named_windows(&mut self) -> Result<Vec<NamedWindow>> {
        let mut windows: Vec<NamedWindow> = Vec::new();

        loop {
            let name = self.parse_unquoted_identifier(Self::expect_identifier)?;
            self.expect_token(TokenType::As)?;
            self.expect_token(TokenType::LParen)?;

            let window_name = if (self.check(TokenType::Identifier)
                || self.check(TokenType::Var)
                || self.check(TokenType::QuotedIdentifier))
                && !self.check(TokenType::Partition)
                && !self.check(TokenType::Order)
                && self.peek_nth(1).is_none_or(|t| {
                    matches!(
                        t.token_type,
                        TokenType::Partition
                            | TokenType::Order
                            | TokenType::Rows
                            | TokenType::Range
                            | TokenType::Groups
                            | TokenType::RParen
                            | TokenType::Comma
                    )
                }) {
                Some(self.parse_unquoted_identifier(Self::expect_identifier)?)
            } else {
                None
            };

            let partition_by = if self.match_keywords(&[TokenType::Partition, TokenType::By]) {
                Some(self.parse_expression_list()?)
            } else {
                None
            };

            let order_by = if self.match_keywords(&[TokenType::Order, TokenType::By]) {
                Some(self.parse_order_by()?)
            } else {
                None
            };

            let frame = self.parse_window_frame()?;

            self.expect_token(TokenType::RParen)?;

            windows.push(NamedWindow {
                name,
                spec: Over {
                    window_name,
                    partition_by: partition_by.unwrap_or_default(),
                    order_by: order_by.map(|o| o.expressions).unwrap_or_default(),
                    frame,
                    alias: None,
                },
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(windows)
    }

    /// Parse query hint /*+ ... */
    pub(crate) fn parse_hint(&mut self) -> Result<Hint> {
        let token = self.advance()?;
        let hint_text = token.text.to_string();

        let expressions = if hint_text.is_empty() {
            Vec::new()
        } else {
            vec![HintExpression::Raw(hint_text)]
        };

        Ok(Hint {
            source_dialect: None,
            expressions,
        })
    }
}
