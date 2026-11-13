//! Snowflake ALTER of APPLICATION, APPLICATION PACKAGE, APPLICATION SERVICE, BACKUP SET, CORTEX SEARCH SERVICE, DATABASE, SEMANTIC VIEW and STREAMLIT.

use crate::parser::_helpers::ddl::snowflake_alter::alter_object;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::_helpers::ddl::snowflake_alter::property_kind;
use crate::parser::constants::CONTACT;
use crate::parser::constants::TAG;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::expressions::schema_objects::models::ObjectProperty;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The options that may follow a release directive's VERSION and PATCH.
const UPGRADE_OPTIONS: &[&str] = &[
    "UPGRADE_AFTER",
    "UPGRADE_IN_MAINTENANCE_WINDOW",
    "UPGRADE_DEADLINE",
];

type Parts = (Vec<Expression>, Vec<ObjectProperty>);

/// The words that start an ALTER APPLICATION PACKAGE action.
const PACKAGE_ACTIONS: &[&str] = &[
    "RENAME",
    "ADD",
    "REGISTER",
    "DEREGISTER",
    "DROP",
    "MODIFY",
    "SET",
    "UNSET",
];

/// The words that start an ALTER APPLICATION SERVICE action.
const SERVICE_ACTIONS: &[&str] = &["SET", "UNSET", "SUSPEND", "RESUME", "UPGRADE"];

impl Parser {
    /// Parse a Snowflake ALTER of a kind this module owns, or None when the next words name another kind.
    pub(crate) fn parse_alter_snowflake_objects(&mut self) -> Result<Option<Expression>> {
        for kind in ["APPLICATION", "DATABASE"] {
            if self.check_text_seq(&[kind, "ROLE"])
                && !self
                    .text_at(2)
                    .is_some_and(|word| matches!(word.as_str(), "RENAME" | "SET" | "UNSET"))
            {
                self.current += 2;
                return self
                    .parse_alter_scoped_role(&format!("{kind} ROLE"))
                    .map(Some);
            }
        }
        if self.is_alter_kind("PACKAGE", PACKAGE_ACTIONS) {
            self.current += 2;
            return self.parse_alter_application_package().map(Some);
        }
        if self.is_alter_kind("SERVICE", SERVICE_ACTIONS) {
            self.current += 2;
            return self.parse_alter_application_service().map(Some);
        }
        if self.match_text_seq(&["APPLICATION"]) {
            return self.parse_alter_application().map(Some);
        }
        if self.match_text_seq(&["BACKUP", "SET"]) {
            return self.parse_alter_backup_set().map(Some);
        }
        if self.match_text_seq(&["CORTEX", "SEARCH", "SERVICE"]) {
            return self.parse_alter_cortex_search_service().map(Some);
        }
        if self.match_token(TokenType::Database) {
            return self.parse_alter_database().map(Some);
        }
        if self.match_text_seq(&["SEMANTIC", "VIEW"]) {
            return self.parse_alter_semantic_view().map(Some);
        }
        if self.match_text_seq(&["STREAMLIT"]) {
            return self.parse_alter_streamlit().map(Some);
        }
        if self.match_text_seq(&["DYNAMIC", "TABLE"]) {
            return self.parse_alter_dynamic_table().map(Some);
        }
        if self.check_identifier("DYNAMIC") && !self.check_next_identifier("ICEBERG") {
            return Err(self.parse_error("Snowflake has ALTER DYNAMIC TABLE"));
        }
        if self.check_text_seq(&["CORTEX"])
            || self.check_text_seq(&["SEMANTIC"])
            || self.check_text_seq(&["BACKUP"]) && !self.check_text_seq(&["BACKUP", "POLICY"])
        {
            return Err(self.parse_error(format!(
                "Snowflake has no ALTER {} {}",
                self.peek_text().to_ascii_uppercase(),
                self.text_at(1).unwrap_or_default()
            )));
        }
        Ok(None)
    }

    /// Parse an object name as an operand.
    pub(super) fn object_operand(&mut self) -> Result<Expression> {
        Ok(Expression::Table(Box::new(self.parse_object_name()?)))
    }

    /// Parse a comma-separated list of object names as one operand.
    fn object_list_operand(&mut self) -> Result<Expression> {
        let mut names = vec![self.object_operand()?];
        while self.match_token(TokenType::Comma) {
            names.push(self.object_operand()?);
        }
        Ok(tuple(names))
    }

