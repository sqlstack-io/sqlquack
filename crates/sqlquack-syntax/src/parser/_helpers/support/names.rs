//! Parser support: names.

use crate::expressions::columns::models::ColumnDef;
use crate::expressions::functions::models::Anonymous;
use crate::expressions::functions::models::Function;
use crate::expressions::names::main::identifier_from_token::identifier_from_token;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::names::models::Star;
use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::Between;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::operators::models::Cast;
use crate::expressions::operators::models::In;
use crate::expressions::operators::models::IsJson;
use crate::expressions::operators::models::JsonUniqueKeys;
use crate::expressions::operators::models::LikeOp;
use crate::expressions::operators::models::PartitionRange;
use crate::expressions::operators::models::Returning;
use crate::expressions::operators::models::RowsFrom;
use crate::expressions::operators::models::UnaryOp;
use crate::expressions::queries::models::Pivot;
use crate::expressions::queries::models::Subquery;
use crate::expressions::statements::models::OutputClause;
use crate::expressions::statements::models::RecursiveWithSearch;
use crate::expressions::statements::models::Refresh;
use crate::expressions::statements::models::Schema;
use crate::expressions::statements::models::SetItem;
use crate::expressions::statements::models::TableAlias;
use crate::expressions::statements::models::Var;
use crate::expressions::tables::models::IndexTableHint;
use crate::expressions::tables::models::Reference;
use crate::expressions::tables::models::RefreshTriggerProperty;
use crate::expressions::tables::models::RowFormatDelimitedProperty;
use crate::expressions::tables::models::RowFormatSerdeProperty;
use crate::expressions::tables::models::SerdeProperties;
use crate::expressions::tables::models::SortKeyProperty;
use crate::expressions::tables::models::WithTableHint;
use crate::expressions::tree::models::Expression;
use crate::expressions::typed_functions::models::Slice;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::Tuple;
use crate::parser::constants::GROUP;
use crate::parser::constants::ORDER;
use crate::parser::constants::QUOTED_TEXT_MIN_LEN;
use crate::parser::constants::SCHEDULE;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    pub(crate) fn date_part_expr_name<'a>(&self, expr: &'a Expression) -> Option<&'a str> {
        self.date_part_identifier_expr_name(expr).or(match expr {
            Expression::Literal(lit) if matches!(lit.as_ref(), Literal::String(_)) => {
                match lit.as_ref() {
                    Literal::String(s) => Some(s.as_str()),
                    _ => None,
                }
            }
            Expression::Cast(cast)
                if cast.format.is_none()
                    && cast.default.is_none()
                    && Self::date_part_cast_target_is_string(&cast.to) =>
            {
                match &cast.this {
                    Expression::Literal(lit) if matches!(lit.as_ref(), Literal::String(_)) => {
                        match lit.as_ref() {
                            Literal::String(s) => Some(s.as_str()),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        })
    }

    pub(crate) fn date_part_cast_target_is_string(data_type: &DataType) -> bool {
        matches!(
            data_type,
            DataType::Char { .. }
                | DataType::VarChar { .. }
                | DataType::String { .. }
                | DataType::Text
                | DataType::TextWithLength { .. }
        ) || matches!(
            data_type,
            DataType::Custom { name, .. }
                if name.eq_ignore_ascii_case("VARCHAR")
                    || name.eq_ignore_ascii_case("NVARCHAR")
                    || name.eq_ignore_ascii_case("VARCHAR(MAX)")
                    || name.eq_ignore_ascii_case("NVARCHAR(MAX)")
        )
    }

    pub(crate) fn try_clickhouse_func_arg_alias(&mut self, expr: Expression) -> Expression {
        expr
    }

    /// Parses range predicates: BETWEEN, LIKE, IN, IS and the like.
    pub(crate) fn parse_range(&mut self) -> Result<Option<Expression>> {
        let mut this = self.parse_bitwise()?;
        if this.is_none() {
            return Ok(None);
        }

        let negate = self.match_token(TokenType::Not);

        if self.match_token(TokenType::Between) {
            let between = self.parse_between_with_expr(this.clone(), negate)?;
            this = Some(between);
            return Ok(this);
        }

        if self.match_token(TokenType::Like) {
            let left = this.clone().ok_or_else(|| {
                crate::tokens::errors::Error::internal("left expression checked above")
            })?;
            let right = self
                .parse_bitwise()?
                .ok_or_else(|| self.parse_error("Expected expression after LIKE"))?;
            let escape = self.parse_escape()?;
            let like = Expression::Like(Box::new(LikeOp {
                negated: negate,
                left,
                right,
                escape,
                quantifier: None,
                inferred_type: None,
            }));
            this = Some(like);
            return Ok(this);
        }

        if self.match_token(TokenType::ILike) {
            let left = this.clone().ok_or_else(|| {
                crate::tokens::errors::Error::internal("left expression checked above")
            })?;
            let right = self
                .parse_bitwise()?
                .ok_or_else(|| self.parse_error("Expected expression after ILIKE"))?;
            let escape = self.parse_escape()?;
            let ilike = Expression::ILike(Box::new(LikeOp {
                negated: negate,
                left,
                right,
                escape,
                quantifier: None,
                inferred_type: None,
            }));
            this = Some(ilike);
            return Ok(this);
        }

        if self.match_token(TokenType::In) {
            let in_expr = self.parse_in_with_expr(this.clone())?;
            this = if negate {
                Some(Expression::Not(Box::new(UnaryOp {
                    this: in_expr,
                    inferred_type: None,
                })))
            } else {
                Some(in_expr)
            };
            return Ok(this);
        }

        if self.match_token(TokenType::Is) {
            let is_expr = self.parse_is_with_expr(this.clone())?;
            this = Some(is_expr);
            return Ok(this);
        }

        if negate && self.match_token(TokenType::Null) {
            if let Some(left) = this {
                let is_null = Expression::Is(Box::new(BinaryOp {
                    left,
                    right: Expression::Null(Null),
                    left_comments: Vec::new(),
                    operator_comments: Vec::new(),
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                }));
                return Ok(Some(Expression::Not(Box::new(UnaryOp {
                    this: is_null,
                    inferred_type: None,
                }))));
            }
        }

        Ok(this)
    }

    /// parse_between_with_expr - Parses BETWEEN expression with given left side
    pub(crate) fn parse_between_with_expr(
        &mut self,
        this: Option<Expression>,
        negate: bool,
    ) -> Result<Expression> {
        let this_expr = match this {
            Some(e) => e,
            None => return Err(self.parse_error("Expected expression before BETWEEN")),
        };

        let symmetric = if self.match_texts(&["SYMMETRIC"]) {
            Some(true)
        } else if self.match_texts(&["ASYMMETRIC"]) {
            Some(false)
        } else {
            None
        };

        let low = self
            .parse_bitwise()?
            .ok_or_else(|| self.parse_error("Expected low expression after BETWEEN"))?;

        if !self.match_token(TokenType::And) {
            return Err(self.parse_error("Expected AND in BETWEEN expression"));
        }

        let high = self
            .parse_bitwise()?
            .ok_or_else(|| self.parse_error("Expected high expression after AND in BETWEEN"))?;

        Ok(Expression::Between(Box::new(Between {
            this: this_expr,
            low,
            high,
            not: negate,
            symmetric,
        })))
    }

    /// parse_in_with_expr - Parses IN expression with given left side
    pub(crate) fn parse_in_with_expr(&mut self, this: Option<Expression>) -> Result<Expression> {
        let this_expr = match this {
            Some(e) => e,
            None => return Err(self.parse_error("Expected expression before IN")),
        };

        if self.check_identifier("UNNEST") {
            self.skip();
            self.expect_token(TokenType::LParen)?;
            let unnest_expr = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Expression::In(Box::new(In {
                this: this_expr,
                expressions: Vec::new(),
                query: None,
                not: false,
                global: false,
                unnest: Some(Box::new(unnest_expr)),
                is_field: false,
            })));
        }

        if !self.match_token(TokenType::LParen) {
            if let Ok(expr) = self.parse_primary() {
                return Ok(Expression::In(Box::new(In {
                    this: this_expr,
                    expressions: vec![expr],
                    query: None,
                    not: false,
                    global: false,
                    unnest: None,
                    is_field: true,
                })));
            }
            return Err(self.parse_error("Expected expression or parenthesized list after IN"));
        }

        if self.check(TokenType::Select) {
            let subquery = self.parse_select()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Expression::In(Box::new(In {
                this: this_expr,
                expressions: Vec::new(),
                query: Some(subquery),
                not: false,
                global: false,
                unnest: None,
                is_field: false,
            })));
        }

        let capacity_hint = self.estimate_expression_list_capacity_until_rparen();
        let expressions = self.parse_expression_list_with_capacity(capacity_hint)?;
        self.expect_token(TokenType::RParen)?;

        if expressions.is_empty() {
            return Err(self.parse_error("Expected expression list after IN"));
        }

        Ok(Expression::In(Box::new(In {
            this: this_expr,
            expressions,
            query: None,
            not: false,
            global: false,
            unnest: None,
            is_field: false,
        })))
    }

    /// parse_is_with_expr - Parses IS expression with given left side
    pub(crate) fn parse_is_with_expr(&mut self, this: Option<Expression>) -> Result<Expression> {
        let this_expr = match this {
            Some(e) => e,
            None => return Err(self.parse_error("Expected expression before IS")),
        };

        let negate = self.match_token(TokenType::Not);

        if self.match_token(TokenType::Null) {
            let is_null = Expression::Is(Box::new(BinaryOp {
                left: this_expr,
                right: Expression::Null(Null),
                left_comments: Vec::new(),
                operator_comments: Vec::new(),
                trailing_comments: Vec::new(),
                inferred_type: None,
            }));
            return if negate {
                Ok(Expression::Not(Box::new(UnaryOp {
                    this: is_null,
                    inferred_type: None,
                })))
            } else {
                Ok(is_null)
            };
        }

        if self.match_texts(&["TRUE"]) {
            let is_true = Expression::Is(Box::new(BinaryOp {
                left: this_expr,
                right: Expression::Boolean(BooleanLiteral { value: true }),
                left_comments: Vec::new(),
                operator_comments: Vec::new(),
                trailing_comments: Vec::new(),
                inferred_type: None,
            }));
            return if negate {
                Ok(Expression::Not(Box::new(UnaryOp {
                    this: is_true,
                    inferred_type: None,
                })))
            } else {
                Ok(is_true)
            };
        }

        if self.match_texts(&["FALSE"]) {
            let is_false = Expression::Is(Box::new(BinaryOp {
                left: this_expr,
                right: Expression::Boolean(BooleanLiteral { value: false }),
                left_comments: Vec::new(),
                operator_comments: Vec::new(),
                trailing_comments: Vec::new(),
                inferred_type: None,
            }));
            return if negate {
                Ok(Expression::Not(Box::new(UnaryOp {
                    this: is_false,
                    inferred_type: None,
                })))
            } else {
                Ok(is_false)
            };
        }

        if self.match_texts(&["JSON"]) {
            let json_type = if self.match_texts(&["VALUE"]) {
                Some("VALUE".to_string())
            } else if self.match_texts(&["SCALAR"]) {
                Some("SCALAR".to_string())
            } else if self.match_texts(&["OBJECT"]) {
                Some("OBJECT".to_string())
            } else if self.match_texts(&["ARRAY"]) {
                Some("ARRAY".to_string())
            } else {
                None
            };

            let unique_keys = if self.match_text_seq(&["WITH", "UNIQUE", "KEYS"]) {
                Some(JsonUniqueKeys::With)
            } else if self.match_text_seq(&["WITHOUT", "UNIQUE", "KEYS"]) {
                Some(JsonUniqueKeys::Without)
            } else if self.match_text_seq(&["UNIQUE", "KEYS"]) {
                Some(JsonUniqueKeys::Shorthand)
            } else {
                None
            };

            return Ok(Expression::IsJson(Box::new(IsJson {
                this: this_expr,
                json_type,
                unique_keys,
                negated: negate,
            })));
        }

        if self.match_text_seq(&["DISTINCT", "FROM"]) {
            let right = self.parse_bitwise()?;
            if let Some(right_expr) = right {
                let expr = if negate {
                    Expression::NullSafeEq(Box::new(BinaryOp {
                        left: this_expr,
                        right: right_expr,
                        left_comments: Vec::new(),
                        operator_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                    }))
                } else {
                    Expression::NullSafeNeq(Box::new(BinaryOp {
                        left: this_expr,
                        right: right_expr,
                        left_comments: Vec::new(),
                        operator_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                    }))
                };
                return Ok(expr);
            }
            return Err(self.parse_error("Expected expression after IS DISTINCT FROM"));
        }

        Err(self.parse_error("Expected NULL, TRUE, FALSE, JSON, or DISTINCT FROM after IS"))
    }

    /// Parses PostgreSQL SEARCH/CYCLE clauses of recursive CTEs.
    pub(crate) fn parse_recursive_with_search(&mut self) -> Result<Option<Box<Expression>>> {
        let kind = if self.match_text_seq(&["SEARCH"]) {
            let search_kind = if self.match_text_seq(&["BREADTH"]) {
                "BREADTH"
            } else if self.match_text_seq(&["DEPTH"]) {
                "DEPTH"
            } else {
                return Ok(None);
            };
            self.match_text_seq(&["FIRST"]);
            self.match_text_seq(&["BY"]);
            search_kind.to_string()
        } else if self.match_token(TokenType::Cycle) {
            "CYCLE".to_string()
        } else {
            return Ok(None);
        };

        let this = self.expect_identifier()?;
        let this_expr = Expression::Identifier(Identifier::new(this));

        let expression = if self.match_text_seq(&["SET"]) {
            let set_col = self.expect_identifier()?;
            Expression::Identifier(Identifier::new(set_col))
        } else {
            return Err(self.parse_error("Expected SET in CYCLE/SEARCH clause"));
        };

        let using = if self.match_token(TokenType::Using) {
            let using_col = self.expect_identifier()?;
            Some(Box::new(Expression::Identifier(Identifier::new(using_col))))
        } else {
            None
        };

        Ok(Some(Box::new(Expression::RecursiveWithSearch(Box::new(
            RecursiveWithSearch {
                kind,
                this: Box::new(this_expr),
                expression: Box::new(expression),
                using,
            },
        )))))
    }

    /// Parses a REFERENCES clause.
    pub(crate) fn parse_references(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::References) {
            return Ok(None);
        }

        let this = self.parse_table()?;
        if this.is_none() {
            return Err(self.parse_error("Expected table name after REFERENCES"));
        }

        let expressions = if self.match_token(TokenType::LParen) {
            let cols = self.parse_identifier_list()?;
            self.expect_token(TokenType::RParen)?;
            cols.into_iter().map(Expression::Identifier).collect()
        } else {
            Vec::new()
        };

        let options = self.parse_fk_constraint_options()?;

        Ok(Some(Expression::Reference(Box::new(Reference {
            this: Box::new(this.ok_or_else(|| {
                crate::tokens::errors::Error::internal("a value the parser checked is missing")
            })?),
            expressions,
            options,
        }))))
    }

    /// Parse key constraint options (ON DELETE CASCADE, ON UPDATE SET NULL, etc.)
    pub(crate) fn parse_fk_constraint_options(&mut self) -> Result<Vec<Expression>> {
        let mut options: Vec<Expression> = Vec::new();

        while self.match_token(TokenType::On) {
            let kind = if self.match_token(TokenType::Delete) {
                "DELETE"
            } else if self.match_token(TokenType::Update) {
                "UPDATE"
            } else {
                break;
            };

            let action = if self.match_text_seq(&["NO", "ACTION"]) {
                "NO ACTION"
            } else if self.match_text_seq(&["SET", "NULL"]) {
                "SET NULL"
            } else if self.match_text_seq(&["SET", "DEFAULT"]) {
                "SET DEFAULT"
            } else if self.match_token(TokenType::Cascade) {
                "CASCADE"
            } else if self.match_token(TokenType::Restrict) {
                "RESTRICT"
            } else {
                continue;
            };

            options.push(Expression::Identifier(Identifier {
                name: format!("ON {} {}", kind, action),
                quoted: false,
                trailing_comments: Vec::new(),
                span: None,
            }));
        }

        if self.match_token(TokenType::Match) {
            let match_type = if self.match_identifier("FULL") {
                "FULL"
            } else if self.match_identifier("PARTIAL") {
                "PARTIAL"
            } else if self.match_identifier("SIMPLE") {
                "SIMPLE"
            } else {
                ""
            };
            if !match_type.is_empty() {
                options.push(Expression::Identifier(Identifier {
                    name: format!("MATCH {}", match_type),
                    quoted: false,
                    trailing_comments: Vec::new(),
                    span: None,
                }));
            }
        }

        Ok(options)
    }

    /// Parses REFRESH TABLE or REFRESH MATERIALIZED VIEW.
    pub(crate) fn parse_refresh(&mut self) -> Result<Option<Expression>> {
        let kind = if self.match_token(TokenType::Table) {
            "TABLE".to_string()
        } else if self.match_text_seq(&["MATERIALIZED", "VIEW"]) {
            "MATERIALIZED VIEW".to_string()
        } else {
            String::new()
        };

        if let Some(s) = self.parse_string()? {
            return Ok(Some(Expression::Refresh(Box::new(Refresh {
                this: Box::new(s),
                kind,
            }))));
        }

        let table_ref = self.parse_table_ref()?;
        let table_expr = Expression::Table(Box::new(table_ref));

        Ok(Some(Expression::Refresh(Box::new(Refresh {
            this: Box::new(table_expr),
            kind,
        }))))
    }

    /// Parses the Doris REFRESH method ON kind [EVERY n unit] [STARTS '...'] clause.
    pub(crate) fn parse_refresh_trigger_property(&mut self) -> Result<RefreshTriggerProperty> {
        let method = self.expect_identifier_or_keyword()?.to_ascii_uppercase();

        self.expect_token(TokenType::On)?;

        let kind_text = self.expect_identifier_or_keyword()?.to_ascii_uppercase();
        let kind = Some(kind_text.clone());

        let (every, unit, starts) = if kind_text == SCHEDULE {
            let every = if self.match_identifier("EVERY") {
                self.parse_number()?.map(Box::new)
            } else {
                None
            };

            let unit = if every.is_some() {
                Some(self.expect_identifier_or_keyword()?.to_ascii_uppercase())
            } else {
                None
            };

            let starts = if self.match_identifier("STARTS") {
                let s = self.expect_string()?;
                Some(Box::new(Expression::Literal(Box::new(Literal::String(s)))))
            } else {
                None
            };

            (every, unit, starts)
        } else {
            (None, None, None)
        };

        Ok(RefreshTriggerProperty {
            method,
            kind,
            every,
            unit,
            starts,
        })
    }

    /// Parses TSQL HISTORY_RETENTION_PERIOD: INFINITE or a number of days, months or years.
    pub(crate) fn parse_retention_period(&mut self) -> Result<Option<Expression>> {
        let number = self.parse_number()?;
        let number_str = number
            .map(|n| match n {
                Expression::Literal(lit) if matches!(lit.as_ref(), Literal::Number(_)) => {
                    match lit.as_ref() {
                        Literal::Number(s) => format!("{} ", s),
                        _ => String::new(),
                    }
                }
                _ => String::new(),
            })
            .unwrap_or_default();

        let unit = self.parse_var_any_token()?;
        let unit_str = unit
            .map(|u| match u {
                Expression::Var(v) => v.this.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        let result = format!("{}{}", number_str, unit_str);
        Ok(Some(Expression::Var(Box::new(Var { this: result }))))
    }

    /// parse_var_any_token - Parses any token as a Var (for flexible parsing)
    pub(crate) fn parse_var_any_token(&mut self) -> Result<Option<Expression>> {
        if !self.is_at_end() {
            let token = self.advance()?;
            Ok(Some(Expression::Var(Box::new(Var {
                this: token.text.to_string(),
            }))))
        } else {
            Ok(None)
        }
    }

    /// Parses a PostgreSQL RETURNING clause.
    pub(crate) fn parse_returning(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Returning) {
            return Ok(None);
        }

        let expressions = self.parse_expression_list()?;

        let into = if self.match_token(TokenType::Into) {
            self.parse_table()?.map(Box::new)
        } else {
            None
        };

        Ok(Some(Expression::Returning(Box::new(Returning {
            expressions,
            into,
        }))))
    }

    /// Parses a TSQL OUTPUT clause with optional aliases.
    pub(crate) fn parse_output_clause(&mut self) -> Result<OutputClause> {
        let mut columns: Vec<Expression> = Vec::new();
        loop {
            let expr = self.parse_expression()?;
            let expr = if self.match_token(TokenType::As) {
                let alias = self.expect_identifier_or_keyword_with_quoted()?;
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
            columns.push(expr);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        let into_table = if self.match_token(TokenType::Into) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(OutputClause {
            columns,
            into_table,
        })
    }

    /// Parses a ROW FORMAT clause, SERDE or DELIMITED.
    pub(crate) fn parse_row(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Format) {
            return Ok(None);
        }
        self.parse_row_format()
    }

    /// Parses a SERDE or DELIMITED row format.
    pub(crate) fn parse_row_format(&mut self) -> Result<Option<Expression>> {
        if self.match_text_seq(&["SERDE"]) {
            let this = self.parse_string()?;
            let serde_properties = self.parse_serde_properties(false)?;

            return Ok(Some(Expression::RowFormatSerdeProperty(Box::new(
                RowFormatSerdeProperty {
                    this: Box::new(this.unwrap_or(Expression::Null(Null))),
                    serde_properties: serde_properties.map(Box::new),
                },
            ))));
        }

        self.match_text_seq(&["DELIMITED"]);

        let mut fields = None;
        let mut escaped = None;
        let mut collection_items = None;
        let mut map_keys = None;
        let mut lines = None;
        let mut null = None;

        if self.match_text_seq(&["FIELDS", "TERMINATED", "BY"]) {
            fields = self.parse_string()?.map(Box::new);
            if self.match_text_seq(&["ESCAPED", "BY"]) {
                escaped = self.parse_string()?.map(Box::new);
            }
        }

        if self.match_text_seq(&["COLLECTION", "ITEMS", "TERMINATED", "BY"]) {
            collection_items = self.parse_string()?.map(Box::new);
        }

        if self.match_text_seq(&["MAP", "KEYS", "TERMINATED", "BY"]) {
            map_keys = self.parse_string()?.map(Box::new);
        }

        if self.match_text_seq(&["LINES", "TERMINATED", "BY"]) {
            lines = self.parse_string()?.map(Box::new);
        }

        if self.match_text_seq(&["NULL", "DEFINED", "AS"]) {
            null = self.parse_string()?.map(Box::new);
        }

        let serde = self.parse_serde_properties(false)?.map(Box::new);

        Ok(Some(Expression::RowFormatDelimitedProperty(Box::new(
            RowFormatDelimitedProperty {
                fields,
                escaped,
                collection_items,
                map_keys,
                lines,
                null,
                serde,
            },
        ))))
    }

    /// Parses a schema definition `(col type, ...)` for CREATE TABLE.
    pub(crate) fn parse_schema(&mut self) -> Result<Option<Expression>> {
        self.parse_schema_with_this(None)
    }

    /// parse_schema_with_this - Parses schema with optional table reference
    pub(crate) fn parse_schema_with_this(
        &mut self,
        this: Option<Expression>,
    ) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::LParen) {
            return Ok(this);
        }

        if self.check(TokenType::Select) || self.check(TokenType::With) {
            self.current -= 1;
            return Ok(this);
        }

        let mut expressions: Vec<Expression> = Vec::new();
        if !self.check(TokenType::RParen) {
            loop {
                if let Some(constraint) = self.parse_constraint()? {
                    expressions.push(constraint);
                } else if let Some(field_def) = self.parse_field_def()? {
                    expressions.push(field_def);
                } else {
                    break;
                }

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.expect_token(TokenType::RParen)?;

        Ok(Some(Expression::Schema(Box::new(Schema {
            this: this.map(Box::new),
            expressions,
        }))))
    }

    /// Parses a schema identifier `name` or `name(columns)`, for TSQL ON filegroup.
    pub(crate) fn parse_schema_identifier(&mut self) -> Result<Expression> {
        let name = self.expect_identifier_with_quoted()?;
        let name_expr = Expression::Identifier(name);

        if self.match_token(TokenType::LParen) {
            let mut columns: Vec<Expression> = Vec::new();
            loop {
                let col = self.expect_identifier_with_quoted()?;
                columns.push(Expression::Identifier(col));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            Ok(Expression::Schema(Box::new(Schema {
                this: Some(Box::new(name_expr)),
                expressions: columns,
            })))
        } else {
            Ok(name_expr)
        }
    }

    /// Parses a SELECT statement or an expression.
    pub(crate) fn parse_select_or_expression(&mut self) -> Result<Option<Expression>> {
        self.with_parser_depth(|parser| parser.parse_select_or_expression_inner())
    }

    #[inline(never)]
    pub(crate) fn parse_select_or_expression_inner(&mut self) -> Result<Option<Expression>> {
        let start_pos = self.current;

        if self.check(TokenType::Select) {
            return Ok(Some(self.parse_select()?));
        }

        if let Some(expr) = self.parse_disjunction()? {
            return Ok(Some(expr));
        }

        self.current = start_pos;

        Ok(None)
    }

    /// Parses [WITH] SERDEPROPERTIES (key = value, ...).
    pub(crate) fn parse_serde_properties(&mut self, with_: bool) -> Result<Option<Expression>> {
        let start_index = self.current;
        let has_with = with_ || self.match_text_seq(&["WITH"]);

        if !self.match_token(TokenType::SerdeProperties) {
            self.current = start_index;
            return Ok(None);
        }

        let mut expressions: Vec<Expression> = Vec::new();
        if self.match_token(TokenType::LParen) {
            loop {
                if self.check(TokenType::RParen) {
                    break;
                }
                let key = self.parse_primary()?;
                if self.match_token(TokenType::Eq) {
                    let value = self.parse_primary()?;
                    expressions.push(Expression::Eq(Box::new(BinaryOp::new(key, value))));
                } else {
                    expressions.push(key);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }

        Ok(Some(Expression::SerdeProperties(Box::new(
            SerdeProperties {
                expressions,
                with_: if has_with {
                    Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: true,
                    })))
                } else {
                    None
                },
            },
        ))))
    }

    /// Parses a SET `variable = value` assignment.
    pub(crate) fn parse_set_item_assignment(&mut self) -> Result<Option<Expression>> {
        let start_index = self.current;

        if self.match_text_seq(&["TRANSACTION"]) {
            return Ok(Some(Expression::SetItem(Box::new(SetItem {
                name: Expression::Var(Box::new(Var {
                    this: "TRANSACTION".to_string(),
                })),
                value: Expression::Null(Null),
                kind: None,
                no_equals: false,
            }))));
        }

        let left = match self.parse_primary_or_var()? {
            Some(left) => Some(left),
            None => self.parse_column()?,
        };

        if left.is_none() {
            self.current = start_index;
            return Ok(None);
        }

        if !self.match_texts(&["=", "TO", ":="]) {
            self.current = start_index;
            return Ok(None);
        }

        let right_val = if self.check(TokenType::String) {
            let text = self.advance_text()?;
            Expression::Literal(Box::new(Literal::String(text)))
        } else if self.check(TokenType::False) {
            self.skip();
            Expression::Boolean(BooleanLiteral { value: false })
        } else if self.check(TokenType::True) {
            self.skip();
            Expression::Boolean(BooleanLiteral { value: true })
        } else {
            let right = match self.parse_id_var()? {
                Some(right) => Some(right),
                None => self.parse_primary_or_var()?,
            };
            match right {
                Some(Expression::Column(col)) => Expression::Var(Box::new(Var {
                    this: col.name.name.clone(),
                })),
                Some(Expression::Identifier(id)) => Expression::Var(Box::new(Var {
                    this: id.name.clone(),
                })),
                Some(other) => other,
                None => Expression::Null(Null),
            }
        };

        Ok(Some(Expression::SetItem(Box::new(SetItem {
            name: left
                .ok_or_else(|| self.parse_error("Expected variable name in SET statement"))?,
            value: right_val,
            kind: None,
            no_equals: false,
        }))))
    }

    /// Parses DuckDB's statement-level PIVOT/UNPIVOT syntax.
    pub(crate) fn parse_simplified_pivot(
        &mut self,
        is_unpivot: bool,
    ) -> Result<Option<Expression>> {
        let this = if self.check(TokenType::LParen) {
            self.skip();
            if self.check(TokenType::Select) || self.check(TokenType::With) {
                let inner = self.parse_statement()?;
                self.expect_token(TokenType::RParen)?;
                Some(Expression::Subquery(Box::new(Subquery {
                    this: inner,
                    alias: None,
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
                })))
            } else {
                self.current -= 1;
                Some(self.parse_primary()?)
            }
        } else {
            Some(self.parse_primary()?)
        };

        let expressions = if self.match_text_seq(&["ON"]) {
            let mut on_exprs: Vec<Expression> = Vec::new();
            loop {
                let on_expr = self.parse_bitwise()?;
                if on_expr.is_none() {
                    break;
                }
                let mut expr = on_expr.ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?;

                if self.match_token(TokenType::In) {
                    if self.match_token(TokenType::LParen) {
                        let mut in_exprs: Vec<Expression> = Vec::new();
                        loop {
                            if self.check(TokenType::RParen) {
                                break;
                            }
                            if let Some(val) = self.parse_select_or_expression()? {
                                in_exprs.push(val);
                            }
                            if !self.match_token(TokenType::Comma) {
                                break;
                            }
                        }
                        self.expect_token(TokenType::RParen)?;
                        expr = Expression::In(Box::new(In {
                            this: expr,
                            expressions: in_exprs,
                            query: None,
                            not: false,
                            global: false,
                            unnest: None,
                            is_field: false,
                        }));
                    }
                } else if self.match_token(TokenType::As) {
                    let alias_name = self.expect_identifier()?;
                    expr =
                        Expression::Alias(Box::new(Alias::new(expr, Identifier::new(alias_name))));
                }

                on_exprs.push(expr);

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            on_exprs
        } else {
            Vec::new()
        };

        let into = self.parse_unpivot_columns()?;

        let using = if self.match_text_seq(&["USING"]) {
            let mut using_exprs: Vec<Expression> = Vec::new();
            loop {
                if self.is_at_end() || self.check(TokenType::Group) || self.check(TokenType::RParen)
                {
                    break;
                }
                let func = self.parse_primary()?;
                let expr = if self.match_token(TokenType::DColon) {
                    let data_type = self.parse_data_type()?;
                    Expression::Cast(Box::new(Cast {
                        this: func,
                        to: data_type,
                        trailing_comments: Vec::new(),
                        double_colon_syntax: true,
                        format: None,
                        default: None,
                        inferred_type: None,
                    }))
                } else {
                    func
                };
                if self.match_token(TokenType::As) {
                    let alias_name = self.expect_identifier()?;
                    using_exprs.push(Expression::Alias(Box::new(Alias::new(
                        expr,
                        Identifier::new(alias_name),
                    ))));
                } else {
                    using_exprs.push(expr);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            using_exprs
        } else {
            Vec::new()
        };

        let group = self.parse_group()?;

        let source = this.ok_or_else(|| {
            crate::tokens::errors::Error::internal("a value the parser checked is missing")
        })?;

        Ok(Some(Expression::Pivot(Box::new(Pivot {
            this: source,
            expressions,
            fields: Vec::new(),
            using,
            group: group.map(Box::new),
            unpivot: is_unpivot,
            into: into.map(Box::new),
            alias: None,
            alias_columns: Vec::new(),
            include_nulls: None,
            default_on_null: None,
            with: None,
        }))))
    }

    /// Parses an array slice `[start:end:step]`.
    pub(crate) fn parse_slice(&mut self) -> Result<Option<Expression>> {
        self.parse_slice_with_this(None)
    }

    /// Implementation of parse_slice with 'this' parameter
    pub(crate) fn parse_slice_with_this(
        &mut self,
        this: Option<Expression>,
    ) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Colon) {
            return Ok(this);
        }

        let end = if self.check(TokenType::Dash) && self.check_next(TokenType::Colon) {
            self.skip();
            Some(Expression::Neg(Box::new(UnaryOp::new(
                Expression::Literal(Box::new(Literal::Number("1".to_string()))),
            ))))
        } else if self.check(TokenType::Colon) || self.check(TokenType::RBracket) {
            None
        } else {
            Some(self.parse_unary()?)
        };

        let step = if self.match_token(TokenType::Colon) {
            if self.check(TokenType::RBracket) {
                None
            } else {
                Some(self.parse_unary()?)
            }
        } else {
            None
        };

        Ok(Some(Expression::Slice(Box::new(Slice {
            this: this.map(Box::new),
            expression: end.map(Box::new),
            step: step.map(Box::new),
        }))))
    }

    /// Parses one slice element without reading `:` as a parameter; empty is `None`.
    pub(crate) fn parse_slice_element(&mut self) -> Result<Option<Expression>> {
        if self.check(TokenType::Colon) || self.check(TokenType::RBracket) {
            return Ok(None);
        }
        if self.check(TokenType::Dash) && self.check_next(TokenType::Colon) {
            self.skip();
            return Ok(Some(Expression::Neg(Box::new(UnaryOp::new(
                Expression::Literal(Box::new(Literal::Number("1".to_string()))),
            )))));
        }
        let expr = self.parse_disjunction()?;
        Ok(expr)
    }

    /// Parses a [COMPOUND] SORTKEY(columns) property.
    pub(crate) fn parse_sortkey(&mut self) -> Result<Option<Expression>> {
        let this = if self.match_token(TokenType::LParen) {
            let mut columns: Vec<Expression> = Vec::new();
            loop {
                if let Some(id) = self.parse_id_var()? {
                    columns.push(id);
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.match_token(TokenType::RParen);

            if columns.is_empty() {
                return Ok(None);
            }

            if columns.len() == 1 {
                columns.into_iter().next().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?
            } else {
                Expression::Tuple(Box::new(Tuple {
                    expressions: columns,
                }))
            }
        } else {
            if let Some(id) = self.parse_id_var()? {
                id
            } else {
                return Ok(None);
            }
        };

        Ok(Some(Expression::SortKeyProperty(Box::new(
            SortKeyProperty {
                this: Box::new(this),
                compound: None,
            },
        ))))
    }

    /// Parses `*` with optional EXCEPT, REPLACE and RENAME.
    pub(crate) fn parse_star(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Star) {
            return Ok(None);
        }

        let except = self.parse_star_except()?;

        let replace = self.parse_star_replace()?;

        let rename = self.parse_star_rename()?;

        Ok(Some(Expression::Star(Star {
            table: None,
            except,
            replace,
            rename,
            trailing_comments: Vec::new(),
            span: None,
        })))
    }

    /// try_parse_identifier - Try to parse an identifier, returning None if not found
    pub(crate) fn try_parse_identifier(&mut self) -> Result<Option<Identifier>> {
        if self.is_identifier_token() {
            let token = self.advance()?;
            let quoted = token.token_type == TokenType::QuotedIdentifier;
            Ok(Some(identifier_from_token(token, quoted)))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn parse_clickhouse_bare_star_like(&mut self) -> Result<Option<Expression>> {
        Ok(None)
    }

    /// Parses a star's EXCEPT/EXCLUDE (cols).
    pub(crate) fn parse_star_except(&mut self) -> Result<Option<Vec<Identifier>>> {
        if !self.match_texts(&["EXCEPT", "EXCLUDE"]) {
            return Ok(None);
        }

        if self.match_token(TokenType::LParen) {
            let mut columns: Vec<Identifier> = Vec::new();
            loop {
                if let Some(id) = self.try_parse_identifier()? {
                    columns.push(id);
                } else if self.is_safe_keyword_as_identifier() {
                    let token = self.advance()?;
                    columns.push(identifier_from_token(token, false));
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.match_token(TokenType::RParen);
            return Ok(Some(columns));
        }

        if let Some(id) = self.try_parse_identifier()? {
            return Ok(Some(vec![id]));
        }

        Ok(None)
    }

    /// Parses a star's REPLACE (expr AS col, ...).
    pub(crate) fn parse_star_replace(&mut self) -> Result<Option<Vec<Alias>>> {
        if !self.match_texts(&["REPLACE"]) {
            return Ok(None);
        }

        if self.match_token(TokenType::LParen) {
            let mut aliases: Vec<Alias> = Vec::new();
            loop {
                if let Some(expr) = self.parse_disjunction()? {
                    let alias_name = if self.match_token(TokenType::As) {
                        self.try_parse_identifier()?
                    } else {
                        None
                    };

                    aliases.push(Alias {
                        this: expr,
                        alias: alias_name.unwrap_or_else(|| Identifier::new("")),
                        column_aliases: Vec::new(),
                        alias_explicit_as: false,
                        alias_keyword: None,
                        pre_alias_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                    });
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.match_token(TokenType::RParen);
            return Ok(Some(aliases));
        }

        Ok(None)
    }

    /// Parses a star's RENAME (old AS new, ...).
    pub(crate) fn parse_star_rename(&mut self) -> Result<Option<Vec<(Identifier, Identifier)>>> {
        if !self.match_texts(&["RENAME"]) {
            return Ok(None);
        }

        if self.match_token(TokenType::LParen) {
            let mut renames: Vec<(Identifier, Identifier)> = Vec::new();
            while let Some(old_name) = self.try_parse_identifier()? {
                if self.match_token(TokenType::As) {
                    if let Some(new_name) = self.try_parse_identifier()? {
                        renames.push((old_name, new_name));
                    }
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.match_token(TokenType::RParen);
            return Ok(Some(renames));
        }

        Ok(None)
    }

    /// Parses a string literal.
    pub(crate) fn parse_string(&mut self) -> Result<Option<Expression>> {
        if self.match_token(TokenType::String) {
            let text = self.previous_text().to_string();
            return Ok(Some(Expression::Literal(Box::new(Literal::String(text)))));
        }
        if self.match_token(TokenType::NationalString) {
            let text = self.previous_text().to_string();
            return Ok(Some(Expression::Literal(Box::new(
                Literal::NationalString(text),
            ))));
        }
        if self.match_token(TokenType::RawString) {
            let text = self.previous_text().to_string();
            return Ok(Some(Expression::Literal(Box::new(Literal::RawString(
                text,
            )))));
        }
        if self.match_token(TokenType::HeredocString) {
            let text = self.previous_text().to_string();
            return Ok(Some(Expression::Literal(Box::new(Literal::String(text)))));
        }
        if self.match_token(TokenType::HexString) {
            let text = self.previous_text().to_string();
            return Ok(Some(Expression::Literal(Box::new(Literal::HexString(
                text,
            )))));
        }
        if self.match_token(TokenType::BitString) {
            let text = self.previous_text().to_string();
            return Ok(Some(Expression::Literal(Box::new(Literal::BitString(
                text,
            )))));
        }
        if self.match_token(TokenType::ByteString) {
            let text = self.previous_text().to_string();
            return Ok(Some(Expression::Literal(Box::new(Literal::ByteString(
                text,
            )))));
        }
        Ok(None)
    }

    /// Parses a string literal as a quoted identifier.
    pub(crate) fn parse_string_as_identifier(&mut self) -> Result<Option<Expression>> {
        if self.match_token(TokenType::String) {
            let text = self.previous_text().to_string();
            let name = if text.starts_with('\'')
                && text.ends_with('\'')
                && text.len() >= QUOTED_TEXT_MIN_LEN
                || text.starts_with('"') && text.ends_with('"') && text.len() >= QUOTED_TEXT_MIN_LEN
            {
                text[1..text.len() - 1].to_string()
            } else {
                text
            };

            Ok(Some(Expression::Identifier(Identifier {
                name,
                quoted: true,
                trailing_comments: Vec::new(),
                span: None,
            })))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn parse_alias_column_list_if_present(&mut self) -> Result<Vec<Identifier>> {
        if !self.match_token(TokenType::LParen) {
            return Ok(Vec::new());
        }

        let mut columns: Vec<Identifier> = Vec::new();
        loop {
            if self.check(TokenType::RParen) {
                break;
            }
            columns.push(self.expect_identifier_or_keyword_with_quoted()?);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(columns)
    }

    /// Parses PostgreSQL ROWS FROM (func(args) AS alias(...), ...) [WITH ORDINALITY].
    pub(crate) fn parse_rows_from(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;

        let mut expressions: Vec<Expression> = Vec::new();

        loop {
            let func_expr = self.parse_rows_from_function()?;
            expressions.push(func_expr);

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        self.expect_token(TokenType::RParen)?;

        let ordinality =
            self.match_token(TokenType::With) && self.match_token(TokenType::Ordinality);

        let alias = if self.match_token(TokenType::As) {
            Some(Box::new(self.parse_rows_from_alias()?))
        } else {
            None
        };

        Ok(Expression::RowsFrom(Box::new(RowsFrom {
            expressions,
            ordinality,
            alias,
        })))
    }

    /// Parse a single function in ROWS FROM: func_name(args) [AS alias(col1 type1, ...)]
    pub(crate) fn parse_rows_from_function(&mut self) -> Result<Expression> {
        let func_name = self.expect_identifier_or_keyword()?;

        self.expect_token(TokenType::LParen)?;
        let args = if self.check(TokenType::RParen) {
            Vec::new()
        } else {
            self.parse_function_arguments()?
        };
        self.expect_token(TokenType::RParen)?;

        let func_expr = Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name: func_name,
            args,
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: None,
            inferred_type: None,
        }));

        if self.match_token(TokenType::As) {
            let alias_expr = self.parse_rows_from_alias()?;
            Ok(Expression::Tuple(Box::new(Tuple {
                expressions: vec![func_expr, alias_expr],
            })))
        } else {
            Ok(func_expr)
        }
    }

    /// Parse ROWS FROM alias with typed columns: alias_name(col1 type1, col2 type2, ...)
    pub(crate) fn parse_rows_from_alias(&mut self) -> Result<Expression> {
        let alias_name = self.expect_identifier_or_keyword_with_quoted()?;

        let columns = if self.match_token(TokenType::LParen) {
            let mut cols: Vec<Expression> = Vec::new();
            loop {
                if self.check(TokenType::RParen) {
                    break;
                }
                let col_name = self.expect_identifier_or_keyword_with_quoted()?;
                let col_type = self.parse_data_type()?;
                let mut col_def = ColumnDef::new(col_name.name.clone(), col_type);
                col_def.name = col_name;
                cols.push(Expression::ColumnDef(Box::new(col_def)));

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            cols
        } else {
            Vec::new()
        };

        Ok(Expression::TableAlias(Box::new(TableAlias {
            this: Some(Box::new(Expression::Identifier(alias_name))),
            columns,
        })))
    }

    /// Parses a table reference with hints, UNNEST and partitions.
    pub(crate) fn parse_table(&mut self) -> Result<Option<Expression>> {
        if self.match_text_seq(&["ROWS", "FROM"]) {
            return Ok(None);
        }
        if self.match_text_seq(&["*"]) {
            return Ok(None);
        }
        if self.match_text_seq(&["NOT", "INDEXED"]) {
            return Ok(None);
        }
        Ok(None)
    }

    /// Parses table hints: SQL Server WITH (...) or MySQL USE/IGNORE/FORCE INDEX.
    pub(crate) fn parse_table_hints(&mut self) -> Result<Option<Expression>> {
        let mut hints: Vec<Expression> = Vec::new();

        if self.match_text_seq(&["WITH"]) && self.match_token(TokenType::LParen) {
            let mut expressions: Vec<Expression> = Vec::new();
            loop {
                if let Some(func) = self.parse_function()? {
                    expressions.push(func);
                } else if let Some(var) = self.parse_var()? {
                    expressions.push(var);
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.match_token(TokenType::RParen);

            if !expressions.is_empty() {
                hints.push(Expression::WithTableHint(Box::new(WithTableHint {
                    expressions,
                })));
            }
        } else {
            while self.match_texts(&["USE", "IGNORE", "FORCE"]) {
                let hint_type = self.previous_text().to_ascii_uppercase();

                let _ = self.match_texts(&["INDEX", "KEY"]);

                let target = if self.match_text_seq(&["FOR"]) {
                    let target_token = self.advance()?;
                    let target_text = target_token.text.to_ascii_uppercase();
                    let full_target = if (target_text == ORDER || target_text == GROUP)
                        && self.check(TokenType::By)
                    {
                        self.skip();
                        format!("{} BY", target_text)
                    } else {
                        target_text
                    };
                    Some(Box::new(Expression::Identifier(Identifier {
                        name: full_target,
                        quoted: false,
                        trailing_comments: Vec::new(),
                        span: None,
                    })))
                } else {
                    None
                };

                let expressions = if self.match_token(TokenType::LParen) {
                    let mut ids: Vec<Expression> = Vec::new();
                    loop {
                        if self.check(TokenType::RParen) {
                            break;
                        }
                        if let Some(id) = self.parse_id_var()? {
                            ids.push(id);
                        } else if self.is_safe_keyword_as_identifier()
                            || self.check(TokenType::PrimaryKey)
                        {
                            let name = self.advance_text()?;
                            ids.push(Expression::Identifier(Identifier::new(name)));
                        } else {
                            break;
                        }
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.match_token(TokenType::RParen);
                    ids
                } else {
                    Vec::new()
                };

                hints.push(Expression::IndexTableHint(Box::new(IndexTableHint {
                    this: Box::new(Expression::Identifier(Identifier {
                        name: hint_type,
                        quoted: false,
                        trailing_comments: Vec::new(),
                        span: None,
                    })),
                    expressions,
                    target,
                })));
            }
        }

        if hints.is_empty() {
            return Ok(None);
        }

        Ok(Some(Expression::Tuple(Box::new(Tuple {
            expressions: hints,
        }))))
    }

    /// Parses TSQL TRUNCATE hints WITH (PARTITIONS(1, 2 TO 5)), ranges included.
    pub(crate) fn parse_truncate_table_hints(&mut self) -> Result<Option<Expression>> {
        if !self.match_text_seq(&["WITH"]) || !self.match_token(TokenType::LParen) {
            return Ok(None);
        }

        let mut hints: Vec<Expression> = Vec::new();

        if self.check_identifier("PARTITIONS") {
            self.skip();
            self.expect_token(TokenType::LParen)?;

            let mut parts: Vec<Expression> = Vec::new();
            loop {
                if self.check(TokenType::RParen) {
                    break;
                }
                let low = self.parse_primary()?;
                if self.match_text_seq(&["TO"]) {
                    let high = self.parse_primary()?;
                    parts.push(Expression::PartitionRange(Box::new(PartitionRange {
                        this: Box::new(low),
                        expression: Some(Box::new(high)),
                        expressions: Vec::new(),
                    })));
                } else {
                    parts.push(low);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;

            hints.push(Expression::Anonymous(Box::new(Anonymous {
                this: Box::new(Expression::Identifier(Identifier {
                    name: "PARTITIONS".to_string(),
                    quoted: false,
                    trailing_comments: Vec::new(),
                    span: None,
                })),
                expressions: parts,
            })));
        } else {
            loop {
                if let Some(func) = self.parse_function()? {
                    hints.push(func);
                } else if let Some(var) = self.parse_var()? {
                    hints.push(var);
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.expect_token(TokenType::RParen)?;

        if hints.is_empty() {
            return Ok(None);
        }

        let hint = Expression::WithTableHint(Box::new(WithTableHint { expressions: hints }));

        Ok(Some(Expression::Tuple(Box::new(Tuple {
            expressions: vec![hint],
        }))))
    }

    /// Parses one table-reference part: an identifier, a string or a placeholder.
    pub(crate) fn parse_table_part(&mut self) -> Result<Option<Expression>> {
        if let Some(id) = self.parse_id_var()? {
            return Ok(Some(id));
        }

        if let Some(str_id) = self.parse_string_as_identifier()? {
            return Ok(Some(str_id));
        }

        if let Some(placeholder) = self.parse_placeholder()? {
            return Ok(Some(placeholder));
        }

        if self.check_keyword_as_identifier() {
            let text = self.peek_text().to_string();
            self.skip();
            return Ok(Some(Expression::Identifier(Identifier {
                name: text,
                quoted: false,
                trailing_comments: Vec::new(),
                span: None,
            })));
        }

        Ok(None)
    }

    /// Whether the current keyword (CLUSTER, TABLE, INDEX, ...) may be an identifier here.
    pub(crate) fn check_keyword_as_identifier(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        let token_type = self.peek().token_type;
        matches!(
            token_type,
            TokenType::Cluster
                | TokenType::Table
                | TokenType::Index
                | TokenType::View
                | TokenType::Database
                | TokenType::Schema
                | TokenType::Column
                | TokenType::Function
                | TokenType::Procedure
                | TokenType::Constraint
                | TokenType::Sequence
                | TokenType::Type
                | TokenType::Partition
                | TokenType::Comment
                | TokenType::Cache
                | TokenType::Commit
                | TokenType::Begin
                | TokenType::End
                | TokenType::Set
                | TokenType::Show
                | TokenType::Describe
                | TokenType::Use
                | TokenType::Execute
                | TokenType::Delete
                | TokenType::Update
                | TokenType::Merge
                | TokenType::Load
                | TokenType::Copy
                | TokenType::Truncate
                | TokenType::Replace
                | TokenType::Refresh
                | TokenType::Rename
                | TokenType::Filter
                | TokenType::Format
                | TokenType::First
                | TokenType::Next
                | TokenType::Last
                | TokenType::Keep
                | TokenType::Match
                | TokenType::Over
                | TokenType::Range
                | TokenType::Rows
                | TokenType::Row
                | TokenType::Offset
                | TokenType::Limit
                | TokenType::Top
                | TokenType::Cube
                | TokenType::Rollup
                | TokenType::Pivot
                | TokenType::Unpivot
                | TokenType::Window
                | TokenType::Recursive
                | TokenType::Unique
                | TokenType::Temporary
                | TokenType::Volatile
                | TokenType::References
                | TokenType::Natural
                | TokenType::Left
                | TokenType::Right
                | TokenType::Full
                | TokenType::Semi
                | TokenType::Anti
                | TokenType::Apply
                | TokenType::All
                | TokenType::Asc
                | TokenType::Desc
                | TokenType::Analyze
        )
    }
}
