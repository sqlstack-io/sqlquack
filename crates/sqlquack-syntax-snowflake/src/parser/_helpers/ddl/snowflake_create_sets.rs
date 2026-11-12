//! Parser ddl: Snowflake CREATE BACKUP SET and SNAPSHOT SET for a database, schema or table, and CREATE APPLICATION from a package or listing, checked against their grammars.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The set kinds, each with the policy its WITH clause names.
const SET_KINDS: [(&[&str], &[&str]); 2] = [
    (&["BACKUP", "SET"], &["BACKUP", "POLICY"]),
    (&["SNAPSHOT", "SET"], &["SNAPSHOT", "POLICY"]),
];

/// The words of a set kind, each of which before `<name> FOR` leaves the kind incomplete.
const SET_PREFIXES: [&str; 3] = ["BACKUP", "SET", "SNAPSHOT"];

/// What a backup or snapshot set is for, as its words after FOR.
const SET_TARGETS: [&[&str]; 4] = [
    &["DATABASE"],
    &["SCHEMA"],
    &["DYNAMIC", "TABLE"],
    &["TABLE"],
];

/// The kinds that APPLICATION starts, which CREATE APPLICATION is not when a name follows them.
const APPLICATION_KINDS: [&str; 3] = ["PACKAGE", "ROLE", "SERVICE"];

impl Parser {
    /// A Snowflake CREATE of a backup or snapshot set or an application; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_set(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() {
            return Ok(None);
        }
        if self.check_identifier("APPLICATION") && self.names_an_application() {
            return self
                .parse_create_application(or_replace, or_alter)
                .map(Some);
        }
        let Some((kind, policy)) = SET_KINDS.iter().find(|(kind, _)| self.check_text_seq(kind))
        else {
            if SET_PREFIXES.iter().any(|word| self.check_text_seq(&[word]))
                && self.text_at(2).as_deref() == Some("FOR")
            {
                return Err(self.parse_error("Expected BACKUP SET or SNAPSHOT SET"));
            }
            return Ok(None);
        };
        self.match_text_seq(kind);
        let if_not_exists = self.parse_set_if_not_exists()?;
        let name = self.parse_object_name()?;
        if !self.match_token(TokenType::For) {
            return Err(self.parse_error("Expected FOR and what the set is for"));
        }
        let target = self.expect_alter_words(&SET_TARGETS)?;
        let operands = vec![Expression::Table(Box::new(self.parse_object_name()?))];
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::With) && !self.check_next(TokenType::Eq) {
                self.skip();
                if self.match_text_seq(policy) {
                    let policy_name = self.parse_object_name()?;
                    properties.push(property(
                        &format!("WITH {}", policy.join(" ")),
                        None,
                        Some(Expression::Table(Box::new(policy_name))),
                    ));
                    continue;
                }
                if self.is_at_end() || self.check(TokenType::Semicolon) {
                    return Err(self.parse_error("Expected a property after WITH"));
                }
            }
            properties.extend(self.parse_create_property_item()?);
        }
        Ok(Some(create_object(
            kind.join(" "),
            (or_replace, or_alter, if_not_exists),
            name,
            format!("FOR {target}"),
            operands,
            properties,
        )))
    }

    /// Whether APPLICATION at the cursor starts CREATE APPLICATION rather than APPLICATION PACKAGE, ROLE or SERVICE: a name follows those words only when FROM comes next.
    fn names_an_application(&self) -> bool {
        let next = self.text_at(1);
        !next
            .as_deref()
            .is_some_and(|word| APPLICATION_KINDS.contains(&word))
            || self.text_at(2).as_deref() == Some("FROM")
    }

    /// Parse CREATE APPLICATION from its kind: a name, FROM an application package or a listing, an optional USING, then its properties.
    fn parse_create_application(&mut self, or_replace: bool, or_alter: bool) -> Result<Expression> {
        self.skip();
        let if_not_exists = self.parse_set_if_not_exists()?;
        let name = self.parse_object_name()?;
        if !self.match_token(TokenType::From) {
            return Err(self.parse_error("Expected FROM APPLICATION PACKAGE or FROM LISTING"));
        }
        let source = self.expect_alter_words(&[&["APPLICATION", "PACKAGE"], &["LISTING"]])?;
        let operands = vec![match source.as_str() {
            "LISTING" if self.check(TokenType::String) => self.parse_string_value()?,
            _ => Expression::Table(Box::new(self.parse_object_name()?)),
        }];
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if self.match_token(TokenType::Using) {
            properties.extend(self.parse_application_using()?);
        }
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::With) && !self.check_next(TokenType::Eq) {
                self.skip();
                if self.is_at_end() || self.check(TokenType::Semicolon) {
                    return Err(self.parse_error("Expected a property after WITH"));
                }
            }
            properties.extend(self.parse_create_property_item()?);
        }
        Ok(create_object(
            "APPLICATION".to_owned(),
            (or_replace, or_alter, if_not_exists),
            name,
            format!("FROM {source}"),
            operands,
            properties,
        ))
    }

    /// The application's USING after USING: VERSION <version> [ PATCH <number> ], RELEASE CHANNEL <channel>, or a version directory path.
    fn parse_application_using(&mut self) -> Result<Vec<ObjectProperty>> {
        if self.check(TokenType::String) {
            return Ok(vec![property(
                "USING",
                None,
                Some(self.parse_string_value()?),
            )]);
        }
        let using = self.expect_alter_words(&[&["VERSION"], &["RELEASE", "CHANNEL"]])?;
        let value = match self.check_text_seq(&["DEFAULT"]) && using == "RELEASE CHANNEL" {
            true => {
                self.skip();
                Identifier::new("DEFAULT".to_owned())
            }
            false => self.parse_simple_name()?,
        };
        let mut properties = vec![property(
            &format!("USING {using}"),
            None,
            Some(Expression::Identifier(value)),
        )];
        if using == "VERSION" && self.match_text_seq(&["PATCH"]) {
            if !self.check(TokenType::Number) {
                return Err(self.parse_error("Expected a patch number"));
            }
            properties.push(property("PATCH", None, Some(self.parse_primary()?)));
        }
        Ok(properties)
    }

    /// An optional IF NOT EXISTS, refusing any other IF.
    fn parse_set_if_not_exists(&mut self) -> Result<bool> {
        if !self.check(TokenType::If) {
            return Ok(false);
        }
        match self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]) {
            true => Ok(true),
            false => Err(self.parse_error("Expected IF NOT EXISTS")),
        }
    }
}

/// A CreateObject of `kind` with the given OR REPLACE, OR ALTER and IF NOT EXISTS flags.
fn create_object(
    kind: String,
    (or_replace, or_alter, if_not_exists): (bool, bool, bool),
    name: sqlquack_syntax::expressions::names::models::TableRef,
    source: String,
    operands: Vec<Expression>,
    properties: Vec<ObjectProperty>,
) -> Expression {
    Expression::CreateObject(Box::new(CreateObject {
        kind,
        modifier: None,
        or_replace,
        or_alter,
        if_not_exists,
        name,
        source,
        operands,
        properties,
    }))
}