    /// Parse a comma-separated list of account identifiers, each `[ <organization_name> . ] <account_name>`.
    fn account_list_operand(&mut self) -> Result<Expression> {
        let mut accounts = vec![self.account_operand()?];
        while self.match_token(TokenType::Comma) {
            accounts.push(self.account_operand()?);
        }
        Ok(tuple(accounts))
    }

    /// Parse `[ <organization_name> . ] <account_name>`, keeping each part and its quoting.
    fn account_operand(&mut self) -> Result<Expression> {
        let first = self.parse_simple_name()?;
        let mut account = TableRef::new("");
        if self.match_token(TokenType::Dot) {
            account.name = self.parse_simple_name()?;
            account.schema = Some(first);
        } else {
            account.name = first;
        }
        Ok(Expression::Table(Box::new(account)))
    }

    /// Parse a parenthesized list of account identifiers.
    fn parse_account_list(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let list = self.account_list_operand()?;
        self.expect_token(TokenType::RParen)?;
        Ok(list)
    }

    /// Parse an integer literal.
    fn number_operand(&mut self) -> Result<Expression> {
        if !self.check(TokenType::Number) {
            return Err(self.parse_error("Expected a number"));
        }
        self.parse_primary()
    }

    /// Parse `RENAME TO <name>` or `SWAP WITH <name>` after its first word.
    fn parse_alter_target(&mut self, second: &str) -> Result<Parts> {
        self.expect_alter_words(&[&[second]])?;
        Ok((vec![self.object_operand()?], vec![]))
    }

    /// Parse a property value: a parenthesized list, NULL, a literal or a name.
    pub(super) fn parse_alter_value(&mut self) -> Result<Expression> {
        if self.match_token(TokenType::LParen) {
            let mut items: Vec<Expression> = Vec::new();
            if self.match_token(TokenType::RParen) {
                return Ok(tuple(items));
            }
            loop {
                let item = self.parse_object_value()?;
                if self.match_token(TokenType::Eq) {
                    items.push(tuple(vec![item, self.parse_object_value()?]));
                } else {
                    items.push(item);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            return Ok(tuple(items));
        }
        if self.check(TokenType::Null) {
            return self.parse_primary();
        }
        self.parse_object_value()
    }

    /// Parse a parenthesized list of string literals.
    pub(super) fn parse_string_list(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut items = vec![self.parse_string_value()?];
        while self.match_token(TokenType::Comma) {
            items.push(self.parse_string_value()?);
        }
        self.expect_token(TokenType::RParen)?;
        Ok(tuple(items))
    }

    /// Parse a parenthesized list of object names.
    fn parse_name_list(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let list = self.object_list_operand()?;
        self.expect_token(TokenType::RParen)?;
        Ok(list)
    }

    /// Parse `<name> = <value>` for each name of `names` that comes next, in any order.
    fn parse_named_options(&mut self, names: &[&str]) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while let Some(name) = names
            .iter()
            .find(|name| self.check_text_seq(&[name]) && self.check_next(TokenType::Eq))
        {
            self.skip();
            self.skip();
            properties.push(property(name, None, Some(self.parse_alter_value()?)));
        }
        Ok(properties)
    }

    /// Parse `<name> = <value>` for one required name.
    fn parse_required_option(&mut self, name: &str) -> Result<ObjectProperty> {
        self.expect_alter_words(&[&[name]])?;
        self.expect_token(TokenType::Eq)?;
        Ok(property(name, None, Some(self.parse_alter_value()?)))
    }

    /// Parse `<name> = <number>` for one required name; Snowflake refuses a string there (errno 1003).
    fn parse_required_number_option(&mut self, name: &str) -> Result<ObjectProperty> {
        self.expect_alter_words(&[&[name]])?;
        self.expect_token(TokenType::Eq)?;
        if !self.check(TokenType::Number) {
            return Err(self.parse_error(format!("Expected a number after {name} =")));
        }
        Ok(property(name, None, Some(self.parse_primary()?)))
    }

    /// Parse the properties of a SET (`set`) or UNSET: TAG and CONTACT lists, PRIMARY KEY, or named properties.
    pub(super) fn parse_alter_settings(&mut self, set: bool) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        let mut contact_list_ends_set = false;
        loop {
            let named = self.check_next(TokenType::Eq) || self.check_next(TokenType::Comma) || {
                let next = self.current + 1;
                next >= self.tokens.len() || self.tokens[next].token_type == TokenType::Semicolon
            };
            if !named && (self.check_text_seq(&["TAG"]) || self.check_text_seq(&["CONTACT"])) {
                let kind = self.peek_text().to_ascii_uppercase();
                self.skip();
                loop {
                    let name = if kind == TAG {
                        self.parse_simple_qualified_name()?
                    } else {
                        self.parse_simple_name()?
                    };
                    let value = if !set {
                        None
                    } else {
                        self.expect_token(TokenType::Eq)?;
                        Some(if kind == CONTACT {
                            self.object_operand()?
                        } else {
                            self.parse_object_value()?
                        })
                    };
                    properties.push(property(&kind, Some(name), value));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                    if kind == CONTACT {
                        contact_list_ends_set = true;
                    }
                }
            } else if self.match_text_seq(&["PRIMARY", "KEY"]) {
                let value = if set {
                    self.expect_token(TokenType::Eq)?;
                    Some(self.parse_name_list()?)
                } else {
                    None
                };
                properties.push(property("PRIMARY KEY", None, value));
            } else {
                let name = self.parse_simple_name()?;
                let value = if set {
                    self.expect_token(TokenType::Eq)?;
                    Some(self.parse_alter_value()?)
                } else if self.match_token(TokenType::Eq) {
                    Some(self.object_operand()?)
                } else {
                    None
                };
                properties.push(property(&property_kind(&name), None, value));
            }
            let comma = self.match_token(TokenType::Comma);
            if self.is_at_end() || self.check(TokenType::Semicolon) || !set && !comma {
                return Ok(properties);
            }
            if contact_list_ends_set {
                return Err(self.parse_error("Expected the end of the statement after CONTACT"));
            }
        }
    }

