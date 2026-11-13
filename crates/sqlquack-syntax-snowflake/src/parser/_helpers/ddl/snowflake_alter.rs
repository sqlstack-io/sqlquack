//! Snowflake ALTER of object kinds that have no statement of their own, such as TASK, TAG or ALERT.

use crate::parser::constants::ONLINE_FEATURE_TABLE;
use crate::parser::constants::SNOWFLAKE_ALTER_KINDS;
use crate::parser::constants::TASK_ACTIONS;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::expressions::schema_objects::models::AlterObject;
use sqlquack_syntax::expressions::schema_objects::models::ObjectProperty;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The kinds whose ALTER Snowflake parses without a name, and before which actions; recorded receipts refuse the omission for other kinds, such as BACKUP POLICY or SECRET, and for SNAPSHOT POLICY's UNSET.
const OMITTED_NAME_ACTIONS: &[(&str, &[&str])] = &[
    ("AGGREGATION POLICY", &["SET", "UNSET"]),
    ("DATABASE", &["SET", "UNSET"]),
    ("AUTHENTICATION POLICY", &["SET", "UNSET"]),
    ("FEATURE POLICY", &["SET", "UNSET"]),
    ("JOIN POLICY", &["SET", "UNSET"]),
    ("MAINTENANCE POLICY", &["SET", "UNSET"]),
    ("MASKING POLICY", &["SET", "UNSET"]),
    ("NETWORK POLICY", &["SET", "UNSET"]),
    ("ONLINE FEATURE TABLE", &["SET"]),
    ("PACKAGES POLICY", &["SET", "UNSET"]),
    ("PASSWORD POLICY", &["SET", "UNSET"]),
    ("SESSION POLICY", &["SET", "UNSET"]),
    ("SNAPSHOT POLICY", &["SET"]),
    ("STORAGE LIFECYCLE POLICY", &["SET", "UNSET"]),
    ("INTEGRATION", &["SET", "UNSET", "REFRESH"]),
    ("CATALOG INTEGRATION", &["SET"]),
    ("EXTERNAL ACCESS INTEGRATION", &["SET", "UNSET"]),
    ("SECURITY INTEGRATION", &["SET"]),
    ("VIEW", &["SET"]),
    ("MATERIALIZED VIEW", &["SET"]),
];

