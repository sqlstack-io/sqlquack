//! Parser queries: modifiers.

use crate::expressions::functions::models::Function;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::operators::models::OnConflict;
use crate::expressions::queries::main::wrap_values_set_operand::wrap_values_set_operand;
use crate::expressions::queries::models::Except;
use crate::expressions::queries::models::Intersect;
use crate::expressions::queries::models::Join;
use crate::expressions::queries::models::Limit;
use crate::expressions::queries::models::OrderBy;
use crate::expressions::queries::models::Sample;
use crate::expressions::queries::models::SampleMethod;
use crate::expressions::queries::models::Subquery;
use crate::expressions::queries::models::Union;
use crate::expressions::queries::models::Where;
use crate::expressions::statements::models::Annotated;
use crate::expressions::statements::models::Command;
use crate::expressions::statements::models::Insert;
use crate::expressions::statements::models::Update;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::Tuple;
use crate::parser::constants::PERCENT_SIGN;
use crate::parser::models::JoinKindParts;
use crate::parser::models::Parser;
use crate::parser::models::SetOperationCorresponding;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parse SAMPLE / TABLESAMPLE / USING SAMPLE clause
    pub(crate) fn parse_sample_clause(&mut self) -> Result<Option<Sample>> {
        let is_using_sample = if self.check(TokenType::Using)
            && self.current + 1 < self.tokens.len()
            && self.tokens[self.current + 1].token_type == TokenType::Sample
        {
            self.skip();
            self.skip();
            true
        } else {
            false
        };

        let use_sample_keyword = if is_using_sample || self.match_token(TokenType::Sample) {
            true
        } else if self.match_token(TokenType::TableSample) {
            false
        } else {
            return Ok(None);
        };

        let (method, method_before_size, explicit_method) =
            if self.match_token(TokenType::Bernoulli) {
                (SampleMethod::Bernoulli, true, true)
            } else if self.match_token(TokenType::System) {
                (SampleMethod::System, true, true)
            } else if self.match_token(TokenType::Block) {
                (SampleMethod::Block, true, true)
            } else if self.match_token(TokenType::Row) {
                (SampleMethod::Row, true, true)
            } else if self.check_identifier("RESERVOIR") {
                self.skip();
                (SampleMethod::Reservoir, true, true)
            } else {
                (SampleMethod::Bernoulli, false, false)
            };

        let has_paren = self.match_token(TokenType::LParen);

        if self.match_identifier("BUCKET") {
            return self.parse_bucket_sample(has_paren, is_using_sample, use_sample_keyword);
        }

        let size = self.parse_unary()?;

        let (method, unit_after_size, is_percent) = if self.check(TokenType::Percent) {
            self.skip();
            if method_before_size {
                (method, true, true)
            } else {
                (SampleMethod::Percent, true, true)
            }
        } else if self.match_token(TokenType::Rows) {
            if method_before_size {
                (method, true, false)
            } else {
                (SampleMethod::Row, true, false)
            }
        } else {
            (method, false, false)
        };

        if has_paren {
            self.expect_token(TokenType::RParen)?;
        }

        let (method, seed, use_seed_keyword, explicit_method) =
            if is_using_sample && self.check(TokenType::LParen) {
                self.skip();
                let method_from_parens =
                    if self.check_identifier("BERNOULLI") || self.check(TokenType::Bernoulli) {
                        self.skip();
                        Some(SampleMethod::Bernoulli)
                    } else if self.check_identifier("SYSTEM") || self.check(TokenType::System) {
                        self.skip();
                        Some(SampleMethod::System)
                    } else if self.check_identifier("RESERVOIR") {
                        self.skip();
                        Some(SampleMethod::Reservoir)
                    } else {
                        None
                    };
                let seed = if self.match_token(TokenType::Comma) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.expect_token(TokenType::RParen)?;
                let final_method = method_from_parens.unwrap_or(method);
                (final_method, seed, false, true)
            } else {
                let (seed, use_seed_keyword) = if self.match_texts(&["SEED"]) {
                    self.expect_token(TokenType::LParen)?;
                    let seed_value = self.parse_expression()?;
                    self.expect_token(TokenType::RParen)?;
                    (Some(seed_value), true)
                } else if self.match_texts(&["REPEATABLE"]) {
                    self.expect_token(TokenType::LParen)?;
                    let seed_value = self.parse_expression()?;
                    self.expect_token(TokenType::RParen)?;
                    (Some(seed_value), false)
                } else {
                    (None, false)
                };
                let explicit_method = explicit_method || unit_after_size;
                (method, seed, use_seed_keyword, explicit_method)
            };

        let (method, unit_after_size) = if is_using_sample && !explicit_method {
            (SampleMethod::Reservoir, false)
        } else if is_using_sample && unit_after_size && !method_before_size {
            if matches!(method, SampleMethod::Percent) {
                (SampleMethod::System, true)
            } else if matches!(method, SampleMethod::Row) {
                (SampleMethod::Reservoir, true)
            } else {
                (method, unit_after_size)
            }
        } else {
            (method, unit_after_size)
        };

        Ok(Some(Sample {
            method,
            size,
            seed,
            offset: None,
            unit_after_size,
            use_sample_keyword,
            explicit_method: true,
            method_before_size: true,
            use_seed_keyword,
            bucket_numerator: None,
            bucket_denominator: None,
            bucket_field: None,
            is_using_sample,
            is_percent,
            suppress_method_output: false,
        }))
    }

    fn parse_bucket_sample(
        &mut self,
        has_paren: bool,
        is_using_sample: bool,
        use_sample_keyword: bool,
    ) -> Result<Option<Sample>> {
        let bucket_numerator = self.parse_primary()?;
        self.match_identifier("OUT");
        self.match_token(TokenType::Of);
        let bucket_denominator = self.parse_primary()?;
        let bucket_field = if self.match_token(TokenType::On) {
            Some(Box::new(self.parse_primary()?))
        } else {
            None
        };
        if has_paren {
            self.expect_token(TokenType::RParen)?;
        }
        Ok(Some(Sample {
            method: SampleMethod::Bucket,
            size: bucket_numerator.clone(),
            seed: None,
            offset: None,
            unit_after_size: false,
            use_sample_keyword,
            explicit_method: true,
            method_before_size: false,
            use_seed_keyword: false,
            bucket_numerator: Some(Box::new(bucket_numerator)),
            bucket_denominator: Some(Box::new(bucket_denominator)),
            bucket_field,
            is_using_sample,
            is_percent: false,
            suppress_method_output: false,
        }))
    }

    /// Parses table-level TABLESAMPLE/SAMPLE method(size [PERCENT|ROWS]).
    pub(crate) fn parse_table_level_sample(&mut self) -> Result<Option<Sample>> {
        let use_sample_keyword = if self.match_token(TokenType::Sample) {
            true
        } else if self.match_token(TokenType::TableSample) {
            false
        } else {
            return Ok(None);
        };
        let _ = use_sample_keyword;

        let (method, explicit_method, method_before_size) = if self.check_identifier("RESERVOIR") {
            self.skip();
            (SampleMethod::Reservoir, true, true)
        } else if self.match_token(TokenType::Bernoulli) {
            (SampleMethod::Bernoulli, true, true)
        } else if self.match_token(TokenType::System) {
            (SampleMethod::System, true, true)
        } else if self.match_token(TokenType::Block) {
            (SampleMethod::Block, true, true)
        } else if self.match_token(TokenType::Row) {
            (SampleMethod::Row, true, true)
        } else {
            (SampleMethod::Bernoulli, false, false)
        };

        self.expect_token(TokenType::LParen)?;

        if self.match_identifier("BUCKET") {
            let bucket_numerator = self.parse_primary()?;
            self.match_identifier("OUT");
            self.match_token(TokenType::Of);
            let bucket_denominator = self.parse_primary()?;
            let bucket_field = if self.match_token(TokenType::On) {
                Some(Box::new(self.parse_primary()?))
            } else {
                None
            };
            self.expect_token(TokenType::RParen)?;
            return Ok(Some(Sample {
                method: SampleMethod::Bucket,
                size: bucket_numerator.clone(),
                seed: None,
                offset: None,
                unit_after_size: false,
                use_sample_keyword,
                explicit_method: true,
                method_before_size: false,
                use_seed_keyword: false,
                bucket_numerator: Some(Box::new(bucket_numerator)),
                bucket_denominator: Some(Box::new(bucket_denominator)),
                bucket_field,
                is_using_sample: false,
                is_percent: false,
                suppress_method_output: false,
            }));
        }

        let size = self.parse_unary()?;

        let (method, unit_after_size, is_percent) =
            if self.check(TokenType::Percent) && self.peek_text().eq_ignore_ascii_case("PERCENT") {
                self.skip();
                if explicit_method {
                    (method, true, true)
                } else {
                    (SampleMethod::Percent, true, true)
                }
            } else if self.match_token(TokenType::Rows) {
                if explicit_method {
                    (method, true, false)
                } else {
                    (SampleMethod::Row, true, false)
                }
            } else if self.check(TokenType::Percent) && self.peek_text() == PERCENT_SIGN {
                self.skip();
                if explicit_method {
                    (method, true, true)
                } else {
                    (SampleMethod::Percent, true, true)
                }
            } else {
                (method, false, false)
            };

        self.expect_token(TokenType::RParen)?;

        let (seed, use_seed_keyword) = if self.match_texts(&["SEED"]) {
            self.expect_token(TokenType::LParen)?;
            let seed_value = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            (Some(seed_value), true)
        } else if self.match_texts(&["REPEATABLE"]) {
            self.expect_token(TokenType::LParen)?;
            let seed_value = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            (Some(seed_value), false)
        } else {
            (None, false)
        };

        Ok(Some(Sample {
            method,
            size,
            seed,
            offset: None,
            unit_after_size,
            use_sample_keyword,
            explicit_method,
            method_before_size,
            use_seed_keyword,
            bucket_numerator: None,
            bucket_denominator: None,
            bucket_field: None,
            is_using_sample: false,
            is_percent,
            suppress_method_output: false,
        }))
    }

    pub(crate) fn check_set_operation_start(&self, token_type: TokenType) -> bool {
        if !self.check(token_type) {
            return false;
        }

        true
    }

    /// Parse set operations (UNION, INTERSECT, EXCEPT)
    pub(crate) fn parse_set_operation(&mut self, left: Expression) -> Result<Expression> {
        let mut result = left;
        let mut found_set_op = false;

        loop {
            let (side, kind) = self.parse_set_operation_side_kind();
            if matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Snowflake)
            ) && (side.is_some() || kind.is_some())
            {
                return Err(self.parse_error(
                    "Snowflake set operations take no LEFT, RIGHT, FULL, INNER or OUTER",
                ));
            }

            let set_op_leading_comments = if self.check_set_operation_start(TokenType::Union)
                || self.check_set_operation_start(TokenType::Intersect)
                || self.check_set_operation_start(TokenType::Except)
            {
                self.current_leading_comments().to_vec()
            } else {
                Vec::new()
            };

            let left = if !set_op_leading_comments.is_empty() {
                Expression::Annotated(Box::new(Annotated {
                    this: result,
                    trailing_comments: set_op_leading_comments,
                    type_assertion: None,
                    type_like: None,
                }))
            } else {
                result
            };

            if self.check_set_operation_start(TokenType::Union) {
                self.skip();
                let all = self.match_token(TokenType::All);
                let distinct = if !all {
                    self.match_token(TokenType::Distinct)
                } else {
                    false
                };

                let SetOperationCorresponding {
                    by_name,
                    strict,
                    corresponding,
                    on_columns,
                } = self.parse_set_operation_corresponding(true)?;

                let kind = if corresponding && !strict && side.is_none() && kind.is_none() {
                    Some("INNER".to_string())
                } else {
                    kind
                };

                let mut right = self.parse_select_or_paren_select()?;
                let (order_by, limit, offset) =
                    self.detach_unparenthesized_set_operation_modifiers(&mut right);
                result = Expression::Union(Box::new(Union {
                    left: wrap_values_set_operand(left),
                    right: wrap_values_set_operand(right),
                    all,
                    distinct,
                    with: None,
                    order_by,
                    limit,
                    offset,
                    distribute_by: None,
                    sort_by: None,
                    cluster_by: None,
                    by_name,
                    side,
                    kind,
                    corresponding,
                    strict,
                    on_columns,
                    span: SourceSpan::default(),
                    locks: Vec::new(),
                }));
                found_set_op = true;
            } else if self.check_set_operation_start(TokenType::Intersect) {
                self.skip();
                let all = self.match_token(TokenType::All);
                let distinct = if !all {
                    self.match_token(TokenType::Distinct)
                } else {
                    false
                };

                let SetOperationCorresponding {
                    by_name,
                    strict,
                    corresponding,
                    on_columns,
                } = self.parse_set_operation_corresponding(false)?;

                let kind = if corresponding && !strict && side.is_none() && kind.is_none() {
                    Some("INNER".to_string())
                } else {
                    kind
                };

                let mut right = self.parse_select_or_paren_select()?;
                let (order_by, limit, offset) =
                    self.detach_unparenthesized_set_operation_modifiers(&mut right);
                result = Expression::Intersect(Box::new(Intersect {
                    left: wrap_values_set_operand(left),
                    right: wrap_values_set_operand(right),
                    all,
                    distinct,
                    with: None,
                    order_by,
                    limit,
                    offset,
                    distribute_by: None,
                    sort_by: None,
                    cluster_by: None,
                    by_name,
                    side,
                    kind,
                    corresponding,
                    strict,
                    on_columns,
                    span: SourceSpan::default(),
                    locks: Vec::new(),
                }));
                if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake) {
                    result = Self::snowflake_intersect_precedence(result);
                }
                found_set_op = true;
            } else if self.check_set_operation_start(TokenType::Except) {
                self.skip();
                let all = self.match_token(TokenType::All);
                let distinct = if !all {
                    self.match_token(TokenType::Distinct)
                } else {
                    false
                };

                let SetOperationCorresponding {
                    by_name,
                    strict,
                    corresponding,
                    on_columns,
                } = self.parse_set_operation_corresponding(false)?;

                let kind = if corresponding && !strict && side.is_none() && kind.is_none() {
                    Some("INNER".to_string())
                } else {
                    kind
                };

                let mut right = self.parse_select_or_paren_select()?;
                let (order_by, limit, offset) =
                    self.detach_unparenthesized_set_operation_modifiers(&mut right);
                result = Expression::Except(Box::new(Except {
                    left: wrap_values_set_operand(left),
                    right: wrap_values_set_operand(right),
                    all,
                    distinct,
                    with: None,
                    order_by,
                    limit,
                    offset,
                    distribute_by: None,
                    sort_by: None,
                    cluster_by: None,
                    by_name,
                    side,
                    kind,
                    corresponding,
                    strict,
                    on_columns,
                    span: SourceSpan::default(),
                    locks: Vec::new(),
                }));
                found_set_op = true;
            } else if side.is_some() || kind.is_some() {
                return Err(self.parse_error(
                    "Expected UNION, INTERSECT, or EXCEPT after set operation modifier",
                ));
            } else {
                result = left;
                break;
            }
        }

        if found_set_op {
            result = self.parse_set_operation_modifiers(result)?;
        }
        Ok(result)
    }

    pub(crate) fn snowflake_intersect_precedence(expression: Expression) -> Expression {
        let Expression::Intersect(mut intersect) = expression else {
            return expression;
        };
        match std::mem::replace(&mut intersect.left, Expression::null()) {
            Expression::Annotated(mut annotated) => {
                intersect.left = std::mem::replace(&mut annotated.this, Expression::null());
                annotated.this =
                    Self::snowflake_intersect_precedence(Expression::Intersect(intersect));
                Expression::Annotated(annotated)
            }
            Expression::Union(mut outer) => {
                intersect.left = std::mem::replace(&mut outer.right, Expression::null());
                outer.order_by = intersect.order_by.take();
                outer.limit = intersect.limit.take();
                outer.offset = intersect.offset.take();
                outer.right =
                    Self::snowflake_intersect_precedence(Expression::Intersect(intersect));
                Expression::Union(outer)
            }
            Expression::Except(mut outer) => {
                intersect.left = std::mem::replace(&mut outer.right, Expression::null());
                outer.order_by = intersect.order_by.take();
                outer.limit = intersect.limit.take();
                outer.offset = intersect.offset.take();
                outer.right =
                    Self::snowflake_intersect_precedence(Expression::Intersect(intersect));
                Expression::Except(outer)
            }
            left => {
                intersect.left = left;
                Expression::Intersect(intersect)
            }
        }
    }

    pub(crate) fn detach_unparenthesized_set_operation_modifiers(
        &self,
        expr: &mut Expression,
    ) -> (
        Option<OrderBy>,
        Option<Box<Expression>>,
        Option<Box<Expression>>,
    ) {
        match expr {
            Expression::Annotated(annotated) => {
                self.detach_unparenthesized_set_operation_modifiers(&mut annotated.this)
            }
            Expression::Select(select) => {
                let order_by = select.order_by.take();
                let limit = select.limit.take().map(Limit::into_set_limit).or_else(|| {
                    select
                        .fetch
                        .take()
                        .map(|fetch| Box::new(Expression::Fetch(Box::new(fetch))))
                });
                let offset = select.offset.take().map(|offset| Box::new(offset.this));
                (order_by, limit, offset)
            }
            _ => (None, None, None),
        }
    }

    /// Parses BigQuery's LEFT/RIGHT/FULL and INNER/OUTER before a set operation keyword.
    pub(crate) fn parse_set_operation_side_kind(&mut self) -> (Option<String>, Option<String>) {
        let start = self.current;
        let mut side = None;
        let mut kind = None;

        if self.check(TokenType::Left)
            || self.check(TokenType::Right)
            || self.check(TokenType::Full)
        {
            let saved = self.current;
            let Ok(side_token) = self.advance() else {
                return (None, None);
            };
            let side_text = side_token.text.to_ascii_uppercase();

            side = Some(side_text);

            if self.match_token(TokenType::Outer) {
                kind = Some("OUTER".to_string());
            } else if self.match_token(TokenType::Inner) {
                kind = Some("INNER".to_string());
            }

            if !(self.check_set_operation_start(TokenType::Union)
                || self.check_set_operation_start(TokenType::Intersect)
                || self.check_set_operation_start(TokenType::Except))
            {
                self.current = saved;
                return (None, None);
            }
        }

        if side.is_none() && (self.check(TokenType::Inner) || self.check(TokenType::Outer)) {
            let Ok(kind_text) = self.advance_text() else {
                return (None, None);
            };
            kind = Some(kind_text.to_ascii_uppercase());
            if !(self.check_set_operation_start(TokenType::Union)
                || self.check_set_operation_start(TokenType::Intersect)
                || self.check_set_operation_start(TokenType::Except))
            {
                self.current = start;
                return (None, None);
            }
        }

        (side, kind)
    }

    /// Parses [STRICT] CORRESPONDING or BY NAME after ALL/DISTINCT.
    pub(crate) fn parse_set_operation_corresponding(
        &mut self,
        union: bool,
    ) -> Result<SetOperationCorresponding> {
        let mut by_name = false;
        let mut strict = false;
        let mut corresponding = false;
        let mut on_columns: Vec<Expression> = Vec::new();

        if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
            && self.check(TokenType::By)
            && !self
                .peek_nth(1)
                .is_some_and(|token| token.text.eq_ignore_ascii_case("NAME"))
        {
            return Err(self.parse_error("Expected NAME after BY"));
        }
        if self.match_token(TokenType::By) && self.match_identifier("NAME") {
            by_name = true;
            if matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Snowflake)
            ) && (!union || self.check(TokenType::On))
            {
                return Err(self.parse_error(
                    "Snowflake allows BY NAME only after UNION, without an ON column list",
                ));
            }
        } else if self.match_identifier("STRICT") {
            if self.match_identifier("CORRESPONDING") {
                strict = true;
                corresponding = true;
            } else {
                self.current -= 1;
            }
        } else if self.match_identifier("CORRESPONDING") {
            corresponding = true;
        }

        let has_column_list = by_name && self.match_token(TokenType::On)
            || corresponding && self.match_token(TokenType::By);
        if has_column_list {
            self.expect_token(TokenType::LParen)?;
            on_columns = self
                .parse_identifier_list()?
                .into_iter()
                .map(|id| {
                    Self::parsed_column(Column {
                        name: id,
                        table: None,
                        join_mark: false,
                        trailing_comments: Vec::new(),
                        span: None,
                        inferred_type: None,
                    })
                })
                .collect();
            self.expect_token(TokenType::RParen)?;
        }

        Ok(SetOperationCorresponding {
            by_name,
            strict,
            corresponding,
            on_columns,
        })
    }

    /// Parse ORDER BY, LIMIT, OFFSET modifiers for set operations
    pub(crate) fn parse_set_operation_modifiers(
        &mut self,
        mut query: Expression,
    ) -> Result<Expression> {
        let order_by = if self.match_token(TokenType::Order) {
            self.expect_token(TokenType::By)?;
            Some(self.parse_order_by()?)
        } else {
            None
        };

        let (limit, offset, fetch) = self.parse_row_limit_modifiers()?;
        let limit = limit
            .map(Limit::into_set_limit)
            .or_else(|| fetch.map(|fetch| Box::new(Expression::Fetch(Box::new(fetch)))));
        let offset = offset.map(|offset| Box::new(offset.this));
        let mut expr = &mut query;
        while let Expression::Annotated(annotated) = expr {
            expr = &mut annotated.this;
        }
        let existing_limit = match &*expr {
            Expression::Union(set) => set.limit.is_some(),
            Expression::Intersect(set) => set.limit.is_some(),
            Expression::Except(set) => set.limit.is_some(),
            _ => false,
        };
        if existing_limit && limit.is_some() {
            return Err(self.parse_error("Multiple LIMIT/FETCH clauses in the same query"));
        }

        match expr {
            Expression::Union(ref mut union) => {
                if order_by.is_some() {
                    union.order_by = order_by;
                }
                if limit.is_some() {
                    union.limit = limit;
                }
                if offset.is_some() {
                    union.offset = offset;
                }
            }
            Expression::Intersect(ref mut intersect) => {
                if order_by.is_some() {
                    intersect.order_by = order_by;
                }
                if limit.is_some() {
                    intersect.limit = limit;
                }
                if offset.is_some() {
                    intersect.offset = offset;
                }
            }
            Expression::Except(ref mut except) => {
                if order_by.is_some() {
                    except.order_by = order_by;
                }
                if limit.is_some() {
                    except.limit = limit;
                }
                if offset.is_some() {
                    except.offset = offset;
                }
            }
            _ => {}
        }
        Ok(query)
    }

    /// Parse either a SELECT/VALUES query operand or a parenthesized query operand.
    pub(crate) fn parse_select_or_paren_select(&mut self) -> Result<Expression> {
        self.with_parser_depth(|parser| parser.parse_select_or_paren_select_inner())
    }

    #[inline(never)]
    pub(crate) fn parse_select_or_paren_select_inner(&mut self) -> Result<Expression> {
        if self.match_token(TokenType::LParen) {
            if self.check(TokenType::Select)
                || self.check(TokenType::With)
                || self.check(TokenType::From)
                || self.check(TokenType::Values)
            {
                let query = self.parse_statement()?;
                self.expect_token(TokenType::RParen)?;
                let alias = if self.match_token(TokenType::As) {
                    Some(self.parse_unquoted_identifier(Self::expect_identifier)?)
                } else {
                    None
                };
                Ok(Expression::Subquery(Box::new(Subquery {
                    this: query,
                    alias,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    order_by: None,
                    limit: None,
                    offset: None,
                    lateral: false,
                    modifiers_inside: false,
                    trailing_comments: Vec::new(),
                    distribute_by: None,
                    sort_by: None,
                    cluster_by: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                })))
            } else if self.check(TokenType::LParen) {
                let inner = self.parse_select_or_paren_select()?;
                let result = self.parse_set_operation(inner)?;
                self.expect_token(TokenType::RParen)?;
                let alias = if self.match_token(TokenType::As) {
                    Some(self.parse_unquoted_identifier(Self::expect_identifier)?)
                } else {
                    None
                };
                Ok(Expression::Subquery(Box::new(Subquery {
                    this: result,
                    alias,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    order_by: None,
                    limit: None,
                    offset: None,
                    lateral: false,
                    modifiers_inside: false,
                    trailing_comments: Vec::new(),
                    distribute_by: None,
                    sort_by: None,
                    cluster_by: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                })))
            } else {
                Err(self.parse_error("Expected SELECT, VALUES, or ( after ("))
            }
        } else if self.check(TokenType::Values) {
            self.parse_values()
        } else if self.check(TokenType::From) {
            self.parse_from_first_query()
        } else if self.check(TokenType::With) {
            self.parse_statement()
        } else {
            self.parse_select_body()
        }
    }

    /// Parses INSERT ... ON CONFLICT [target | ON CONSTRAINT name] DO NOTHING | DO UPDATE.
    pub(crate) fn parse_on_conflict(&mut self) -> Result<Expression> {
        let constraint =
            if self.match_token(TokenType::On) && self.match_token(TokenType::Constraint) {
                let name = self.expect_identifier()?;
                Some(Box::new(Expression::Identifier(Identifier::new(name))))
            } else {
                None
            };

        let conflict_keys = if constraint.is_none() && self.match_token(TokenType::LParen) {
            let mut keys: Vec<Expression> = Vec::new();
            while !self.check(TokenType::RParen) && !self.is_at_end() {
                if let Some(ordered) = self.parse_ordered_item()? {
                    keys.push(Expression::Ordered(Box::new(ordered)));
                } else {
                    keys.push(self.parse_expression()?);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            Some(Box::new(Expression::Tuple(Box::new(Tuple {
                expressions: keys,
            }))))
        } else {
            None
        };

        let index_predicate = if self.match_token(TokenType::Where) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        if !self.match_identifier("DO") {
            return Err(self.parse_error("Expected DO after ON CONFLICT"));
        }

        let action = if self.match_identifier("NOTHING") {
            Some(Box::new(Expression::Identifier(Identifier::new(
                "NOTHING".to_string(),
            ))))
        } else if self.match_token(TokenType::Update) {
            self.expect_token(TokenType::Set)?;
            let mut sets: Vec<Expression> = Vec::new();
            loop {
                let col_name = self.expect_identifier_with_quoted()?;
                let column = if self.match_token(TokenType::Dot) {
                    let col = self.expect_identifier_with_quoted()?;
                    Self::parsed_column(Column {
                        name: col,
                        table: Some(col_name),
                        join_mark: false,
                        trailing_comments: Vec::new(),
                        span: None,
                        inferred_type: None,
                    })
                } else {
                    Expression::Identifier(col_name)
                };
                self.expect_token(TokenType::Eq)?;
                let value = self.parse_expression()?;
                sets.push(Expression::Eq(Box::new(BinaryOp {
                    left: column,
                    right: value,
                    left_comments: Vec::new(),
                    operator_comments: Vec::new(),
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                    span: SourceSpan::default(),
                })));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            Some(Box::new(Expression::Tuple(Box::new(Tuple {
                expressions: sets,
            }))))
        } else {
            return Err(self.parse_error("Expected NOTHING or UPDATE after DO"));
        };

        let where_ = if self.match_token(TokenType::Where) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        Ok(Expression::OnConflict(Box::new(OnConflict {
            duplicate: None,
            expressions: Vec::new(),
            action,
            conflict_keys,
            index_predicate,
            constraint,
            where_,
        })))
    }

    /// Parse MySQL REPLACE [INTO] statement or REPLACE() function call
    pub(crate) fn parse_replace(&mut self) -> Result<Expression> {
        let replace_token = self.expect_token(TokenType::Replace)?;
        let leading_comments = replace_token.comments;

        if self.check(TokenType::LParen) {
            self.expect_token(TokenType::LParen)?;
            let args = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: "REPLACE".to_string(),
                args,
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: SourceSpan::default(),
                inferred_type: None,
            })));
        }

        self.match_token(TokenType::Into);

        let table_name = self.expect_identifier_or_safe_keyword_with_quoted()?;
        let table = if self.match_token(TokenType::Dot) {
            let second_name = self.expect_identifier_or_safe_keyword_with_quoted()?;
            TableRef {
                name: second_name,
                schema: Some(table_name),
                catalog: None,
                alias: None,
                alias_explicit_as: false,
                column_aliases: Vec::new(),
                leading_comments: Vec::new(),
                trailing_comments: Vec::new(),
                when: None,
                only: false,
                final_: false,
                table_sample: None,
                hints: Vec::new(),
                system_time: None,
                partitions: Vec::new(),
                identifier_func: None,
                changes: None,
                version: None,
                span: None,
            }
        } else {
            TableRef::new(table_name.name)
        };

        let columns = if self.match_token(TokenType::LParen) {
            let mut cols: Vec<Identifier> = Vec::new();
            loop {
                if self.check(TokenType::RParen) {
                    break;
                }
                let col = self.expect_identifier_with_quoted()?;
                cols.push(col);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            cols
        } else {
            Vec::new()
        };

        let mut values: Vec<Vec<Expression>> = Vec::new();
        let query = if self.match_token(TokenType::Values) {
            loop {
                self.expect_token(TokenType::LParen)?;
                let row = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                values.push(row);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            None
        } else if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            Some(self.parse_statement()?)
        } else {
            None
        };

        Ok(Expression::Insert(Box::new(Insert {
            table,
            columns,
            values,
            query,
            overwrite: false,
            partition: Vec::new(),
            directory: None,
            returning: Vec::new(),
            output: None,
            on_conflict: None,
            leading_comments,
            if_exists: false,
            with: None,
            ignore: false,
            source_alias: None,
            alias: None,
            alias_explicit_as: false,
            default_values: false,
            by_name: false,
            conflict_action: None,
            is_replace: true,
            replace_where: None,
            source: None,
            hint: None,
            function_target: None,
            partition_by: None,
            settings: Vec::new(),
            overriding: None,
            returning_aliases: Vec::new(),
        })))
    }

    /// Parse UPDATE statement
    pub(crate) fn parse_update(&mut self) -> Result<Expression> {
        let update_token = self.expect_token(TokenType::Update)?;
        let leading_comments = update_token.comments;
        let hint = if self.check(TokenType::Hint) {
            Some(self.parse_hint()?)
        } else {
            None
        };

        if self.check_identifier("STATISTICS") {
            let mut parts = vec!["UPDATE".to_string()];
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                parts.push(self.advance_text()?);
            }
            return Ok(Expression::Command(Box::new(Command {
                this: parts.join(" "),
            })));
        }

        let has_only = self.match_token(TokenType::Only);

        let first_name = self.expect_identifier_with_quoted()?;
        let mut table = self.parse_update_table(first_name)?;
        table.trailing_comments = self.previous_trailing_comments().to_vec();
        if has_only {
            table.only = true;
        }

        if self.match_token(TokenType::As) {
            table.alias = Some(self.expect_identifier_with_quoted()?);
            table.alias_explicit_as = true;
        } else if self.is_identifier_token() && !self.check(TokenType::Set) {
            table.alias = Some(self.expect_identifier_with_quoted()?);
            table.alias_explicit_as = false;
        }

        let extra_tables = self.parse_update_extra_tables()?;

        let table_joins = self.parse_update_table_joins()?;

        let (from_before_set, early_from_clause, early_from_joins) =
            if self.match_token(TokenType::From) {
                let from_clause = self.parse_from()?;
                let from_joins = self.parse_joins()?;
                (true, Some(from_clause), from_joins)
            } else {
                (false, None, Vec::new())
            };

        self.expect_token(TokenType::Set)?;

        let set = self.parse_update_assignments()?;

        let output = if self.match_token(TokenType::Output) {
            Some(self.parse_output_clause()?)
        } else {
            None
        };

        let (from_clause, from_joins) = if from_before_set {
            (early_from_clause, early_from_joins)
        } else if self.match_token(TokenType::From) {
            let from_clause = Some(self.parse_from()?);
            let from_joins = self.parse_joins()?;
            (from_clause, from_joins)
        } else {
            (None, Vec::new())
        };

        let where_clause = if self.match_token(TokenType::Where) {
            Some(Where {
                this: self.parse_expression()?,
            })
        } else {
            None
        };

        let returning = if self.match_token(TokenType::Returning) {
            self.parse_select_expressions()?
        } else {
            Vec::new()
        };

        let order_by = if self.match_keywords(&[TokenType::Order, TokenType::By]) {
            Some(self.parse_order_by()?)
        } else {
            None
        };

        let limit = if self.match_token(TokenType::Limit) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Expression::Update(Box::new(Update {
            table,
            hint,
            extra_tables,
            table_joins,
            set,
            from_clause,
            from_joins,
            where_clause,
            returning,
            output,
            with: None,
            leading_comments,
            limit,
            order_by,
            from_before_set,
            assignments: Vec::new(),
            current_of: None,
            returning_aliases: Vec::new(),
        })))
    }

    fn parse_update_table(&mut self, first_name: Identifier) -> Result<TableRef> {
        Ok(if self.match_token(TokenType::Dot) {
            let second_name = self.expect_identifier_with_quoted()?;
            if self.match_token(TokenType::Dot) {
                let table_name = self.expect_identifier_with_quoted()?;
                TableRef {
                    name: table_name,
                    schema: Some(second_name),
                    catalog: Some(first_name),
                    alias: None,
                    alias_explicit_as: false,
                    column_aliases: Vec::new(),
                    leading_comments: Vec::new(),
                    trailing_comments: Vec::new(),
                    when: None,
                    only: false,
                    final_: false,
                    table_sample: None,
                    hints: Vec::new(),
                    system_time: None,
                    partitions: Vec::new(),
                    identifier_func: None,
                    changes: None,
                    version: None,
                    span: None,
                }
            } else {
                TableRef {
                    name: second_name,
                    schema: Some(first_name),
                    catalog: None,
                    alias: None,
                    alias_explicit_as: false,
                    column_aliases: Vec::new(),
                    leading_comments: Vec::new(),
                    trailing_comments: Vec::new(),
                    when: None,
                    only: false,
                    final_: false,
                    table_sample: None,
                    hints: Vec::new(),
                    system_time: None,
                    partitions: Vec::new(),
                    identifier_func: None,
                    changes: None,
                    version: None,
                    span: None,
                }
            }
        } else {
            TableRef::from_identifier(first_name)
        })
    }

    fn parse_update_extra_tables(&mut self) -> Result<Vec<TableRef>> {
        let mut extra_tables: Vec<TableRef> = Vec::new();
        while self.match_token(TokenType::Comma) {
            let first_name = self.expect_identifier_with_quoted()?;
            let mut extra_table = if self.match_token(TokenType::Dot) {
                let second_name = self.expect_identifier_with_quoted()?;
                if self.match_token(TokenType::Dot) {
                    let table_name = self.expect_identifier_with_quoted()?;
                    TableRef {
                        name: table_name,
                        schema: Some(second_name),
                        catalog: Some(first_name),
                        alias: None,
                        alias_explicit_as: false,
                        column_aliases: Vec::new(),
                        leading_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        when: None,
                        only: false,
                        final_: false,
                        table_sample: None,
                        hints: Vec::new(),
                        system_time: None,
                        partitions: Vec::new(),
                        identifier_func: None,
                        changes: None,
                        version: None,
                        span: None,
                    }
                } else {
                    TableRef {
                        name: second_name,
                        schema: Some(first_name),
                        catalog: None,
                        alias: None,
                        alias_explicit_as: false,
                        column_aliases: Vec::new(),
                        leading_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        when: None,
                        only: false,
                        final_: false,
                        table_sample: None,
                        hints: Vec::new(),
                        system_time: None,
                        partitions: Vec::new(),
                        identifier_func: None,
                        changes: None,
                        version: None,
                        span: None,
                    }
                }
            } else {
                TableRef::from_identifier(first_name)
            };
            if self.match_token(TokenType::As) {
                extra_table.alias = Some(self.expect_identifier_with_quoted()?);
                extra_table.alias_explicit_as = true;
            } else if self.is_identifier_token()
                && !self.check(TokenType::Set)
                && !self.check_keyword()
            {
                extra_table.alias = Some(self.expect_identifier_with_quoted()?);
                extra_table.alias_explicit_as = false;
            }
            extra_tables.push(extra_table);
        }
        Ok(extra_tables)
    }

    fn parse_update_table_joins(&mut self) -> Result<Vec<Join>> {
        let mut table_joins: Vec<Join> = Vec::new();
        while let Some(JoinKindParts {
            kind,
            use_inner_keyword,
            use_outer_keyword,
            ..
        }) = self.try_parse_join_kind()
        {
            if self.check(TokenType::Join) {
                self.skip();
            }
            let join_expr = self.parse_table_expression()?;
            let on_condition = if self.match_token(TokenType::On) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            table_joins.push(Join {
                this: join_expr,
                on: on_condition,
                using: Vec::new(),
                kind,
                use_inner_keyword,
                use_outer_keyword,
                deferred_condition: false,
                join_hint: None,
                match_condition: None,
                pivots: Vec::new(),
                comments: Vec::new(),
                nesting_group: 0,
                directed: false,
                first_last: None,
                first_last_alias: None,
                span: SourceSpan::default(),
            });
        }
        Ok(table_joins)
    }

    fn parse_update_assignments(&mut self) -> Result<Vec<(Identifier, Expression)>> {
        let mut set: Vec<(Identifier, Expression)> = Vec::new();
        loop {
            let mut col_ident = self.expect_identifier_or_safe_keyword_with_quoted()?;
            while self.match_token(TokenType::Dot) {
                let part = self.expect_identifier_or_safe_keyword_with_quoted()?;
                col_ident = Identifier {
                    name: format!("{}.{}", col_ident.name, part.name),
                    quoted: col_ident.quoted || part.quoted,
                    trailing_comments: Vec::new(),
                    span: col_ident
                        .span
                        .zip(part.span)
                        .map(|(start, end)| start.through(end)),
                };
            }
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_expression()?;
            set.push((col_ident, value));

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(set)
    }
}