    /// Parse `SET ...` or `UNSET ...` after the word.
    pub(super) fn parse_alter_set(&mut self, set: bool) -> Result<Parts> {
        Ok((vec![], self.parse_alter_settings(set)?))
    }

    /// Parse `<policy_name> [ FORCE ]` after `{ FEATURE | MAINTENANCE | BACKUP } POLICY`, with the action's suffix.
    fn parse_alter_policy(&mut self) -> Result<(Parts, String)> {
        let policy = self.object_operand()?;
        let suffix = if self.match_text_seq(&["FORCE"]) {
            " FORCE"
        } else {
            ""
        };
        Ok(((vec![policy], vec![]), suffix.to_owned()))
    }

    /// Whether APPLICATION `word` starts that kind; before an application action the next word cannot start, `word` names an application.
    fn is_alter_kind(&self, word: &str, actions: &[&str]) -> bool {
        let application_action = self
            .text_at(2)
            .is_some_and(|next| matches!(next.as_str(), "RENAME" | "SET" | "UNSET" | "UPGRADE"));
        let kind_action = self
            .text_at(3)
            .is_some_and(|next| actions.contains(&next.as_str()));
        self.check_text_seq(&["APPLICATION", word]) && (!application_action || kind_action)
    }

    /// Parse ALTER DATABASE after its kind.
    fn parse_alter_database(&mut self) -> Result<Expression> {
        let head = self.parse_alter_head_or_omitted("DATABASE")?;
        let mut action = self.expect_alter_words(&[
            &["RENAME"],
            &["SWAP"],
            &["RESUME", "DISCOVERY"],
            &["SUSPEND", "DISCOVERY"],
            &["REFRESH"],
            &["PRIMARY"],
            &["ENABLE", "REPLICATION"],
            &["ENABLE", "FAILOVER"],
            &["DISABLE", "REPLICATION"],
            &["DISABLE", "FAILOVER"],
            &["SET", "FEATURE", "POLICY"],
            &["UNSET", "FEATURE", "POLICY"],
            &["UPDATE", "LINKED_CATALOG"],
            &["SET"],
            &["UNSET"],
        ])?;
        let parts = match action.as_str() {
            "RENAME" => self.parse_alter_target("TO")?,
            "SWAP" => self.parse_alter_target("WITH")?,
            "SET FEATURE POLICY" => {
                let (parts, suffix) = self.parse_alter_policy()?;
                action.push_str(&suffix);
                parts
            }
            "SET" => self.parse_alter_set(true)?,
            "UNSET" => self.parse_alter_set(false)?,
            "UPDATE LINKED_CATALOG" => {
                let (parts, suffix) = self.parse_linked_catalog_update()?;
                action.push_str(&suffix);
                parts
            }
            _ if action.starts_with("ENABLE") || action.starts_with("DISABLE") => {
                let mut operands: Vec<Expression> = Vec::new();
                if action.starts_with("ENABLE") || self.check_text_seq(&["TO"]) {
                    self.expect_alter_words(&[&["TO", "ACCOUNTS"]])?;
                    operands.push(self.account_list_operand()?);
                }
                if matches!(action.as_str(), "ENABLE REPLICATION")
                    && self.match_text_seq(&["IGNORE", "EDITION", "CHECK"])
                {
                    action.push_str(" IGNORE EDITION CHECK");
                }
                (operands, vec![])
            }
            _ => (vec![], vec![]),
        };
        Ok(alter_object("DATABASE", head, action, parts))
    }

