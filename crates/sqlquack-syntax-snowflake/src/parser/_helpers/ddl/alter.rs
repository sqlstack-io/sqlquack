//! Parser ddl: alter.

use std::sync::Arc;

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::main::split_mysql_table_index_key_parts::split_mysql_table_index_key_parts;
use sqlquack_syntax::expressions::columns::models::AlterColumnAction;
use sqlquack_syntax::expressions::columns::models::AlterTableAction;
use sqlquack_syntax::expressions::columns::models::ColumnDef;
use sqlquack_syntax::expressions::columns::models::ColumnPosition;
use sqlquack_syntax::expressions::columns::models::TableConstraint;
use sqlquack_syntax::expressions::names::models::BooleanLiteral;
use sqlquack_syntax::expressions::names::models::Column;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::expressions::operators::models::Export;
use sqlquack_syntax::expressions::queries::models::Values;
use sqlquack_syntax::expressions::statements::models::Cache;
use sqlquack_syntax::expressions::statements::models::LoadData;
use sqlquack_syntax::expressions::statements::models::Pragma;
use sqlquack_syntax::expressions::statements::models::Raw;
use sqlquack_syntax::expressions::statements::models::Truncate;
use sqlquack_syntax::expressions::statements::models::TruncateIdentity;
use sqlquack_syntax::expressions::statements::models::TruncateTableEntry;
use sqlquack_syntax::expressions::statements::models::TruncateTarget;
use sqlquack_syntax::expressions::statements::models::TryCatch;
use sqlquack_syntax::expressions::statements::models::Uncache;
use sqlquack_syntax::expressions::statements::models::Use;
use sqlquack_syntax::expressions::statements::models::UseKind;
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::expressions::tables::models::DataDeletionProperty;
use sqlquack_syntax::expressions::tables::models::WithSystemVersioningProperty;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Parse ALTER TABLE action
    pub(crate) fn parse_alter_action(&mut self) -> Result<AlterTableAction> {
        if let Some(action) = self.parse_data_metric_function_action()? {
            Ok(AlterTableAction::DataMetricFunction(action))
        } else if self.match_token(TokenType::Add) {
            self.parse_alter_add_action()
        } else if self.match_token(TokenType::Drop) {
            self.parse_alter_drop_action()
        } else if self.match_token(TokenType::Rename) {
            self.parse_alter_rename_action()
        } else if self.match_token(TokenType::Alter) {
            self.parse_alter_alter_action()
        } else if self.match_identifier("MODIFY") {
            self.parse_alter_modify_action()
        } else if self.match_identifier("CHANGE") {
            self.parse_alter_change_action()
        } else if !self.is_snowflake() && self.match_token(TokenType::Constraint) {
            let name = Some(self.expect_identifier_with_quoted()?);
            let constraint = self.parse_constraint_definition(name)?;
            Ok(AlterTableAction::AddConstraint(constraint))
        } else if !self.is_snowflake()
            && (self.check(TokenType::PrimaryKey)
                || self.check(TokenType::ForeignKey)
                || self.check(TokenType::Unique))
        {
            let constraint = self.parse_table_constraint()?;
            Ok(AlterTableAction::AddConstraint(constraint))
        } else if self.match_token(TokenType::Delete) {
            self.expect_token(TokenType::Where)?;
            let where_clause = self.parse_expression()?;
            Ok(AlterTableAction::Delete { where_clause })
        } else if self.match_keyword("SWAP") {
            self.expect_token(TokenType::With)?;
            let target = self.parse_table_ref()?;
            Ok(AlterTableAction::SwapWith(target))
        } else if self.match_token(TokenType::Set) {
            self.parse_alter_set_action()
        } else if self.match_keyword("UNSET") {
            self.parse_alter_unset_action()
        } else if self.match_keyword("CLUSTER") {
            self.expect_token(TokenType::By)?;
            if self.match_identifier("NONE") {
                return Ok(AlterTableAction::Raw {
                    sql: "CLUSTER BY NONE".to_string(),
                });
            }
            self.expect_token(TokenType::LParen)?;
            let ordered = self.parse_order_by_list()?;
            let expressions: Vec<Expression> = ordered
                .into_iter()
                .map(|o| Expression::Ordered(Box::new(o)))
                .collect();
            self.expect_token(TokenType::RParen)?;
            Ok(AlterTableAction::ClusterBy { expressions })
        } else if self.match_token(TokenType::AutoIncrement) {
            self.expect_token(TokenType::Eq)?;
            let start = self.current;
            self.parse_expression()?;
            Ok(AlterTableAction::Raw {
                sql: format!("AUTO_INCREMENT={}", self.tokens_to_sql(start, self.current)),
            })
        } else if self.match_token(TokenType::Replace) {
            self.parse_alter_replace_action()
        } else {
            self.parse_alter_partition_maintenance_or_other()
        }
    }

    fn parse_alter_modify_action(&mut self) -> Result<AlterTableAction> {
        let mut tokens: Vec<(String, TokenType)> = vec![("MODIFY".to_string(), TokenType::Var)];
        self.match_token(TokenType::Column);
        tokens.push(("COLUMN".to_string(), TokenType::Column));
        let mut paren_depth = 0i32;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::Comma) && paren_depth == 0 {
                break;
            }
            let token = self.advance()?;
            if token.token_type == TokenType::LParen {
                paren_depth += 1;
            } else if token.token_type == TokenType::RParen {
                paren_depth -= 1;
            }
            tokens.push((self.raw_sql_token_text(&token), token.token_type));
        }
        Ok(AlterTableAction::Raw {
            sql: self.join_command_tokens(tokens),
        })
    }

    fn parse_alter_replace_action(&mut self) -> Result<AlterTableAction> {
        if self.match_token(TokenType::Partition) {
            let partition_expr = if self.match_text_seq(&["ALL"]) {
                Expression::Identifier(Identifier::new("ALL".to_string()))
            } else if self.match_text_seq(&["ID"]) {
                let id_val = self.parse_expression()?;
                let id_str = match &id_val {
                    Expression::Literal(lit) if matches!(lit.as_ref(), Literal::String(_)) => {
                        let Literal::String(s) = lit.as_ref() else {
                            return Err(sqlquack_syntax::tokens::errors::Error::internal(
                                "the parser reached a state its checks rule out",
                            ));
                        };
                        format!("ID '{}'", s)
                    }
                    _ => format!("ID {}", "?"),
                };
                Expression::Raw(Raw { sql: id_str })
            } else {
                self.parse_expression()?
            };
            let source = if self.match_token(TokenType::From) {
                let tref = self.parse_table_ref()?;
                Some(Box::new(Expression::Table(Box::new(tref))))
            } else {
                None
            };
            Ok(AlterTableAction::ReplacePartition {
                partition: partition_expr,
                source,
            })
        } else {
            Err(self.parse_error("Expected PARTITION after REPLACE in ALTER TABLE"))
        }
    }

    fn parse_alter_change_action(&mut self) -> Result<AlterTableAction> {
        self.match_token(TokenType::Column);
        let old_name = self.parse_unquoted_identifier(Self::expect_identifier)?;
        let new_name = self.parse_unquoted_identifier(Self::expect_identifier)?;
        let data_type = if !self.is_at_end()
            && !self.check(TokenType::Comment)
            && !self.check(TokenType::Comma)
            && !self.check(TokenType::Semicolon)
        {
            let tok = self.peek();
            if tok.token_type.is_keyword()
                || tok.token_type == TokenType::Identifier
                || tok.token_type == TokenType::Var
            {
                Some(self.parse_data_type()?)
            } else {
                None
            }
        } else {
            None
        };
        let comment = if self.match_token(TokenType::Comment) {
            Some(self.expect_string()?)
        } else {
            None
        };
        let cascade = self.match_text_seq(&["CASCADE"]);
        if !cascade {
            self.match_text_seq(&["RESTRICT"]);
        }
        Ok(AlterTableAction::ChangeColumn {
            old_name,
            new_name,
            data_type,
            comment,
            cascade,
        })
    }

    fn parse_alter_rename_action(&mut self) -> Result<AlterTableAction> {
        if let Some(action) = self.parse_snowflake_rename_constraint()? {
            return Ok(action);
        }
        if self.match_token(TokenType::Index) || self.match_token(TokenType::Key) {
            let old_name = self.expect_identifier_or_keyword_with_quoted()?;
            self.expect_token(TokenType::To)?;
            let new_name = self.expect_identifier_or_keyword_with_quoted()?;
            Ok(AlterTableAction::Raw {
                sql: format!("RENAME INDEX {} TO {}", old_name.name, new_name.name),
            })
        } else if self.match_token(TokenType::Column) {
            let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
            let old_name = self.expect_identifier_or_safe_keyword_with_quoted()?;

            self.expect_token(TokenType::To)?;
            let new_name = self.expect_identifier_or_safe_keyword_with_quoted()?;

            Ok(AlterTableAction::RenameColumn {
                old_name,
                new_name,
                if_exists,
            })
        } else if self.match_token(TokenType::To) {
            let new_name = self.parse_table_ref()?;
            Ok(AlterTableAction::RenameTable(new_name))
        } else if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
            let first_name = self.expect_identifier_with_quoted()?;
            if self.match_token(TokenType::To) {
                let new_name = self.expect_identifier_with_quoted()?;
                Ok(AlterTableAction::RenameColumn {
                    old_name: first_name,
                    new_name,
                    if_exists: false,
                })
            } else {
                Ok(AlterTableAction::RenameTable(TableRef::new(
                    first_name.name,
                )))
            }
        } else {
            Err(self.parse_error("Expected COLUMN or TO after RENAME"))
        }
    }

    fn parse_alter_alter_action(&mut self) -> Result<AlterTableAction> {
        if self.match_token(TokenType::Index) {
            let name = self.expect_identifier_with_quoted()?;
            let visible = if self.match_identifier("VISIBLE") {
                true
            } else if self.match_identifier("INVISIBLE") {
                false
            } else {
                return Err(
                    self.parse_error("Expected VISIBLE or INVISIBLE after ALTER INDEX name")
                );
            };
            Ok(AlterTableAction::AlterIndex { name, visible })
        } else if self.check_identifier("SORTKEY") {
            self.skip();
            if self.match_texts(&["AUTO", "NONE"]) {
                let style = self.previous_text().to_ascii_uppercase();
                Ok(AlterTableAction::AlterSortKey {
                    this: Some(style),
                    expressions: Vec::new(),
                    compound: false,
                })
            } else if self.check(TokenType::LParen) {
                let wrapped = self.parse_wrapped_id_vars()?;
                let expressions = if let Some(Expression::Tuple(t)) = wrapped {
                    t.expressions
                } else {
                    Vec::new()
                };
                Ok(AlterTableAction::AlterSortKey {
                    this: None,
                    expressions,
                    compound: false,
                })
            } else {
                Err(self.parse_error("Expected AUTO, NONE, or (columns) after SORTKEY"))
            }
        } else if self.check_identifier("COMPOUND") {
            self.skip();
            if !self.match_identifier("SORTKEY") {
                return Err(self.parse_error("Expected SORTKEY after COMPOUND"));
            }
            if self.check(TokenType::LParen) {
                let wrapped = self.parse_wrapped_id_vars()?;
                let expressions = if let Some(Expression::Tuple(t)) = wrapped {
                    t.expressions
                } else {
                    Vec::new()
                };
                Ok(AlterTableAction::AlterSortKey {
                    this: None,
                    expressions,
                    compound: true,
                })
            } else {
                Err(self.parse_error("Expected (columns) after COMPOUND SORTKEY"))
            }
        } else if self.check_identifier("DISTSTYLE") {
            self.skip();
            if self.match_texts(&["ALL", "EVEN", "AUTO"]) {
                let style = self.previous_text().to_ascii_uppercase();
                Ok(AlterTableAction::AlterDistStyle {
                    style,
                    distkey: None,
                })
            } else if self.match_token(TokenType::Key) || self.match_identifier("KEY") {
                if !self.match_identifier("DISTKEY") {
                    return Err(self.parse_error("Expected DISTKEY after DISTSTYLE KEY"));
                }
                let col = self.expect_identifier_with_quoted()?;
                Ok(AlterTableAction::AlterDistStyle {
                    style: "KEY".to_string(),
                    distkey: Some(col),
                })
            } else {
                Err(self.parse_error("Expected ALL, EVEN, AUTO, or KEY after DISTSTYLE"))
            }
        } else if self.check_identifier("DISTKEY") {
            self.skip();
            let col = self.expect_identifier_with_quoted()?;
            Ok(AlterTableAction::AlterDistStyle {
                style: "KEY".to_string(),
                distkey: Some(col),
            })
        } else {
            self.match_token(TokenType::Column);
            let name = self.expect_identifier_with_quoted()?;
            let action = self.parse_alter_column_action()?;
            Ok(AlterTableAction::AlterColumn {
                name,
                action,
                use_modify_keyword: false,
            })
        }
    }

    fn parse_alter_unset_action(&mut self) -> Result<AlterTableAction> {
        if self.match_keyword("TAG") {
            let mut names: Vec<String> = Vec::new();
            loop {
                let mut name = self.expect_identifier_or_keyword()?;
                while self.match_token(TokenType::Dot) {
                    let next = self.expect_identifier_or_keyword()?;
                    name = format!("{}.{}", name, next);
                }
                names.push(name);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            Ok(AlterTableAction::UnsetTag { names })
        } else if self.peek_nth(1).map(|t| {
            t.token_type != TokenType::Comma
                && t.token_type != TokenType::Semicolon
                && t.token_type != TokenType::Eof
        }) == Some(true)
            && !self.is_at_end()
            && self.peek_nth(1).map(|t| t.token_type != TokenType::Eq) == Some(true)
        {
            let mut tokens: Vec<(String, TokenType)> = vec![("UNSET".to_string(), TokenType::Var)];
            while !self.is_at_end()
                && !self.check(TokenType::Semicolon)
                && !self.check(TokenType::Eof)
            {
                if self.check(TokenType::Comma) {
                    break;
                }
                tokens.push((self.advance_text()?, TokenType::Var));
            }
            Ok(AlterTableAction::Raw {
                sql: self.join_command_tokens(tokens),
            })
        } else {
            let mut properties: Vec<String> = Vec::new();
            loop {
                let name = self.expect_identifier_or_keyword()?;
                properties.push(name);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            Ok(AlterTableAction::UnsetProperty { properties })
        }
    }

    fn parse_alter_partition_maintenance_or_other(&mut self) -> Result<AlterTableAction> {
        if self.check_identifier("REORGANIZE")
            || self.check_identifier("COALESCE")
            || self.check_identifier("EXCHANGE")
            || self.check_identifier("ANALYZE")
            || self.check_identifier("OPTIMIZE")
            || self.check_identifier("REBUILD")
            || self.check_identifier("REPAIR")
            || self.check_identifier("DISCARD")
            || self.check_identifier("IMPORT")
        {
            let keyword = self.advance_text()?;
            let mut tokens: Vec<(String, TokenType)> = vec![(keyword, TokenType::Var)];
            let mut paren_depth = 0i32;
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                if self.check(TokenType::Comma) && paren_depth == 0 {
                    break;
                }
                let token = self.advance()?;
                if token.token_type == TokenType::LParen {
                    paren_depth += 1;
                }
                if token.token_type == TokenType::RParen {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        break;
                    }
                }
                let text = if token.token_type == TokenType::QuotedIdentifier {
                    format!("\"{}\"", token.text)
                } else if token.token_type == TokenType::String {
                    format!("'{}'", token.text)
                } else {
                    token.text.to_string()
                };
                tokens.push((text, token.token_type));
            }
            Ok(AlterTableAction::Raw {
                sql: self.join_command_tokens(tokens),
            })
        } else {
            Err(self.parse_error(format!(
                "Expected ADD, DROP, RENAME, ALTER, SET, UNSET, SWAP, CLUSTER, or REPLACE in ALTER TABLE, got {:?}",
                self.peek().token_type
            )))
        }
    }

    fn parse_alter_set_action(&mut self) -> Result<AlterTableAction> {
        if let Some(action) = self.parse_snowflake_table_set()? {
            return Ok(action);
        }
        if self.check(TokenType::LParen) {
            self.parse_alter_set_options()
        } else if self.match_keyword("TAG") {
            self.parse_alter_set_tag()
        } else if self.check_identifier("LOGGED") {
            self.skip();
            Ok(AlterTableAction::SetAttribute {
                attribute: "LOGGED".to_string(),
            })
        } else if self.check_identifier("UNLOGGED") {
            self.skip();
            Ok(AlterTableAction::SetAttribute {
                attribute: "UNLOGGED".to_string(),
            })
        } else if self.match_identifier("WITHOUT") {
            let what = self.expect_identifier_or_keyword()?;
            Ok(AlterTableAction::SetAttribute {
                attribute: format!("WITHOUT {}", what),
            })
        } else if self.check_identifier("ACCESS") {
            self.skip();
            if !self.match_identifier("METHOD") {
                return Err(self.parse_error("Expected METHOD after ACCESS"));
            }
            let method = self.expect_identifier_or_keyword()?;
            Ok(AlterTableAction::SetAttribute {
                attribute: format!("ACCESS METHOD {}", method),
            })
        } else if self.check_identifier("TABLESPACE") {
            self.skip();
            let name = self.expect_identifier_or_keyword()?;
            Ok(AlterTableAction::SetAttribute {
                attribute: format!("TABLESPACE {}", name),
            })
        } else if self.check_identifier("STAGE_FILE_FORMAT") {
            self.skip();
            let options = self.parse_wrapped_options()?;
            Ok(AlterTableAction::SetStageFileFormat { options })
        } else if self.check_identifier("STAGE_COPY_OPTIONS") {
            self.skip();
            let options = self.parse_wrapped_options()?;
            Ok(AlterTableAction::SetStageCopyOptions { options })
        } else if self.match_token(TokenType::Authorization) {
            let mut auth_text = String::new();
            if self.match_texts(&["ROLE"]) {
                auth_text.push_str("ROLE ");
            }
            let user = self.expect_identifier_or_keyword()?;
            auth_text.push_str(&user);
            Ok(AlterTableAction::SetAttribute {
                attribute: format!("AUTHORIZATION {}", auth_text),
            })
        } else if self.match_identifier("PROPERTIES") {
            self.parse_alter_set_properties()
        } else if self.match_text_seq(&["TABLE", "PROPERTIES"]) {
            self.parse_alter_set_table_properties()
        } else if self.match_text_seq(&["LOCATION"]) {
            let location = self.expect_string()?;
            Ok(AlterTableAction::SetLocation { location })
        } else if self.match_text_seq(&["FILE", "FORMAT"]) {
            let format = self.expect_identifier_or_keyword()?;
            Ok(AlterTableAction::SetFileFormat { format })
        } else if self.peek_nth(1).map(|t| t.token_type) != Some(TokenType::Eq) {
            self.parse_alter_set_raw()
        } else {
            self.parse_alter_set_properties_list()
        }
    }

    fn parse_alter_set_options(&mut self) -> Result<AlterTableAction> {
        self.skip();
        let mut expressions: Vec<Expression> = Vec::new();
        loop {
            if self.check(TokenType::RParen) {
                break;
            }
            if self.check_identifier("SYSTEM_VERSIONING") {
                let expr = self.parse_system_versioning_option()?;
                expressions.push(expr);
            } else if self.check_identifier("DATA_DELETION") {
                let expr = self.parse_data_deletion_option()?;
                expressions.push(expr);
            } else {
                let expr = self.parse_expression()?;
                expressions.push(expr);
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(AlterTableAction::SetOptions { expressions })
    }

    fn parse_alter_set_tag(&mut self) -> Result<AlterTableAction> {
        let mut tags: Vec<(String, Expression)> = Vec::new();
        loop {
            let mut key = self.expect_identifier_or_keyword()?;
            while self.match_token(TokenType::Dot) {
                let next = self.expect_identifier_or_keyword()?;
                key = format!("{}.{}", key, next);
            }
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_primary()?;
            tags.push((key, value));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(AlterTableAction::SetTag { expressions: tags })
    }

    fn parse_alter_set_properties(&mut self) -> Result<AlterTableAction> {
        let mut properties: Vec<(String, Expression)> = Vec::new();
        loop {
            let key = if self.check(TokenType::String) {
                self.expect_string()?
            } else {
                self.expect_identifier_or_keyword()?
            };
            self.expect_token(TokenType::Eq)?;
            let value = if self.match_token(TokenType::Default) {
                Expression::Var(Box::new(
                    sqlquack_syntax::expressions::statements::models::Var {
                        this: "DEFAULT".to_string(),
                    },
                ))
            } else {
                self.parse_expression()?
            };
            properties.push((key, value));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(AlterTableAction::SetProperty { properties })
    }

    fn parse_alter_set_table_properties(&mut self) -> Result<AlterTableAction> {
        self.expect_token(TokenType::LParen)?;
        let mut properties: Vec<(Expression, Expression)> = Vec::new();
        loop {
            if self.check(TokenType::RParen) {
                break;
            }
            let key = self.parse_primary()?;
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_primary()?;
            properties.push((key, value));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(AlterTableAction::SetTableProperties { properties })
    }

    fn parse_alter_set_raw(&mut self) -> Result<AlterTableAction> {
        let mut tokens: Vec<(String, TokenType)> = vec![("SET".to_string(), TokenType::Set)];
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::Comma) {
                break;
            }
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
        Ok(AlterTableAction::Raw {
            sql: self.join_command_tokens(tokens),
        })
    }

    fn parse_alter_set_properties_list(&mut self) -> Result<AlterTableAction> {
        let mut properties: Vec<(String, Expression)> = Vec::new();
        loop {
            let key = self.expect_identifier_or_keyword()?;
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_expression()?;
            properties.push((key, value));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(AlterTableAction::SetProperty { properties })
    }

    fn parse_alter_drop_action(&mut self) -> Result<AlterTableAction> {
        if let Some(action) = self.parse_snowflake_drop_key()? {
            return Ok(action);
        }
        if self.check_identifier("CLUSTERING")
            && self.peek_nth(1).map(|t| t.text.eq_ignore_ascii_case("KEY")) == Some(true)
        {
            self.skip();
            self.skip();
            return Ok(AlterTableAction::Raw {
                sql: "DROP CLUSTERING KEY".to_string(),
            });
        }
        if self.check_identifier("SEARCH")
            && self
                .peek_nth(1)
                .map(|t| t.text.eq_ignore_ascii_case("OPTIMIZATION"))
                == Some(true)
        {
            let mut tokens: Vec<(String, TokenType)> = vec![("DROP".to_string(), TokenType::Drop)];
            let mut paren_depth = 0i32;
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                if self.check(TokenType::Comma) && paren_depth == 0 {
                    break;
                }
                let token = self.advance()?;
                if token.token_type == TokenType::LParen {
                    paren_depth += 1;
                }
                if token.token_type == TokenType::RParen {
                    paren_depth -= 1;
                }
                tokens.push((token.text.to_string(), token.token_type));
            }
            return Ok(AlterTableAction::Raw {
                sql: self.join_command_tokens(tokens),
            });
        }

        if self.check(TokenType::Row)
            && self
                .peek_nth(1)
                .map(|t| t.text.eq_ignore_ascii_case("ACCESS"))
                == Some(true)
            || self.check_identifier("ALL")
                && self.peek_nth(1).map(|t| t.text.eq_ignore_ascii_case("ROW")) == Some(true)
        {
            let mut tokens: Vec<(String, TokenType)> = vec![("DROP".to_string(), TokenType::Drop)];
            let mut paren_depth = 0i32;
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                if self.check(TokenType::Comma) && paren_depth == 0 {
                    break;
                }
                let token = self.advance()?;
                if token.token_type == TokenType::LParen {
                    paren_depth += 1;
                }
                if token.token_type == TokenType::RParen {
                    paren_depth -= 1;
                }
                tokens.push((token.text.to_string(), token.token_type));
            }
            return Ok(AlterTableAction::Raw {
                sql: self.join_command_tokens(tokens),
            });
        }

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);

        if self.match_token(TokenType::PrimaryKey)
            || self.match_identifier("PRIMARY") && self.match_token(TokenType::Key)
        {
            self.match_token(TokenType::Key);
            let trailing_if_exists =
                if_exists || self.match_keywords(&[TokenType::If, TokenType::Exists]);
            let sql = if trailing_if_exists {
                "DROP PRIMARY KEY IF EXISTS".to_string()
            } else {
                "DROP PRIMARY KEY".to_string()
            };
            Ok(AlterTableAction::Raw { sql })
        } else if self.match_token(TokenType::Index) || self.match_token(TokenType::Key) {
            let name = self.expect_identifier_with_quoted()?;
            Ok(AlterTableAction::Raw {
                sql: format!("DROP INDEX {}", self.format_identifier_for_raw_sql(&name)),
            })
        } else if self.match_token(TokenType::Partition) {
            self.parse_alter_drop_partitions(if_exists)
        } else if self.match_token(TokenType::Column) {
            let if_exists = if_exists || self.match_keywords(&[TokenType::If, TokenType::Exists]);
            let name = { self.expect_identifier_with_quoted()? };

            let cascade = self.match_token(TokenType::Cascade);
            Ok(AlterTableAction::DropColumn {
                name,
                if_exists,
                cascade,
            })
        } else if self.match_token(TokenType::Constraint) {
            let name = self.expect_identifier_with_quoted()?;
            Ok(AlterTableAction::DropConstraint { name, if_exists })
        } else if self.match_keywords(&[TokenType::ForeignKey, TokenType::Key]) {
            let name = self.expect_identifier_with_quoted()?;
            Ok(AlterTableAction::DropForeignKey { name })
        } else if self.check_identifier("COLUMNS") && self.check_next(TokenType::LParen) {
            self.skip();
            self.expect_token(TokenType::LParen)?;
            let mut names: Vec<Identifier> = Vec::new();
            loop {
                let name = self.expect_identifier_with_quoted()?;
                names.push(name);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            Ok(AlterTableAction::DropColumns { names })
        } else {
            let name = self.expect_identifier_with_quoted()?;

            let cascade = self.match_token(TokenType::Cascade);
            Ok(AlterTableAction::DropColumn {
                name,
                if_exists,
                cascade,
            })
        }
    }

    fn parse_alter_drop_partitions(&mut self, if_exists: bool) -> Result<AlterTableAction> {
        let mut partitions: Vec<Vec<(Identifier, Expression)>> = Vec::new();
        loop {
            if self.check(TokenType::LParen) {
                {
                    self.skip();
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
                    partitions.push(parts);
                }
            } else if self.match_text_seq(&["ALL"]) {
                partitions.push(vec![(
                    Identifier::new("ALL".to_string()),
                    Expression::Boolean(BooleanLiteral { value: true }),
                )]);
            } else if self.match_text_seq(&["ID"]) {
                let id_val = self.parse_expression()?;
                partitions.push(vec![(Identifier::new("ID".to_string()), id_val)]);
            } else {
                if self.applies_snowflake_context_rules() && self.check_next(TokenType::String) {
                    return Err(self.parse_error("Expected a partition, not a typed string"));
                }
                let expr = self.parse_expression()?;
                partitions.push(vec![(Identifier::new("__expr__".to_string()), expr)]);
            }
            if self.match_token(TokenType::Comma) {
                if !self.match_token(TokenType::Partition) {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(AlterTableAction::DropPartition {
            partitions,
            if_exists,
        })
    }

    fn parse_alter_add_action(&mut self) -> Result<AlterTableAction> {
        if self.check_identifier("SEARCH")
            && self
                .peek_nth(1)
                .map(|t| t.text.eq_ignore_ascii_case("OPTIMIZATION"))
                == Some(true)
        {
            let mut tokens: Vec<(String, TokenType)> = vec![("ADD".to_string(), TokenType::Add)];
            let mut paren_depth = 0i32;
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                if self.check(TokenType::Comma) && paren_depth == 0 {
                    break;
                }
                let token = self.advance()?;
                if token.token_type == TokenType::LParen {
                    paren_depth += 1;
                }
                if token.token_type == TokenType::RParen {
                    paren_depth -= 1;
                }
                tokens.push((token.text.to_string(), token.token_type));
            }
            return Ok(AlterTableAction::Raw {
                sql: self.join_command_tokens(tokens),
            });
        }
        if self.check(TokenType::Row)
            && self
                .peek_nth(1)
                .map(|t| t.text.eq_ignore_ascii_case("ACCESS"))
                == Some(true)
        {
            let mut tokens: Vec<(String, TokenType)> = vec![("ADD".to_string(), TokenType::Add)];
            let mut paren_depth = 0i32;
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                if self.check(TokenType::Comma) && paren_depth == 0 {
                    break;
                }
                let token = self.advance()?;
                if token.token_type == TokenType::LParen {
                    paren_depth += 1;
                }
                if token.token_type == TokenType::RParen {
                    paren_depth -= 1;
                }
                tokens.push((token.text.to_string(), token.token_type));
            }
            return Ok(AlterTableAction::Raw {
                sql: self.join_command_tokens(tokens),
            });
        }
        if let Some(action) = self.parse_snowflake_added_constraint()? {
            Ok(action)
        } else if self.match_token(TokenType::Constraint) {
            let name = Some(self.expect_identifier_with_quoted()?);
            let constraint = self.parse_constraint_definition(name)?;
            Ok(AlterTableAction::AddConstraint(constraint))
        } else if self.check(TokenType::PrimaryKey)
            || self.check(TokenType::ForeignKey)
            || self.check(TokenType::Check)
        {
            let constraint = self.parse_table_constraint()?;
            Ok(AlterTableAction::AddConstraint(constraint))
        } else if self.check(TokenType::Index)
            || self.check(TokenType::Key)
            || self.check(TokenType::Unique)
            || self.check_identifier("FULLTEXT")
            || self.check_identifier("SPATIAL")
        {
            self.parse_alter_add_index()
        } else if self.match_identifier("COLUMNS") {
            self.expect_token(TokenType::LParen)?;
            let mut columns: Vec<ColumnDef> = Vec::new();
            loop {
                let col_def = self.parse_column_def()?;
                columns.push(col_def);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            let cascade = self.match_token(TokenType::Cascade);
            Ok(AlterTableAction::AddColumns { columns, cascade })
        } else if self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]) {
            self.parse_alter_add_if_not_exists()
        } else if self.check(TokenType::Partition) {
            self.parse_alter_add_partition()
        } else {
            self.parse_alter_add_column()
        }
    }

    fn parse_alter_add_index(&mut self) -> Result<AlterTableAction> {
        let kind = if self.match_token(TokenType::Unique) {
            Some("UNIQUE".to_string())
        } else if self.match_identifier("FULLTEXT") {
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

        let name = if !self.check(TokenType::LParen) && !self.check(TokenType::Using) {
            Some(self.expect_identifier_with_quoted()?)
        } else {
            None
        };

        self.expect_token(TokenType::LParen)?;
        let key_parts = self.parse_mysql_index_key_parts()?;
        self.expect_token(TokenType::RParen)?;
        let (columns, key_parts) = split_mysql_table_index_key_parts(key_parts);

        let modifiers = self.parse_constraint_modifiers()?;

        Ok(AlterTableAction::AddConstraint(TableConstraint::Index {
            name,
            columns,
            key_parts,
            kind,
            modifiers,
            use_key_keyword,
            expression: None,
            index_type: None,
            granularity: None,
            include_columns: Vec::new(),
        }))
    }

    fn parse_alter_add_if_not_exists(&mut self) -> Result<AlterTableAction> {
        if self.match_token(TokenType::Partition) {
            self.expect_token(TokenType::LParen)?;
            let mut partition_exprs: Vec<Expression> = Vec::new();
            loop {
                if let Some(expr) = self.parse_conjunction()? {
                    partition_exprs.push(expr);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            let partition = Expression::Partition(Box::new(
                sqlquack_syntax::expressions::tables::models::Partition {
                    expressions: partition_exprs,
                    subpartition: false,
                },
            ));
            let location = if self.match_text_seq(&["LOCATION"]) {
                self.parse_property()?
            } else {
                None
            };
            Ok(AlterTableAction::AddPartition {
                partition,
                if_not_exists: true,
                location,
            })
        } else {
            let col_def = self.parse_added_column_def()?;
            Ok(AlterTableAction::AddColumn {
                column: col_def,
                if_not_exists: true,
                position: None,
            })
        }
    }

    fn parse_alter_add_partition(&mut self) -> Result<AlterTableAction> {
        if matches!(
            self.config.dialect,
            Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
        ) {
            return Err(self.parse_error("Snowflake adds partitions to external tables only"));
        }
        self.skip();
        self.expect_token(TokenType::LParen)?;
        let mut partition_exprs: Vec<Expression> = Vec::new();
        loop {
            if let Some(expr) = self.parse_conjunction()? {
                partition_exprs.push(expr);
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        let partition = Expression::Partition(Box::new(
            sqlquack_syntax::expressions::tables::models::Partition {
                expressions: partition_exprs,
                subpartition: false,
            },
        ));
        let location = if self.match_text_seq(&["LOCATION"]) {
            Some(self.parse_primary()?)
        } else {
            None
        };
        Ok(AlterTableAction::AddPartition {
            partition,
            if_not_exists: false,
            location,
        })
    }

    fn parse_alter_add_column(&mut self) -> Result<AlterTableAction> {
        let has_column_keyword = self.match_token(TokenType::Column);

        if !has_column_keyword && !self.is_snowflake() && self.check(TokenType::LParen) {
            self.skip();
            let mut columns: Vec<ColumnDef> = Vec::new();
            loop {
                let col_def = self.parse_column_def()?;
                columns.push(col_def);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            Ok(AlterTableAction::AddColumns {
                columns,
                cascade: false,
            })
        } else {
            let if_not_exists =
                self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
            let col_def = self.parse_added_column_def()?;
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
            Ok(AlterTableAction::AddColumn {
                column: col_def,
                if_not_exists,
                position,
            })
        }
    }

    /// Parses TSQL SYSTEM_VERSIONING = OFF|ON[(...)] in ALTER TABLE SET.
    pub(crate) fn parse_system_versioning_option(&mut self) -> Result<Expression> {
        self.skip();
        self.expect_token(TokenType::Eq)?;

        let mut prop = WithSystemVersioningProperty {
            on: None,
            this: None,
            data_consistency: None,
            retention_period: None,
            with_: None,
        };

        if self.match_identifier("OFF") {
            return Ok(Expression::WithSystemVersioningProperty(Box::new(prop)));
        }

        if self.match_token(TokenType::On) || self.match_identifier("ON") {
            prop.on = Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            })));
        }

        if self.match_token(TokenType::LParen) {
            loop {
                if self.check(TokenType::RParen) {
                    break;
                }
                if self.match_identifier("HISTORY_TABLE") {
                    self.expect_token(TokenType::Eq)?;
                    let table = self.parse_table_ref()?;
                    prop.this = Some(Box::new(Expression::Table(Box::new(table))));
                } else if self.match_identifier("DATA_CONSISTENCY_CHECK") {
                    self.expect_token(TokenType::Eq)?;
                    let val = self.expect_identifier_or_keyword()?;
                    prop.data_consistency = Some(Box::new(Expression::Identifier(
                        Identifier::new(val.to_ascii_uppercase()),
                    )));
                } else if self.match_identifier("HISTORY_RETENTION_PERIOD") {
                    self.expect_token(TokenType::Eq)?;
                    if let Some(rp) = self.parse_retention_period()? {
                        prop.retention_period = Some(Box::new(rp));
                    }
                } else {
                    self.skip();
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }

        Ok(Expression::WithSystemVersioningProperty(Box::new(prop)))
    }

    /// Parses TSQL DATA_DELETION = ON|OFF[(...)] in ALTER TABLE SET.
    pub(crate) fn parse_data_deletion_option(&mut self) -> Result<Expression> {
        self.skip();
        self.expect_token(TokenType::Eq)?;

        let on = self.match_identifier("ON") || self.match_token(TokenType::On);
        if !on {
            self.match_identifier("OFF");
        }

        let on_expr = Box::new(Expression::Boolean(BooleanLiteral { value: on }));
        let mut filter_column = None;
        let mut retention_period = None;

        if self.match_token(TokenType::LParen) {
            loop {
                if self.check(TokenType::RParen) || self.is_at_end() {
                    break;
                }
                if self.match_identifier("FILTER_COLUMN") {
                    self.expect_token(TokenType::Eq)?;
                    let col = self.expect_identifier_or_keyword()?;
                    filter_column = Some(Box::new(Self::parsed_column(Column {
                        name: Identifier::new(col),
                        table: None,
                        join_mark: false,
                        trailing_comments: Vec::new(),
                        span: None,
                        inferred_type: None,
                    })));
                } else if self.match_identifier("RETENTION_PERIOD") {
                    self.expect_token(TokenType::Eq)?;
                    if let Some(rp) = self.parse_retention_period()? {
                        retention_period = Some(Box::new(rp));
                    }
                } else {
                    self.skip();
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }

        Ok(Expression::DataDeletionProperty(Box::new(
            DataDeletionProperty {
                on: on_expr,
                filter_column,
                retention_period,
            },
        )))
    }

    /// A Snowflake `MODIFY [ COLUMN ] <column> <action>` that ends the action, or None (position unchanged).
    pub(crate) fn parse_snowflake_modify_column(&mut self) -> Result<Option<AlterTableAction>> {
        if !self.is_snowflake() || !self.check_identifier("MODIFY") {
            return Ok(None);
        }
        let start = self.current;
        self.skip();
        self.match_token(TokenType::Column);
        let typed = self.parse_simple_name().and_then(|name| {
            Ok(AlterTableAction::AlterColumn {
                name,
                action: self.parse_alter_column_action()?,
                use_modify_keyword: true,
            })
        });
        let ends =
            self.is_at_end() || self.check(TokenType::Comma) || self.check(TokenType::Semicolon);
        match typed {
            Ok(action) if ends => Ok(Some(action)),
            _ => {
                self.current = start;
                Ok(None)
            }
        }
    }

    /// A Snowflake `ALTER ( [ COLUMN ] <column> <action> [ , ... ] )` list, or None (position unchanged) for any other action.
    pub(crate) fn parse_snowflake_alter_column_list(
        &mut self,
    ) -> Result<Option<Vec<AlterTableAction>>> {
        if !self.is_snowflake()
            || !self.check(TokenType::Alter)
            || !self.check_next(TokenType::LParen)
        {
            return Ok(None);
        }
        self.current += 2;
        let mut actions: Vec<AlterTableAction> = Vec::new();
        loop {
            self.match_token(TokenType::Column);
            let name = self.parse_simple_name()?;
            let action = self.parse_alter_column_action()?;
            actions.push(AlterTableAction::AlterColumn {
                name,
                action,
                use_modify_keyword: false,
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Some(actions))
    }

    /// Parse ALTER COLUMN action
    pub(crate) fn parse_alter_column_action(&mut self) -> Result<AlterColumnAction> {
        if self.is_snowflake() {
            if self.match_token(TokenType::Default) {
                return Ok(AlterColumnAction::SetDefault(self.parse_expression()?));
            }
            if self.match_text_seq(&["UNSET", "COMMENT"]) {
                return Ok(AlterColumnAction::UnsetComment);
            }
            if self.match_keywords(&[TokenType::Not, TokenType::Null]) {
                return Ok(AlterColumnAction::SetNotNull);
            }
            if self.check(TokenType::Set) && self.check_next(TokenType::Default) {
                self.current += 2;
                return Ok(AlterColumnAction::SetDefault(self.parse_expression()?));
            }
            if self.match_text_seq(&["SET", "WRITE", "DEFAULT"])
                || self.match_text_seq(&["WRITE", "DEFAULT"])
            {
                return Ok(AlterColumnAction::SetWriteDefault(self.parse_expression()?));
            }
            if self.match_text_seq(&["DROP", "WRITE", "DEFAULT"]) {
                return Ok(AlterColumnAction::DropWriteDefault);
            }
            if self.check_identifier("UNSET") {
                return Err(self.parse_error("Expected COMMENT, a policy or TAG after UNSET"));
            }
            if self.check(TokenType::Comment) && !self.check_next(TokenType::String) {
                return Err(self.parse_error("Expected a string after COMMENT"));
            }
        }
        if self.match_token(TokenType::Set) {
            if self.match_keywords(&[TokenType::Not, TokenType::Null]) {
                Ok(AlterColumnAction::SetNotNull)
            } else if self.match_token(TokenType::Default) {
                let expr = self.parse_primary()?;
                Ok(AlterColumnAction::SetDefault(expr))
            } else if self.match_identifier("DATA") {
                let typed = self.match_token(TokenType::Type) || self.match_identifier("TYPE");
                if !typed && self.is_snowflake() {
                    return Err(self.parse_error("Expected TYPE after SET DATA"));
                }
                let data_type = self.parse_data_type()?;
                let collate = if self.match_token(TokenType::Collate) {
                    if self.check(TokenType::String) {
                        let text = self.advance_text()?;
                        Some(format!("'{}'", text))
                    } else {
                        Some(self.expect_identifier_or_keyword()?)
                    }
                } else {
                    None
                };
                let using = if self.match_token(TokenType::Using) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                let rename = match self.is_snowflake() && self.match_text_seq(&["RENAME"]) {
                    true if self.match_text_seq(&["FIELDS"]) => Some("RENAME FIELDS".to_string()),
                    true => Some("RENAME".to_string()),
                    false => None,
                };
                Ok(AlterColumnAction::SetDataType {
                    data_type,
                    using,
                    collate,
                    rename,
                })
            } else if self.match_identifier("VISIBLE") {
                Ok(AlterColumnAction::SetVisible)
            } else if self.match_identifier("INVISIBLE") {
                Ok(AlterColumnAction::SetInvisible)
            } else {
                Err(self.parse_error("Expected NOT NULL, DEFAULT, VISIBLE, or INVISIBLE after SET"))
            }
        } else if self.match_token(TokenType::Drop) {
            if self.match_keywords(&[TokenType::Not, TokenType::Null]) {
                Ok(AlterColumnAction::DropNotNull)
            } else if self.match_token(TokenType::Default) {
                Ok(AlterColumnAction::DropDefault)
            } else {
                Err(self.parse_error("Expected NOT NULL or DEFAULT after DROP"))
            }
        } else if self.match_token(TokenType::Comment) {
            let comment = self.expect_string()?;
            Ok(AlterColumnAction::Comment(comment))
        } else if self.match_token(TokenType::Type)
            || self.match_identifier("TYPE")
            || self.is_identifier_token()
            || self.is_snowflake() && self.check_keyword() && !self.check(TokenType::Default)
        {
            let data_type = self.parse_data_type()?;
            let collate = if self.match_token(TokenType::Collate) {
                if self.check(TokenType::String) {
                    Some(self.advance_text()?)
                } else {
                    Some(self.expect_identifier_or_keyword()?)
                }
            } else {
                None
            };
            let using = if self.match_token(TokenType::Using) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            Ok(AlterColumnAction::SetDataType {
                data_type,
                using,
                collate,
                rename: None,
            })
        } else {
            Err(self.parse_error("Expected SET, DROP, or TYPE in ALTER COLUMN"))
        }
    }

    /// Parse TRUNCATE statement
    pub(crate) fn parse_truncate(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Truncate)?;

        let target = if self.match_token(TokenType::Database) {
            TruncateTarget::Database
        } else {
            self.match_token(TokenType::Temporary);
            self.match_token(TokenType::Table);
            TruncateTarget::Table
        };

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);

        let has_only = self.match_token(TokenType::Only);
        let error_table = self.is_snowflake()
            && self.check_identifier("ERROR_TABLE")
            && self.check_next(TokenType::LParen);
        if error_table {
            self.skip();
            self.skip();
        }
        let mut table = self.parse_object_name_ref()?;
        if error_table {
            self.expect_token(TokenType::RParen)?;
        }
        if has_only {
            table.only = true;
        }

        let first_star = self.match_token(TokenType::Star);

        if self.check(TokenType::With) && self.check_next(TokenType::LParen) {
            if let Some(hint_expr) = self.parse_truncate_table_hints()? {
                match hint_expr {
                    Expression::Tuple(tuple) => {
                        table.hints = tuple.expressions;
                    }
                    other => {
                        table.hints = vec![other];
                    }
                }
            }
        }

        let on_cluster = self.parse_on_cluster_clause()?;

        let mut extra_tables: Vec<TruncateTableEntry> = Vec::new();
        if first_star {
            extra_tables.push(TruncateTableEntry {
                table: table.clone(),
                star: true,
            });
        }
        while self.match_token(TokenType::Comma) {
            let extra_only = self.match_token(TokenType::Only);
            let mut extra_table = self.parse_table_ref()?;
            if extra_only {
                extra_table.only = true;
            }
            let extra_star = self.match_token(TokenType::Star);
            extra_tables.push(TruncateTableEntry {
                table: extra_table,
                star: extra_star,
            });
        }

        let identity = if self.match_token(TokenType::Restart) {
            self.match_token(TokenType::Identity);
            Some(TruncateIdentity::Restart)
        } else if self.match_identifier("CONTINUE") {
            self.match_token(TokenType::Identity);
            Some(TruncateIdentity::Continue)
        } else {
            None
        };

        let cascade = self.match_token(TokenType::Cascade);
        let restrict = if !cascade {
            self.match_token(TokenType::Restrict)
        } else {
            false
        };

        let partition = self.parse_partition()?;

        Ok(Expression::Truncate(Box::new(Truncate {
            target,
            if_exists,
            table,
            on_cluster,
            cascade,
            extra_tables,
            identity,
            restrict,
            partition: partition.map(Box::new),
            error_table,
        })))
    }

    /// Parse VALUES table constructor: VALUES (1, 'a'), (2, 'b')
    pub(crate) fn parse_values(&mut self) -> Result<Expression> {
        self.parse_values_with_set_operations(true)
    }

    pub(crate) fn parse_values_with_set_operations(
        &mut self,
        parse_set_operations: bool,
    ) -> Result<Expression> {
        self.expect_token(TokenType::Values)?;

        let mut expressions: Vec<Tuple> = Vec::new();

        if !self.check(TokenType::LParen) {
            loop {
                let val = self.parse_expression()?;
                expressions.push(Tuple {
                    expressions: vec![val],
                });
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        } else {
            loop {
                self.expect_token(TokenType::LParen)?;
                let row_values = self.parse_values_expression_list()?;
                self.expect_token(TokenType::RParen)?;

                expressions.push(Tuple {
                    expressions: row_values,
                });

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        let (alias, column_aliases) = if self.match_token(TokenType::As) {
            let alias_name = self.expect_identifier()?;
            let alias = Some(Identifier::new(alias_name));

            let col_aliases = if self.match_token(TokenType::LParen) {
                let aliases = self.parse_identifier_list()?;
                self.expect_token(TokenType::RParen)?;
                aliases
            } else {
                Vec::new()
            };
            (alias, col_aliases)
        } else if self.check(TokenType::Var) && !self.check_keyword() {
            let alias_name = self.advance_text()?;
            let alias = Some(Identifier::new(alias_name));
            let col_aliases = if self.match_token(TokenType::LParen) {
                let aliases = self.parse_identifier_list()?;
                self.expect_token(TokenType::RParen)?;
                aliases
            } else {
                Vec::new()
            };
            (alias, col_aliases)
        } else {
            (None, Vec::new())
        };

        let values_expr = Expression::Values(Box::new(Values {
            expressions,
            alias,
            column_aliases,
        }));

        if parse_set_operations {
            self.parse_set_operation(values_expr)
        } else {
            Ok(values_expr)
        }
    }

    /// Parse USE statement: USE db, USE DATABASE x, USE SCHEMA x.y, USE ROLE x, etc.
    pub(crate) fn parse_use(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Use)?;

        if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("SECONDARY") {
            self.skip();
            if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("ROLES") {
                self.skip();
                let snowflake = self.is_snowflake();
                let mut roles: Vec<String> = Vec::new();
                let mut listed: Vec<Identifier> = Vec::new();
                loop {
                    if snowflake && self.check(TokenType::QuotedIdentifier) {
                        listed.push(Identifier::quoted(self.peek_text()));
                        roles.push(self.parse_quoted_role()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    } else if self.check(TokenType::Var)
                        || self.check(TokenType::All)
                        || self.check(TokenType::Identifier)
                    {
                        let role = self.advance_text()?;
                        listed.push(Identifier::new(role.clone()));
                        roles.push(role);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if snowflake && roles.is_empty() {
                    return Err(self.parse_error("Expected ALL, NONE or a role after ROLES"));
                }
                let name = if roles.is_empty() {
                    "ALL".to_string()
                } else {
                    roles.join(", ")
                };
                let keyword = matches!(roles.as_slice(), [role] if role.eq_ignore_ascii_case("ALL") || role.eq_ignore_ascii_case("NONE"));
                return Ok(Expression::Use(Box::new(Use {
                    kind: Some(UseKind::SecondaryRoles),
                    this: Identifier::new(name),
                    roles: match snowflake && !keyword {
                        true => listed,
                        false => Vec::new(),
                    },
                    parts: Vec::new(),
                })));
            }
        }

        let kind_is_name = self.applies_snowflake_context_rules()
            && matches!(
                self.tokens
                    .get(self.current + 1)
                    .map(|token| token.token_type),
                None | Some(TokenType::Semicolon | TokenType::Dot)
            );
        let kind = if kind_is_name {
            None
        } else if self.match_token(TokenType::Database) {
            Some(UseKind::Database)
        } else if self.match_token(TokenType::Schema) {
            Some(UseKind::Schema)
        } else if self.match_token(TokenType::Warehouse) {
            Some(UseKind::Warehouse)
        } else if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("ROLE") {
            self.skip();
            Some(UseKind::Role)
        } else if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("CATALOG") {
            self.skip();
            Some(UseKind::Catalog)
        } else {
            None
        };

        let mut ident = self.expect_identifier_or_keyword_with_quoted()?;
        self.reject_reserved_name_part(&ident)?;
        let mut parts: Vec<Identifier> = Vec::new();

        if self.match_token(TokenType::Dot) {
            let second_part = self.expect_identifier_or_keyword_with_quoted()?;
            self.reject_reserved_name_part(&second_part)?;
            if self.applies_snowflake_context_rules() {
                parts = vec![ident.clone(), second_part.clone()];
            }
            ident.name = format!("{}.{}", ident.name, second_part.name);
        }

        Ok(Expression::Use(Box::new(Use {
            kind,
            this: ident,
            roles: Vec::new(),
            parts,
        })))
    }

    /// Parses BigQuery EXPORT DATA [WITH CONNECTION c] OPTIONS (...) AS SELECT ...
    pub(crate) fn parse_export_data(&mut self) -> Result<Expression> {
        self.skip();

        if !self.match_identifier("DATA") {
            return Err(self.parse_error("Expected DATA after EXPORT"));
        }

        let connection = if self.match_text_seq(&["WITH", "CONNECTION"]) {
            let first = self.expect_identifier()?;
            let connection_name = if self.match_token(TokenType::Dot) {
                let second = self.expect_identifier()?;
                if self.match_token(TokenType::Dot) {
                    let third = self.expect_identifier()?;
                    format!("{}.{}.{}", first, second, third)
                } else {
                    format!("{}.{}", first, second)
                }
            } else {
                first
            };
            Some(Box::new(Expression::Identifier(Identifier::new(
                connection_name,
            ))))
        } else {
            None
        };

        let options = if self.match_identifier("OPTIONS") {
            self.parse_options_list()?
        } else {
            Vec::new()
        };

        self.expect_token(TokenType::As)?;

        let query = self.parse_statement()?;

        Ok(Expression::Export(Box::new(Export {
            this: Box::new(query),
            connection,
            options,
        })))
    }

    /// Parses Spark CACHE [LAZY] TABLE name [OPTIONS(...)] [AS query].
    pub(crate) fn parse_cache(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Cache)?;

        let lazy = self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("LAZY");
        if lazy {
            self.skip();
        }

        self.expect_token(TokenType::Table)?;
        let table = self.parse_unquoted_identifier(Self::expect_identifier)?;

        let options = if self.check(TokenType::Var)
            && self.peek_text().eq_ignore_ascii_case("OPTIONS")
        {
            self.skip();
            self.expect_token(TokenType::LParen)?;
            let mut opts: Vec<(Expression, Expression)> = Vec::new();
            loop {
                let key = if self.check(TokenType::NationalString) {
                    let token = self.advance()?;
                    Expression::Literal(Box::new(Literal::NationalString(token.text)))
                } else if self.check(TokenType::String) {
                    let token = self.advance()?;
                    Expression::Literal(Box::new(Literal::String(token.text)))
                } else {
                    Expression::Identifier(self.parse_unquoted_identifier(Self::expect_identifier)?)
                };
                let _ = self.match_token(TokenType::Eq);
                let value = self.parse_expression()?;
                opts.push((key, value));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            opts
        } else {
            Vec::new()
        };

        let query = if self.match_token(TokenType::As)
            || self.check(TokenType::Select)
            || self.check(TokenType::With)
        {
            Some(self.parse_statement()?)
        } else {
            None
        };

        Ok(Expression::Cache(Box::new(Cache {
            table,
            lazy,
            options,
            query,
        })))
    }

    /// Parses Spark UNCACHE TABLE [IF EXISTS] name.
    pub(crate) fn parse_uncache(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Uncache)?;
        self.expect_token(TokenType::Table)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let table = self.parse_unquoted_identifier(Self::expect_identifier)?;

        Ok(Expression::Uncache(Box::new(Uncache { table, if_exists })))
    }

    /// Parses Hive LOAD DATA [LOCAL] INPATH 'path' [OVERWRITE] INTO TABLE t [...].
    pub(crate) fn parse_load_data(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Load)?;

        let data_token = self.advance()?;
        if !data_token.text.eq_ignore_ascii_case("DATA") {
            return Err(self.parse_error("Expected DATA after LOAD"));
        }

        let local = self.match_token(TokenType::Local);

        self.expect_token(TokenType::Inpath)?;

        let inpath = if self.check(TokenType::String) {
            self.advance_text()?
        } else {
            return Err(self.parse_error("Expected string literal after INPATH"));
        };

        let overwrite = self.match_token(TokenType::Overwrite);

        self.expect_token(TokenType::Into)?;
        self.expect_token(TokenType::Table)?;

        let table = Expression::Table(Box::new(self.parse_table_ref()?));

        let partition = if self.match_token(TokenType::Partition) {
            self.expect_token(TokenType::LParen)?;
            let mut partitions: Vec<(Identifier, Expression)> = Vec::new();
            loop {
                let col = self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?;
                self.expect_token(TokenType::Eq)?;
                let val = self.parse_expression()?;
                partitions.push((col, val));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            partitions
        } else {
            Vec::new()
        };

        let input_format = if self.match_token(TokenType::InputFormat) {
            if self.check(TokenType::String) {
                Some(self.advance_text()?)
            } else {
                return Err(self.parse_error("Expected string literal after INPUTFORMAT"));
            }
        } else {
            None
        };

        let serde = if self.match_token(TokenType::Serde) {
            if self.check(TokenType::String) {
                Some(self.advance_text()?)
            } else {
                return Err(self.parse_error("Expected string literal after SERDE"));
            }
        } else {
            None
        };

        Ok(Expression::LoadData(Box::new(LoadData {
            local,
            inpath,
            overwrite,
            table,
            partition,
            input_format,
            serde,
        })))
    }

    /// Parses SQLite PRAGMA [schema.]name [= value | (args)].
    pub(crate) fn parse_pragma(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Pragma)?;

        let first_name = self.expect_identifier_or_keyword()?;

        let (schema, name) = if self.match_token(TokenType::Dot) {
            let pragma_name = self.expect_identifier_or_keyword()?;
            (
                Some(Identifier::new(first_name)),
                Identifier::new(pragma_name),
            )
        } else {
            (None, Identifier::new(first_name))
        };

        let (value, args, use_assignment_syntax) = if self.match_token(TokenType::Eq) {
            let val = if self.match_token(TokenType::On) || self.match_identifier("ON") {
                Expression::Var(Box::new(Var {
                    this: "on".to_string(),
                }))
            } else if self.match_identifier("OFF") {
                Expression::Var(Box::new(Var {
                    this: "off".to_string(),
                }))
            } else {
                self.parse_expression()?
            };
            (Some(val), Vec::new(), true)
        } else if self.match_token(TokenType::LParen) {
            let mut arguments: Vec<Expression> = Vec::new();
            if !self.check(TokenType::RParen) {
                loop {
                    let arg = if self.match_token(TokenType::On) || self.match_identifier("ON") {
                        Expression::Var(Box::new(Var {
                            this: "on".to_string(),
                        }))
                    } else if self.match_identifier("OFF") {
                        Expression::Var(Box::new(Var {
                            this: "off".to_string(),
                        }))
                    } else {
                        self.parse_expression()?
                    };
                    arguments.push(arg);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
            (None, arguments, false)
        } else {
            (None, Vec::new(), false)
        };

        Ok(Expression::Pragma(Box::new(Pragma {
            schema,
            name,
            value,
            args,
            use_assignment_syntax,
        })))
    }

    pub(crate) fn parse_tsql_try_catch(&mut self) -> Result<Expression> {
        self.skip();
        let try_body = self.parse_tsql_block_body("TRY")?;
        let catch_body = if self.check(TokenType::Begin)
            && self.current + 1 < self.tokens.len()
            && self.tokens[self.current + 1]
                .text
                .eq_ignore_ascii_case("CATCH")
        {
            self.skip();
            self.skip();
            Some(self.parse_tsql_block_body("CATCH")?)
        } else {
            None
        };

        Ok(Expression::TryCatch(Box::new(TryCatch {
            try_body,
            catch_body,
        })))
    }

    pub(crate) fn parse_tsql_block_body(&mut self, block_kind: &str) -> Result<Vec<Expression>> {
        let tokens = self.collect_tsql_block_body_tokens(block_kind)?;
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        let mut parser = Parser::with_config(tokens, self.config.clone());
        parser.recursion = Arc::clone(&self.recursion);
        parser.parse()
    }
}
