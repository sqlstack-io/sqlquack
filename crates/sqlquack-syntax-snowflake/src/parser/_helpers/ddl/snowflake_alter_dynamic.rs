//! Snowflake ALTER DYNAMIC TABLE.

use crate::parser::_helpers::ddl::snowflake_alter::alter_object;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::_helpers::ddl::snowflake_alter_objects::tuple;
use crate::parser::constants::DROP_ROW_ACCESS_POLICY;
use crate::parser::constants::SET;
use crate::parser::constants::SET_MASKING_POLICY;
use crate::parser::constants::SET_TAG;
use crate::parser::constants::UNSET_COMMENT;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::models::AlterColumn;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::ObjectProperty;
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The operands and properties of one ALTER action.
type Parts = (Vec<Expression>, Vec<ObjectProperty>);

/// The actions of an ALTER DYNAMIC TABLE, as their leading words; longer sequences come first.
const ACTIONS: &[&[&str]] = &[
    &["REFRESH", "COPY", "SESSION"],
    &["REFRESH"],
    &["RENAME", "TO"],
    &["SWAP", "WITH"],
    &["SUSPEND", "RECLUSTER"],
    &["RESUME", "RECLUSTER"],
    &["SUSPEND", "SEARCH", "OPTIMIZATION"],
    &["RESUME", "SEARCH", "OPTIMIZATION"],
    &["SUSPEND"],
    &["RESUME"],
    &["CLUSTER", "BY"],
    &["DROP", "CLUSTERING", "KEY"],
    &["ADD", "SEARCH", "OPTIMIZATION"],
    &["DROP", "SEARCH", "OPTIMIZATION"],
    &["ADD", "STORAGE", "LIFECYCLE", "POLICY"],
    &["DROP", "STORAGE", "LIFECYCLE", "POLICY"],
    &["ADD", "ROW", "ACCESS", "POLICY"],
    &["DROP", "ALL", "ROW", "ACCESS", "POLICIES"],
    &["DROP", "ROW", "ACCESS", "POLICY"],
    &["SET", "AGGREGATION", "POLICY"],
    &["UNSET", "AGGREGATION", "POLICY"],
    &["SET", "TAG"],
    &["UNSET", "TAG"],
    &["SET"],
    &["UNSET"],
    &["ALTER"],
    &["MODIFY"],
];

impl Parser {
    /// Parse ALTER DYNAMIC TABLE after its kind.
    pub(super) fn parse_alter_dynamic_table(&mut self) -> Result<Expression> {
        let head = if self.omits_dynamic_table_name() {
            (
                sqlquack_syntax::expressions::names::models::TableRef::new(""),
                false,
            )
        } else {
            self.parse_alter_object_head()?
        };
        let mut names: Vec<Expression> = Vec::new();
        while self.match_token(TokenType::Comma) {
            names.push(Expression::Table(Box::new(self.parse_object_name()?)));
        }
        let action = self.expect_alter_words(if names.is_empty() {
            ACTIONS
        } else {
            &[&["REFRESH", "COPY", "SESSION"], &["REFRESH"]]
        })?;
        let (operands, properties) = self.parse_dynamic_table_action(&action)?;
        names.extend(operands);
        Ok(alter_object(
            "DYNAMIC TABLE",
            head,
            action,
            (names, properties),
        ))
    }

    /// Whether the table name is omitted before `SET <property> =` which Snowflake parses and refuses by name check.
    fn omits_dynamic_table_name(&self) -> bool {
        let follows = |offset: usize, types: &[TokenType]| {
            self.tokens
                .get(self.current + offset)
                .is_some_and(|token| types.contains(&token.token_type))
        };
        (self.check_identifier("SET") || self.check(TokenType::Set)) && follows(2, &[TokenType::Eq])
    }