    /// Parse the change after `UPDATE LINKED_CATALOG`, with the action's suffix.
    fn parse_linked_catalog_update(&mut self) -> Result<(Parts, String)> {
        let change = self.expect_alter_words(&[&["ADD"], &["REMOVE"], &["SET"], &["UNSET"]])?;
        let mut suffix: String = format!(" {change}");
        let namespaces: &[&[&str]] = &[&["ALLOWED_NAMESPACES"], &["BLOCKED_NAMESPACES"]];
        match change.as_str() {
            "ADD" | "REMOVE" => {
                let list = self.parse_string_list()?;
                let word = match change.as_str() {
                    "ADD" => "TO",
                    _ => "FROM",
                };
                self.expect_alter_words(&[&[word]])?;
                let target = self.expect_alter_words(namespaces)?;
                suffix.push_str(&format!(" {word} {target}"));
                Ok(((vec![list], vec![]), suffix))
            }
            "SET" => {
                let name = self.expect_alter_words(&[
                    &["ALLOWED_WRITE_OPERATIONS"],
                    &["SYNC_INTERVAL_SECONDS"],
                ])?;
                self.expect_token(TokenType::Eq)?;
                let value = match name.as_str() {
                    "SYNC_INTERVAL_SECONDS" => self.parse_object_value()?,
                    _ => {
                        let mode = self.expect_alter_words(&[&["NONE"], &["ALL"]])?;
                        Expression::Identifier(Identifier::new(mode))
                    }
                };
                Ok(((vec![], vec![property(&name, None, Some(value))]), suffix))
            }
            _ => {
                let target = self.expect_alter_words(namespaces)?;
                suffix.push_str(&format!(" {target}"));
                Ok(((vec![], vec![]), suffix))
            }
        }
    }

