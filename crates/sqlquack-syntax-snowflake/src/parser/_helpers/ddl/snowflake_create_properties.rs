//! Snowflake CREATE for kinds defined only by `<property> = <value>` lists: users, policies, integrations and groups.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::constants::{
    SNOWFLAKE_BODY_POLICY_KINDS, SNOWFLAKE_KIND_WORDS_ALONE, SNOWFLAKE_PROPERTY_KINDS,
    SNOWFLAKE_TAGS_LAST_KINDS,
};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
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
        if self.check(TokenType::Alter) || self.check(TokenType::String) {
            return Err(self.parse_error("Expected an object kind after CREATE"));
        }
        let database_role = or_alter && self.names_a_database_role();
        let Some(parts) = SNOWFLAKE_PROPERTY_KINDS
            .iter()
            .map(|kind| kind.split(' ').collect::<Vec<_>>())
            .filter(|parts| self.check_text_seq(parts))
            .max_by_key(Vec::len)
            .or_else(|| database_role.then(|| vec!["DATABASE", "ROLE"]))
        else {
            let body = SNOWFLAKE_BODY_POLICY_KINDS
                .iter()
                .any(|kind| self.check_text_seq(&kind.split(' ').collect::<Vec<_>>()));
            if !body && self.names_a_partial_kind() {
                return Err(self.parse_error("Expected a complete object kind after CREATE"));
            }
            return Ok(None);
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
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::With) && !self.check_next(TokenType::Eq) {
                self.skip();
                if self.is_at_end() || self.check(TokenType::Semicolon) {
                    return Err(self.parse_error("Expected a property after WITH"));
                }
                continue;
            }
            let tags = self.check_identifier("TAG") && self.check_next(TokenType::LParen);
            properties.extend(self.parse_create_property_item()?);
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
        }))))
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
        let mut signature: Vec<Expression> = Vec::new();
        while !self.check(TokenType::RParen) {
            let first = self.parse_simple_name()?;
            if self.check(TokenType::Comma) || self.check(TokenType::RParen) {
                signature.push(Expression::Identifier(first));
            } else {
                let data_type = self.parse_data_type()?;
                signature.push(Expression::PropertyEQ(Box::new(BinaryOp::new(
                    Expression::Identifier(first),
                    Expression::DataType(data_type),
                ))));
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        if !self.match_text_seq(&["RETURNS"]) {
            return Err(self.parse_error("Expected RETURNS"));
        }
        if !self.is_identifier_token() || self.check(TokenType::QuotedIdentifier) {
            return Err(self.parse_error("Expected a return type"));
        }
        let returns = Expression::DataType(self.parse_data_type()?);
        self.expect_token(TokenType::Arrow)?;
        let body = self.parse_expression()?;
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::With) && !self.check_next(TokenType::Eq) {
                self.skip();
                if self.is_at_end() || self.check(TokenType::Semicolon) {
                    return Err(self.parse_error("Expected a property after WITH"));
                }
                continue;
            }
            properties.extend(self.parse_create_property_item()?);
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: parts.join(" "),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: "AS".to_owned(),
            operands: vec![
                Expression::Tuple(Box::new(Tuple {
                    expressions: signature,
                })),
                returns,
                body,
            ],
            properties,
        }))))
    }

    /// Whether the words at the cursor are part of a property or body kind, not all of it, before a name and then `;`, AS, WITH or `<name> =`.
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
                    TokenType::Semicolon | TokenType::As | TokenType::With
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

    /// One item: TAG ( <name> = <value> , ... ), TRIGGERS ON <n> PERCENT DO <action> ..., AS <body>, IGNORE EDITION CHECK, or <property> = <value>.
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
        let value = self.parse_create_property_value(false)?;
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
                    let key = if self.check(TokenType::String) {
                        self.parse_primary()?
                    } else {
                        Expression::Identifier(self.parse_simple_name()?)
                    };
                    self.expect_token(TokenType::Eq)?;
                    let value = self.parse_create_property_value(false)?;
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
        let mut first = self.parse_object_value()?;
        if let Expression::Table(table) = &mut first {
            while self.continues_value_words() {
                let word = self.advance_text()?;
                table.name.name = format!("{} {}", table.name.name, word);
            }
        }
        if in_list || !self.continues_name_list() {
            return Ok(first);
        }
        let mut expressions = vec![first];
        while self.continues_name_list() {
            self.skip();
            let mut next = self.parse_object_value()?;
            if let Expression::Table(table) = &mut next {
                while self.continues_value_words() {
                    let word = self.advance_text()?;
                    table.name.name = format!("{} {}", table.name.name, word);
                }
            }
            expressions.push(next);
        }
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
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

/// Whether `kind` has `text` as one of its words.
fn kind_has_word(kind: &str, text: &str) -> bool {
    kind.split(' ').any(|word| text.eq_ignore_ascii_case(word))
}
