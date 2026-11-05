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
        if self.check_identifier("FUTURE") || self.check_identifier("GRANTS") {
            let future = self.match_identifier("FUTURE");
            if !self.match_identifier("GRANTS") {
                return Err(self.parse_error("Expected GRANTS after FUTURE"));
            }
            return self.parse_snowflake_show_grants(future).map(Some);
        }
        let shared = SHARED_SHOW_KINDS
            .iter()
            .any(|kind| self.check_text_seq(kind))
            || self.check_next(TokenType::Dot);
        if shared {
            return Ok(None);
        }
        let Some(kind) = SNOWFLAKE_SHOW_KINDS
            .iter()
            .find(|kind| self.match_text_seq(kind))
            .map(|kind| kind.join(" "))
        else {
            return Err(self.parse_error("Expected an object kind after SHOW"));
        };
        let mut show = Show::new(kind.clone());
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
        let on_targets: &[&[&str]] = match kind.as_str() {
            "FEATURE POLICIES" | "MAINTENANCE POLICIES" => {
                &[&["ACCOUNT"], &["APPLICATION"], &["DATABASE"]]
            }
            "MULTI PARTY APPROVAL POLICIES" => &[&["ACCOUNT"]],
            _ => &[],
        };
        if !on_targets.is_empty() && self.match_token(TokenType::On) {
            let on = self.expect_alter_words(on_targets)?;
            if on != "ACCOUNT" {
                show.scope = Some(self.parse_show_name()?);
            }
            show.scope_kind = Some(format!("ON {on}"));
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
        let for_targets: &[&[&str]] = match kind.as_str() {
            "MFA METHODS" | "USER KEY PAIRS" | "USER WORKLOAD IDENTITY AUTHENTICATION METHODS" => {
                &[&["USER"]]
            }
            "USER PATS" | "USER PROGRAMMATIC ACCESS TOKENS" => {
                &[&["USER"], &["EXTERNAL", "CONSUMER"]]
            }
            _ => &[],
        };
        if !for_targets.is_empty() && self.match_token(TokenType::For) {
            let target = self.expect_alter_words(for_targets)?;
            if target != "USER" {
                show.scope_kind = Some(format!("FOR {target}"));
            }
            show.for_target = Some(self.parse_show_name()?);
        }
        Ok(Some(Expression::Show(Box::new(show))))
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

    /// SHOW [ FUTURE ] GRANTS after GRANTS: its one ON, TO, OF or IN clause, then LIMIT.
    fn parse_snowflake_show_grants(&mut self, future: bool) -> Result<Expression> {
        let mut show = Show::new(if future { "FUTURE GRANTS" } else { "GRANTS" }.to_owned());
        let clause = if future {
            Some(self.expect_alter_words(&[
                &["IN", "DATABASE"],
                &["IN", "SCHEMA"],
                &["IN", "DCM", "PROJECT"],
                &["TO", "ROLE"],
                &["TO", "DATABASE", "ROLE"],
            ])?)
        } else if self.match_token(TokenType::On) {
            Some(self.parse_show_grants_on(&mut show)?)
        } else if self.match_token(TokenType::To) {
            Some(self.parse_show_grants_to(&mut show)?)
        } else if self.match_text_seq(&["OF"]) {
            let words = self.expect_alter_words(&[
                &["APPLICATION", "ROLE"],
                &["SERVICE", "ROLE"],
                &["DATABASE", "ROLE"],
                &["ROLE"],
                &["SHARE"],
            ])?;
            Some(format!("OF {words}"))
        } else if self.match_token(TokenType::In) {
            self.expect_alter_words(&[&["DCM", "PROJECT"]])?;
            Some("IN DCM PROJECT".to_owned())
        } else {
            None
        };
        if let Some(clause) = clause {
            if show.scope.is_none() && clause != "ON ACCOUNT" {
                show.scope = Some(match clause.ends_with("SERVICE ROLE") {
                    true => self.parse_show_instance_role()?,
                    false => self.parse_show_name()?,
                });
            }
            show.scope_kind = Some(clause);
        }
        if show.scope_kind.as_deref() == Some("TO SHARE") && self.match_token(TokenType::In) {
            self.expect_alter_words(&[&["APPLICATION", "PACKAGE"]])?;
            show.target = Some(self.parse_show_name()?);
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
        }
        Ok(Expression::Show(Box::new(show)))
    }

    /// The clause after SHOW GRANTS ON: ACCOUNT, or an object kind of one or more words and the object's name.
    fn parse_show_grants_on(&mut self, show: &mut Show) -> Result<String> {
        if self.match_identifier("ACCOUNT") {
            return Ok("ON ACCOUNT".to_owned());
        }
        let mut kind: Vec<String> = Vec::new();
        while (self.is_identifier_token() || self.check_keyword())
            && !self.check(TokenType::QuotedIdentifier)
            && self.peek_nth(1).is_some_and(|next| {
                (matches!(
                    next.token_type,
                    TokenType::Var | TokenType::Identifier | TokenType::QuotedIdentifier
                ) || next.token_type.is_keyword())
                    && !next.text.eq_ignore_ascii_case("LIMIT")
            })
        {
            kind.push(self.advance_text()?.to_ascii_uppercase());
        }
        if kind.is_empty() {
            return Err(self.parse_error("Expected an object kind after ON"));
        }
        show.scope = Some(self.parse_show_name()?);
        if self.check(TokenType::LParen) {
            show.privileges = self.parse_function_param_types()?;
        }
        Ok(format!("ON {}", kind.join(" ")))
    }

    /// The grantee after SHOW GRANTS TO; a class instance role names its class as the target.
    fn parse_show_grants_to(&mut self, show: &mut Show) -> Result<String> {
        const GRANTEES: &[&[&str]] = &[
            &["APPLICATION", "ROLE"],
            &["APPLICATION"],
            &["SERVICE", "ROLE"],
            &["DATABASE", "ROLE"],
            &["ROLE"],
            &["SHARE"],
            &["USER"],
        ];
        if let Some(words) = GRANTEES.iter().find(|words| self.match_text_seq(words)) {
            return Ok(format!("TO {}", words.join(" ")));
        }
        show.target = Some(self.parse_show_name()?);
        self.expect_alter_words(&[&["ROLE"]])?;
        show.scope = Some(self.parse_show_instance_role()?);
        Ok("TO INSTANCE ROLE".to_owned())
    }

    /// `<instance_name>!<role_name>`, as a pair.
    fn parse_show_instance_role(&mut self) -> Result<Expression> {
        let instance = self.parse_show_name()?;
        self.expect_token(TokenType::Exclamation)?;
        let role = Expression::Identifier(self.parse_simple_name()?);
        Ok(Expression::Tuple(Box::new(
            crate::expressions::types::models::Tuple {
                expressions: vec![instance, role],
            },
        )))
    }
}
