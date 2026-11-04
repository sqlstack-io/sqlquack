//! Snowflake ALTER of object kinds with no statement of their own: EXPERIMENT, SNAPSHOT, SNAPSHOT SET, TAG, TASK and TASK GRAPH.

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::schema_objects::models::AlterObject;
use crate::expressions::schema_objects::models::ObjectProperty;
use crate::expressions::tree::models::Expression;
use crate::parser::constants::SNOWFLAKE_ALTER_KINDS;
use crate::parser::constants::TASK_ACTIONS;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parse a Snowflake ALTER of a kind this module owns, or None when the next words name another kind.
    pub(crate) fn parse_alter_snowflake_object(&mut self) -> Result<Option<Expression>> {
        if let Some(expr) = self.parse_alter_snowflake_objects()? {
            return Ok(Some(expr));
        }
        if self.check_identifier("TASK")
            && self.check_next_identifier("GRAPH")
            && !self
                .text_at(2)
                .is_some_and(|text| TASK_ACTIONS.contains(&text.as_str()))
        {
            self.current += 2;
            return self.parse_alter_task_graph().map(Some);
        }
        if self.match_identifier("TASK") {
            return self.parse_alter_task().map(Some);
        }
        if self.match_text_seq(&["SNAPSHOT", "SET"]) {
            return self.parse_alter_snapshot_set().map(Some);
        }
        if self.check_identifier("SNAPSHOT") && !self.check_next_identifier("POLICY") {
            self.skip();
            let head = self.parse_alter_object_head()?;
            let action = self.expect_alter_words(&[&["SET", "COMMENT"]])?;
            self.expect_token(TokenType::Eq)?;
            let comment = property("COMMENT", None, Some(self.parse_object_value()?));
            return Ok(Some(alter_object(
                "SNAPSHOT",
                head,
                action,
                (vec![], vec![comment]),
            )));
        }
        if self.match_identifier("TAG") {
            return self.parse_alter_tag().map(Some);
        }
        if self.match_identifier("EXPERIMENT") {
            return self.parse_alter_experiment().map(Some);
        }
        if !self.is_at_end()
            && !SNOWFLAKE_ALTER_KINDS.contains(&self.peek_text().to_ascii_uppercase().as_str())
        {
            return Err(self.parse_error(format!(
                "Snowflake has no ALTER {}",
                self.peek_text().to_ascii_uppercase()
            )));
        }
        Ok(None)
    }

    /// The uppercase text of the token `offset` tokens ahead, if there is one.
    pub(crate) fn text_at(&self, offset: usize) -> Option<String> {
        self.tokens
            .get(self.current + offset)
            .map(|token| token.text.to_ascii_uppercase())
    }

    /// Parse `[ IF EXISTS ] <name>` after the kind.
    pub(crate) fn parse_alter_object_head(&mut self) -> Result<(TableRef, bool)> {
        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        Ok((self.parse_object_name()?, if_exists))
    }

    /// Parse an object name: an identifier, never a string or an unquoted reserved keyword.
    pub(crate) fn parse_object_name(&mut self) -> Result<TableRef> {
        if self.check(TokenType::String)
            || self.is_snowflake_reserved_keyword()
            || !self.is_object_name_start()
        {
            return Err(self.parse_error("Expected an object name"));
        }
        self.parse_table_ref()
    }

    /// Whether the current token can start an object name.
    fn is_object_name_start(&self) -> bool {
        !self.is_at_end()
            && (self.is_identifier_token() || self.check_keyword())
            && !self.check(TokenType::Semicolon)
    }

    /// Parse a simple name, such as a contact purpose or a property: an identifier, not a reserved keyword.
    pub(crate) fn parse_simple_name(&mut self) -> Result<Identifier> {
        if self.check(TokenType::String)
            || self.is_snowflake_reserved_keyword()
            || !self.is_object_name_start()
        {
            return Err(self.parse_error("Expected a name"));
        }
        self.expect_identifier_or_safe_keyword_with_quoted()
    }

    /// Parse a property value: a literal, TRUE, FALSE or a name.
    pub(crate) fn parse_object_value(&mut self) -> Result<Expression> {
        if self.check(TokenType::String)
            || self.check(TokenType::Number)
            || self.check(TokenType::DollarString)
        {
            return self.parse_primary();
        }
        if self.check(TokenType::True) || self.check(TokenType::False) {
            return self.parse_primary();
        }
        if self.check(TokenType::Dash)
            && matches!(
                self.tokens
                    .get(self.current + 1)
                    .map(|token| token.token_type),
                Some(TokenType::Number)
            )
        {
            return self.parse_unary();
        }
        Ok(Expression::Table(Box::new(self.parse_object_name()?)))
    }

    /// Parse a string literal value.
    pub(crate) fn parse_string_value(&mut self) -> Result<Expression> {
        if !self.check(TokenType::String) {
            return Err(self.parse_error("Expected a string"));
        }
        self.parse_primary()
    }

    /// Consume one of `choices`, each a sequence of words, and return its words joined.
    pub(crate) fn expect_alter_words(&mut self, choices: &[&[&str]]) -> Result<String> {
        match choices.iter().find(|words| self.match_text_seq(words)) {
            Some(words) => Ok(words.join(" ")),
            None => Err(self.parse_error(format!(
                "Expected {}",
                choices
                    .iter()
                    .map(|words| words.join(" "))
                    .collect::<Vec<_>>()
                    .join(" or ")
            ))),
        }
    }

    /// Parse a comma-separated list of object names.
    fn parse_object_name_list(&mut self) -> Result<Vec<Expression>> {
        let mut names = vec![Expression::Table(Box::new(self.parse_object_name()?))];
        while self.match_token(TokenType::Comma) {
            names.push(Expression::Table(Box::new(self.parse_object_name()?)));
        }
        Ok(names)
    }

    /// Parse the properties of a SET (`set`) or UNSET action; `graph` admits TASK_DEFAULTS and always reads CONTACT as contacts.
    fn parse_alter_object_properties(
        &mut self,
        set: bool,
        graph: bool,
    ) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let next_is_value = self.check_next(TokenType::Eq);
            if !next_is_value
                && !self.check_next(TokenType::Semicolon)
                && !self.check_next(TokenType::Comma)
                && self.match_identifier("TAG")
            {
                loop {
                    let name = self.parse_simple_qualified_name()?;
                    let value = if set {
                        self.expect_token(TokenType::Eq)?;
                        Some(self.parse_object_value()?)
                    } else {
                        None
                    };
                    properties.push(property("TAG", Some(name), value));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            } else if (graph
                || !next_is_value
                    && !self.check_next(TokenType::Semicolon)
                    && !self.check_next(TokenType::Comma))
                && self.match_identifier("CONTACT")
            {
                loop {
                    let purpose = self.parse_simple_name()?;
                    let value = if set {
                        self.expect_token(TokenType::Eq)?;
                        Some(Expression::Table(Box::new(self.parse_object_name()?)))
                    } else {
                        None
                    };
                    properties.push(property("CONTACT", Some(purpose), value));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            } else if self.match_text_seq(&["EXECUTE"]) {
                self.expect_alter_words(&[&["AS", "USER"]])?;
                let value = if set {
                    Some(Expression::Table(Box::new(self.parse_object_name()?)))
                } else {
                    None
                };
                properties.push(property("EXECUTE AS USER", None, value));
            } else if graph && !next_is_value && self.match_identifier("TASK_DEFAULTS") {
                self.expect_token(TokenType::LParen)?;
                loop {
                    let name = self.parse_simple_name()?;
                    let value = if set {
                        self.expect_token(TokenType::Eq)?;
                        Some(self.parse_object_value()?)
                    } else {
                        None
                    };
                    properties.push(property("TASK_DEFAULTS", Some(name), value));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
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
            let comma = self.match_token(TokenType::Comma);
            if self.is_at_end() || self.check(TokenType::Semicolon) || !set && !comma {
                return Ok(properties);
            }
        }
    }

    /// Parse a possibly qualified name, such as a tag name, as one identifier.
    pub(crate) fn parse_simple_qualified_name(&mut self) -> Result<Identifier> {
        let mut name = self.parse_simple_name()?;
        while self.match_token(TokenType::Dot) {
            let part = self.parse_simple_name()?;
            name.name = format!("{}.{}", name.name, part.name);
        }
        Ok(name)
    }

    /// Parse ALTER TASK after its kind.
    fn parse_alter_task(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let action = self.expect_alter_words(&[
            &["RESUME"],
            &["SUSPEND"],
            &["REMOVE", "WHEN"],
            &["REMOVE", "AFTER"],
            &["ADD", "AFTER"],
            &["MODIFY", "AS"],
            &["MODIFY", "WHEN"],
            &["SET"],
            &["UNSET"],
        ])?;
        let (operands, properties) = match action.as_str() {
            "REMOVE AFTER" | "ADD AFTER" => (self.parse_object_name_list()?, vec![]),
            "MODIFY AS" => (vec![self.parse_statement()?], vec![]),
            "MODIFY WHEN" => (vec![self.parse_expression()?], vec![]),
            "SET" => (vec![], self.parse_alter_object_properties(true, false)?),
            "UNSET" => (vec![], self.parse_alter_object_properties(false, false)?),
            _ => (vec![], vec![]),
        };
        Ok(alter_object("TASK", head, action, (operands, properties)))
    }

    /// Parse ALTER TASK GRAPH after its kind.
    fn parse_alter_task_graph(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let action = self.expect_alter_words(&[
            &["COMMIT"],
            &["REVERT"],
            &["RESUME"],
            &["SUSPEND"],
            &["REMOVE", "WHEN"],
            &["MODIFY", "WHEN"],
            &["SET"],
            &["UNSET"],
        ])?;
        let (operands, properties) = match action.as_str() {
            "MODIFY WHEN" => (vec![self.parse_expression()?], vec![]),
            "SET" => (vec![], self.parse_alter_object_properties(true, true)?),
            "UNSET" => (vec![], self.parse_alter_object_properties(false, true)?),
            _ => (vec![], vec![]),
        };
        Ok(alter_object(
            "TASK GRAPH",
            head,
            action,
            (operands, properties),
        ))
    }

    /// Parse ALTER SNAPSHOT SET after its kind.
    fn parse_alter_snapshot_set(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["ADD", "SNAPSHOT"],
            &["APPLY", "SNAPSHOT", "POLICY"],
            &["DELETE", "SNAPSHOT", "IDENTIFIER"],
            &["MODIFY", "SNAPSHOT", "IDENTIFIER"],
            &["RESUME", "SNAPSHOT"],
            &["SUSPEND", "SNAPSHOT"],
            &["SET"],
            &["UNSET"],
        ])?;
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        match action.as_str() {
            "APPLY SNAPSHOT POLICY" => {
                operands.push(Expression::Table(Box::new(self.parse_object_name()?)));
                if self.match_identifier("FORCE") {
                    action.push_str(" FORCE");
                }
            }
            "DELETE SNAPSHOT IDENTIFIER" => operands.push(self.parse_string_value()?),
            "MODIFY SNAPSHOT IDENTIFIER" => {
                operands.push(self.parse_string_value()?);
                let change = self.expect_alter_words(&[
                    &["ADD", "LEGAL", "HOLD"],
                    &["REMOVE", "LEGAL", "HOLD"],
                    &["SET", "COMMENT"],
                    &["UNSET", "COMMENT"],
                ])?;
                if let "SET COMMENT" = change.as_str() {
                    self.expect_token(TokenType::Eq)?;
                    properties.push(property("COMMENT", None, Some(self.parse_string_value()?)));
                }
                action = format!("{action} {change}");
            }
            "RESUME SNAPSHOT" | "SUSPEND SNAPSHOT" => {
                if let Some(which) = ["CREATION", "EXPIRATION"]
                    .iter()
                    .find(|which| self.match_identifier(which))
                {
                    action = format!("{action} {which}");
                }
                action = format!("{action} {}", self.expect_alter_words(&[&["POLICY"]])?);
            }
            "SET" => properties = self.parse_alter_object_properties(true, false)?,
            "UNSET" => properties = self.parse_alter_object_properties(false, false)?,
            _ => {}
        }
        Ok(alter_object(
            "SNAPSHOT SET",
            head,
            action,
            (operands, properties),
        ))
    }

    /// Parse ALTER TAG after its kind.
    fn parse_alter_tag(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["RENAME", "TO"],
            &["ADD", "ALLOWED_VALUES"],
            &["DROP", "ALLOWED_VALUES"],
            &["SET", "DATA", "MOVEMENT", "POLICY"],
            &["SET", "MASKING", "POLICY"],
            &["UNSET", "MASKING", "POLICY"],
            &["SET"],
            &["UNSET"],
        ])?;
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        match action.as_str() {
            "RENAME TO" => operands.push(Expression::Table(Box::new(self.parse_object_name()?))),
            "ADD ALLOWED_VALUES" | "DROP ALLOWED_VALUES" => {
                operands = self.parse_allowed_values()?
            }
            "SET DATA MOVEMENT POLICY" => {
                operands.push(Expression::Table(Box::new(self.parse_object_name()?)));
                if self.match_identifier("FORCE") {
                    action.push_str(" FORCE");
                }
            }
            "SET MASKING POLICY" => {
                operands = self.parse_masking_policies()?;
                if self.match_identifier("FORCE") {
                    action.push_str(" FORCE");
                }
            }
            "UNSET MASKING POLICY" => operands = self.parse_masking_policies()?,
            "SET" => loop {
                let option = self.expect_alter_words(&[
                    &["ALLOWED_VALUES"],
                    &["MULTI_VALUE"],
                    &["PROPAGATE"],
                    &["ON_CONFLICT"],
                    &["COMMENT"],
                ])?;
                match option.as_str() {
                    "ALLOWED_VALUES" => {
                        for value in self.parse_allowed_values()? {
                            properties.push(property("ALLOWED_VALUES", None, Some(value)));
                        }
                    }
                    _ => {
                        self.expect_token(TokenType::Eq)?;
                        properties.push(property(&option, None, Some(self.parse_object_value()?)));
                    }
                }
                if self.is_at_end() || self.check(TokenType::Semicolon) {
                    break;
                }
            },
            _ => loop {
                let option = self.expect_alter_words(&[
                    &["ALLOWED_VALUES"],
                    &["MULTI_VALUE"],
                    &["PROPAGATE"],
                    &["ON_CONFLICT"],
                    &["COMMENT"],
                ])?;
                properties.push(property(&option, None, None));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            },
        }
        Ok(alter_object("TAG", head, action, (operands, properties)))
    }

    /// Parse `<policy> [ , MASKING POLICY <policy> ... ]` after MASKING POLICY.
    fn parse_masking_policies(&mut self) -> Result<Vec<Expression>> {
        let mut policies = vec![Expression::Table(Box::new(self.parse_object_name()?))];
        while self.match_token(TokenType::Comma) {
            self.expect_alter_words(&[&["MASKING", "POLICY"]])?;
            policies.push(Expression::Table(Box::new(self.parse_object_name()?)));
        }
        Ok(policies)
    }

    /// Parse a tag's allowed values: one or more comma-separated values.
    fn parse_allowed_values(&mut self) -> Result<Vec<Expression>> {
        let mut values = vec![self.parse_object_value()?];
        while self.match_token(TokenType::Comma) {
            values.push(self.parse_object_value()?);
        }
        Ok(values)
    }

    /// Parse ALTER EXPERIMENT after its kind.
    fn parse_alter_experiment(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["ABORT"],
            &["ADD", "RUN"],
            &["DROP", "RUN"],
            &["COMMIT", "RUN"],
            &["MODIFY", "RUN"],
        ])?;
        let mut operands: Vec<Expression> = Vec::new();
        match action.as_str() {
            "ABORT" => {}
            "COMMIT RUN" => {
                operands.push(Expression::Table(Box::new(self.parse_object_name()?)));
                if self.match_text_seq(&["WITH", "STATUS"]) {
                    self.expect_token(TokenType::Eq)?;
                    operands.push(self.parse_string_value()?);
                    action.push_str(" WITH STATUS");
                }
            }
            "MODIFY RUN" => {
                operands.push(Expression::Table(Box::new(self.parse_object_name()?)));
                self.expect_alter_words(&[&["SET", "STATUS"]])?;
                self.expect_token(TokenType::Eq)?;
                operands.push(self.parse_string_value()?);
                action.push_str(" SET STATUS");
            }
            _ => operands.push(Expression::Table(Box::new(self.parse_object_name()?))),
        }
        Ok(alter_object(
            "EXPERIMENT",
            head,
            action,
            (operands, Vec::new()),
        ))
    }
}

/// An ALTER of `kind` on the named object, with its action, operands and properties.
pub(crate) fn alter_object(
    kind: &str,
    (name, if_exists): (TableRef, bool),
    action: String,
    (operands, properties): (Vec<Expression>, Vec<ObjectProperty>),
) -> Expression {
    Expression::AlterObject(Box::new(AlterObject {
        kind: kind.to_owned(),
        name,
        if_exists,
        action,
        operands,
        properties,
    }))
}

/// One property of kind `kind`.
pub(crate) fn property(
    kind: &str,
    name: Option<Identifier>,
    value: Option<Expression>,
) -> ObjectProperty {
    ObjectProperty {
        kind: kind.to_owned(),
        name,
        value,
    }
}