    /// Parse what follows the words of an ALTER DYNAMIC TABLE action.
    fn parse_dynamic_table_action(&mut self, action: &str) -> Result<Parts> {
        Ok(match action {
            "RENAME TO" | "SWAP WITH" | "DROP ROW ACCESS POLICY" => {
                let name = self.policy_operand()?;
                if action == DROP_ROW_ACCESS_POLICY && self.match_token(TokenType::Comma) {
                    self.expect_alter_words(&[&["ADD", "ROW", "ACCESS", "POLICY"]])?;
                    let added = self.policy_operand()?;
                    self.expect_token(TokenType::On)?;
                    return Ok((vec![name, added, self.row_access_columns()?], vec![]));
                }
                (vec![name], vec![])
            }
            "CLUSTER BY" => {
                self.expect_token(TokenType::LParen)?;
                let mut keys = vec![self.parse_expression()?];
                while self.match_token(TokenType::Comma) {
                    keys.push(self.parse_expression()?);
                }
                self.expect_token(TokenType::RParen)?;
                (vec![tuple(keys)], vec![])
            }
            "ADD SEARCH OPTIMIZATION" | "DROP SEARCH OPTIMIZATION" => {
                (self.parse_search_targets(action)?, vec![])
            }
            "ADD STORAGE LIFECYCLE POLICY" => {
                let policy = self.policy_operand()?;
                self.expect_token(TokenType::On)?;
                (vec![policy, self.column_list_operand()?], vec![])
            }
            "ADD ROW ACCESS POLICY" => {
                let policy = self.policy_operand()?;
                self.expect_token(TokenType::On)?;
                (vec![policy, self.row_access_columns()?], vec![])
            }
            "SET AGGREGATION POLICY" => {
                let policy = self.policy_operand()?;
                let mut properties: Vec<ObjectProperty> = Vec::new();
                if self.match_text_seq(&["ENTITY"]) {
                    self.expect_alter_words(&[&["KEY"]])?;
                    properties.push(property(
                        "ENTITY KEY",
                        None,
                        Some(self.column_list_operand()?),
                    ));
                }
                if self.match_text_seq(&["FORCE"]) {
                    properties.push(property("FORCE", None, None));
                }
                (vec![policy], properties)
            }
            "UNSET TAG" if self.is_at_end() || self.check(TokenType::Semicolon) => (vec![], vec![]),
            "SET TAG" if self.check(TokenType::Eq) => {
                self.skip();
                let value = self.parse_object_value()?;
                (
                    vec![],
                    vec![property("TAG", Some(Identifier::new("")), Some(value))],
                )
            }
            "SET TAG" | "UNSET TAG" => (vec![], self.parse_tag_list(action == SET_TAG)?),
            "SET" | "UNSET" => (vec![], self.parse_dynamic_table_properties(action == SET)?),
            "ALTER" => (self.parse_dynamic_column_actions(false)?, vec![]),
            "MODIFY" => (self.parse_dynamic_column_actions(true)?, vec![]),
            _ => (vec![], vec![]),
        })
    }

    /// Parse a policy, table or tag name operand.
    fn policy_operand(&mut self) -> Result<Expression> {
        Ok(Expression::Table(Box::new(self.parse_object_name()?)))
    }

    /// Parse the row access policy columns, which Snowflake parses even when empty.
    fn row_access_columns(&mut self) -> Result<Expression> {
        if self.check(TokenType::LParen) && self.check_next(TokenType::RParen) {
            self.current += 2;
            return Ok(tuple(vec![]));
        }
        self.column_list_operand()
    }

    /// Parse `( <col_name> [ , ... ] )`.
    fn column_list_operand(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut columns = vec![Expression::Identifier(self.parse_simple_name()?)];
        while self.match_token(TokenType::Comma) {
            columns.push(Expression::Identifier(self.parse_simple_name()?));
        }
        self.expect_token(TokenType::RParen)?;
        Ok(tuple(columns))
    }

