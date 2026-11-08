//! Parser ddl: Snowflake ALTER of account-level objects: failover and replication groups, connections, integrations and organization objects.

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::schema_objects::models::ObjectProperty;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::Tuple;
use crate::parser::_helpers::ddl::snowflake_alter::alter_object;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::_helpers::ddl::snowflake_alter::property_kind;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// The operands and properties of one ALTER action.
type ActionParts = (Vec<Expression>, Vec<ObjectProperty>);

/// The lists a group's ADD and REMOVE change.
const ALLOWED_LISTS: &[&[&str]] = &[
    &["ALLOWED_ACCOUNTS"],
    &["ALLOWED_DATABASES"],
    &["ALLOWED_EXTERNAL_VOLUMES"],
    &["ALLOWED_SHARES"],
];

impl Parser {
    /// Parse a Snowflake ALTER of a group, a connection or an organization object, or None when the next words name another kind.
    pub(super) fn parse_alter_snowflake_group(&mut self) -> Result<Option<Expression>> {
        for kind in ["FAILOVER", "REPLICATION"] {
            if self.check_identifier(kind) {
                self.skip();
                self.expect_alter_words(&[&["GROUP"]])?;
                return self.parse_alter_group(kind).map(Some);
            }
        }
        if self.match_identifier("CONNECTION") {
            return self.parse_alter_connection().map(Some);
        }
        if self.match_identifier("ACCOUNT") {
            return self.parse_alter_account().map(Some);
        }
        if let Some(kind) = self.match_integration_kind()? {
            return self.parse_alter_integration(&kind).map(Some);
        }
        if self.match_identifier("ORGANIZATION") {
            return self.parse_alter_organization().map(Some);
        }
        if self.check_identifier("GROUP") || self.check_identifier("PROFILE") {
            return Err(self.parse_error(format!(
                "Snowflake has no ALTER {}",
                self.peek_text().to_ascii_uppercase()
            )));
        }
        Ok(None)
    }