impl Parser {
    /// Parse a Snowflake ALTER of a kind this module owns, or None when the next words name another kind.
    pub(crate) fn parse_alter_snowflake_object(&mut self) -> Result<Option<Expression>> {
        if let Some(kind) = self.parse_snowflake_class_kind()? {
            return self.parse_alter_snowflake_class(&kind).map(Some);
        }
        if let Some(expr) = self.parse_alter_snowflake_objects()? {
            return Ok(Some(expr));
        }
        if let Some(expr) = self.parse_alter_openflow_or_role()? {
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
        let policy_named = self.tokens.get(self.current + 2).is_some_and(|token| {
            token.token_type != TokenType::QuotedIdentifier
                && token.text.eq_ignore_ascii_case("SET")
        }) && self.text_at(3).as_deref() == Some("COMMENT");
        if self.check_identifier("SNAPSHOT")
            && (!self.check_next_identifier("POLICY") || policy_named)
        {
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
        if self.match_identifier("ALERT") {
            return self.parse_alter_alert().map(Some);
        }
        if self.match_text_seq(&["ONLINE", "FEATURE", "TABLE"]) {
            return self.parse_alter_online_feature_table().map(Some);
        }
        if self.check_identifier("ONLINE")
            || self.check_identifier("FEATURE") && !self.check_next_identifier("POLICY")
        {
            return Err(self.parse_error(
                "Snowflake has ALTER ONLINE FEATURE TABLE and ALTER FEATURE POLICY only",
            ));
        }
        if self.match_identifier("USER") {
            return self.parse_alter_user().map(Some);
        }
        if self.match_identifier("VARIABLE") {
            return self.parse_alter_variable().map(Some);
        }
        if self.match_token(TokenType::Function) || self.match_identifier("FUNCTION") {
            return self.parse_alter_function().map(Some);
        }
        if let Some(alter) = self.parse_alter_snowflake_group()? {
            return Ok(Some(alter));
        }
        if let Some(alter) = self.parse_alter_snowflake_resource()? {
            return Ok(Some(alter));
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
    pub(super) fn text_at(&self, offset: usize) -> Option<String> {
        self.tokens
            .get(self.current + offset)
            .map(|token| token.text.to_ascii_uppercase())
    }

    /// Parse `[ IF EXISTS ] <name>` after the kind.
    pub(super) fn parse_alter_object_head(&mut self) -> Result<(TableRef, bool)> {
        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        Ok((self.parse_object_name()?, if_exists))
    }

    /// Parse the head of `kind`, or an omitted name where Snowflake parses one: before `SET <property> =`, an `UNSET <property>` list, or an INTEGRATION's `REFRESH` of one of its secrets.
    pub(super) fn parse_alter_head_or_omitted(&mut self, kind: &str) -> Result<(TableRef, bool)> {
        let word_at = |offset: usize| {
            self.tokens.get(self.current + offset).is_some_and(|token| {
                !matches!(
                    token.token_type,
                    TokenType::String | TokenType::QuotedIdentifier
                ) && token
                    .text
                    .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                    && !crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS
                        .contains(&token.text.to_ascii_uppercase().as_str())
            })
        };
        let token_at = |offset: usize, token_type: TokenType| {
            self.tokens
                .get(self.current + offset)
                .is_some_and(|token| token.token_type == token_type)
        };
        let actions = OMITTED_NAME_ACTIONS
            .iter()
            .find(|(omits, _)| *omits == kind)
            .map_or(&[][..], |(_, actions)| *actions);
        let omitted = match self.text_at(0).as_deref() {
            Some(action @ "SET") if actions.contains(&action) => {
                word_at(1) && token_at(2, TokenType::Eq)
            }
            Some(action @ "UNSET") if actions.contains(&action) => {
                word_at(1) && (self.is_statement_end_at(2) || token_at(2, TokenType::Comma))
            }
            Some(action @ "REFRESH") if actions.contains(&action) => matches!(
                self.text_at(1).as_deref(),
                Some(
                    "OAUTH_CLIENT_SECRET_2"
                        | "OAUTH_CLIENT_SECRET"
                        | "SAML2_SNOWFLAKE_PRIVATE_KEY"
                        | "METADATA_URL"
                )
            ),
            _ => false,
        };
        match omitted {
            true => Ok((TableRef::new(""), false)),
            false => self.parse_alter_object_head(),
        }
    }

    /// Parse an object name: an identifier, never a string or an unquoted reserved keyword.
    pub(crate) fn parse_object_name(&mut self) -> Result<TableRef> {
        if self.check(TokenType::String)
            || self.is_snowflake_reserved_keyword()
            || !self.is_object_name_start()
        {
            return Err(self.parse_error("Expected an object name"));
        }
        let start = self.current;
        let name = self.parse_table_ref()?;
        for pair in self.tokens[start..self.current].windows(2) {
            let reserved = crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS
                .contains(&pair[1].text.to_ascii_uppercase().as_str());
            if pair[0].token_type == TokenType::Dot
                && pair[1].token_type != TokenType::QuotedIdentifier
                && reserved
            {
                return Err(
                    self.parse_error("Expected an object name part, not a reserved keyword")
                );
            }
        }
        Ok(name)
    }

    /// Whether the current token can start an object name.
    fn is_object_name_start(&self) -> bool {
        !self.is_at_end()
            && (self.is_identifier_token() || self.check_keyword())
            && !self.check(TokenType::Semicolon)
    }

    /// Parse a simple name, such as a contact purpose or a property: an identifier, not a reserved keyword.
    pub(super) fn parse_simple_name(&mut self) -> Result<Identifier> {
        if self.check(TokenType::String)
            || self.is_snowflake_reserved_keyword()
            || !self.is_object_name_start()
        {
            return Err(self.parse_error("Expected a name"));
        }
        self.expect_identifier_or_safe_keyword_with_quoted()
    }

    /// Parse a property value: a literal, TRUE, FALSE or a name.
    pub(super) fn parse_object_value(&mut self) -> Result<Expression> {
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
    pub(super) fn parse_string_value(&mut self) -> Result<Expression> {
        if !self.check(TokenType::String) {
            return Err(self.parse_error("Expected a string"));
        }
        self.parse_primary()
    }

    /// Consume one of `choices`, each a sequence of words, and return its words joined.
    pub(super) fn expect_alter_words(&mut self, choices: &[&[&str]]) -> Result<String> {
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
    pub(super) fn parse_object_name_list(&mut self) -> Result<Vec<Expression>> {
        let mut names = vec![Expression::Table(Box::new(self.parse_object_name()?))];
        while self.match_token(TokenType::Comma) {
            names.push(Expression::Table(Box::new(self.parse_object_name()?)));
        }
        Ok(names)
    }

    /// Parse the properties of a SET (`set`) or UNSET action; `graph` admits TASK_DEFAULTS and always reads CONTACT as contacts.
    pub(super) fn parse_alter_object_properties(
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
                properties.push(property(&property_kind(&name), None, value));
            }
            let comma = self.match_token(TokenType::Comma);
            if self.is_at_end() || self.check(TokenType::Semicolon) || !set && !comma {
                return Ok(properties);
            }
        }
    }

    /// Parse a possibly qualified name, such as a tag name, as one identifier.
    pub(super) fn parse_simple_qualified_name(&mut self) -> Result<Identifier> {
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

    /// Parse ALTER ALERT after its kind.
    fn parse_alter_alert(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["RESUME"],
            &["SUSPEND"],
            &["FROM", "TEMPLATE"],
            &["MODIFY", "CONDITION", "EXISTS"],
            &["MODIFY", "ACTION"],
            &["SET"],
            &["UNSET"],
        ])?;
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        match action.as_str() {
            "FROM TEMPLATE" => {
                operands.push(Expression::Table(Box::new(self.parse_object_name()?)));
                action.push_str(&format!(" {}", self.expect_alter_words(&[&["SET"]])?));
                properties = self.parse_alter_object_properties(true, false)?;
            }
            "MODIFY CONDITION EXISTS" => {
                self.expect_token(TokenType::LParen)?;
                operands.push(self.parse_statement()?);
                self.expect_token(TokenType::RParen)?;
            }
            "MODIFY ACTION" => operands.push(self.parse_statement()?),
            "SET" => properties = self.parse_alter_object_properties(true, false)?,
            "UNSET" => properties = self.parse_alter_object_properties(false, false)?,
            _ => {}
        }
        Ok(alter_object("ALERT", head, action, (operands, properties)))
    }

    /// Parse ALTER ONLINE FEATURE TABLE after its kind.
    fn parse_alter_online_feature_table(&mut self) -> Result<Expression> {
        let head = self.parse_alter_head_or_omitted(ONLINE_FEATURE_TABLE)?;
        let action = self.expect_alter_words(&[
            &["SUSPEND"],
            &["RESUME"],
            &["REFRESH"],
            &["RENAME", "TO"],
            &["SET"],
            &["UNSET"],
        ])?;
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        match action.as_str() {
            "RENAME TO" => operands.push(Expression::Table(Box::new(self.parse_object_name()?))),
            "SET" => properties = self.parse_alter_object_properties(true, false)?,
            "UNSET" => {
                self.expect_alter_words(&[&["TAG"]])?;
                loop {
                    let name = self.parse_simple_qualified_name()?;
                    properties.push(property("TAG", Some(name), None));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            _ => {}
        }
        Ok(alter_object(
            "ONLINE FEATURE TABLE",
            head,
            action,
            (operands, properties),
        ))
    }

    /// Parse ALTER VARIABLE after its kind.
    fn parse_alter_variable(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let action = self.expect_alter_words(&[
            &["RENAME", "TO"],
            &["SET", "DEFAULT"],
            &["DROP", "DEFAULT"],
            &["SET", "COMMENT"],
            &["UNSET", "COMMENT"],
        ])?;
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        match action.as_str() {
            "RENAME TO" => operands.push(Expression::Table(Box::new(self.parse_object_name()?))),
            "SET DEFAULT" => operands.push(self.parse_expression()?),
            "SET COMMENT" => {
                self.expect_token(TokenType::Eq)?;
                properties.push(property("COMMENT", None, Some(self.parse_string_value()?)));
            }
            _ => {}
        }
        Ok(alter_object(
            "VARIABLE",
            head,
            action,
            (operands, properties),
        ))
    }

    /// Parse ALTER FUNCTION after its kind: UDF, external, service and data metric functions.
    fn parse_alter_function(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let arguments = self.parse_routine_signature()?;
        let mut action = self.expect_alter_words(&[&["RENAME", "TO"], &["SET"], &["UNSET"]])?;
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        match action.as_str() {
            "RENAME TO" => operands.push(Expression::Table(Box::new(self.parse_object_name()?))),
            "SET" if self.check_identifier("SECURE") && self.is_statement_end_at(1) => {
                self.skip();
                action.push_str(" SECURE");
            }
            "SET" => properties = self.parse_routine_set_properties()?,
            _ => properties = self.parse_alter_object_properties(false, false)?,
        }
        let Expression::AlterObject(mut alter) =
            alter_object("FUNCTION", head, action, (operands, properties))
        else {
            return Err(self.parse_error("Expected an ALTER statement"));
        };
        alter.arguments = arguments;
        Ok(Expression::AlterObject(alter))
    }

    /// Whether the token `offset` tokens ahead ends the statement.
    pub(super) fn is_statement_end_at(&self, offset: usize) -> bool {
        self.tokens
            .get(self.current + offset)
            .is_none_or(|token| token.token_type == TokenType::Semicolon)
    }

    /// Parse `( [ <type> , ... ] )` or `( TABLE( <type> , ... ) [ , TABLE( ... ) ] )` after a routine name.
    pub(super) fn parse_routine_signature(&mut self) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::LParen)?;
        let mut arguments: Vec<Expression> = Vec::new();
        if self.match_token(TokenType::RParen) {
            return Ok(arguments);
        }
        loop {
            if self.check(TokenType::Table) && self.check_next(TokenType::LParen) {
                self.current += 2;
                let mut columns = vec![Expression::DataType(self.parse_data_type()?)];
                while self.match_token(TokenType::Comma) {
                    columns.push(Expression::DataType(self.parse_data_type()?));
                }
                self.expect_token(TokenType::RParen)?;
                arguments.push(Expression::Tuple(Box::new(Tuple {
                    expressions: columns,
                })));
            } else {
                arguments.push(Expression::DataType(self.parse_data_type()?));
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(arguments)
    }

    /// Parse the properties after a routine's SET: TAG pairs, or `<name> = <value>` with list values.
    fn parse_routine_set_properties(&mut self) -> Result<Vec<ObjectProperty>> {
        if self.check_identifier("TAG") && !self.check_next(TokenType::Eq) {
            return self.parse_alter_object_properties(true, false);
        }
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let name = self.parse_simple_name()?;
            self.expect_token(TokenType::Eq)?;
            let value = if self.check(TokenType::LParen) {
                self.parse_routine_value_list()?
            } else {
                self.parse_object_value()?
            };
            properties.push(property(&property_kind(&name), None, Some(value)));
            self.match_token(TokenType::Comma);
            if self.is_at_end() || self.check(TokenType::Semicolon) {
                return Ok(properties);
            }
        }
    }

    /// Parse `( [ <item> [ , ... ] ] )`, each item a value or a `'<key>' = <value>` pair.
    pub(super) fn parse_routine_value_list(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut items: Vec<Expression> = Vec::new();
        if !self.check(TokenType::RParen) {
            loop {
                let item = self.parse_object_value()?;
                items.push(if self.match_token(TokenType::Eq) {
                    Expression::Tuple(Box::new(Tuple {
                        expressions: vec![item, self.parse_object_value()?],
                    }))
                } else {
                    item
                });
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions: items })))
    }
}

/// An ALTER of `kind` on the named object, with its action, operands and properties.
pub(super) fn alter_object(
    kind: &str,
    (name, if_exists): (TableRef, bool),
    action: String,
    (operands, properties): (Vec<Expression>, Vec<ObjectProperty>),
) -> Expression {
    Expression::AlterObject(Box::new(AlterObject {
        kind: kind.to_owned(),
        name: Some(name),
        arguments: Vec::new(),
        if_exists,
        action,
        operands,
        properties,
    }))
}

/// A property's kind from its name: upper-cased when unquoted, as written when quoted.
pub(super) fn property_kind(name: &Identifier) -> String {
    match name.quoted {
        true => name.name.clone(),
        false => name.name.to_ascii_uppercase(),
    }
}

/// One property of kind `kind`.
pub(super) fn property(
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