    /// Parse `[ ON <target> [ , ... ] ]` after the words of a search optimization `action`; a DROP also takes a string.
    pub(super) fn parse_search_targets(&mut self, action: &str) -> Result<Vec<Expression>> {
        let mut targets: Vec<Expression> = Vec::new();
        if self.match_token(TokenType::On) {
            loop {
                if action.starts_with("DROP") && self.check(TokenType::String) {
                    targets.push(self.parse_primary()?);
                } else {
                    targets.push(self.search_target(action.starts_with("ADD"))?);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        Ok(targets)
    }

    /// Parse one search optimization target: `<method> ( <target> )`, or, unless `add`, a column name or an expression ID.
    fn search_target(&mut self, add: bool) -> Result<Expression> {
        if !add && self.check(TokenType::Number) {
            return self.parse_primary();
        }
        let name = self.parse_simple_name()?;
        if add || self.check(TokenType::LParen) {
            return Ok(tuple(vec![
                Expression::Identifier(name),
                self.column_list_operand()?,
            ]));
        }
        Ok(Expression::Identifier(name))
    }

    /// Parse `<tag_name> = '<tag_value>' [ , ... ]` when `set`, else `<tag_name> [ , ... ]`, stopping before a comma that starts another column.
    pub(super) fn parse_tag_list(&mut self, set: bool) -> Result<Vec<ObjectProperty>> {
        let mut tags: Vec<ObjectProperty> = Vec::new();
        loop {
            let name = self.parse_simple_qualified_name()?;
            let value = if set {
                self.expect_token(TokenType::Eq)?;
                Some(self.parse_object_value()?)
            } else {
                None
            };
            tags.push(property("TAG", Some(name), value));
            let next_is_tag = self.check(TokenType::Comma)
                && self.tokens.get(self.current + 2).is_none_or(|token| {
                    if set {
                        matches!(token.token_type, TokenType::Eq | TokenType::Dot)
                    } else {
                        matches!(
                            token.token_type,
                            TokenType::Comma | TokenType::Semicolon | TokenType::Dot
                        )
                    }
                });
            if !next_is_tag {
                return Ok(tags);
            }
            self.skip();
        }
    }

    /// Parse the properties of a SET (`set`) or UNSET action; a SET of FROZEN WHERE or EXECUTE AS USER sets nothing else.
    fn parse_dynamic_table_properties(&mut self, set: bool) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let alone = set
                && (self.check_text_seq(&["FROZEN", "WHERE"]) || self.check_text_seq(&["EXECUTE"]));
            if alone && !properties.is_empty() {
                return Err(
                    self.parse_error("FROZEN WHERE and EXECUTE AS USER are set on their own")
                );
            }
            if self.match_text_seq(&["FROZEN", "WHERE"]) {
                let value = if set {
                    self.expect_token(TokenType::LParen)?;
                    let condition = self.parse_expression()?;
                    self.expect_token(TokenType::RParen)?;
                    Some(condition)
                } else {
                    None
                };
                properties.push(property("FROZEN WHERE", None, value));
            } else if self.match_text_seq(&["EXECUTE"]) {
                self.expect_alter_words(&[&["AS", "USER"]])?;
                let value = if set {
                    let user = self.policy_operand()?;
                    match self.match_text_seq(&["USE", "SECONDARY", "ROLES"]) {
                        true => {
                            let mut expressions = vec![user];
                            expressions.extend(self.parse_secondary_roles()?);
                            Some(tuple(expressions))
                        }
                        false => Some(user),
                    }
                } else {
                    None
                };
                properties.push(property("EXECUTE AS USER", None, value));
            } else {
                let name = self.parse_simple_name()?;
                let value = if set {
                    self.expect_token(TokenType::Eq)?;
                    Some(self.parse_object_value()?)
                } else {
                    None
                };
                properties.push(property(&name.name.to_ascii_uppercase(), None, value));
            }
            if alone && !self.is_at_end() && !self.check(TokenType::Semicolon) {
                return Err(
                    self.parse_error("FROZEN WHERE and EXECUTE AS USER are set on their own")
                );
            }
            let comma = self.match_token(TokenType::Comma);
            if self.is_at_end() || self.check(TokenType::Semicolon) || !set && !comma {
                return Ok(properties);
            }
        }
    }

    /// Parse the column actions of `{ ALTER | MODIFY } [ ( ] [ COLUMN ] <col> <action> [ , ... ] [ ) ]`, each as one operand; without COLUMN, no column unsets a tag and ALTER (`modify` false) unsets no masking policy.
    fn parse_dynamic_column_actions(&mut self, modify: bool) -> Result<Vec<Expression>> {
        let parenthesized = self.match_token(TokenType::LParen);
        let mut columns: Vec<Expression> = Vec::new();
        loop {
            let column_word = self.match_token(TokenType::Column);
            let column = Expression::Identifier(self.parse_simple_name()?);
            if !column_word && self.check_text_seq(&["UNSET", "TAG"]) {
                return Err(self.parse_error("Expected COLUMN before a column that unsets a tag"));
            }
            if !column_word && !modify && self.check_text_seq(&["UNSET", "MASKING", "POLICY"]) {
                return Err(self.parse_error(
                    "Expected COLUMN before a column that ALTER unsets a masking policy on",
                ));
            }
            let action = self.expect_alter_words(&[
                &["COMMENT"],
                &["UNSET", "COMMENT"],
                &["SET", "MASKING", "POLICY"],
                &["UNSET", "MASKING", "POLICY"],
                &["SET", "PROJECTION", "POLICY"],
                &["UNSET", "PROJECTION", "POLICY"],
                &["SET", "TAG"],
                &["UNSET", "TAG"],
            ])?;
            let mut properties: Vec<ObjectProperty> = Vec::new();
            match action.as_str() {
                "COMMENT" | "UNSET COMMENT" => {
                    let set = action != UNSET_COMMENT;
                    let comment = if set {
                        Some(Box::new(self.parse_string_value()?))
                    } else {
                        None
                    };
                    columns.push(Expression::AlterColumn(Box::new(AlterColumn {
                        this: Box::new(column),
                        dtype: None,
                        collate: None,
                        using: None,
                        default: None,
                        drop: (!set).then(|| {
                            Box::new(Expression::Var(Box::new(Var {
                                this: "COMMENT".to_owned(),
                            })))
                        }),
                        comment,
                        allow_null: None,
                        visible: None,
                        rename_to: None,
                    })));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                    continue;
                }
                "SET MASKING POLICY" | "SET PROJECTION POLICY" => {
                    properties.push(property(&action, None, Some(self.policy_operand()?)));
                    if action == SET_MASKING_POLICY && self.match_token(TokenType::Using) {
                        properties.push(property("USING", None, Some(self.column_list_operand()?)));
                    }
                    if self.match_text_seq(&["FORCE"]) {
                        properties.push(property("FORCE", None, None));
                    }
                }
                "SET TAG" | "UNSET TAG" => {
                    properties = self.parse_tag_list(action == SET_TAG)?;
                }
                _ => properties.push(property(&action, None, None)),
            }
            let mut items = vec![column, Expression::Identifier(Identifier::new(action))];
            for item in properties {
                items.extend(item.name.map(Expression::Identifier));
                items.extend(item.value);
            }
            columns.push(tuple(items));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        if parenthesized {
            self.expect_token(TokenType::RParen)?;
        }
        Ok(columns)
    }
}
