//! Parser ddl: constraints.

use crate::expressions::columns::main::split_mysql_table_index_key_parts::split_mysql_table_index_key_parts;
use crate::expressions::columns::models::AlterTable;
use crate::expressions::columns::models::AlterTableAction;
use crate::expressions::columns::models::ColumnPosition;
use crate::expressions::columns::models::ConstraintModifiers;
use crate::expressions::columns::models::ExcludeElement;
use crate::expressions::columns::models::ForeignKeyRef;
use crate::expressions::columns::models::MatchType;
use crate::expressions::columns::models::ReferentialAction;
use crate::expressions::columns::models::TableConstraint;
use crate::expressions::names::main::identifier_from_token::identifier_from_token;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::names::models::TableRef;
use crate::expressions::schema_objects::models::CreateView;
use crate::expressions::schema_objects::models::DropNamespace;
use crate::expressions::schema_objects::models::DropType;
use crate::expressions::schema_objects::models::DropView;
use crate::expressions::schema_objects::models::FunctionSecurity;
use crate::expressions::schema_objects::models::ViewColumn;
use crate::expressions::statements::main::snowflake_table_option_to_raw::snowflake_table_option_to_raw;
use crate::expressions::statements::models::Command;
use crate::expressions::statements::models::Raw;
use crate::expressions::statements::models::Schema;
use crate::expressions::tables::main::split_mysql_create_index_key_parts::split_mysql_create_index_key_parts;
use crate::expressions::tables::models::CreateIndex;
use crate::expressions::tables::models::DropIndex;
use crate::expressions::tables::models::DropObject;
use crate::expressions::tables::models::DropTable;
use crate::expressions::tables::models::IndexColumn;
use crate::expressions::tables::models::IndexKeyPart;
use crate::expressions::tables::models::Property;
use crate::expressions::tables::models::Tags;
use crate::expressions::tables::models::UniqueKeyProperty;
use crate::expressions::tree::models::Expression;
use crate::parser::constants::AT_SIGN;
use crate::parser::constants::SNOWFLAKE_DROP_KINDS;
use crate::parser::models::Parser;
use crate::parser::models::ViewModifiers;
use crate::tokens::models::Token;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parse constraint definition (after optional CONSTRAINT name)
    pub(crate) fn parse_constraint_definition(
        &mut self,
        name: Option<Identifier>,
    ) -> Result<TableConstraint> {
        if self.match_keywords(&[TokenType::PrimaryKey, TokenType::Key]) {
            self.parse_primary_key_constraint(name)
        } else if self.match_token(TokenType::Unique) {
            self.parse_unique_constraint(name)
        } else if self.match_keywords(&[TokenType::ForeignKey, TokenType::Key]) {
            self.parse_foreign_key_constraint(name)
        } else if self.match_token(TokenType::Check) {
            self.parse_check_constraint(name)
        } else if self.match_token(TokenType::Exclude) {
            self.parse_exclude_constraint(name)
        } else {
            if self.match_token(TokenType::Default) {
                let expression = self.parse_expression()?;
                self.expect_token(TokenType::For)?;
                let column = self.expect_identifier_with_quoted()?;
                Ok(TableConstraint::Default {
                    name,
                    expression,
                    column,
                })
            } else {
                Err(self
                    .parse_error("Expected PRIMARY KEY, UNIQUE, FOREIGN KEY, CHECK, or EXCLUDE"))
            }
        }
    }

    fn parse_primary_key_constraint(
        &mut self,
        name: Option<Identifier>,
    ) -> Result<TableConstraint> {
        let clustered = if self.check_identifier("CLUSTERED") {
            self.skip();
            Some("CLUSTERED".to_string())
        } else if self.check_identifier("NONCLUSTERED") {
            self.skip();
            Some("NONCLUSTERED".to_string())
        } else {
            None
        };

        let actual_name = if name.is_none() && !self.check(TokenType::LParen) {
            {
                if self.is_identifier_token() || self.check(TokenType::QuotedIdentifier) {
                    Some(self.expect_identifier_with_quoted()?)
                } else {
                    None
                }
            }
        } else {
            name.clone()
        };
        let mut key_parts: Vec<IndexKeyPart> = Vec::new();
        let (columns, timeseries_columns) = {
            self.expect_token(TokenType::LParen)?;
            let timeseries_columns: Vec<Identifier> = Vec::new();
            let cols = if self.check(TokenType::RParen) {
                Vec::new()
            } else {
                {
                    let (columns, parts) = self.parse_constraint_index_columns()?;
                    key_parts = parts;
                    columns
                }
            };
            self.expect_token(TokenType::RParen)?;
            (cols, timeseries_columns)
        };
        if key_parts.is_empty() && columns.iter().any(|column| column.quoted) {
            key_parts = columns
                .iter()
                .map(|column| IndexKeyPart {
                    expression: Box::new(Expression::Identifier(column.clone())),
                    prefix_length: None,
                    desc: false,
                    asc: false,
                    nulls_first: None,
                    opclass: None,
                })
                .collect();
        }
        let include_columns = if self.match_identifier("INCLUDE") {
            self.expect_token(TokenType::LParen)?;
            let cols = self.parse_identifier_list()?;
            self.expect_token(TokenType::RParen)?;
            cols
        } else {
            Vec::new()
        };
        let mut modifiers = self.parse_constraint_modifiers()?;
        modifiers.clustered = clustered;
        let has_constraint_keyword = name.is_some();
        Ok(TableConstraint::PrimaryKey {
            name: actual_name.or(name),
            columns,
            key_parts,
            timeseries_columns,
            include_columns,
            modifiers,
            has_constraint_keyword,
        })
    }

    fn parse_unique_constraint(&mut self, name: Option<Identifier>) -> Result<TableConstraint> {
        let clustered = if self.check_identifier("CLUSTERED") {
            self.skip();
            Some("CLUSTERED".to_string())
        } else if self.check_identifier("NONCLUSTERED") {
            self.skip();
            Some("NONCLUSTERED".to_string())
        } else {
            None
        };

        let use_key_keyword =
            self.match_token(TokenType::Key) || self.match_token(TokenType::Index);

        let nulls_not_distinct = self.match_text_seq(&["NULLS", "NOT", "DISTINCT"]);

        let actual_name =
            if name.is_none() && self.is_identifier_token() && !self.check_next(TokenType::Comma) {
                if self.check_next(TokenType::LParen) {
                    Some(self.expect_identifier_with_quoted()?)
                } else {
                    None
                }
            } else {
                name.clone()
            };

        if self.match_token(TokenType::LParen) {
            let (columns, key_parts) = { self.parse_constraint_index_columns()? };
            self.expect_token(TokenType::RParen)?;
            let mut modifiers = self.parse_constraint_modifiers()?;
            modifiers.clustered = clustered;
            if use_key_keyword {
                Ok(TableConstraint::Index {
                    name: actual_name.or(name),
                    columns,
                    key_parts,
                    kind: Some("UNIQUE".to_string()),
                    modifiers,
                    use_key_keyword,
                    expression: None,
                    index_type: None,
                    granularity: None,
                })
            } else {
                let has_constraint_keyword = name.is_some();
                Ok(TableConstraint::Unique {
                    name: actual_name.or(name),
                    columns,
                    key_parts,
                    columns_parenthesized: true,
                    modifiers,
                    has_constraint_keyword,
                    nulls_not_distinct,
                })
            }
        } else {
            let col_name = self.expect_identifier()?;
            let mut modifiers = self.parse_constraint_modifiers()?;
            modifiers.clustered = clustered;
            let has_constraint_keyword = name.is_some();
            Ok(TableConstraint::Unique {
                name: actual_name.or(name),
                columns: vec![Identifier::new(col_name)],
                key_parts: Vec::new(),
                columns_parenthesized: false,
                modifiers,
                has_constraint_keyword,
                nulls_not_distinct,
            })
        }
    }

    fn parse_foreign_key_constraint(
        &mut self,
        name: Option<Identifier>,
    ) -> Result<TableConstraint> {
        self.expect_token(TokenType::LParen)?;
        let columns = self.parse_identifier_list()?;
        self.expect_token(TokenType::RParen)?;
        if self.match_token(TokenType::References) {
            let references = self.parse_foreign_key_ref()?;
            let modifiers = self.parse_constraint_modifiers()?;
            Ok(TableConstraint::ForeignKey {
                name,
                columns,
                references: Some(Box::new(references)),
                on_delete: None,
                on_update: None,
                modifiers,
            })
        } else {
            let mut on_delete = None;
            let mut on_update = None;
            loop {
                if self.check(TokenType::On) {
                    let saved = self.current;
                    self.skip();
                    if self.match_token(TokenType::Delete) {
                        on_delete = Some(self.parse_referential_action()?);
                    } else if self.match_token(TokenType::Update) {
                        on_update = Some(self.parse_referential_action()?);
                    } else {
                        self.current = saved;
                        break;
                    }
                } else {
                    break;
                }
            }
            let modifiers = self.parse_constraint_modifiers()?;
            Ok(TableConstraint::ForeignKey {
                name,
                columns,
                references: None,
                on_delete,
                on_update,
                modifiers,
            })
        }
    }

    fn parse_check_constraint(&mut self, name: Option<Identifier>) -> Result<TableConstraint> {
        let expression = if self.match_token(TokenType::LParen) {
            let expr = if self.check(TokenType::Select) || self.check(TokenType::With) {
                self.parse_statement()?
            } else {
                self.parse_expression()?
            };
            self.expect_token(TokenType::RParen)?;
            expr
        } else {
            self.expect_token(TokenType::LParen)?;
            return Err(crate::tokens::errors::Error::internal(
                "the parser reached a state its checks rule out",
            ));
        };
        let modifiers = self.parse_constraint_modifiers()?;
        Ok(TableConstraint::Check {
            name,
            expression,
            modifiers,
        })
    }

    fn parse_exclude_constraint(&mut self, name: Option<Identifier>) -> Result<TableConstraint> {
        let using = if self.match_token(TokenType::Using) {
            Some(self.expect_identifier()?)
        } else {
            None
        };

        self.expect_token(TokenType::LParen)?;
        let mut elements: Vec<ExcludeElement> = Vec::new();
        loop {
            let mut expr_parts: Vec<String> = Vec::new();
            let mut paren_depth = 0;
            while !self.is_at_end() {
                if self.check(TokenType::LParen) {
                    paren_depth += 1;
                    expr_parts.push(self.advance_text()?);
                } else if self.check(TokenType::RParen) {
                    if paren_depth == 0 {
                        break;
                    }
                    paren_depth -= 1;
                    expr_parts.push(self.advance_text()?);
                } else if paren_depth == 0 && self.check(TokenType::With) {
                    break;
                } else if self.check(TokenType::String) {
                    let token = self.advance()?;
                    expr_parts.push(format!("'{}'", token.text));
                } else {
                    expr_parts.push(self.advance_text()?);
                }
            }
            let expression = expr_parts
                .join(" ")
                .replace(" (", "(")
                .replace(" )", ")")
                .replace("( ", "(")
                .replace(" ,", ",");

            self.expect_token(TokenType::With)?;
            let operator = self.advance_text()?;

            elements.push(ExcludeElement {
                expression,
                operator,
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;

        let include_columns = if self.match_identifier("INCLUDE") {
            self.expect_token(TokenType::LParen)?;
            let cols = self.parse_identifier_list()?;
            self.expect_token(TokenType::RParen)?;
            cols
        } else {
            Vec::new()
        };

        let with_params = if self.match_token(TokenType::With) {
            self.expect_token(TokenType::LParen)?;
            let mut params: Vec<(String, String)> = Vec::new();
            loop {
                let key = self.expect_identifier()?;
                self.expect_token(TokenType::Eq)?;
                let val = self.advance_text()?;
                params.push((key, val));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            params
        } else {
            Vec::new()
        };

        let using_index_tablespace =
            if self.check(TokenType::Using) && self.check_next(TokenType::Index) {
                self.skip();
                self.skip();
                if self.match_identifier("TABLESPACE") {
                    Some(self.expect_identifier()?)
                } else {
                    None
                }
            } else {
                None
            };

        let where_clause = if self.match_token(TokenType::Where) {
            self.expect_token(TokenType::LParen)?;
            let expr = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            Some(Box::new(expr))
        } else {
            None
        };

        let modifiers = self.parse_constraint_modifiers()?;
        Ok(TableConstraint::Exclude {
            name,
            using,
            elements,
            include_columns,
            where_clause,
            with_params,
            using_index_tablespace,
            modifiers,
        })
    }

    /// Parses a MySQL [FULLTEXT|SPATIAL] INDEX|KEY [name] [USING method] (cols) constraint.
    pub(crate) fn parse_index_table_constraint(&mut self) -> Result<TableConstraint> {
        let kind = if self.match_identifier("FULLTEXT") {
            Some("FULLTEXT".to_string())
        } else if self.match_identifier("SPATIAL") {
            Some("SPATIAL".to_string())
        } else {
            None
        };

        let use_key_keyword = if self.match_token(TokenType::Key) {
            true
        } else {
            self.match_token(TokenType::Index);
            false
        };

        let early_using = if self.check(TokenType::Using) {
            self.match_token(TokenType::Using);
            if self.match_identifier("BTREE") {
                Some("BTREE".to_string())
            } else if self.match_identifier("HASH") {
                Some("HASH".to_string())
            } else {
                None
            }
        } else {
            None
        };

        let name = if !self.check(TokenType::LParen)
            && !self.check(TokenType::Using)
            && self.is_identifier_token()
        {
            Some(identifier_from_token(self.advance()?, false))
        } else {
            None
        };

        let late_using = if early_using.is_none() && self.match_token(TokenType::Using) {
            if self.match_identifier("BTREE") {
                Some("BTREE".to_string())
            } else if self.match_identifier("HASH") {
                Some("HASH".to_string())
            } else {
                None
            }
        } else {
            None
        };

        self.expect_token(TokenType::LParen)?;
        let key_parts = self.parse_mysql_index_key_parts()?;
        self.expect_token(TokenType::RParen)?;
        let (columns, key_parts) = split_mysql_table_index_key_parts(key_parts);

        let mut modifiers = self.parse_constraint_modifiers()?;

        if early_using.is_some() {
            modifiers.using = early_using;
            modifiers.using_before_columns = true;
        } else if late_using.is_some() {
            modifiers.using = late_using;
            modifiers.using_before_columns = true;
        }

        Ok(TableConstraint::Index {
            name,
            columns,
            key_parts,
            kind,
            modifiers,
            use_key_keyword,
            expression: None,
            index_type: None,
            granularity: None,
        })
    }

    /// Parse constraint modifiers like ENFORCED, DEFERRABLE, NORELY, USING, etc.
    pub(crate) fn parse_constraint_modifiers(&mut self) -> Result<ConstraintModifiers> {
        let mut modifiers = ConstraintModifiers::default();
        loop {
            if self.match_token(TokenType::Not) {
                if self.match_identifier("ENFORCED") {
                    modifiers.enforced = Some(false);
                } else if self.match_identifier("DEFERRABLE") {
                    modifiers.deferrable = Some(false);
                } else if self.match_identifier("VALID") {
                    modifiers.not_valid = true;
                }
            } else if self.match_identifier("ENFORCED") {
                modifiers.enforced = Some(true);
            } else if self.match_identifier("DEFERRABLE") {
                modifiers.deferrable = Some(true);
            } else if self.match_identifier("INITIALLY") {
                if self.match_identifier("DEFERRED") {
                    modifiers.initially_deferred = Some(true);
                } else if self.match_identifier("IMMEDIATE") {
                    modifiers.initially_deferred = Some(false);
                }
            } else if self.match_identifier("NORELY") {
                modifiers.norely = true;
            } else if self.match_identifier("RELY") {
                modifiers.rely = true;
            } else if self.match_token(TokenType::Using) {
                if self.match_identifier("BTREE") {
                    modifiers.using = Some("BTREE".to_string());
                } else if self.match_identifier("HASH") {
                    modifiers.using = Some("HASH".to_string());
                }
            } else if self.match_token(TokenType::Comment) {
                if self.check(TokenType::String) {
                    modifiers.comment = Some(self.advance_text()?);
                }
            } else if self.match_identifier("VISIBLE") {
                modifiers.visible = Some(true);
            } else if self.match_identifier("INVISIBLE") {
                modifiers.visible = Some(false);
            } else if self.match_identifier("ENGINE_ATTRIBUTE") {
                self.match_token(TokenType::Eq);
                if self.check(TokenType::String) {
                    modifiers.engine_attribute = Some(self.advance_text()?);
                }
            } else if self.check(TokenType::With) {
                let saved_with = self.current;
                self.skip();
                if self.match_identifier("PARSER") {
                    if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                        modifiers.with_parser = Some(self.advance_text()?);
                    }
                } else if self.check(TokenType::LParen) {
                    self.skip();
                    loop {
                        if self.check(TokenType::RParen) || self.is_at_end() {
                            break;
                        }
                        let Ok(key) = self.advance_text() else { break };
                        if self.match_token(TokenType::Eq) {
                            let Ok(value) = self.advance_text() else {
                                break;
                            };
                            modifiers.with_options.push((key, value));
                        }
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    let _ = self.match_token(TokenType::RParen);
                } else {
                    self.current = saved_with;
                    break;
                }
            } else if self.check(TokenType::On) {
                let saved_on = self.current;
                self.skip();
                if self.match_identifier("CONFLICT") {
                    if self.match_token(TokenType::Rollback) {
                        modifiers.on_conflict = Some("ROLLBACK".to_string());
                    } else if self.match_identifier("ABORT") {
                        modifiers.on_conflict = Some("ABORT".to_string());
                    } else if self.match_identifier("FAIL") {
                        modifiers.on_conflict = Some("FAIL".to_string());
                    } else if self.match_token(TokenType::Ignore) {
                        modifiers.on_conflict = Some("IGNORE".to_string());
                    } else if self.match_token(TokenType::Replace) {
                        modifiers.on_conflict = Some("REPLACE".to_string());
                    }
                } else if self.is_identifier_token() || self.check(TokenType::QuotedIdentifier) {
                    let quoted = self.check(TokenType::QuotedIdentifier);
                    let Ok(name) = self.advance_text() else { break };
                    modifiers.on_filegroup = Some(Identifier {
                        name,
                        quoted,
                        trailing_comments: Vec::new(),
                        span: None,
                    });
                } else {
                    self.current = saved_on;
                    break;
                }
            } else {
                break;
            }
        }
        Ok(modifiers)
    }

    /// Parse foreign key reference
    pub(crate) fn parse_foreign_key_ref(&mut self) -> Result<ForeignKeyRef> {
        let table = self.parse_table_ref()?;

        let columns = if self.match_token(TokenType::LParen) {
            let cols = self.parse_identifier_list()?;
            self.expect_token(TokenType::RParen)?;
            cols
        } else {
            Vec::new()
        };

        let match_type = if self.match_token(TokenType::Match) {
            if self.check(TokenType::Full) {
                self.skip();
                Some(MatchType::Full)
            } else if self.check(TokenType::Identifier) || self.check(TokenType::Var) {
                let text = self.advance_text()?.to_ascii_uppercase();
                match text.as_str() {
                    "PARTIAL" => Some(MatchType::Partial),
                    "SIMPLE" => Some(MatchType::Simple),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        let mut on_delete = None;
        let mut on_update = None;
        let mut on_update_first = false;
        let mut first_clause = true;

        for _ in 0..2 {
            if on_delete.is_none() && self.match_keywords(&[TokenType::On, TokenType::Delete]) {
                on_delete = Some(self.parse_referential_action()?);
            } else if on_update.is_none()
                && self.match_keywords(&[TokenType::On, TokenType::Update])
            {
                if first_clause {
                    on_update_first = true;
                }
                on_update = Some(self.parse_referential_action()?);
            } else {
                break;
            }
            first_clause = false;
        }

        let mut match_after_actions = false;
        let match_type = if match_type.is_none() && self.match_token(TokenType::Match) {
            match_after_actions = on_delete.is_some() || on_update.is_some();
            if self.check(TokenType::Full) {
                self.skip();
                Some(MatchType::Full)
            } else if self.check(TokenType::Identifier) || self.check(TokenType::Var) {
                let text = self.advance_text()?.to_ascii_uppercase();
                match text.as_str() {
                    "PARTIAL" => Some(MatchType::Partial),
                    "SIMPLE" => Some(MatchType::Simple),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            match_type
        };

        let deferrable = if self.match_identifier("DEFERRABLE") {
            Some(true)
        } else if self.match_token(TokenType::Not) && self.match_identifier("DEFERRABLE") {
            Some(false)
        } else {
            None
        };

        Ok(ForeignKeyRef {
            table,
            columns,
            on_delete,
            on_update,
            on_update_first,
            match_type,
            match_after_actions,
            constraint_name: None,
            deferrable,
            has_foreign_key_keywords: false,
        })
    }

    /// Parse referential action (CASCADE, SET NULL, etc.)
    pub(crate) fn parse_referential_action(&mut self) -> Result<ReferentialAction> {
        if self.match_token(TokenType::Cascade) {
            Ok(ReferentialAction::Cascade)
        } else if self.match_keywords(&[TokenType::Set, TokenType::Null]) {
            Ok(ReferentialAction::SetNull)
        } else if self.match_keywords(&[TokenType::Set, TokenType::Default]) {
            Ok(ReferentialAction::SetDefault)
        } else if self.match_token(TokenType::Restrict) {
            Ok(ReferentialAction::Restrict)
        } else if self.match_token(TokenType::No) {
            if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("ACTION") {
                self.skip();
            }
            Ok(ReferentialAction::NoAction)
        } else {
            Err(self.parse_error("Expected CASCADE, SET NULL, SET DEFAULT, RESTRICT, or NO ACTION"))
        }
    }

    /// Parse Snowflake TAG clause: TAG (key='value', key2='value2')
    pub(crate) fn parse_tags(&mut self) -> Result<Tags> {
        self.expect_token(TokenType::LParen)?;
        let mut expressions: Vec<Expression> = Vec::new();

        loop {
            let key = self.expect_identifier_or_keyword()?;
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_primary()?;

            expressions.push(Expression::Property(Box::new(Property {
                this: Box::new(Expression::Identifier(Identifier::new(key))),
                value: Some(Box::new(value)),
            })));

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        self.expect_token(TokenType::RParen)?;

        Ok(Tags { expressions })
    }

    /// Parse CREATE VIEW
    pub(crate) fn parse_create_view(&mut self, modifiers: ViewModifiers) -> Result<Expression> {
        let ViewModifiers {
            or_replace,
            or_alter,
            materialized,
            temporary,
            algorithm,
            definer,
            security,
            secure,
        } = modifiers;
        self.expect_token(TokenType::View)?;

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);

        let name = self.parse_table_ref()?;

        let on_cluster = self.parse_on_cluster_clause()?;

        let to_table = if self.match_token(TokenType::To) {
            Some(self.parse_table_ref()?)
        } else {
            None
        };

        let copy_grants = self.match_text_seq(&["COPY", "GRANTS"]);
        let mut row_access_policy = self.parse_snowflake_row_access_policy_clause();
        let mut table_properties: Vec<Expression> = Vec::new();
        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) {
            table_properties.extend(
                self.parse_snowflake_inline_table_options()?
                    .into_iter()
                    .map(snowflake_table_option_to_raw),
            );
        }

        let (schema, unique_key, columns) =
            self.parse_create_view_columns_or_schema(materialized)?;

        let copy_grants = copy_grants || self.match_text_seq(&["COPY", "GRANTS"]);
        if row_access_policy.is_none() {
            row_access_policy = self.parse_snowflake_row_access_policy_clause();
        }

        let mut security = security;
        let mut security_sql_style = security.is_some();
        let mut security_after_name = false;
        let mut view_comment = None;

        loop {
            if view_comment.is_none() && self.match_token(TokenType::Comment) {
                let _ = self.match_token(TokenType::Eq);
                view_comment = Some(self.expect_string()?);
            } else if security.is_none()
                && self.check_identifier("SQL")
                && self.current + 1 < self.tokens.len()
                && self.tokens[self.current + 1]
                    .text
                    .eq_ignore_ascii_case("SECURITY")
            {
                self.skip();
                self.skip();
                security = self.parse_create_view_security();
                security_sql_style = true;
                security_after_name = true;
            } else if security.is_none() && self.match_identifier("SECURITY") {
                security = self.parse_create_view_security();
                security_sql_style = false;
                security_after_name = false;
            } else {
                break;
            }
        }

        let tags = self.parse_create_view_tags()?;

        let options = if self.match_identifier("OPTIONS") {
            self.parse_options_list()?
        } else {
            Vec::new()
        };

        let build = self.parse_create_view_build()?;

        let refresh = if self.match_token(TokenType::Refresh) {
            {
                Some(Box::new(self.parse_refresh_trigger_property()?))
            }
        } else {
            None
        };

        let to_table = if to_table.is_none() && self.match_token(TokenType::To) {
            Some(self.parse_table_ref()?)
        } else {
            to_table
        };

        let auto_refresh = self.parse_create_view_auto_refresh();

        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) {
            table_properties.extend(
                self.parse_snowflake_inline_table_options()?
                    .into_iter()
                    .map(snowflake_table_option_to_raw),
            );
        }

        let clickhouse_population = { None };

        let has_as = self.match_token(TokenType::As);
        if !has_as && !self.check(TokenType::Select) && !self.check(TokenType::With) {
            return Ok(Expression::CreateView(Box::new(CreateView {
                name,
                columns,
                query: Expression::Null(Null),
                or_replace,
                or_alter,
                if_not_exists,
                materialized,
                temporary,
                secure,
                algorithm,
                definer,
                security,
                security_sql_style,
                security_after_name,
                query_parenthesized: false,
                locking_mode: None,
                locking_access: None,
                copy_grants,
                comment: view_comment,
                row_access_policy,
                tags,
                options,
                build,
                refresh,
                schema: schema.map(Box::new),
                unique_key: unique_key.map(Box::new),
                no_schema_binding: false,
                auto_refresh,
                clickhouse_population: clickhouse_population.clone(),
                on_cluster,
                to_table,
                table_properties,
            })));
        }

        let mut locking_mode: Option<String> = None;
        let mut locking_access: Option<String> = None;
        if self.match_token(TokenType::Lock) || self.match_identifier("LOCKING") {
            if self.match_token(TokenType::Row) {
                locking_mode = Some("ROW".to_string());
            } else if self.match_token(TokenType::Table) {
                locking_mode = Some("TABLE".to_string());
            } else if self.match_token(TokenType::Database) || self.match_identifier("DATABASE") {
                locking_mode = Some("DATABASE".to_string());
            }
            if self.match_token(TokenType::For) {
                if self.match_identifier("ACCESS") {
                    locking_access = Some("ACCESS".to_string());
                } else if self.match_identifier("READ") {
                    locking_access = Some("READ".to_string());
                } else if self.match_identifier("WRITE") {
                    locking_access = Some("WRITE".to_string());
                }
            }
        }

        let query_parenthesized = self.check(TokenType::LParen);
        let query = self.parse_create_view_query(query_parenthesized)?;

        let no_schema_binding = self.match_text_seq(&["WITH", "NO", "SCHEMA", "BINDING"]);

        Ok(Expression::CreateView(Box::new(CreateView {
            name,
            columns,
            query,
            or_replace,
            or_alter,
            if_not_exists,
            materialized,
            temporary,
            secure,
            algorithm,
            definer,
            security,
            security_sql_style,
            security_after_name,
            query_parenthesized,
            locking_mode,
            locking_access,
            copy_grants,
            comment: view_comment,
            row_access_policy,
            tags,
            options,
            build,
            refresh,
            schema: schema.map(Box::new),
            unique_key: unique_key.map(Box::new),
            no_schema_binding,
            auto_refresh,
            clickhouse_population,
            on_cluster,
            to_table,
            table_properties,
        })))
    }

    fn parse_create_view_columns_or_schema(
        &mut self,
        materialized: bool,
    ) -> Result<(Option<Schema>, Option<UniqueKeyProperty>, Vec<ViewColumn>)> {
        let mut schema: Option<Schema> = None;
        let mut unique_key: Option<UniqueKeyProperty> = None;

        let columns = if self.check(TokenType::LParen) {
            if materialized {
                let saved_pos = self.current;

                if let Some(Expression::Schema(parsed_schema)) = self.parse_schema()? {
                    let parsed_schema = *parsed_schema;

                    schema = Some(parsed_schema);

                    if self.match_text_seq(&["KEY"]) {
                        let exprs = self.parse_composite_key_expressions()?;
                        unique_key = Some(UniqueKeyProperty { expressions: exprs });
                    }

                    Vec::new()
                } else {
                    self.current = saved_pos;
                    self.parse_view_columns()?
                }
            } else {
                self.parse_view_columns()?
            }
        } else {
            Vec::new()
        };
        Ok((schema, unique_key, columns))
    }

    fn parse_create_view_tags(&mut self) -> Result<Vec<(String, String)>> {
        Ok(if self.match_identifier("TAG") {
            let mut tag_list: Vec<(String, String)> = Vec::new();
            if self.match_token(TokenType::LParen) {
                loop {
                    let tag_name = self.expect_identifier()?;
                    let tag_value = if self.match_token(TokenType::Eq) {
                        self.expect_string()?
                    } else {
                        String::new()
                    };
                    tag_list.push((tag_name, tag_value));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
            }
            tag_list
        } else {
            Vec::new()
        })
    }

    fn parse_create_view_build(&mut self) -> Result<Option<String>> {
        Ok(if self.match_identifier("BUILD") {
            if self.match_identifier("IMMEDIATE") {
                Some("IMMEDIATE".to_string())
            } else if self.match_identifier("DEFERRED") {
                Some("DEFERRED".to_string())
            } else {
                let value = self.expect_identifier_or_keyword()?;
                Some(value.to_ascii_uppercase())
            }
        } else {
            None
        })
    }

    fn parse_create_view_auto_refresh(&mut self) -> Option<bool> {
        if self.match_text_seq(&["AUTO", "REFRESH"]) {
            if self.match_identifier("YES") {
                Some(true)
            } else if self.match_identifier("NO") {
                Some(false)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn parse_create_view_query(&mut self, query_parenthesized: bool) -> Result<Expression> {
        Ok(if self.check(TokenType::With) {
            self.parse_statement()?
        } else if query_parenthesized {
            self.skip();
            let inner = if self.check(TokenType::With) {
                self.parse_statement()?
            } else {
                self.parse_select()?
            };
            self.expect_token(TokenType::RParen)?;
            inner
        } else {
            self.parse_select()?
        })
    }

    pub(crate) fn parse_create_view_security(&mut self) -> Option<FunctionSecurity> {
        if self.match_identifier("DEFINER") {
            Some(FunctionSecurity::Definer)
        } else if self.match_identifier("INVOKER") {
            Some(FunctionSecurity::Invoker)
        } else if self.match_identifier("NONE") {
            Some(FunctionSecurity::None)
        } else {
            None
        }
    }

    /// Parses a view column list without types: (c1, c2 OPTIONS(...) COMMENT '...').
    pub(crate) fn parse_view_columns(&mut self) -> Result<Vec<ViewColumn>> {
        self.expect_token(TokenType::LParen)?;
        let mut cols: Vec<ViewColumn> = Vec::new();
        loop {
            let col_name = self.expect_identifier()?;
            let options = if self.match_identifier("OPTIONS") {
                self.parse_options_list()?
            } else {
                Vec::new()
            };
            let comment = if self.match_token(TokenType::Comment) {
                Some(self.expect_string()?)
            } else {
                None
            };
            cols.push(ViewColumn {
                name: Identifier::new(col_name),
                comment,
                options,
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(cols)
    }

    /// Parse CREATE [CLUSTERED|NONCLUSTERED] INDEX
    pub(crate) fn parse_create_index_with_clustered(
        &mut self,
        unique: bool,
        clustered: Option<String>,
    ) -> Result<Expression> {
        self.expect_token(TokenType::Index)?;

        let concurrently = self.match_identifier("CONCURRENTLY");

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);

        let name = if if_not_exists && self.check(TokenType::On) {
            Identifier::new("")
        } else {
            self.expect_identifier_with_quoted()?
        };
        self.expect_token(TokenType::On)?;
        let table = self.parse_table_ref()?;

        let using = if self.match_token(TokenType::Using) {
            Some(self.expect_identifier()?)
        } else {
            None
        };

        let (columns, key_parts) = if self.match_token(TokenType::LParen) {
            let parts = { Vec::new() };
            let cols = if parts.is_empty() {
                self.parse_index_columns()?
            } else {
                Vec::new()
            };
            self.expect_token(TokenType::RParen)?;
            if parts.is_empty() {
                (cols, Vec::new())
            } else {
                split_mysql_create_index_key_parts(parts)
            }
        } else if clustered
            .as_ref()
            .is_some_and(|c| c.contains("COLUMNSTORE"))
        {
            (Vec::new(), Vec::new())
        } else {
            self.expect_token(TokenType::LParen)?;
            let cols = self.parse_index_columns()?;
            self.expect_token(TokenType::RParen)?;
            (cols, Vec::new())
        };

        let include_columns = if self.match_identifier("INCLUDE") {
            self.expect_token(TokenType::LParen)?;
            let mut cols: Vec<Identifier> = Vec::new();
            loop {
                cols.push(self.expect_identifier_with_quoted()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            cols
        } else {
            Vec::new()
        };

        let with_options = if self.check(TokenType::With) {
            if self
                .peek_nth(1)
                .is_some_and(|t| t.token_type == TokenType::LParen)
            {
                self.skip();
                self.parse_with_properties()?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let where_clause = if self.match_token(TokenType::Where) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let on_filegroup = if self.match_token(TokenType::On) {
            let token = self.advance()?;
            let mut filegroup = token.text.to_string();
            if self.match_token(TokenType::LParen) {
                filegroup.push('(');
                loop {
                    let col_token = self.advance()?;
                    if col_token.token_type == TokenType::QuotedIdentifier {
                        filegroup.push('[');
                        filegroup.push_str(&col_token.text);
                        filegroup.push(']');
                    } else {
                        filegroup.push_str(&col_token.text);
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                    filegroup.push_str(", ");
                }
                self.expect_token(TokenType::RParen)?;
                filegroup.push(')');
            }
            Some(filegroup)
        } else {
            None
        };

        Ok(Expression::CreateIndex(Box::new(CreateIndex {
            name,
            table,
            columns,
            key_parts,
            unique,
            if_not_exists,
            using,
            clustered,
            concurrently,
            where_clause,
            include_columns,
            with_options,
            on_filegroup,
        })))
    }

    /// Parse index columns - can be identifiers or expressions (like function calls)
    pub(crate) fn parse_index_columns(&mut self) -> Result<Vec<IndexColumn>> {
        let mut columns: Vec<IndexColumn> = Vec::new();
        loop {
            let expr = self.parse_expression()?;

            let column = match &expr {
                Expression::Identifier(ident) => ident.clone(),
                Expression::Column(col) => col.name.clone(),
                Expression::Function(_func) => Identifier::new(self.expression_to_sql(&expr)),
                _ => Identifier::new(self.expression_to_sql(&expr)),
            };

            let opclass = if self.is_identifier_token()
                && !self.check(TokenType::Asc)
                && !self.check(TokenType::Desc)
                && !self.check(TokenType::Nulls)
            {
                let token = self.advance()?;
                let mut opclass_name = self.index_opclass_token_sql(&token);
                while self.match_token(TokenType::Dot) {
                    opclass_name.push('.');
                    if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                        let token = self.advance()?;
                        opclass_name.push_str(&self.index_opclass_token_sql(&token));
                    }
                }
                Some(opclass_name)
            } else {
                None
            };

            let desc = self.match_token(TokenType::Desc);
            let asc = if !desc {
                self.match_token(TokenType::Asc)
            } else {
                false
            };
            let nulls_first = if self.match_token(TokenType::Nulls) {
                if self.match_token(TokenType::First) {
                    Some(true)
                } else if self.match_token(TokenType::Last) {
                    Some(false)
                } else {
                    None
                }
            } else {
                None
            };
            columns.push(IndexColumn {
                column,
                desc,
                asc,
                nulls_first,
                opclass,
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(columns)
    }

    pub(crate) fn index_opclass_token_sql(&self, token: &Token) -> String {
        if token.token_type != TokenType::QuotedIdentifier {
            return token.text.clone();
        }

        if self.source.is_some() {
            let original = self.source_text_range(token.span.start, token.span.end);
            if !original.is_empty() {
                return original;
            }
        }

        format!("\"{}\"", token.text.replace('"', "\"\""))
    }

    /// Convert an expression to its SQL string representation (simple version for index expressions)
    pub(crate) fn expression_to_sql(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(ident) => ident.name.clone(),
            Expression::Function(func) => {
                let args = func
                    .args
                    .iter()
                    .map(|a| self.expression_to_sql(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", func.name, args)
            }
            Expression::Column(col) => {
                if let Some(ref table) = col.table {
                    format!("{}.{}", table, col.name)
                } else {
                    col.name.to_string()
                }
            }
            Expression::Literal(lit) => match lit.as_ref() {
                Literal::String(s) => format!("'{}'", s),
                Literal::Number(n) => n.clone(),
                _ => "?".to_string(),
            },
            Expression::Null(_) => "NULL".to_string(),
            Expression::Boolean(b) => {
                if b.value {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            _ => "?".to_string(),
        }
    }

    /// Parse DROP statement
    pub(crate) fn parse_drop(&mut self) -> Result<Expression> {
        let leading_comments = self.current_leading_comments().to_vec();
        self.expect_token(TokenType::Drop)?;

        if self.check_identifier("ICEBERG")
            && self.current + 1 < self.tokens.len()
            && self.tokens[self.current + 1].token_type == TokenType::Table
        {
            self.skip();
            return self.parse_drop_table_with_iceberg(leading_comments, true);
        }

        match self.peek().token_type {
            TokenType::Table => self.parse_drop_table_with_iceberg(leading_comments, false),
            TokenType::View => self.parse_drop_view(false),
            TokenType::Materialized => {
                self.skip();
                self.parse_drop_view(true)
            }
            TokenType::Index => self.parse_drop_index(),
            TokenType::Schema => self.parse_drop_schema(),
            TokenType::Database => self.parse_drop_database(),
            TokenType::Function => self.parse_drop_function(),
            TokenType::Procedure => self.parse_drop_procedure(),
            TokenType::Sequence => self.parse_drop_sequence(),
            TokenType::Trigger => self.parse_drop_trigger(),
            TokenType::Type => self.parse_drop_type(),
            TokenType::Domain => {
                self.skip();
                let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
                let name = self.parse_table_ref()?;
                let cascade = self.match_token(TokenType::Cascade);
                if !cascade {
                    self.match_token(TokenType::Restrict);
                }
                Ok(Expression::DropType(Box::new(DropType {
                    name,
                    if_exists,
                    cascade,
                })))
            }
            TokenType::Namespace => {
                self.skip();
                let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
                let mut name_parts = vec![self.expect_identifier()?];
                while self.match_token(TokenType::Dot) {
                    name_parts.push(self.expect_identifier()?);
                }
                let name = Identifier::new(name_parts.join("."));
                let cascade = self.match_token(TokenType::Cascade);
                if !cascade {
                    self.match_token(TokenType::Restrict);
                }
                Ok(Expression::DropNamespace(Box::new(DropNamespace {
                    name,
                    if_exists,
                    cascade,
                })))
            }
            _ => {
                if matches!(
                    self.config.dialect,
                    Some(crate::tokens::models::DialectType::Snowflake)
                ) {
                    if let Some(kind) = SNOWFLAKE_DROP_KINDS
                        .iter()
                        .find(|kind| self.match_text_seq(kind))
                    {
                        return self.parse_drop_snowflake_object(kind.join(" "));
                    }
                    let text_upper = self.peek_text().to_ascii_uppercase();
                    let is_snowflake_drop =
                        matches!(text_upper.as_str(), "INTEGRATION" | "NETWORK" | "SHARE");
                    if is_snowflake_drop {
                        self.skip();
                        let mut tokens: Vec<(String, TokenType)> = vec![
                            ("DROP".to_string(), TokenType::Var),
                            (
                                self.previous_text().to_ascii_uppercase(),
                                self.previous().token_type,
                            ),
                        ];
                        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                            let token = self.advance()?;
                            let text = if token.token_type == TokenType::QuotedIdentifier {
                                format!("\"{}\"", token.text)
                            } else if token.token_type == TokenType::String {
                                format!("'{}'", token.text)
                            } else {
                                token.text.to_string()
                            };
                            tokens.push((text, token.token_type));
                        }
                        return Ok(Expression::Command(Box::new(Command {
                            this: self.join_command_tokens(tokens),
                        })));
                    }
                }
                Err(self.parse_error(format!(
                    "Expected TABLE, VIEW, INDEX, SCHEMA, DATABASE, FUNCTION, PROCEDURE, SEQUENCE, TRIGGER, TYPE, or NAMESPACE after DROP, got {:?}",
                    self.peek().token_type
                )))
            }
        }
    }

    /// Parse a Snowflake DROP of `kind`, already consumed: the name and the options that kind takes.
    fn parse_drop_snowflake_object(&mut self, kind: String) -> Result<Expression> {
        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.parse_object_name()?;
        let (cascade, restrict, force) = match kind.as_str() {
            "APPLICATION" | "OPENFLOW RUNTIME" => {
                (self.match_token(TokenType::Cascade), false, false)
            }
            "EXTERNAL TABLE" => {
                let cascade = self.match_token(TokenType::Cascade);
                (
                    cascade,
                    !cascade && self.match_token(TokenType::Restrict),
                    false,
                )
            }
            "SERVICE" => (false, false, self.match_texts(&["FORCE"])),
            _ => (false, false, false),
        };
        Ok(Expression::DropObject(Box::new(DropObject {
            kind,
            name,
            if_exists,
            cascade,
            restrict,
            force,
        })))
    }

    /// Parse DROP TABLE
    pub(crate) fn parse_drop_table_with_iceberg(
        &mut self,
        leading_comments: Vec<String>,
        iceberg: bool,
    ) -> Result<Expression> {
        self.expect_token(TokenType::Table)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);

        let mut names: Vec<TableRef> = Vec::new();
        loop {
            names.push(self.parse_object_name_ref()?);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        let mut cascade = false;
        let mut cascade_constraints = false;
        let mut restrict = false;
        if self.match_token(TokenType::Cascade) {
            if self.match_identifier("CONSTRAINTS") {
                cascade_constraints = true;
            } else {
                cascade = true;
            }
        } else {
            restrict = self.match_token(TokenType::Restrict);
        }

        let purge = self.match_identifier("PURGE");

        let sync = { false };

        Ok(Expression::DropTable(Box::new(DropTable {
            names,
            if_exists,
            cascade,
            cascade_constraints,
            purge,
            leading_comments,
            object_id_args: None,
            sync,
            iceberg,
            restrict,
        })))
    }

    /// Parse DROP VIEW
    pub(crate) fn parse_drop_view(&mut self, materialized: bool) -> Result<Expression> {
        self.expect_token(TokenType::View)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.parse_object_name_ref()?;

        Ok(Expression::DropView(Box::new(DropView {
            name,
            if_exists,
            materialized,
        })))
    }

    /// Parse DROP INDEX
    pub(crate) fn parse_drop_index(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Index)?;

        let concurrently = self.match_identifier("CONCURRENTLY");

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);

        let name = self.parse_object_name_ref()?;

        let table = if self.match_token(TokenType::On) {
            Some(self.parse_table_ref()?)
        } else {
            None
        };

        Ok(Expression::DropIndex(Box::new(DropIndex {
            name,
            table,
            if_exists,
            concurrently,
        })))
    }

    /// Parse ALTER statement
    pub(crate) fn parse_alter(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Alter)?;

        let alter_table_modifier = if self.check_identifier("ICEBERG") {
            self.skip();
            Some("ICEBERG".to_string())
        } else {
            None
        };

        match self.peek().token_type {
            TokenType::Table => self.parse_alter_table(alter_table_modifier),
            TokenType::View => self.parse_alter_view_with_modifiers(None, None, None),
            TokenType::Index => self.parse_alter_index(),
            TokenType::Sequence => self.parse_alter_sequence(),
            _ if self.check_identifier("SESSION") && !self.check_next_identifier("POLICY") => {
                self.skip();
                match self.parse_alter_session()? {
                    Some(expr) => Ok(expr),
                    None => Ok(Expression::Command(Box::new(Command {
                        this: "ALTER SESSION".to_string(),
                    }))),
                }
            }
            _ if matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Snowflake)
            ) =>
            {
                match self.parse_alter_snowflake_object()? {
                    Some(expr) => Ok(expr),
                    None => self.parse_alter_view_or_raw(),
                }
            }
            _ => self.parse_alter_view_or_raw(),
        }
    }

    fn parse_alter_table(&mut self, alter_table_modifier: Option<String>) -> Result<Expression> {
        self.skip();
        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let has_only = self.match_token(TokenType::Only);
        let mut name = self.parse_table_ref()?;
        if has_only {
            name.only = true;
        }

        let on_cluster = self.parse_on_cluster_clause()?;

        let partition = if self.match_token(TokenType::Partition) {
            self.expect_token(TokenType::LParen)?;
            let mut parts: Vec<(Identifier, Expression)> = Vec::new();
            loop {
                let key = self.expect_identifier()?;
                self.expect_token(TokenType::Eq)?;
                let value = self.parse_expression()?;
                parts.push((Identifier::new(key), value));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            Some(parts)
        } else {
            None
        };

        let (actions, with_check_modifier) = self.parse_alter_table_actions()?;

        let (algorithm, lock) = self.parse_alter_table_algorithm_and_lock()?;

        Ok(Expression::AlterTable(Box::new(AlterTable {
            name,
            actions,
            if_exists,
            algorithm,
            lock,
            with_check: with_check_modifier,
            partition,
            on_cluster,
            table_modifier: alter_table_modifier,
        })))
    }

    fn parse_alter_view_or_raw(&mut self) -> Result<Expression> {
        let mut view_algorithm = None;
        let mut view_definer = None;
        let mut view_sql_security = None;

        loop {
            if self.check_identifier("ALGORITHM") {
                self.skip();
                self.expect_token(TokenType::Eq)?;
                view_algorithm = Some(self.expect_identifier_or_keyword()?.to_ascii_uppercase());
            } else if self.check_identifier("DEFINER") {
                self.skip();
                self.expect_token(TokenType::Eq)?;
                let mut definer_str = String::new();
                if self.check(TokenType::String) {
                    definer_str.push_str(&format!("'{}'", self.advance_text()?));
                } else {
                    definer_str.push_str(&self.expect_identifier_or_keyword()?);
                }
                if !self.is_at_end() && self.peek_text() == AT_SIGN {
                    definer_str.push_str(&self.advance_text()?);
                    if self.check(TokenType::String) {
                        definer_str.push_str(&format!("'{}'", self.advance_text()?));
                    } else if !self.is_at_end() {
                        definer_str.push_str(&self.advance_text()?);
                    }
                }
                view_definer = Some(definer_str);
            } else if self.check_identifier("SQL") {
                self.skip();
                if self.match_identifier("SECURITY") {
                    self.match_token(TokenType::Eq);
                    view_sql_security =
                        Some(self.expect_identifier_or_keyword()?.to_ascii_uppercase());
                }
            } else {
                break;
            }
        }

        if self.check(TokenType::View) {
            self.parse_alter_view_with_modifiers(view_algorithm, view_definer, view_sql_security)
        } else {
            let start = self.current;
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                self.skip();
            }
            let sql = self.tokens_to_sql(start, self.current);
            Ok(Expression::Raw(Raw {
                sql: format!("ALTER {}", sql),
            }))
        }
    }

    fn parse_alter_table_actions(&mut self) -> Result<(Vec<AlterTableAction>, Option<String>)> {
        let mut actions: Vec<AlterTableAction> = Vec::new();
        let mut last_was_add_column = false;
        let mut with_check_modifier: Option<String> = None;

        loop {
            if self.check_identifier("ALGORITHM") || self.check_identifier("LOCK") {
                break;
            }

            if self.check(TokenType::With) {
                let saved = self.current;
                self.skip();
                if self.check(TokenType::Check) {
                    self.skip();
                    with_check_modifier = Some("WITH CHECK".to_string());
                } else if self.check_identifier("NOCHECK") {
                    self.skip();
                    with_check_modifier = Some("WITH NOCHECK".to_string());
                } else {
                    self.current = saved;
                }
            }

            if last_was_add_column
                && !self.check(TokenType::Add)
                && !self.check(TokenType::Drop)
                && !self.check(TokenType::Alter)
                && !self.check(TokenType::Rename)
                && !self.check(TokenType::Set)
                && !self.check_identifier("MODIFY")
                && !self.check(TokenType::Delete)
                && !self.check(TokenType::Update)
                && !self.check_identifier("DETACH")
                && !self.check_identifier("ATTACH")
                && !self.check_identifier("FREEZE")
                && !self.check_identifier("CLEAR")
                && !self.check_identifier("MATERIALIZE")
                && !self.check(TokenType::Comment)
                && !self.check(TokenType::Replace)
                && !self.check_identifier("MOVE")
                && !self.check_identifier("REMOVE")
                && !self.check_identifier("APPLY")
            {
                self.match_token(TokenType::Column);
                let if_not_exists =
                    self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
                let col_def = self.parse_column_def()?;
                let position = if self.match_token(TokenType::First) {
                    Some(ColumnPosition::First)
                } else if self.match_token(TokenType::After) {
                    let after_col = self.expect_identifier()?;
                    let after_name = if self.match_token(TokenType::Dot) {
                        let field = self.expect_identifier()?;
                        format!("{}.{}", after_col, field)
                    } else {
                        after_col
                    };
                    Some(ColumnPosition::After(Identifier::new(after_name)))
                } else {
                    None
                };
                actions.push(AlterTableAction::AddColumn {
                    column: col_def,
                    if_not_exists,
                    position,
                });
            } else {
                if self.check_identifier("ALGORITHM") || self.check_identifier("LOCK") {
                    self.current -= 1;
                    break;
                }
                let action = self.parse_alter_action()?;
                last_was_add_column = matches!(action, AlterTableAction::AddColumn { .. });
                actions.push(action);
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok((actions, with_check_modifier))
    }

    fn parse_alter_table_algorithm_and_lock(&mut self) -> Result<(Option<String>, Option<String>)> {
        let mut algorithm = None;
        let mut lock = None;
        loop {
            if self.check_identifier("ALGORITHM") {
                self.skip();
                self.expect_token(TokenType::Eq)?;
                algorithm = Some(self.expect_identifier_or_keyword()?.to_ascii_uppercase());
                self.match_token(TokenType::Comma);
            } else if self.check_identifier("LOCK") {
                self.skip();
                self.expect_token(TokenType::Eq)?;
                lock = Some(self.expect_identifier_or_keyword()?.to_ascii_uppercase());
                self.match_token(TokenType::Comma);
            } else if self.match_token(TokenType::Comma) {
                if self.check_identifier("ALGORITHM") {
                    self.skip();
                    self.expect_token(TokenType::Eq)?;
                    algorithm = Some(self.expect_identifier_or_keyword()?.to_ascii_uppercase());
                } else if self.check_identifier("LOCK") {
                    self.skip();
                    self.expect_token(TokenType::Eq)?;
                    lock = Some(self.expect_identifier_or_keyword()?.to_ascii_uppercase());
                } else {
                    self.current -= 1;
                    break;
                }
            } else {
                break;
            }
        }
        Ok((algorithm, lock))
    }
}
