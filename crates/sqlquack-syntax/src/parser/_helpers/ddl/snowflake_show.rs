//! Snowflake SHOW: a documented kind, then the clauses that kind takes, each with its own operand.

use crate::expressions::queries::models::Limit;
use crate::expressions::statements::models::Show;
use crate::expressions::tree::models::Expression;
use crate::parser::constants::{
    SHARED_SHOW_KINDS, SNOWFLAKE_SHOW_CONTAINERS, SNOWFLAKE_SHOW_EXTRA_CONTAINERS,
    SNOWFLAKE_SHOW_GENERAL_CONTAINERS, SNOWFLAKE_SHOW_KINDS, SNOWFLAKE_SHOW_REQUIRED_CONTAINERS,
};
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// A Snowflake SHOW after SHOW and TERSE; None leaves the kinds the shared SHOW reads to it.
    pub(crate) fn parse_snowflake_show(&mut self, terse: bool) -> Result<Option<Expression>> {
        let class = self.applies_snowflake_context_rules() && self.check_next(TokenType::Dot);
        let shared = SHARED_SHOW_KINDS
            .iter()
            .any(|kind| self.check_text_seq(kind))
            || self.check_next(TokenType::Dot);
        if shared && !class {
            return Ok(None);
        }
        let target = match class {
            true => Some(self.parse_show_name()?),
            false => None,
        };
        let kind = match class {
            true if self.match_identifier("INSTANCES") => "CLASS INSTANCES".to_owned(),
            true => "CLASS".to_owned(),
            false => self.parse_show_kind()?,
        };
        let mut show = Show::new(kind.clone());
        show.target = target;
        show.terse = terse;
        show.history = self.match_identifier("HISTORY");
        if matches!(kind.as_str(), "DELEGATED AUTHORIZATIONS BY USER") {
            show.for_target = Some(self.parse_show_name()?);
            return Ok(Some(Expression::Show(Box::new(show))));
        }
        if matches!(kind.as_str(), "OBJECTS")
            && self.match_text_seq(&["OWNED", "BY", "APPLICATION"])
        {
            show.scope_kind = Some("OWNED BY APPLICATION".to_owned());
            show.scope = Some(self.parse_show_name()?);
            return Ok(Some(Expression::Show(Box::new(show))));
        }
        if self.match_token(TokenType::Like) {
            show.like = Some(self.parse_show_string("LIKE")?);
        }
        let required = SNOWFLAKE_SHOW_REQUIRED_CONTAINERS
            .iter()
            .find(|(name, _)| *name == kind)
            .map(|(_, containers)| *containers);
        if required.is_some() || self.check(TokenType::In) {
            self.expect_token(TokenType::In)?;
            let (container, name) = self.parse_show_container(&kind)?;
            let allowed = |container: &str| match required {
                Some(required) => required.contains(&container),
                None => {
                    SNOWFLAKE_SHOW_GENERAL_CONTAINERS.contains(&container)
                        || SNOWFLAKE_SHOW_EXTRA_CONTAINERS
                            .iter()
                            .any(|(name, extra)| *name == kind && extra.contains(&container))
                }
            };
            if !allowed(container.as_deref().unwrap_or("")) {
                return Err(self.parse_error(format!("Expected a container {kind} is shown in")));
            }
            show.scope_kind = container;
            show.scope = name;
        }
        let mut show = self.parse_show_follow_up(&kind, show)?;
        if self.match_identifier("STARTS") {
            self.expect_token(TokenType::With)?;
            show.starts_with = Some(self.parse_show_string("STARTS WITH")?);
        }
        if self.match_token(TokenType::Limit) {
            if !self.check(TokenType::Number) {
                return Err(self.parse_error("Expected a row count after LIMIT"));
            }
            show.limit = Some(Box::new(Limit {
                this: self.parse_primary()?,
                percent: false,
                comments: Vec::new(),
            }));
            if self.match_token(TokenType::From) {
                show.from = Some(self.parse_show_string("FROM")?);
            }
        }
        if show.scope_kind.is_none() && show.scope.is_none() && self.match_token(TokenType::In) {
            let (container, name) = self.parse_show_container(&kind)?;
            show.scope_kind = container;
            show.scope = name;
        }
        if self.match_token(TokenType::With) {
            show = self.parse_show_with(&kind, show)?;
        }
        if matches!(kind.as_str(), "MFA METHODS") && self.match_token(TokenType::For) {
            if !self.match_identifier("USER") {
                return Err(self.parse_error("Expected USER after FOR"));
            }
            show.for_target = Some(self.parse_show_name()?);
        }
        Ok(Some(Expression::Show(Box::new(show))))
    }

    /// A documented object kind, such as `TABLES`.
    fn parse_show_kind(&mut self) -> Result<String> {
        match SNOWFLAKE_SHOW_KINDS
            .iter()
            .find(|kind| self.match_text_seq(kind))
        {
            Some(kind) => Ok(kind.join(" ")),
            None => Err(self.parse_error("Expected an object kind after SHOW")),
        }
    }

    /// A quoted string operand after `clause`.
    fn parse_show_string(&mut self, clause: &str) -> Result<Expression> {
        match self.check(TokenType::String) {
            true => self.parse_primary(),
            false => Err(self.parse_error(format!("Expected a string after {clause}"))),
        }
    }

    /// An object name, kept as written.
    fn parse_show_name(&mut self) -> Result<Expression> {
        Ok(Expression::Table(Box::new(self.parse_object_name()?)))
    }

    /// The container after IN and its name; ACCOUNT, DATABASE and SCHEMA may stand alone.
    fn parse_show_container(&mut self, kind: &str) -> Result<(Option<String>, Option<Expression>)> {
        let Some(container) = SNOWFLAKE_SHOW_CONTAINERS
            .iter()
            .find(|container| self.match_text_seq(container))
            .map(|container| container.join(" "))
        else {
            return Ok((None, Some(self.parse_show_name()?)));
        };
        let groups = matches!(kind, "FAILOVER GROUPS" | "REPLICATION GROUPS");
        let optional = match container.as_str() {
            "ACCOUNT" => !groups,
            "DATABASE" | "SCHEMA" => !matches!(kind, "DATABASE ROLES" | "SCHEMA VARIABLES"),
            "TABLE" | "VIEW" => true,
            _ => false,
        };
        let ends = self.is_at_end()
            || self.check(TokenType::Semicolon)
            || self.check(TokenType::Like)
            || self.check(TokenType::Limit)
            || self.check_identifier("STARTS");
        let name = match (container.as_str(), optional && ends) {
            (_, true) => None,
            ("ACCOUNT", _) if !groups => None,
            _ => Some(self.parse_show_name()?),
        };
        Ok((Some(container), name))
    }

    /// What some kinds take after their container: a package, run, channel, metric or version.
    fn parse_show_follow_up(&mut self, kind: &str, mut show: Show) -> Result<Show> {
        let artifact = show.scope_kind.as_deref() == Some("ARTIFACT REPOSITORY");
        let model = show.scope_kind.as_deref() == Some("MODEL");
        let (words, required): (&[&str], bool) = match kind {
            "VERSIONS" if artifact => (&["FOR", "PACKAGE"], true),
            "SHARED CONTENT" => (&["FOR", "VERSION"], true),
            "RELEASE DIRECTIVES" => (&["FOR", "RELEASE", "CHANNEL"], false),
            "SEMANTIC DIMENSIONS" => (&["FOR", "METRIC"], false),
            "RUN METRICS" | "RUN PARAMETERS" => (&["RUN"], false),
            "FUNCTIONS" if model => (&["VERSION"], false),
            _ => return Ok(show),
        };
        if self.match_text_seq(words) {
            let name = self.parse_show_name()?;
            match words {
                ["FOR", ..] => show.for_target = Some(name),
                _ => show.target = Some(name),
            }
        } else if required {
            return Err(
                self.parse_error(format!("Expected {} after the container", words.join(" ")))
            );
        }
        Ok(show)
    }

    /// `WITH PRIVILEGES <privilege> [ , ... ]`, or `WITH PRIMARY <account>.<database>`.
    fn parse_show_with(&mut self, kind: &str, mut show: Show) -> Result<Show> {
        if matches!(kind, "REPLICATION DATABASES") && self.match_text_seq(&["PRIMARY"]) {
            show.target = Some(self.parse_show_name()?);
            return Ok(show);
        }
        if !self.match_identifier("PRIVILEGES") {
            return Err(self.parse_error("Expected PRIVILEGES after WITH"));
        }
        loop {
            if !(self.is_identifier_token() || self.check_keyword()) {
                return Err(self.parse_error("Expected a privilege"));
            }
            show.privileges
                .push(self.advance_text()?.to_ascii_uppercase());
            if !self.match_token(TokenType::Comma) {
                return Ok(show);
            }
        }
    }
}