    /// Parse ALTER FAILOVER GROUP or ALTER REPLICATION GROUP after its kind.
    fn parse_alter_group(&mut self, kind: &str) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut actions: Vec<&[&str]> = vec![
            &["REFRESH"],
            &["RESUME"],
            &["SUSPEND", "IMMEDIATE"],
            &["SUSPEND"],
            &["RENAME", "TO"],
            &["SET"],
            &["UNSET"],
            &["ADD"],
            &["REMOVE"],
            &["MOVE", "DATABASES"],
            &["MOVE", "SHARES"],
            &["MOVE", "EXTERNAL", "VOLUMES"],
        ];
        if matches!(kind, "FAILOVER") {
            actions.push(&["PRIMARY"]);
        }
        let mut action = self.expect_alter_words(&actions)?;
        let parts = match action.as_str() {
            "RENAME TO" => (vec![self.parse_group_name()?], vec![]),
            "SET" => (vec![], self.parse_group_set()?),
            "UNSET" => (vec![], self.parse_group_unset()?),
            "ADD" | "REMOVE" => {
                let accounts = self.is_account_list();
                let names = match accounts {
                    true => self.parse_account_names()?,
                    false => self.parse_object_name_list()?,
                };
                let preposition: &[&[&str]] = match action.as_str() {
                    "ADD" => &[&["TO"]],
                    _ => &[&["FROM"]],
                };
                action.push(' ');
                action.push_str(&self.expect_alter_words(preposition)?);
                let list = self.expect_alter_words(ALLOWED_LISTS)?;
                action.push(' ');
                action.push_str(&list);
                if matches!(action.as_str(), "ADD TO ALLOWED_ACCOUNTS")
                    && self.match_text_seq(&["IGNORE", "EDITION", "CHECK"])
                {
                    action.push_str(" IGNORE EDITION CHECK");
                }
                (names, vec![])
            }
            "PRIMARY" | "REFRESH" | "RESUME" | "SUSPEND" | "SUSPEND IMMEDIATE" => (vec![], vec![]),
            _ => {
                let mut operands = self.parse_object_name_list()?;
                self.expect_alter_words(&[&["TO"]])?;
                self.expect_alter_words(&[&[kind, "GROUP"]])?;
                action.push_str(&format!(" TO {kind} GROUP"));
                operands.push(self.parse_group_name()?);
                (operands, vec![])
            }
        };
        Ok(alter_object(&format!("{kind} GROUP"), head, action, parts))
    }

    /// Parse a group's SET: TAG pairs, or `<name> = <value> [ , ... ]` properties.
    fn parse_group_set(&mut self) -> Result<Vec<ObjectProperty>> {
        if self.check_identifier("TAG") && !self.check_next(TokenType::Eq) {
            self.skip();
            return self.parse_group_tags(true);
        }
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let name = self.parse_simple_name()?;
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_group_values()?;
            properties.push(property(&property_kind(&name), None, Some(value)));
            if self.is_at_end() || self.check(TokenType::Semicolon) {
                return Ok(properties);
            }
        }
    }

    /// Parse a group property's values: comma-separated literals or names, a name of several words such as `SECURITY INTEGRATIONS`.
    fn parse_group_values(&mut self) -> Result<Expression> {
        let mut values: Vec<Expression> = Vec::new();
        loop {
            let value = self.parse_object_value()?;
            let Expression::Table(mut name) = value else {
                values.push(value);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
                continue;
            };
            while !self.is_at_end() && self.check(TokenType::Var) && !self.check_next(TokenType::Eq)
            {
                name.name.name = format!("{} {}", name.name.name, self.peek_text());
                self.skip();
            }
            values.push(Expression::Table(name));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(match values.len() {
            1 => values.remove(0),
            _ => Expression::Tuple(Box::new(Tuple {
                expressions: values,
            })),
        })
    }

    /// Parse TAG names, with their values when `set`, comma-separated.
    fn parse_group_tags(&mut self, set: bool) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
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
                return Ok(properties);
            }
        }
    }

    /// Parse an UNSET: TAG names, or property names, comma-separated.
    fn parse_group_unset(&mut self) -> Result<Vec<ObjectProperty>> {
        if self.check_identifier("TAG") && !self.is_statement_end_at(1) {
            self.skip();
            return self.parse_group_tags(false);
        }
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let name = self.parse_simple_name()?;
            properties.push(property(&property_kind(&name), None, None));
            if !self.match_token(TokenType::Comma) {
                return Ok(properties);
            }
        }
    }

    /// Whether the next tokens start `<org_name>.<account_name>`.
    fn is_account_list(&self) -> bool {
        matches!(
            self.tokens
                .get(self.current + 1)
                .map(|token| token.token_type),
            Some(TokenType::Dot)
        )
    }

    /// Parse `<org_name>.<account_name> [ , ... ]`, refusing a reserved keyword as either part.
    pub(super) fn parse_account_names(&mut self) -> Result<Vec<Expression>> {
        let mut accounts: Vec<Expression> = Vec::new();
        loop {
            let mut parts: Vec<Identifier> = vec![self.parse_simple_name()?];
            while self.match_token(TokenType::Dot) {
                parts.push(self.parse_simple_name()?);
            }
            let mut account = TableRef::new("");
            account.name = parts.pop().unwrap_or_else(Identifier::empty);
            account.schema = parts.pop();
            account.catalog = parts.pop();
            accounts.push(Expression::Table(Box::new(account)));
            if !self.match_token(TokenType::Comma) {
                return Ok(accounts);
            }
        }
    }

    /// Parse an object name as an operand.
    fn parse_group_name(&mut self) -> Result<Expression> {
        Ok(Expression::Table(Box::new(self.parse_object_name()?)))
    }

    /// Parse ALTER ACCOUNT after its kind: the current account, or a named one.
    fn parse_alter_account(&mut self) -> Result<Expression> {
        let named = !matches!(
            self.text_at(0).as_deref(),
            Some("SET" | "UNSET" | "ADD" | "REMOVE")
        );
        let head = match named {
            true => (self.parse_object_name()?, false),
            false => (TableRef::new(""), false),
        };
        let mut action = self.expect_alter_words(&[
            &["SET"],
            &["UNSET"],
            &["RENAME", "TO"],
            &["DROP", "OLD", "ORGANIZATION", "URL"],
            &["DROP", "OLD", "URL"],
            &["ADD", "ORGANIZATION", "USER", "GROUP"],
            &["REMOVE", "ORGANIZATION", "USER", "GROUP"],
        ])?;
        let set = matches!(action.as_str(), "SET");
        let parts: ActionParts = match action.as_str() {
            "SET" | "UNSET" => {
                if let Some(policy) = self.match_account_policy() {
                    action.push(' ');
                    action.push_str(&policy);
                    let operands = match set {
                        true => vec![self.parse_group_name()?],
                        false => vec![],
                    };
                    let force = set && self.match_identifier("FORCE");
                    if let Some(scope) = self.match_policy_scope() {
                        action.push_str(&format!(" FOR ALL {scope}"));
                    }
                    if set && (force || self.match_identifier("FORCE")) {
                        action.push_str(" FORCE");
                    }
                    (operands, vec![])
                } else if self.check_identifier("TAG")
                    && !self.check_next(TokenType::Eq)
                    && !self.is_statement_end_at(1)
                {
                    self.skip();
                    (vec![], self.parse_group_tags(set)?)
                } else if self.check_identifier("CONTACT")
                    && !self.check_next(TokenType::Eq)
                    && !self.is_statement_end_at(1)
                {
                    self.skip();
                    (vec![], self.parse_account_contacts(set)?)
                } else if set {
                    (vec![], self.parse_integration_set()?)
                } else {
                    (vec![], self.parse_group_unset_names()?)
                }
            }
            "RENAME TO" => {
                let name = self.parse_group_name()?;
                let mut properties: Vec<ObjectProperty> = Vec::new();
                if self.match_identifier("SAVE_OLD_URL") {
                    self.expect_token(TokenType::Eq)?;
                    properties.push(property(
                        "SAVE_OLD_URL",
                        None,
                        Some(self.parse_object_value()?),
                    ));
                }
                (vec![name], properties)
            }
            "ADD ORGANIZATION USER GROUP" | "REMOVE ORGANIZATION USER GROUP" => {
                (vec![self.parse_group_name()?], vec![])
            }
            _ => (vec![], vec![]),
        };
        Ok(alter_object("ACCOUNT", head, action, parts))
    }

    /// Consume `<kind> POLICY` for a policy an account sets, returning its words.
    fn match_account_policy(&mut self) -> Option<String> {
        const POLICIES: &[&[&str]] = &[
            &["AUTHENTICATION", "POLICY"],
            &["SESSION", "POLICY"],
            &["PASSWORD", "POLICY"],
            &["PACKAGES", "POLICY"],
            &["DATA", "MOVEMENT", "POLICY"],
            &["MULTI", "PARTY", "APPROVAL", "POLICY"],
            &["FEATURE", "POLICY"],
            &["MAINTENANCE", "POLICY"],
            &["INGRESS", "POLICY"],
        ];
        POLICIES
            .iter()
            .find(|words| self.match_text_seq(words))
            .map(|words| words.join(" "))
    }

    /// Consume `FOR ALL <scope>` after an account policy, returning the scope's words.
    fn match_policy_scope(&mut self) -> Option<String> {
        const SCOPES: &[&[&str]] = &[
            &["FOR", "ALL", "PERSON", "USERS"],
            &["FOR", "ALL", "SERVICE", "USERS"],
            &["FOR", "ALL", "APPLICATIONS"],
            &["FOR", "ALL", "PERSONAL", "DATABASES"],
            &["FOR", "ALL", "DATABASES"],
        ];
        SCOPES
            .iter()
            .find(|words| self.match_text_seq(words))
            .map(|words| words[2..].join(" "))
    }

    /// Parse CONTACT purposes, with their contacts when `set`, comma-separated.
    fn parse_account_contacts(&mut self, set: bool) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let purpose = self.parse_simple_name()?;
            let value = match set {
                true => {
                    self.expect_token(TokenType::Eq)?;
                    Some(self.parse_group_name()?)
                }
                false => None,
            };
            properties.push(property("CONTACT", Some(purpose), value));
            if !self.match_token(TokenType::Comma) {
                return Ok(properties);
            }
        }
    }

    /// Consume `[ <integration_type> ] INTEGRATION` and return the kind, or None when the next words name another kind.
    fn match_integration_kind(&mut self) -> Result<Option<String>> {
        if self.match_identifier("INTEGRATION") {
            return Ok(Some("INTEGRATION".to_owned()));
        }
        let kind: &[&str] = match self.text_at(0).as_deref() {
            Some("EXTERNAL") if self.check_next_identifier("ACCESS") => &["EXTERNAL", "ACCESS"],
            Some("API") => &["API"],
            Some("CATALOG") => &["CATALOG"],
            Some("NOTIFICATION") => &["NOTIFICATION"],
            Some("SECURITY") => &["SECURITY"],
            Some("STORAGE") if self.check_next_identifier("INTEGRATION") => &["STORAGE"],
            _ => return Ok(None),
        };
        self.current += kind.len();
        self.expect_alter_words(&[&["INTEGRATION"]])?;
        Ok(Some(format!("{} INTEGRATION", kind.join(" "))))
    }

    /// Parse ALTER [ <integration_type> ] INTEGRATION after its kind.
    fn parse_alter_integration(&mut self, kind: &str) -> Result<Expression> {
        let head = self.parse_alter_head_or_omitted(kind)?;
        let mut action = self.expect_alter_words(&[&["SET"], &["UNSET"], &["REFRESH"]])?;
        let parts: ActionParts = match action.as_str() {
            "SET" if self.check_identifier("TAG") && !self.check_next(TokenType::Eq) => {
                self.skip();
                (vec![], self.parse_group_tags(true)?)
            }
            "SET" => (vec![], self.parse_integration_set()?),
            "UNSET" => (vec![], self.parse_group_unset()?),
            _ => {
                let key = self.expect_alter_words(&[
                    &["OAUTH_CLIENT_SECRET_2"],
                    &["OAUTH_CLIENT_SECRET"],
                    &["SAML2_SNOWFLAKE_PRIVATE_KEY"],
                    &["METADATA_URL"],
                ])?;
                action.push(' ');
                action.push_str(&key);
                (vec![], vec![])
            }
        };
        Ok(alter_object(kind, head, action, parts))
    }

    /// Parse an integration's SET: `<name> = <value>` properties, at least one, where a value may be a parenthesized list.
    fn parse_integration_set(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let name = self.parse_simple_name()?;
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_integration_value()?;
            properties.push(property(&property_kind(&name), None, Some(value)));
            if self.is_at_end() || self.check(TokenType::Semicolon) {
                return Ok(properties);
            }
        }
    }

    /// Parse an integration property value: ALL, a literal or name, or `( ... )` of values or of `<key> = <value>` pairs, where a key may be any word, such as LOGIN_IDP_REDIRECT's DEFAULT.
    fn parse_integration_value(&mut self) -> Result<Expression> {
        if self.text_at(0).as_deref() == Some("ALL") {
            self.skip();
            return Ok(Expression::Table(Box::new(TableRef::new("ALL"))));
        }
        if !self.match_token(TokenType::LParen) {
            return self.parse_object_value();
        }
        let mut items: Vec<Expression> = Vec::new();
        while !self.check(TokenType::RParen) {
            if self.check_next(TokenType::Eq) {
                let key = if self.check(TokenType::String) {
                    self.parse_string_value()?
                } else if self.is_identifier_token() || self.check_keyword() {
                    let word = self.peek_text().to_ascii_uppercase();
                    self.skip();
                    Expression::Table(Box::new(TableRef::new(word)))
                } else {
                    return Err(self.parse_error("Expected a key"));
                };
                self.expect_token(TokenType::Eq)?;
                let value = self.parse_object_value()?;
                items.push(Expression::Tuple(Box::new(Tuple {
                    expressions: vec![key, value],
                })));
            } else {
                items.push(self.parse_object_value()?);
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions: items })))
    }

    /// Parse ALTER CONNECTION after its kind.
    fn parse_alter_connection(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["PRIMARY"],
            &["ENABLE", "FAILOVER", "TO", "ACCOUNTS"],
            &["DISABLE", "FAILOVER"],
            &["SET", "COMMENT"],
            &["UNSET", "COMMENT"],
        ])?;
        let parts: ActionParts = match action.as_str() {
            "ENABLE FAILOVER TO ACCOUNTS" => (self.parse_account_names()?, vec![]),
            "DISABLE FAILOVER" if self.match_text_seq(&["TO", "ACCOUNTS"]) => {
                action.push_str(" TO ACCOUNTS");
                (self.parse_account_names()?, vec![])
            }
            "SET COMMENT" => {
                self.expect_token(TokenType::Eq)?;
                (
                    vec![],
                    vec![property("COMMENT", None, Some(self.parse_object_value()?))],
                )
            }
            _ => (vec![], vec![]),
        };
        Ok(alter_object("CONNECTION", head, action, parts))
    }

    /// Parse ALTER ORGANIZATION after its first word: the organization itself, or its ACCOUNT, PROFILE, USER or USER GROUP.
    fn parse_alter_organization(&mut self) -> Result<Expression> {
        if self.match_text_seq(&["USER", "GROUP"]) {
            return self.parse_alter_organization_user_group();
        }
        if self.match_identifier("USER") {
            let head = self.parse_alter_object_head()?;
            let action = self.expect_alter_words(&[&["SET"], &["UNSET"]])?;
            let properties = match action.as_str() {
                "SET" if self.check_identifier("VISIBILITY") && self.check_next(TokenType::Eq) => {
                    self.current += 2;
                    self.expect_token(TokenType::All)?;
                    let value = Expression::Identifier(Identifier::new("ALL"));
                    vec![property("VISIBILITY", None, Some(value))]
                }
                "SET" => self.parse_organization_set()?,
                _ => self.parse_group_unset()?,
            };
            return Ok(alter_object(
                "ORGANIZATION USER",
                head,
                action,
                (vec![], properties),
            ));
        }
        let renames = self
            .tokens
            .get(self.current + 1)
            .is_some_and(|token| token.text.eq_ignore_ascii_case("RENAME"));
        if renames {
            let head = self.parse_alter_object_head()?;
            let action = self.expect_alter_words(&[&["RENAME", "TO"]])?;
            let parts: ActionParts = (vec![self.parse_group_name()?], vec![]);
            return Ok(alter_object("ORGANIZATION", head, action, parts));
        }
        if self.match_identifier("PROFILE") {
            return self.parse_alter_organization_profile();
        }
        let kind = match self.match_identifier("ACCOUNT") {
            true => "ORGANIZATION ACCOUNT",
            false => "ORGANIZATION",
        };
        let head = (TableRef::new(""), false);
        let mut action = self.expect_alter_words(&[&["SET"], &["UNSET"]])?;
        let routing =
            matches!(kind, "ORGANIZATION") && self.match_text_seq(&["EVENT", "ROUTING", "TABLE"]);
        let parts: ActionParts = match (action.as_str(), routing) {
            ("SET", true) => {
                let table = self.parse_group_name()?;
                self.expect_alter_words(&[&["FOR", "ALL", "APPLICATION", "LISTINGS"]])?;
                action.push_str(" EVENT ROUTING TABLE FOR ALL APPLICATION LISTINGS");
                (vec![table], vec![])
            }
            (_, true) => {
                self.expect_alter_words(&[&["FOR", "ALL", "APPLICATION", "LISTINGS"]])?;
                action.push_str(" EVENT ROUTING TABLE FOR ALL APPLICATION LISTINGS");
                (vec![], vec![])
            }
            ("SET", false) => (vec![], self.parse_organization_set()?),
            _ => (vec![], self.parse_group_unset_names()?),
        };
        Ok(alter_object(kind, head, action, parts))
    }

    /// Parse `<name> = <value>` properties, at least one; a value may be a parenthesized list.
    fn parse_organization_set(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let name = self.parse_simple_name()?;
            self.expect_token(TokenType::Eq)?;
            let value = if self.match_token(TokenType::LParen) {
                let mut items: Vec<Expression> = Vec::new();
                while !self.check(TokenType::RParen) {
                    items.push(self.parse_object_value()?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                Expression::Tuple(Box::new(Tuple { expressions: items }))
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

    /// Parse property names, comma-separated, with no TAG list.
    fn parse_group_unset_names(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let name = self.parse_simple_name()?;
            properties.push(property(&property_kind(&name), None, None));
            if !self.match_token(TokenType::Comma) {
                return Ok(properties);
            }
        }
    }

    /// Parse ALTER ORGANIZATION USER GROUP after its kind.
    fn parse_alter_organization_user_group(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["ADD", "ORGANIZATION", "USERS"],
            &["REMOVE", "ORGANIZATION", "USERS"],
            &["SET", "IS_GRANTABLE"],
            &["SET", "VISIBILITY"],
        ])?;
        let parts: ActionParts = match action.as_str() {
            "SET IS_GRANTABLE" => {
                self.expect_token(TokenType::Eq)?;
                (vec![self.parse_object_value()?], vec![])
            }
            "SET VISIBILITY" => {
                self.expect_token(TokenType::Eq)?;
                let scope =
                    self.expect_alter_words(&[&["ALL"], &["ACCOUNTS"], &["REGION", "GROUPS"]])?;
                action.push_str(&format!(" = {scope}"));
                match scope.as_str() {
                    "ALL" => (vec![], vec![]),
                    "ACCOUNTS" => (self.parse_object_name_list()?, vec![]),
                    _ => {
                        let mut groups = vec![self.parse_string_value()?];
                        while self.match_token(TokenType::Comma) {
                            groups.push(self.parse_string_value()?);
                        }
                        (groups, vec![])
                    }
                }
            }
            _ => (self.parse_object_name_list()?, vec![]),
        };
        Ok(alter_object("ORGANIZATION USER GROUP", head, action, parts))
    }

    /// Parse ALTER ORGANIZATION PROFILE after its kind.
    fn parse_alter_organization_profile(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[
            &["ABORT"],
            &["COMMIT"],
            &["PUBLISH"],
            &["AS"],
            &["RENAME", "TO"],
            &["ADD", "LIVE", "VERSION"],
            &["ADD", "VERSION"],
        ])?;
        let operands: Vec<Expression> = match action.as_str() {
            "AS" => vec![self.parse_string_value()?],
            "RENAME TO" => vec![self.parse_group_name()?],
            "ADD LIVE VERSION" | "ADD VERSION" => {
                if self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]) {
                    action.push_str(" IF NOT EXISTS");
                }
                let mut operands: Vec<Expression> = Vec::new();
                if !self.check(TokenType::From) {
                    operands.push(self.parse_group_name()?);
                }
                self.expect_token(TokenType::From)?;
                if action.starts_with("ADD LIVE") {
                    self.expect_alter_words(&[&["LAST"]])?;
                    action.push_str(" FROM LAST");
                } else {
                    action.push_str(" FROM");
                    operands.push(self.parse_location_value()?);
                }
                operands
            }
            _ => vec![],
        };
        Ok(alter_object(
            "ORGANIZATION PROFILE",
            head,
            action,
            (operands, vec![]),
        ))
    }
}