    /// Parse ALTER APPLICATION after its kind.
    fn parse_alter_application(&mut self) -> Result<Expression> {
        if let Some(action) = [
            ["SET", "SPECIFICATION"].as_slice(),
            &["DROP", "SPECIFICATION"],
            &["SET", "CONFIGURATION", "DEFINITION"],
            &["DROP", "CONFIGURATION", "DEFINITION"],
        ]
        .into_iter()
        .find(|words| self.check_text_seq(words))
        {
            self.current += action.len();
            let name = self.object_operand()?;
            let properties = if matches!(action[0], "SET") {
                self.parse_alter_settings(true)?
            } else {
                vec![]
            };
            let head = (TableRef::new(""), false);
            return Ok(alter_object(
                "APPLICATION",
                head,
                action.join(" "),
                (vec![name], properties),
            ));
        }
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["RENAME"],
            &["SET", "FEATURE", "POLICY"],
            &["SET", "MAINTENANCE", "POLICY"],
            &["SET", "SHARED", "TELEMETRY", "EVENTS"],
            &["SET", "CONFIGURATION"],
            &["UNSET", "FEATURE", "POLICY"],
            &["UNSET", "MAINTENANCE", "POLICY"],
            &["UNSET", "REFERENCES"],
            &["UNSET", "CONFIGURATION"],
            &["SET"],
            &["UNSET"],
            &["UPGRADE"],
            &["APPROVE", "SPECIFICATION"],
            &["DECLINE", "SPECIFICATION"],
        ])?;
        let parts = match action.as_str() {
            "RENAME" => self.parse_alter_target("TO")?,
            "SET FEATURE POLICY" | "SET MAINTENANCE POLICY" => {
                let (parts, suffix) = self.parse_alter_policy()?;
                action.push_str(&suffix);
                parts
            }
            "SET SHARED TELEMETRY EVENTS" => (vec![self.parse_string_list()?], vec![]),
            "SET CONFIGURATION" => {
                let name = self.object_operand()?;
                (vec![name], vec![self.parse_required_option("VALUE")?])
            }
            "UNSET CONFIGURATION"
                if self.is_statement_end_at(0) || self.check(TokenType::Comma) =>
            {
                self.current -= 1;
                action = "UNSET".to_owned();
                self.parse_alter_set(false)?
            }
            "UNSET CONFIGURATION" => (vec![self.object_operand()?], vec![]),
            "UNSET REFERENCES" => {
                if self.check(TokenType::LParen) {
                    (vec![self.parse_string_list()?], vec![])
                } else {
                    (vec![], vec![])
                }
            }
            "SET" => self.parse_alter_set(true)?,
            "UNSET" => self.parse_alter_set(false)?,
            "UPGRADE" => {
                if self.match_text_seq(&["USING", "VERSION"]) {
                    action.push_str(" USING VERSION");
                    let mut operands = vec![self.object_operand()?];
                    if self.match_text_seq(&["PATCH"]) {
                        action.push_str(" PATCH");
                        operands.push(self.number_operand()?);
                    }
                    (operands, vec![])
                } else if self.match_text_seq(&["USING"]) {
                    action.push_str(" USING");
                    (vec![self.parse_string_value()?], vec![])
                } else {
                    (vec![], vec![])
                }
            }
            "APPROVE SPECIFICATION" | "DECLINE SPECIFICATION" => {
                let name = self.object_operand()?;
                (
                    vec![name],
                    vec![self.parse_required_number_option("SEQUENCE_NUMBER")?],
                )
            }
            _ => (vec![], vec![]),
        };
        Ok(alter_object("APPLICATION", head, action, parts))
    }

    /// Parse ALTER APPLICATION PACKAGE after its kind.
    fn parse_alter_application_package(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["RENAME"],
            &["ADD", "VERSION"],
            &["REGISTER", "VERSION"],
            &["DEREGISTER", "VERSION"],
            &["DROP", "VERSION"],
            &["ADD", "PATCH"],
            &["MODIFY", "RELEASE", "CHANNEL"],
            &["MODIFY", "RELEASE", "DIRECTIVE"],
            &["SET", "DEFAULT", "RELEASE", "DIRECTIVE"],
            &["SET", "RELEASE", "DIRECTIVE"],
            &["UNSET", "RELEASE", "DIRECTIVE"],
            &["SET"],
            &["UNSET"],
        ])?;
        let parts = match action.as_str() {
            "RENAME" => self.parse_alter_target("TO")?,
            "ADD VERSION" | "REGISTER VERSION" => {
                let mut operands: Vec<Expression> = Vec::new();
                if !self.check_text_seq(&["USING"]) {
                    operands.push(self.object_operand()?);
                }
                self.parse_version_source(operands)?
            }
            "ADD PATCH" => {
                let mut operands: Vec<Expression> = Vec::new();
                if self.check(TokenType::Number) {
                    operands.push(self.number_operand()?);
                }
                self.expect_alter_words(&[&["FOR", "VERSION"]])?;
                if !self.check_text_seq(&["USING"]) {
                    operands.push(self.object_operand()?);
                }
                self.parse_version_source(operands)?
            }
            "DEREGISTER VERSION" | "DROP VERSION" => (vec![self.object_operand()?], vec![]),
            "MODIFY RELEASE CHANNEL" => {
                let channel = self.object_operand()?;
                let change = self.expect_alter_words(&[
                    &["ADD", "VERSION"],
                    &["DROP", "VERSION"],
                    &["ADD", "ACCOUNTS"],
                    &["REMOVE", "ACCOUNTS"],
                    &["SET", "ACCOUNTS"],
                    &["MODIFY", "RELEASE", "DIRECTIVE"],
                    &["SET", "DEFAULT", "RELEASE", "DIRECTIVE"],
                    &["SET", "RELEASE", "DIRECTIVE"],
                    &["UNSET", "RELEASE", "DIRECTIVE"],
                ])?;
                action.push_str(&format!(" {change}"));
                let (mut operands, properties) = match change.as_str() {
                    "ADD VERSION" | "DROP VERSION" => (vec![self.object_operand()?], vec![]),
                    "ADD ACCOUNTS" | "REMOVE ACCOUNTS" | "SET ACCOUNTS" => {
                        self.expect_token(TokenType::Eq)?;
                        (vec![self.parse_account_list()?], vec![])
                    }
                    _ => {
                        let (parts, suffix) = self.parse_release_directive(&change, &action)?;
                        action.push_str(&suffix);
                        parts
                    }
                };
                operands.insert(0, channel);
                (operands, properties)
            }
            _ if action.contains("RELEASE DIRECTIVE") => {
                let change = action.clone();
                let (parts, suffix) = self.parse_release_directive(&change, &change)?;
                action.push_str(&suffix);
                parts
            }
            "SET" => self.parse_alter_set(true)?,
            _ => {
                let parts = self.parse_alter_set(false)?;
                if parts.1.len() > 1 && parts.1.iter().any(|p| p.kind != TAG) {
                    return Err(self.parse_error("Snowflake unsets one package property at a time"));
                }
                parts
            }
        };
        Ok(alter_object("APPLICATION PACKAGE", head, action, parts))
    }

    /// Parse `USING <path> [ LABEL = '<label>' ]` after a version or patch.
    fn parse_version_source(&mut self, mut operands: Vec<Expression>) -> Result<Parts> {
        self.expect_alter_words(&[&["USING"]])?;
        operands.push(self.parse_string_value()?);
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if self.match_text_seq(&["LABEL"]) {
            self.expect_token(TokenType::Eq)?;
            properties.push(property("LABEL", None, Some(self.parse_string_value()?)));
        }
        Ok((operands, properties))
    }

    /// Parse the rest of a release directive change named by `change`, with the action's suffix.
    fn parse_release_directive(&mut self, change: &str, action: &str) -> Result<(Parts, String)> {
        let mut suffix: String = String::new();
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if !matches!(change, "SET DEFAULT RELEASE DIRECTIVE") {
            operands.push(self.object_operand()?);
        }
        if change.starts_with("UNSET") {
            return Ok(((operands, properties), suffix));
        }
        let mut version_required = true;
        let accounts = if change.starts_with("SET RELEASE") {
            self.expect_alter_words(&[&["ACCOUNTS"]])?;
            true
        } else if change.starts_with("MODIFY") {
            match self.match_one_of(&["ADD", "REMOVE"]) {
                Some(word) => {
                    self.expect_alter_words(&[&["ACCOUNTS"]])?;
                    suffix.push_str(&format!(" {word} ACCOUNTS"));
                    version_required = false;
                    true
                }
                None => false,
            }
        } else {
            false
        };
        if accounts {
            self.expect_token(TokenType::Eq)?;
            properties.push(property("ACCOUNTS", None, Some(self.parse_account_list()?)));
        }
        if version_required || self.check_text_seq(&["VERSION"]) {
            self.expect_alter_words(&[&["VERSION"]])?;
            self.expect_token(TokenType::Eq)?;
            properties.push(property("VERSION", None, Some(self.object_operand()?)));
            self.expect_alter_words(&[&["PATCH"]])?;
            self.expect_token(TokenType::Eq)?;
            properties.push(property("PATCH", None, Some(self.number_operand()?)));
        }
        if version_required {
            properties.extend(self.parse_named_options(UPGRADE_OPTIONS)?);
        }
        if !action.contains("REMOVE")
            && !suffix.contains("REMOVE")
            && self.match_text_seq(&["FORCE"])
        {
            suffix.push_str(" FORCE");
        }
        Ok(((operands, properties), suffix))
    }

    /// Parse ALTER APPLICATION SERVICE after its kind.
    fn parse_alter_application_service(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["SET"],
            &["UNSET"],
            &["SUSPEND"],
            &["RESUME"],
            &["UPGRADE"],
        ])?;
        let parts = match action.as_str() {
            "SET" => self.parse_alter_set(true)?,
            "UNSET" => self.parse_alter_set(false)?,
            "UPGRADE" if self.match_text_seq(&["TO", "VERSION"]) => {
                action.push_str(" TO VERSION");
                (vec![self.object_operand()?], vec![])
            }
            _ => (vec![], vec![]),
        };
        Ok(alter_object("APPLICATION SERVICE", head, action, parts))
    }

    /// Parse ALTER BACKUP SET after its kind.
    fn parse_alter_backup_set(&mut self) -> Result<Expression> {
        let head = (self.parse_object_name()?, false);
        let mut action = self.expect_alter_words(&[
            &["ADD", "BACKUP"],
            &["APPLY", "BACKUP", "POLICY"],
            &["DELETE", "BACKUP", "IDENTIFIER"],
            &["MODIFY", "BACKUP", "IDENTIFIER"],
            &["RENAME"],
            &["RESUME", "BACKUP"],
            &["SUSPEND", "BACKUP"],
            &["SET"],
            &["UNSET"],
        ])?;
        let parts = match action.as_str() {
            "APPLY BACKUP POLICY" => {
                let (parts, suffix) = self.parse_alter_policy()?;
                action.push_str(&suffix);
                parts
            }
            "DELETE BACKUP IDENTIFIER" => (vec![self.parse_string_value()?], vec![]),
            "MODIFY BACKUP IDENTIFIER" => {
                let id = self.parse_string_value()?;
                let change = self.expect_alter_words(&[
                    &["ADD", "LEGAL", "HOLD"],
                    &["REMOVE", "LEGAL", "HOLD"],
                    &["SET", "COMMENT"],
                    &["UNSET", "COMMENT"],
                ])?;
                action.push_str(&format!(" {change}"));
                let mut properties: Vec<ObjectProperty> = Vec::new();
                if matches!(change.as_str(), "SET COMMENT") {
                    self.expect_token(TokenType::Eq)?;
                    let comment = self.parse_string_value()?;
                    properties.push(property("COMMENT", None, Some(comment)));
                }
                (vec![id], properties)
            }
            "RENAME" => self.parse_alter_target("TO")?,
            "RESUME BACKUP" | "SUSPEND BACKUP" => {
                if let Some(which) = self.match_one_of(&["CREATION", "EXPIRATION"]) {
                    action.push_str(&format!(" {which}"));
                }
                self.expect_alter_words(&[&["POLICY"]])?;
                action.push_str(" POLICY");
                (vec![], vec![])
            }
            "SET" => self.parse_alter_set(true)?,
            "UNSET" => self.parse_alter_set(false)?,
            _ => (vec![], vec![]),
        };
        Ok(alter_object("BACKUP SET", head, action, parts))
    }

    /// Parse ALTER APPLICATION ROLE or ALTER DATABASE ROLE after its kind.
    fn parse_alter_scoped_role(&mut self, kind: &str) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let action = self.expect_alter_words(&[&["RENAME"], &["SET"], &["UNSET"]])?;
        let parts = match action.as_str() {
            "RENAME" => self.parse_alter_target("TO")?,
            "SET" => self.parse_alter_set(true)?,
            _ => self.parse_alter_set(false)?,
        };
        Ok(alter_object(kind, head, action, parts))
    }

    /// Consume the first of `words` that comes next and return it.
    fn match_one_of(&mut self, words: &[&str]) -> Option<String> {
        let word = words.iter().find(|word| self.check_text_seq(&[word]))?;
        self.skip();
        Some((*word).to_owned())
    }

    /// Parse ALTER CORTEX SEARCH SERVICE after its kind.
    fn parse_alter_cortex_search_service(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["ADD", "SCORING", "PROFILE"],
            &["DROP", "SCORING", "PROFILE"],
            &["REFRESH"],
            &["SUSPEND"],
            &["RESUME"],
            &["SET", "ATTRIBUTES"],
            &["UNSET", "ATTRIBUTES"],
            &["SET"],
            &["UNSET"],
        ])?;
        let parts = match action.as_str() {
            "ADD SCORING PROFILE" => {
                if self.match_text_seq(&["IF", "NOT", "EXISTS"]) {
                    action.push_str(" IF NOT EXISTS");
                }
                let name = self.object_operand()?;
                (vec![name, self.parse_object_value()?], vec![])
            }
            "DROP SCORING PROFILE" => {
                if self.match_keywords(&[TokenType::If, TokenType::Exists]) {
                    action.push_str(" IF EXISTS");
                }
                (vec![self.object_operand()?], vec![])
            }
            "SUSPEND" | "RESUME" => {
                if let Some(which) = self.match_one_of(&["INDEXING", "SERVING"]) {
                    action.push_str(&format!(" {which}"));
                }
                (vec![], vec![])
            }
            "SET ATTRIBUTES" => (vec![self.parse_name_list()?], vec![]),
            "SET" => self.parse_alter_set(true)?,
            "UNSET" => {
                if !self.check_text_seq(&["PRIMARY", "KEY"])
                    && !(self.check_text_seq(&["TAG"])
                        && self.text_at(1).is_some()
                        && !self.check_next(TokenType::Semicolon))
                {
                    return Err(self.parse_error("Expected PRIMARY KEY or TAG"));
                }
                self.parse_alter_set(false)?
            }
            _ => (vec![], vec![]),
        };
        Ok(alter_object("CORTEX SEARCH SERVICE", head, action, parts))
    }

    /// Parse ALTER SEMANTIC VIEW after its kind.
    fn parse_alter_semantic_view(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let action = self.expect_alter_words(&[
            &["ADD", "MATERIALIZATION"],
            &["DROP", "MATERIALIZATION"],
            &["REFRESH", "MATERIALIZATION"],
            &["RESUME", "MATERIALIZATION"],
            &["SUSPEND", "MATERIALIZATION"],
            &["RENAME"],
            &["SET"],
            &["UNSET"],
        ])?;
        let parts = match action.as_str() {
            "ADD MATERIALIZATION" => self.parse_materialization()?,
            "RENAME" => self.parse_alter_target("TO")?,
            "SET" => self.parse_alter_set(true)?,
            "UNSET" => self.parse_alter_set(false)?,
            _ => (vec![self.object_operand()?], vec![]),
        };
        Ok(alter_object("SEMANTIC VIEW", head, action, parts))
    }

    /// Parse a materialization's name, options, dimensions, metrics and filter.
    fn parse_materialization(&mut self) -> Result<Parts> {
        let mut operands = vec![self.object_operand()?];
        let mut properties = vec![self.parse_required_option("WAREHOUSE")?];
        properties.extend(self.parse_named_options(&["REFRESH_MODE"])?);
        if self.match_text_seq(&["IMMUTABLE", "WHERE"]) {
            properties.push(property(
                "IMMUTABLE WHERE",
                None,
                Some(self.parse_condition()?),
            ));
        }
        self.expect_alter_words(&[&["AS", "DIMENSIONS"]])?;
        operands.push(self.object_list_operand()?);
        self.expect_alter_words(&[&["METRICS"]])?;
        operands.push(self.object_list_operand()?);
        if self.match_token(TokenType::Where) {
            properties.push(property("WHERE", None, Some(self.parse_condition()?)));
        }
        Ok((operands, properties))
    }

    /// Parse a parenthesized condition.
    fn parse_condition(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let condition = self.parse_expression()?;
        self.expect_token(TokenType::RParen)?;
        Ok(condition)
    }

    /// Parse ALTER STREAMLIT after its kind.
    fn parse_alter_streamlit(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["ABORT"],
            &["ADD", "LIVE", "VERSION", "FROM", "LAST"],
            &["COMMIT"],
            &["PULL"],
            &["PUSH"],
            &["RENAME"],
            &["SET"],
            &["UNSET"],
        ])?;
        let parts = match action.as_str() {
            "PUSH" => {
                let mut operands: Vec<Expression> = Vec::new();
                if self.match_text_seq(&["TO"]) {
                    action.push_str(" TO");
                    operands.push(self.parse_string_value()?);
                }
                let properties = self.parse_named_options(&[
                    "GIT_CREDENTIALS",
                    "USERNAME",
                    "PASSWORD",
                    "NAME",
                    "EMAIL",
                    "COMMENT",
                ])?;
                (operands, properties)
            }
            "RENAME" => self.parse_alter_target("TO")?,
            "SET" => self.parse_alter_set(true)?,
            "UNSET" => self.parse_alter_set(false)?,
            _ => (vec![], vec![]),
        };
        Ok(alter_object("STREAMLIT", head, action, parts))
    }
}

/// A parenthesized list of expressions.
pub(super) fn tuple(expressions: Vec<Expression>) -> Expression {
    Expression::Tuple(Box::new(Tuple { expressions }))
}
