//! Parser ddl: ALTER TABLE and its actions.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::models::AlterTable;
use sqlquack_syntax::expressions::columns::models::AlterTableAction;
use sqlquack_syntax::expressions::columns::models::ColumnPosition;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    pub(super) fn parse_alter_table(
        &mut self,
        alter_table_modifier: Option<String>,
    ) -> Result<Expression> {
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

    fn parse_alter_table_actions(&mut self) -> Result<(Vec<AlterTableAction>, Option<String>)> {
        let mut actions: Vec<AlterTableAction> = Vec::new();
        let mut last_was_add_column = false;
        let mut last_was_alter_column = false;
        let mut dropping_columns: Option<(bool, bool)> = None;
        let mut with_check_modifier: Option<String> = None;

        loop {
            if last_was_alter_column {
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
                continue;
            }
            if let Some((if_exists, column_keyword)) =
                dropping_columns.filter(|_| !self.check_alter_table_action_start())
            {
                actions.push(AlterTableAction::DropColumn {
                    name: self.parse_simple_name()?,
                    if_exists,
                    cascade: false,
                    column_keyword,
                });
                if !self.match_token(TokenType::Comma) {
                    break;
                }
                continue;
            }
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
                let column_keyword = self.match_token(TokenType::Column);
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
                actions.push(AlterTableAction::AddColumn {
                    column: col_def,
                    if_not_exists,
                    position,
                    column_keyword,
                });
            } else {
                if self.check_identifier("ALGORITHM") || self.check_identifier("LOCK") {
                    self.current -= 1;
                    break;
                }
                if let Some(listed) = self.parse_snowflake_alter_column_list()? {
                    actions.extend(listed);
                    (last_was_add_column, last_was_alter_column, dropping_columns) =
                        (false, false, None);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                    continue;
                }
                let typed = self.parse_snowflake_modify_column()?;
                let action = match typed {
                    Some(action) => action,
                    None => self.parse_alter_action()?,
                };
                last_was_add_column = matches!(action, AlterTableAction::AddColumn { .. });
                last_was_alter_column =
                    self.is_snowflake() && matches!(action, AlterTableAction::AlterColumn { .. });
                dropping_columns = match &action {
                    AlterTableAction::DropColumn {
                        if_exists,
                        cascade: false,
                        column_keyword,
                        ..
                    } if self.is_snowflake() => Some((*if_exists, *column_keyword)),
                    _ => None,
                };
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

    /// Whether the next words start another ALTER TABLE action rather than continuing a Snowflake column list.
    fn check_alter_table_action_start(&self) -> bool {
        [
            TokenType::Add,
            TokenType::Drop,
            TokenType::Alter,
            TokenType::Rename,
            TokenType::Set,
            TokenType::Cluster,
        ]
        .iter()
        .any(|kind| self.check(*kind))
            || [
                "MODIFY",
                "UNSET",
                "SWAP",
                "CLUSTER",
                "SUSPEND",
                "RESUME",
                "RECLUSTER",
            ]
            .iter()
            .any(|word| self.check_identifier(word))
    }
}
