//! Parser statements: dml.

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::queries::models::Join;
use crate::expressions::queries::models::Where;
use crate::expressions::schema_objects::models::FunctionSecurity;
use crate::expressions::statements::models::Delete;
use crate::expressions::statements::models::Paren;
use crate::expressions::statements::models::Raw;
use crate::expressions::tree::models::Expression;
use crate::parser::constants::ALGORITHM;
use crate::parser::constants::DEFINER;
use crate::parser::constants::SQL;
use crate::parser::models::Parser;
use crate::parser::models::ViewModifiers;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parses DELETE, including USING forms and MySQL multi-table deletes.
    pub(crate) fn parse_delete(&mut self) -> Result<Expression> {
        let delete_token = self.expect_token(TokenType::Delete)?;
        let leading_comments = delete_token.comments;
        let hint = if self.check(TokenType::Hint) {
            Some(self.parse_hint()?)
        } else {
            None
        };

        let mut tables: Vec<TableRef> = Vec::new();
        let mut early_output = None;
        let _has_from = if self.check(TokenType::From) {
            self.skip();
            true
        } else {
            loop {
                let tref = self.parse_table_ref()?;
                tables.push(tref);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            if self.match_token(TokenType::Output) {
                early_output = Some(self.parse_output_clause()?);
            }
            if self.check(TokenType::From) {
                self.skip();
                true
            } else {
                false
            }
        };

        let has_only = self.match_token(TokenType::Only);
        let mut table = if _has_from {
            self.parse_table_ref()?
        } else {
            if !tables.is_empty() {
                tables.remove(0)
            } else {
                return Err(self.parse_error("Expected table name in DELETE statement"));
            }
        };
        if has_only {
            table.only = true;
        }

        let on_cluster = self.parse_on_cluster_clause()?;

        let mut extra_from_tables: Vec<TableRef> = Vec::new();
        if _has_from
            && tables.is_empty()
            && self.check(TokenType::Comma)
            && !self.check(TokenType::Where)
        {
            while self.match_token(TokenType::Comma) {
                let extra_name = self.expect_identifier_with_quoted()?;
                let extra_ref = TableRef::from_identifier(extra_name);
                extra_from_tables.push(extra_ref);
            }
        }

        let mut tables_from_using = false;
        if !extra_from_tables.is_empty() {
            tables.push(table.clone());
            tables.append(&mut extra_from_tables);
            tables_from_using = true;
        }

        let force_index = if self.match_text_seq(&["FORCE", "INDEX"]) {
            self.expect_token(TokenType::LParen)?;
            let idx_name = self.expect_identifier_with_quoted()?;
            self.expect_token(TokenType::RParen)?;
            Some(idx_name.name)
        } else {
            None
        };

        let (alias, alias_explicit_as) = self.parse_delete_alias(&force_index)?;

        let mut joins = self.parse_joins()?;

        let using;
        (using, joins) = self.parse_delete_using(joins)?;

        let output = if early_output.is_some() {
            early_output
        } else if self.match_token(TokenType::Output) {
            Some(self.parse_output_clause()?)
        } else {
            None
        };

        let where_clause = if self.match_token(TokenType::Where) {
            Some(Where {
                this: self.parse_expression()?,
            })
        } else {
            None
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

        let returning = if self.match_token(TokenType::Returning) {
            self.parse_select_expressions()?
        } else {
            Vec::new()
        };

        Ok(Expression::Delete(Box::new(Delete {
            table,
            hint,
            on_cluster,
            alias,
            alias_explicit_as,
            using,
            where_clause,
            output,
            leading_comments,
            with: None,
            limit,
            order_by,
            returning,
            tables,
            tables_from_using,
            joins,
            force_index,
            no_from: !_has_from,
        })))
    }

    fn parse_delete_alias(
        &mut self,
        force_index: &Option<String>,
    ) -> Result<(Option<Identifier>, bool)> {
        Ok(
            if force_index.is_none() && self.match_token(TokenType::As) {
                (Some(self.expect_identifier_with_quoted()?), true)
            } else if force_index.is_none()
                && self.is_identifier_token()
                && !self.check(TokenType::Using)
                && !self.check(TokenType::Where)
                && !self.check(TokenType::Inner)
                && !self.check(TokenType::Left)
                && !self.check(TokenType::Right)
                && !self.check(TokenType::Cross)
                && !self.check(TokenType::Full)
                && !self.check(TokenType::Join)
                && !self.check_identifier("FORCE")
            {
                (Some(self.expect_identifier_with_quoted()?), false)
            } else {
                (None, false)
            },
        )
    }

    fn parse_delete_using(&mut self, mut joins: Vec<Join>) -> Result<(Vec<TableRef>, Vec<Join>)> {
        let mut using: Vec<TableRef> = Vec::new();
        if self.match_token(TokenType::Using) {
            loop {
                if self.check(TokenType::LParen) {
                    let is_values = self.current + 1 < self.tokens.len()
                        && self.tokens[self.current + 1].token_type == TokenType::Values;
                    let subquery = if is_values {
                        self.skip();
                        let values = self.parse_values()?;
                        self.expect_token(TokenType::RParen)?;
                        Expression::Paren(Box::new(Paren {
                            this: values,
                            trailing_comments: Vec::new(),
                            span: SourceSpan::default(),
                        }))
                    } else {
                        self.parse_primary()?
                    };
                    let using_alias = if self.match_token(TokenType::As) {
                        let alias_name = self.expect_identifier_with_quoted()?;
                        let col_aliases = if self.match_token(TokenType::LParen) {
                            let aliases = self.parse_identifier_list()?;
                            self.expect_token(TokenType::RParen)?;
                            aliases
                        } else {
                            Vec::new()
                        };
                        Some((alias_name, col_aliases))
                    } else {
                        None
                    };
                    let mut tref = TableRef::new("");
                    if let Some((alias_name, col_aliases)) = using_alias {
                        tref.alias = Some(alias_name);
                        tref.alias_explicit_as = true;
                        tref.column_aliases = col_aliases;
                    }
                    tref.hints = vec![subquery];
                    using.push(tref);
                } else {
                    let using_table = self.expect_identifier_with_quoted()?;
                    let mut using_ref = TableRef::from_identifier(using_table);

                    if self.check_join_keyword() {
                        using.push(using_ref);
                        let mut using_joins = self.parse_joins()?;
                        joins.append(&mut using_joins);
                        break;
                    }

                    if self.match_token(TokenType::As) {
                        using_ref.alias = Some(self.expect_identifier_with_quoted()?);
                        using_ref.alias_explicit_as = true;
                    } else if self.is_identifier_token()
                        && !self.check(TokenType::Comma)
                        && !self.check(TokenType::Where)
                    {
                        using_ref.alias = Some(self.expect_identifier_with_quoted()?);
                    }
                    using.push(using_ref);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        Ok((using, joins))
    }

    /// Parse a CREATE statement
    pub(crate) fn parse_create(&mut self) -> Result<Expression> {
        let create_pos = self.current;
        let create_token = self.expect_token(TokenType::Create)?;
        let leading_comments = create_token.comments;

        let or_replace = self.match_keywords(&[TokenType::Or, TokenType::Replace]);
        let or_alter = !or_replace && self.match_text_seq(&["OR", "ALTER"]);
        if self.is_snowflake() && self.check(TokenType::Or) {
            return Err(self.parse_error("Expected REPLACE or ALTER after CREATE OR"));
        }

        let hana_modifier: Option<String> = None;
        let temporary = hana_modifier
            .as_ref()
            .is_some_and(|m| m.contains("TEMPORARY"))
            || self.match_token(TokenType::Temporary);

        let materialized = self.match_token(TokenType::Materialized);

        let (algorithm, definer, security) = self.parse_create_view_options()?;

        let secure = self.match_identifier("SECURE");

        let table_modifier: Option<String> = self.parse_create_table_modifier(hana_modifier);

        if table_modifier.as_deref() == Some("DICTIONARY") {
            return self.parse_create_table(
                or_replace,
                temporary,
                leading_comments,
                table_modifier.as_deref(),
            );
        }

        if self.is_snowflake() && !or_alter && !temporary && !materialized && !secure {
            if let Some(create) =
                self.parse_snowflake_create_from(or_replace, table_modifier.as_deref())?
            {
                return Ok(create);
            }
        }
        if self.is_snowflake() && !or_alter && !materialized && !secure {
            if let Some(create) =
                self.parse_snowflake_create_clone(or_replace, temporary, table_modifier.as_deref())?
            {
                return Ok(create);
            }
        }

        match self.peek().token_type {
            TokenType::Table => {
                if self.current + 1 < self.tokens.len()
                    && self.tokens[self.current + 1].token_type == TokenType::Function
                {
                    self.skip();
                    return self.parse_create_function(or_replace, or_alter, temporary, true);
                }
                let modifier = if materialized {
                    Some("MATERIALIZED")
                } else {
                    table_modifier.as_deref()
                };
                self.parse_create_table(or_replace, temporary, leading_comments, modifier)
            }
            TokenType::Dictionary => {
                self.parse_create_table(or_replace, temporary, leading_comments, Some("DICTIONARY"))
            }
            TokenType::View => self.parse_create_view(ViewModifiers {
                or_replace,
                or_alter,
                materialized,
                temporary,
                algorithm,
                definer,
                security,
                secure,
            }),
            TokenType::Unique => {
                self.skip();
                let clustered = if self.check_identifier("CLUSTERED") {
                    self.skip();
                    Some("CLUSTERED".to_string())
                } else if self.check_identifier("NONCLUSTERED") {
                    self.skip();
                    Some("NONCLUSTERED".to_string())
                } else {
                    None
                };
                if self.check_identifier("COLUMNSTORE") {
                    self.skip();
                    let clustered = clustered
                        .map(|c| format!("{} COLUMNSTORE", c))
                        .or_else(|| Some("COLUMNSTORE".to_string()));
                    self.parse_create_index_with_clustered(true, clustered)
                } else {
                    self.parse_create_index_with_clustered(true, clustered)
                }
            }
            TokenType::Index => self.parse_create_index_with_clustered(false, None),
            TokenType::Schema => self.parse_create_schema(leading_comments, or_replace),
            TokenType::Database => self.parse_create_database(or_replace),
            TokenType::Function => {
                self.parse_create_function(or_replace, or_alter, temporary, false)
            }
            TokenType::Procedure => self.parse_create_procedure(or_replace, or_alter),
            TokenType::Sequence => self.parse_create_sequence(temporary, or_replace),
            TokenType::Trigger => {
                self.parse_create_trigger(or_replace, or_alter, false, create_pos)
            }
            TokenType::Constraint => {
                self.skip();
                self.parse_create_trigger(or_replace, or_alter, true, create_pos)
            }
            TokenType::Type => self.parse_create_type(or_replace),
            TokenType::Domain => self.parse_create_domain(),
            _ => self.parse_create_other_object(materialized, or_replace, temporary),
        }
    }

    fn parse_create_other_object(
        &mut self,
        materialized: bool,
        or_replace: bool,
        temporary: bool,
    ) -> Result<Expression> {
        if self.check_identifier("CLUSTERED") || self.check_identifier("NONCLUSTERED") {
            let clustered_text = self.advance_text()?.to_ascii_uppercase();
            let clustered = if self.check_identifier("COLUMNSTORE") {
                self.skip();
                Some(format!("{} COLUMNSTORE", clustered_text))
            } else {
                Some(clustered_text)
            };
            return self.parse_create_index_with_clustered(false, clustered);
        }
        if self.check_identifier("COLUMNSTORE") && {
            let pos = self.current;

            pos + 1 < self.tokens.len() && self.tokens[pos + 1].token_type == TokenType::Index
        } {
            self.skip();
            return self.parse_create_index_with_clustered(
                false,
                Some("NONCLUSTERED COLUMNSTORE".to_string()),
            );
        }
        if self.check_identifier("TAG") {
            return self.parse_create_tag(or_replace);
        }
        if self.check_identifier("STAGE") {
            return self.parse_create_stage(or_replace, temporary);
        }
        if self.check_identifier("STREAM") {
            return self.parse_create_stream(or_replace);
        }
        if self.check_identifier("TASK") {
            return self.parse_create_task(or_replace);
        }
        if (self.check_identifier("FILE") || self.check(TokenType::File)) && {
            let next = self.current + 1;
            next < self.tokens.len() && (self.tokens[next].text.eq_ignore_ascii_case("FORMAT"))
        } {
            return self.parse_create_file_format(or_replace, temporary);
        }
        if self.check_identifier("SYNONYM") {
            self.skip();
            let name = self.parse_table_ref()?;
            self.expect_token(TokenType::For)?;
            let target = self.parse_table_ref()?;
            return Ok(Expression::CreateSynonym(Box::new(
                crate::expressions::schema_objects::models::CreateSynonym { name, target },
            )));
        }
        if self.check_identifier("STREAMING") || self.check_identifier("LIVE") {
            let start = self.current;
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                self.skip();
            }
            let sql = self.tokens_to_sql(start, self.current);
            let mut prefix = String::from("CREATE");
            if or_replace {
                prefix.push_str(" OR REPLACE");
            }
            prefix.push(' ');
            prefix.push_str(&sql);
            return Ok(Expression::Raw(Raw { sql: prefix }));
        }
        {
            if self.is_snowflake()
                && self.tokens.get(self.current + 1).is_some_and(|t| {
                    ["CLONE", "FROM", "AS", "USING", "LIKE", "("]
                        .iter()
                        .any(|word| t.text.eq_ignore_ascii_case(word))
                })
            {
                return Err(self.parse_error("Expected an object kind and a name"));
            }
            let start = self.current;
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                self.skip();
            }
            let sql = self.tokens_to_sql(start, self.current);
            let mut prefix = String::from("CREATE");
            if or_replace {
                prefix.push_str(" OR REPLACE");
            }
            if temporary {
                prefix.push_str(" TEMPORARY");
            }
            if materialized {
                prefix.push_str(" MATERIALIZED");
            }
            prefix.push(' ');
            prefix.push_str(&sql);
            Ok(Expression::Raw(Raw { sql: prefix }))
        }
    }

    fn parse_create_view_options(
        &mut self,
    ) -> Result<(Option<String>, Option<String>, Option<FunctionSecurity>)> {
        let mut algorithm: Option<String> = None;
        let mut definer: Option<String> = None;
        let mut security: Option<FunctionSecurity> = None;

        while self.match_identifier("ALGORITHM")
            || self.match_identifier("DEFINER")
            || self.match_identifier("SQL")
        {
            let option_name = self.previous_text().to_ascii_uppercase();

            if option_name == ALGORITHM && self.match_token(TokenType::Eq) {
                let value = self.expect_identifier_or_keyword()?;
                algorithm = Some(value.to_ascii_uppercase());
            } else if option_name == DEFINER && self.match_token(TokenType::Eq) {
                let mut definer_value = String::new();
                while !self.is_at_end()
                    && !self.check(TokenType::View)
                    && !self.check_identifier("ALGORITHM")
                    && !self.check_identifier("DEFINER")
                    && !self.check_identifier("SQL")
                    && !self.check_identifier("SECURITY")
                {
                    definer_value.push_str(&self.advance_text()?);
                }
                definer = Some(definer_value);
            } else if option_name == SQL && self.match_identifier("SECURITY") {
                if self.match_identifier("DEFINER") {
                    security = Some(FunctionSecurity::Definer);
                } else if self.match_identifier("INVOKER") {
                    security = Some(FunctionSecurity::Invoker);
                }
            }
        }
        Ok((algorithm, definer, security))
    }

    fn parse_create_table_modifier(&mut self, hana_modifier: Option<String>) -> Option<String> {
        if hana_modifier.is_some() {
            hana_modifier
        } else if self.check_identifier("DYNAMIC") {
            self.skip();
            Some("DYNAMIC".to_string())
        } else if self.check_identifier("ICEBERG") {
            self.skip();
            Some("ICEBERG".to_string())
        } else if self.check_identifier("EXTERNAL") {
            self.skip();
            Some("EXTERNAL".to_string())
        } else if self.check_identifier("HYBRID") {
            self.skip();
            Some("HYBRID".to_string())
        } else if self.check_identifier("TRANSIENT") {
            self.skip();
            Some("TRANSIENT".to_string())
        } else if self.check_identifier("UNLOGGED") {
            self.skip();
            Some("UNLOGGED".to_string())
        } else if self.check_identifier("DICTIONARY") || self.check(TokenType::Dictionary) {
            self.skip();
            Some("DICTIONARY".to_string())
        } else {
            None
        }
    }

    /// In Snowflake every DECLARE opens a scripting block; there is no standalone DECLARE.
    pub(crate) fn check_snowflake_scripting_block(&self) -> bool {
        self.check(TokenType::Declare)
    }

    /// The structured block, parsed by its own owner.
    pub(crate) fn parse_raw_scripting_block(&mut self) -> Result<Expression> {
        self.parse_scripting_block()
    }
}
