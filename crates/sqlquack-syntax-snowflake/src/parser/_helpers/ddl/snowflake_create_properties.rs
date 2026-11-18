//! Snowflake CREATE for kinds defined only by `<property> = <value>` lists: users, policies, integrations and groups.

use crate::parser::_helpers::ddl::snowflake_alter::keyword_values;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::_helpers::ddl::snowflake_alter::value_phrase;
use crate::parser::constants::{
    SNOWFLAKE_BODY_POLICY_KINDS, SNOWFLAKE_GROUP_NAME_PROPERTIES, SNOWFLAKE_KIND_WORDS_ALONE,
    SNOWFLAKE_PROPERTY_KINDS, SNOWFLAKE_STRING_KEY_MAP_PROPERTIES, SNOWFLAKE_TAGS_LAST_KINDS,
};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::schema_objects::models::{
    CreateObject, FunctionParameter, ObjectProperty,
};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE of a property-list kind after CREATE [ OR REPLACE | OR ALTER ]; None for any other kind.
    pub(crate) fn parse_snowflake_create_properties(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        let start = self.current;
        if self.peek().token_type != TokenType::QuotedIdentifier
            && ["DEPLOYMENT", "FORMAT", "GRAPH"]
                .iter()
                .any(|word| self.peek().text.eq_ignore_ascii_case(word))
        {
            return Err(self.parse_error("Expected a complete object kind after CREATE"));
        }
        if let Some(create) = self.parse_snowflake_create_own_kind(or_replace, or_alter)? {
            return Ok(Some(create));
        }
        if self.check(TokenType::Alter) || self.check(TokenType::String) {
            return Err(self.parse_error("Expected an object kind after CREATE"));
        }
        let database_role = or_alter && self.names_a_database_role();
        let organization = self.names_an_organization();
        let Some(parts) = SNOWFLAKE_PROPERTY_KINDS
            .iter()
            .map(|kind| kind.split(' ').collect::<Vec<_>>())
            .filter(|parts| !organization && self.check_text_seq(parts))
            .max_by_key(Vec::len)
            .or_else(|| database_role.then(|| vec!["DATABASE", "ROLE"]))
            .or_else(|| organization.then(|| vec!["ORGANIZATION"]))
        else {
            let body = SNOWFLAKE_BODY_POLICY_KINDS
                .iter()
                .any(|kind| self.check_text_seq(&kind.split(' ').collect::<Vec<_>>()));
            if !body && self.names_a_partial_kind() {
                return Err(self.parse_error("Expected a complete object kind after CREATE"));
            }
            return self.parse_snowflake_create_container(or_replace, or_alter);
        };
        if parts == ["MODEL", "MONITOR"] && self.model_name_ends_at(self.current + 2) {
            return Ok(None);
        }
        self.match_text_seq(&parts);
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        if if_not_exists && or_replace && parts == ["WAREHOUSE"] {
            return Err(self.parse_error("OR REPLACE and IF NOT EXISTS cannot be combined"));
        }
        let name = self.parse_object_name()?;
        if self.check_identifier("CLONE") || self.check_text_seq(&["AS", "REPLICA"]) {
            self.current = start;
            return Ok(None);
        }
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if parts == ["COMPUTE", "POOL"] && self.match_text_seq(&["FOR", "APPLICATION"]) {
            let application = self.parse_simple_name()?;
            properties.push(property(
                "FOR APPLICATION",
                None,
                Some(Expression::Identifier(application)),
            ));
        }
        let mut with = false;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::With) && !self.check_next(TokenType::Eq) {
                self.skip();
                if self.is_at_end() || self.check(TokenType::Semicolon) {
                    return Err(self.parse_error("Expected a property after WITH"));
                }
                with = true;
                continue;
            }
            if parts == ["ORGANIZATION"] && !self.check_next(TokenType::Eq) {
                return Err(self.parse_error("Expected <property> = <value>"));
            }
            let tags = self.check_identifier("TAG") && self.check_next(TokenType::LParen);
            if tags && or_alter && parts == ["SECRET"] {
                return Err(self.parse_error("CREATE OR ALTER and DEFINE SECRET take no tags"));
            }
            if matches!(parts.as_slice(), ["FAILOVER" | "REPLICATION", "GROUP"]) {
                self.check_group_name_value()?;
                if self.check_identifier("REPLICATION_SCHEDULE")
                    && self.check_next(TokenType::Eq)
                    && self.peek_nth(2).map(|token| token.token_type) != Some(TokenType::String)
                {
                    return Err(self.parse_error("Expected a string as the replication schedule"));
                }
            }
            properties.extend(with_written(
                std::mem::take(&mut with),
                self.parse_create_property_item()?,
            ));
            self.match_token(TokenType::Comma);
            if tags
                && SNOWFLAKE_TAGS_LAST_KINDS.contains(&parts.join(" ").as_str())
                && !self.is_at_end()
                && !self.check(TokenType::Semicolon)
            {
                return Err(self.parse_error("Expected the statement end after the tag list"));
            }
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: parts.join(" "),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: String::new(),
            operands: Vec::new(),
            properties,
            ..Default::default()
        }))))
    }

    /// A Snowflake CREATE of a kind with its own grammar, such as a class instance, stream, service or alert; None for any other kind.
    fn parse_snowflake_create_own_kind(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if let Some(class) = self.parse_snowflake_create_class(or_replace, or_alter)? {
            return Ok(Some(class));
        }
        if let Some(table) = self.parse_snowflake_create_external_table(or_replace, or_alter)? {
            return Ok(Some(table));
        }
        if let Some(table) = self.parse_snowflake_create_query_table(or_replace, or_alter)? {
            return Ok(Some(table));
        }
        if let Some(table) = self.parse_snowflake_create_event_table(or_replace, or_alter)? {
            return Ok(Some(table));
        }
        if let Some(stream) = self.parse_snowflake_create_stream(or_replace, or_alter)? {
            return Ok(Some(stream));
        }
        if let Some(view) = self.parse_snowflake_create_semantic_view(or_replace, or_alter)? {
            return Ok(Some(view));
        }
        if let Some(set) = self.parse_snowflake_create_set(or_replace, or_alter)? {
            return Ok(Some(set));
        }
        if let Some(service) = self.parse_snowflake_create_search_service(or_replace, or_alter)? {
            return Ok(Some(service));
        }
        if let Some(service) = self.parse_snowflake_create_service(or_replace, or_alter)? {
            return Ok(Some(service));
        }
        if let Some(listing) = self.parse_snowflake_create_listing(or_replace, or_alter)? {
            return Ok(Some(listing));
        }
        if let Some(app) = self.parse_snowflake_create_streamlit(or_replace, or_alter)? {
            return Ok(Some(app));
        }
        if let Some(alert) = self.parse_snowflake_create_alert(or_replace, or_alter)? {
            return Ok(Some(alert));
        }
        if let Some(rule) = self.parse_snowflake_create_movement_rule(or_replace, or_alter)? {
            return Ok(Some(rule));
        }
        if let Some(variable) = self.parse_snowflake_create_variable(or_replace, or_alter)? {
            return Ok(Some(variable));
        }
        if let Some(instance) =
            self.parse_snowflake_create_postgres_instance(or_replace, or_alter)?
        {
            return Ok(Some(instance));
        }
        if let Some(openflow) = self.parse_snowflake_create_openflow(or_replace, or_alter)? {
            return Ok(Some(openflow));
        }
        Ok(None)
    }

    /// Refuse a string as the first value of a group property that names objects, a quoted integration type, and a comma in its list followed by a string, another property or nothing, as Snowflake does.
    fn check_group_name_value(&self) -> Result<()> {
        let key = self.peek_text().to_ascii_uppercase();
        if !self.check_next(TokenType::Eq)
            || !SNOWFLAKE_GROUP_NAME_PROPERTIES.contains(&key.as_str())
        {
            return Ok(());
        }
        let refused = matches!(
            (self.peek_nth(2).map(|token| token.token_type), key.as_str()),
            (Some(TokenType::String), _)
                | (
                    Some(TokenType::QuotedIdentifier),
                    "ALLOWED_INTEGRATION_TYPES"
                )
        );
        if refused {
            return Err(self.parse_error(format!("Expected a name as the value of {key}")));
        }
        let at = |offset: usize| self.tokens.get(self.current + offset);
        let mut offset = 3;
        while let Some(token) = at(offset) {
            match token.token_type {
                TokenType::Semicolon => break,
                TokenType::Comma => {
                    let next_is_name = at(offset + 1).is_some_and(|next| {
                        !matches!(next.token_type, TokenType::String | TokenType::Semicolon)
                    }) && at(offset + 2)
                        .is_none_or(|eq| eq.token_type != TokenType::Eq);
                    if !next_is_name {
                        return Err(self.parse_error(format!(
                            "Expected a name after the comma in the list of {key}"
                        )));
                    }
                }
                _ if at(offset + 1).is_some_and(|next| next.token_type == TokenType::Eq) => break,
                _ if token.text.eq_ignore_ascii_case("TAG")
                    && at(offset + 1).is_some_and(|next| next.token_type == TokenType::LParen) =>
                {
                    break;
                }
                _ => {}
            }
            offset += 1;
        }
        Ok(())
    }

    /// A Snowflake CREATE of a body policy, `<kind> <name> AS ( <signature> ) RETURNS <type> -> <body> [ <property> ... ]`; None for any other kind.
    pub(crate) fn parse_snowflake_create_body_policy(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        let Some(parts) = SNOWFLAKE_BODY_POLICY_KINDS
            .iter()
            .map(|kind| kind.split(' ').collect::<Vec<_>>())
            .find(|parts| self.check_text_seq(parts))
        else {
            return Ok(None);
        };
        self.match_text_seq(&parts);
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        self.expect_token(TokenType::As)?;
        self.expect_token(TokenType::LParen)?;
        let mut signature: Vec<FunctionParameter> = Vec::new();
        while !self.check(TokenType::RParen) {
            let at = self.current;
            let first = self.parse_simple_name()?;
            let name = match self.check(TokenType::Comma) || self.check(TokenType::RParen) {
                true => {
                    self.current = at;
                    None
                }
                false => Some(first),
            };
            signature.push(FunctionParameter {
                comment: None,
                comment_written: None,
                name,
                data_type: self.parse_data_type()?,
                mode: None,
                default: None,
                mode_text: None,
                span: Default::default(),
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
            if self.check(TokenType::RParen) {
                return Err(self.parse_error("Expected an argument after the comma"));
            }
        }
        self.expect_token(TokenType::RParen)?;
        if !self.match_text_seq(&["RETURNS"]) {
            return Err(self.parse_error("Expected RETURNS"));
        }
        if !self.is_identifier_token() || self.check(TokenType::QuotedIdentifier) {
            return Err(self.parse_error("Expected a return type"));
        }
        let returns = self.parse_data_type()?;
        self.expect_token(TokenType::Arrow)?;
        let body = self.parse_expression()?;
        let mut properties: Vec<ObjectProperty> = Vec::new();
        let mut with = false;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::With) && !self.check_next(TokenType::Eq) {
                self.skip();
                if self.is_at_end() || self.check(TokenType::Semicolon) {
                    return Err(self.parse_error("Expected a property after WITH"));
                }
                with = true;
                continue;
            }
            properties.extend(with_written(
                std::mem::take(&mut with),
                self.parse_create_property_item()?,
            ));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: parts.join(" "),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: String::new(),
            operands: Vec::new(),
            properties,
            signature: Some(signature),
            returns: Some(returns),
            body: Some(Box::new(body)),
            ..Default::default()
        }))))
    }

    /// Whether CREATE names an organization itself: `ORGANIZATION <name>`, where a kind word such as ACCOUNT or USER is the name when the statement ends after it or a property follows it.
    fn names_an_organization(&self) -> bool {
        if self.peek().token_type == TokenType::QuotedIdentifier
            || !self.check_identifier("ORGANIZATION")
        {
            return false;
        }
        if !matches!(
            self.text_at(1).as_deref(),
            Some("ACCOUNT" | "LISTING" | "PROFILE" | "USER")
        ) {
            return true;
        }
        let at = |offset: usize| self.tokens.get(self.current + offset).map(|t| t.token_type);
        matches!(at(2), None | Some(TokenType::Semicolon)) || at(3) == Some(TokenType::Eq)
    }

    /// Whether the words at the cursor are part of a property or body kind, not all of it, before a name and then `;`, AS, WITH, FROM or `<name> =`.
    fn names_a_partial_kind(&self) -> bool {
        let kinds = SNOWFLAKE_PROPERTY_KINDS
            .iter()
            .chain(SNOWFLAKE_BODY_POLICY_KINDS)
            .filter(|kind| kind.contains(' ') && !kind.starts_with("APPLICATION "));
        if SNOWFLAKE_KIND_WORDS_ALONE.contains(&self.peek().text.to_ascii_uppercase().as_str()) {
            return false;
        }
        let mut words = 0;
        while self.tokens.get(self.current + words).is_some_and(|token| {
            matches!(token.token_type, TokenType::Var | TokenType::Identifier)
                || token.text.chars().all(|c| c.is_ascii_alphabetic())
        }) && kinds
            .clone()
            .any(|kind| kind_has_word(kind, &self.tokens[self.current + words].text))
        {
            words += 1;
        }
        let ends_name = |offset: usize| {
            self.tokens.get(self.current + offset).is_some_and(|token| {
                matches!(
                    token.token_type,
                    TokenType::Semicolon | TokenType::As | TokenType::With | TokenType::From
                )
            }) || self
                .tokens
                .get(self.current + offset + 1)
                .is_some_and(|token| token.token_type == TokenType::Eq)
        };
        words > 0 && self.check_grant_name_like(words) && ends_name(words + 1)
    }

    /// Whether the token `offset` ahead is a plain or quoted identifier.
    fn check_grant_name_like(&self, offset: usize) -> bool {
        self.tokens.get(self.current + offset).is_some_and(|token| {
            matches!(
                token.token_type,
                TokenType::Var | TokenType::Identifier | TokenType::QuotedIdentifier
            )
        })
    }

    /// A Snowflake CREATE VARIABLE: its name, its type as the operand, then DEFAULT and COMMENT; None for any other CREATE.
    fn parse_snowflake_create_variable(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        let start = self.current;
        if or_alter
            || !self.applies_snowflake_context_rules()
            || !self.match_text_seq(&["VARIABLE"])
        {
            return Ok(None);
        }
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        if self.check(TokenType::String) || self.is_statement_end_at(0) {
            return Err(self.parse_error("Expected the variable's type"));
        }
        let operands = vec![Expression::DataType(self.parse_data_type()?)];
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if self.match_token(TokenType::Default) {
            if self.is_statement_end_at(0) {
                return Err(self.parse_error("Expected a default value"));
            }
            properties.push(property("DEFAULT", None, Some(self.parse_expression()?)));
        }
        if self.match_text_seq(&["COMMENT"]) {
            self.expect_token(TokenType::Eq)?;
            if !self.check(TokenType::String) {
                return Err(self.parse_error("Expected a string after COMMENT ="));
            }
            properties.push(property("COMMENT", None, Some(self.parse_object_value()?)));
        }
        if !self.is_statement_end_at(0) {
            self.current = start;
            return Ok(None);
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: "VARIABLE".to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: String::new(),
            operands,
            properties,
            ..Default::default()
        }))))
    }

    /// One item: TAG ( <name> = <value> , ... ), TRIGGERS ON <n> PERCENT DO <action> ..., a tag's ALLOWED_VALUES <value> , ..., AS <body>, IGNORE EDITION CHECK, or <property> = <value>.
    pub(super) fn parse_create_property_item(&mut self) -> Result<Vec<ObjectProperty>> {
        if self.check_identifier("TAG") && self.check_next(TokenType::LParen) {
            self.skip();
            self.skip();
            let mut tags: Vec<ObjectProperty> = Vec::new();
            loop {
                let name = self.parse_simple_qualified_name()?;
                self.expect_token(TokenType::Eq)?;
                tags.push(property(
                    "TAG",
                    Some(name),
                    Some(self.parse_object_value()?),
                ));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            return Ok(tags);
        }
        if self.check_identifier("TRIGGERS") && !self.check_next(TokenType::Eq) {
            self.skip();
            let mut triggers: Vec<ObjectProperty> = Vec::new();
            while self.match_token(TokenType::On) {
                if !self.check(TokenType::Number) {
                    return Err(self.parse_error("Expected a number before PERCENT"));
                }
                let percent = self.parse_object_value()?;
                self.expect_alter_words(&[&["PERCENT", "DO"]])?;
                let action = self.parse_simple_name()?;
                triggers.push(property(
                    "TRIGGER",
                    Some(action),
                    Some(Expression::Tuple(Box::new(Tuple {
                        expressions: vec![percent],
                    }))),
                ));
            }
            if triggers.is_empty() {
                return Err(self.parse_error("Expected ON <percent> PERCENT DO <action>"));
            }
            return Ok(triggers);
        }
        if self.check_identifier("ALLOWED_VALUES") && !self.check_next(TokenType::Eq) {
            self.skip();
            let mut values: Vec<Expression> = vec![self.parse_object_value()?];
            while self.match_token(TokenType::Comma) {
                values.push(self.parse_object_value()?);
            }
            return Ok(vec![property(
                "ALLOWED_VALUES",
                None,
                Some(Expression::Tuple(Box::new(Tuple {
                    expressions: values,
                }))),
            )]);
        }
        if self.check(TokenType::As) {
            self.skip();
            return Ok(vec![property("AS", None, Some(self.parse_object_value()?))]);
        }
        if self.match_text_seq(&["RETENTION", "LOCK"]) {
            return Ok(vec![property("RETENTION LOCK", None, None)]);
        }
        if self.match_text_seq(&["LANGUAGE"]) {
            let language = self.parse_simple_name()?;
            return Ok(vec![property(
                "LANGUAGE",
                None,
                Some(Expression::Identifier(language)),
            )]);
        }
        if self.match_text_seq(&["IGNORE", "EDITION", "CHECK"]) {
            return Ok(vec![property("IGNORE EDITION CHECK", None, None)]);
        }
        let name = self.parse_simple_name()?;
        self.expect_token(TokenType::Eq)?;
        if SNOWFLAKE_STRING_KEY_MAP_PROPERTIES.contains(&name.name.to_ascii_uppercase().as_str()) {
            self.check_snowflake_string_keys()?;
        }
        let value = keyword_values(&name.name, self.parse_create_property_value(false)?);
        Ok(vec![property(
            &name.name.to_ascii_uppercase(),
            None,
            Some(value),
        )])
    }

    /// A value, a parenthesized list of values or `<name> = <value>` pairs, or a bare name list; a list item ends at its comma.
    fn parse_create_property_value(&mut self, in_list: bool) -> Result<Expression> {
        if self.match_token(TokenType::LParen) {
            let mut expressions: Vec<Expression> = Vec::new();
            while !self.check(TokenType::RParen) {
                if self.check_next(TokenType::Eq) {
                    let (key, value) = if self.check(TokenType::String) {
                        let key = self.parse_primary()?;
                        self.expect_token(TokenType::Eq)?;
                        (key, self.parse_create_property_value(false)?)
                    } else {
                        let name = self.parse_simple_name()?;
                        self.expect_token(TokenType::Eq)?;
                        let value = self.parse_create_property_value(false)?;
                        let value = match name.quoted {
                            true => value,
                            false => keyword_values(&name.name, value),
                        };
                        (Expression::Identifier(name), value)
                    };
                    expressions.push(Expression::PropertyEQ(Box::new(BinaryOp::new(key, value))));
                    if !self.match_token(TokenType::Comma) {
                        continue;
                    }
                } else {
                    expressions.push(self.parse_create_property_value(true)?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                if self.check(TokenType::RParen) {
                    return Err(self.parse_error("Expected a value after the comma"));
                }
            }
            self.expect_token(TokenType::RParen)?;
            return Ok(Expression::Tuple(Box::new(Tuple { expressions })));
        }
        if self.check(TokenType::Null) {
            return self.parse_primary();
        }
        if self.check_text_seq(&["ALL"]) || self.check_text_seq(&["NONE"]) {
            let word = self.advance_text()?.to_ascii_uppercase();
            return Ok(Expression::Identifier(Identifier::new(word)));
        }
        let first = self.parse_value_phrase()?;
        if in_list || !self.continues_name_list() {
            return Ok(first);
        }
        let mut expressions = vec![first];
        while self.continues_name_list() {
            self.skip();
            expressions.push(self.parse_value_phrase()?);
        }
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
    }

    /// A value, or a keyword phrase such as SECURITY INTEGRATIONS when plain words follow a name.
    fn parse_value_phrase(&mut self) -> Result<Expression> {
        let value = self.parse_object_value()?;
        let Expression::Table(name) = value else {
            return Ok(value);
        };
        let mut more: Vec<String> = Vec::new();
        while self.continues_value_words() {
            more.push(self.advance_text()?);
        }
        Ok(value_phrase(name, more))
    }

    /// Whether the next word continues a multi-word value, such as SECURITY INTEGRATIONS: a plain word that starts no `<name> =` or clause.
    fn continues_value_words(&self) -> bool {
        self.is_identifier_token()
            && !self.check(TokenType::QuotedIdentifier)
            && !self.check_next(TokenType::Eq)
            && !self.check_next(TokenType::Dot)
            && !self.check_next(TokenType::LParen)
            && !["IGNORE", "TAG", "TRIGGERS", "LANGUAGE", "RETENTION"]
                .iter()
                .any(|word| self.check_identifier(word))
    }

    /// Whether a comma continues an unparenthesized list: the word after it starts no `<name> =`.
    fn continues_name_list(&self) -> bool {
        self.check(TokenType::Comma)
            && self
                .tokens
                .get(self.current + 2)
                .is_none_or(|token| token.token_type != TokenType::Eq)
            && self.tokens.get(self.current + 1).is_some_and(|token| {
                !matches!(token.token_type, TokenType::Semicolon | TokenType::LParen)
                    && !token.text.eq_ignore_ascii_case("TAG")
            })
    }
}

/// `properties` with their `with` flag set when a WITH was written before the item they came from.
pub(super) fn with_written(with: bool, mut properties: Vec<ObjectProperty>) -> Vec<ObjectProperty> {
    for found in &mut properties {
        found.with = with;
    }
    properties
}

/// Whether `kind` has `text` as one of its words.
fn kind_has_word(kind: &str, text: &str) -> bool {
    kind.split(' ').any(|word| text.eq_ignore_ascii_case(word))
}
