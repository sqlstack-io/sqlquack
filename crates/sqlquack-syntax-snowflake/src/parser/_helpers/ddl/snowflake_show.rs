//! Snowflake SHOW: a documented kind, then the clauses that kind takes, each with its own operand.

use crate::parser::constants::ACCOUNT;
use crate::parser::constants::ON_ACCOUNT;
use crate::parser::constants::USER;
use crate::parser::constants::{
    SHARED_SHOW_KINDS, SNOWFLAKE_SHOW_CONTAINERS, SNOWFLAKE_SHOW_EXTRA_CONTAINERS,
    SNOWFLAKE_SHOW_GENERAL_CONTAINERS, SNOWFLAKE_SHOW_KINDS, SNOWFLAKE_SHOW_REQUIRED_CONTAINERS,
};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::queries::models::Limit;
use sqlquack_syntax::expressions::statements::models::Show;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

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
        if self.match_text_seq(&["CALLER", "GRANTS"]) {
            return self.parse_snowflake_show_caller_grants().map(Some);
        }
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
        if matches!(kind.as_str(), "SERVICES") {
            show = self.parse_show_services_filters(show)?;
        }
        if matches!(kind.as_str(), "OPENFLOW DATA PLANE INTEGRATIONS") {
            return Ok(Some(Expression::Show(Box::new(show))));
        }
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
            if on != ACCOUNT {
                show.scope = Some(self.parse_show_name()?);
            }
            show.scope_kind = Some(format!("ON {on}"));
            return Ok(Some(Expression::Show(Box::new(show))));
        }
        if self.match_token(TokenType::Like) {
            show.like = Some(self.parse_show_string("LIKE")?);
        }
        let mut show = self.parse_show_on_user(&kind, show)?;
        let required = SNOWFLAKE_SHOW_REQUIRED_CONTAINERS
            .iter()
            .find(|(name, _)| *name == kind)
            .map(|(_, containers)| *containers);
        if show.scope_kind.is_none() && (required.is_some() || self.check(TokenType::In)) {
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
        let show = self.parse_show_follow_up(&kind, show)?;
        let mut show = self.parse_show_starts_and_limit(&kind, show)?;
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
            if target != USER {
                show.scope_kind = Some(format!("FOR {target}"));
            }
            show.for_target = Some(self.parse_show_name()?);
        }
        Ok(Some(Expression::Show(Box::new(show))))
    }

    /// `[ EXCLUDE JOBS ] [ OF TYPE <type> ]` after SHOW SERVICES, kept in the kind as written.
    fn parse_show_services_filters(&mut self, mut show: Show) -> Result<Show> {
        if self.match_text_seq(&["EXCLUDE", "JOBS"]) {
            show.this.push_str(" EXCLUDE JOBS");
        }
        if self.match_text_seq(&["OF", "TYPE"]) {
            if !(self.is_identifier_token() || self.check_keyword()) {
                return Err(self.parse_error("Expected a service type after OF TYPE"));
            }
            let service_type = self.advance_text()?.to_ascii_uppercase();
            show.this = format!("{} OF TYPE {service_type}", show.this);
        }
        Ok(show)
    }

    /// `[ STARTS WITH '<prefix>' ]`, then ROOT ONLY for tasks, then `[ LIMIT <rows> [ FROM '<name>' ] ]`.
    fn parse_show_starts_and_limit(&mut self, kind: &str, mut show: Show) -> Result<Show> {
        if self.match_identifier("STARTS") {
            self.expect_token(TokenType::With)?;
            show.starts_with = Some(self.parse_show_string("STARTS WITH")?);
        }
        if matches!(kind, "TASKS") && self.match_text_seq(&["ROOT", "ONLY"]) {
            show.this.push_str(" ROOT ONLY");
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
        Ok(show)
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

    /// Whether a LIMIT ends the statement after ACCOUNT, where Snowflake reads it as the account's name.
    fn is_limit_name(&self) -> bool {
        self.check(TokenType::Limit) && self.is_statement_end_at(1)
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
            "DATABASE" | "SCHEMA" => !matches!(kind, "SCHEMA VARIABLES"),
            "APPLICATION" => !matches!(
                kind,
                "PRIVILEGES" | "REFERENCES" | "TELEMETRY EVENT DEFINITIONS"
            ),
            "TABLE" | "VIEW" => true,
            "REPLICATION GROUP" | "FAILOVER GROUP" => matches!(kind, "DATABASES" | "SHARES"),
            "SERVICE" => matches!(kind, "ROLES" | "ENDPOINTS"),
            "CLASS" => matches!(kind, "ROLES" | "FUNCTIONS" | "PROCEDURES"),
            "ORGANIZATION PROFILE" => true,
            _ => false,
        };
        let ends = self.is_at_end()
            || self.check(TokenType::Semicolon)
            || self.check(TokenType::Like)
            || self.check(TokenType::Limit)
            || self.check_identifier("STARTS");
        let name = match (container.as_str(), optional && ends) {
            ("ACCOUNT", _) if self.is_limit_name() => Some(self.parse_show_name()?),
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

    /// `[ ON ACCOUNT | ON USER <name> ]` after LIKE, for the policy kinds a user can be shown with.
    fn parse_show_on_user(&mut self, kind: &str, mut show: Show) -> Result<Show> {
        let on_user = matches!(
            kind,
            "AUTHENTICATION POLICIES"
                | "INGRESS POLICIES"
                | "PASSWORD POLICIES"
                | "SESSION POLICIES"
        );
        if on_user && self.match_token(TokenType::On) {
            let on = self.expect_alter_words(&[&["ACCOUNT"], &["USER"]])?;
            if on == USER {
                show.scope = Some(self.parse_show_name()?);
            }
            show.scope_kind = Some(format!("ON {on}"));
        }
        Ok(show)
    }

    /// SHOW CALLER GRANTS: ON as for SHOW GRANTS; TO takes ROLE, DATABASE ROLE, DATABASE or a name.
    fn parse_snowflake_show_caller_grants(&mut self) -> Result<Expression> {
        if !self.match_token(TokenType::To) {
            let mut grants = self.parse_snowflake_show_grants(false)?;
            if let Expression::Show(show) = &mut grants {
                show.this = "CALLER GRANTS".to_owned();
            }
            return Ok(grants);
        }
        let mut show = Show::new("CALLER GRANTS".to_owned());
        let kinds: &[&[&str]] = &[&["DATABASE", "ROLE"], &["ROLE"], &["DATABASE"]];
        let kind = kinds.iter().find(|words| self.match_text_seq(words));
        show.scope_kind = Some(match kind {
            Some(words) => format!("TO {}", words.join(" ")),
            None => "TO".to_owned(),
        });
        let bare_role = kind.is_some_and(|words| words == &["ROLE"]) && self.is_statement_end_at(0);
        if !bare_role {
            show.scope = Some(self.parse_show_name()?);
        }
        Ok(Expression::Show(Box::new(show)))
    }

    /// SHOW [ FUTURE ] GRANTS after GRANTS: its one ON, TO, OF or IN clause, then LIMIT.
    fn parse_snowflake_show_grants(&mut self, future: bool) -> Result<Expression> {
        let show = Show::new(if future { "FUTURE GRANTS" } else { "GRANTS" }.to_owned());
        let (clause, mut show) = if future {
            let words = self.expect_alter_words(&[
                &["IN", "DATABASE"],
                &["IN", "SCHEMA"],
                &["IN", "DCM", "PROJECT"],
                &["TO", "ROLE"],
                &["TO", "DATABASE", "ROLE"],
            ])?;
            (Some(words), show)
        } else if self.match_token(TokenType::On) {
            let (clause, show) = self.parse_show_grants_on(show)?;
            (Some(clause), show)
        } else if self.match_token(TokenType::To) {
            let (clause, show) = self.parse_show_grants_to(show)?;
            (Some(clause), show)
        } else if self.match_text_seq(&["OF"]) {
            let kinds: &[&[&str]] = &[
                &["APPLICATION", "ROLE"],
                &["SERVICE", "ROLE"],
                &["DATABASE", "ROLE"],
                &["ROLE"],
                &["SHARE"],
            ];
            match kinds.iter().find(|words| self.match_text_seq(words)) {
                Some(words) => (Some(format!("OF {}", words.join(" "))), show),
                None => (Some("OF".to_owned()), show),
            }
        } else if self.match_token(TokenType::In) {
            self.expect_alter_words(&[&["DCM", "PROJECT"]])?;
            (Some("IN DCM PROJECT".to_owned()), show)
        } else {
            (None, show)
        };
        if let Some(clause) = clause {
            let bare = !future
                && self.is_statement_end_at(0)
                && matches!(
                    clause.as_str(),
                    "TO USER"
                        | "TO ROLE"
                        | "TO SHARE"
                        | "TO APPLICATION"
                        | "TO APPLICATION ROLE"
                        | "OF ROLE"
                        | "OF SHARE"
                );
            if show.scope.is_none() && clause != ON_ACCOUNT && !bare {
                show.scope = Some(match clause.as_str() {
                    "TO SERVICE ROLE" | "OF SERVICE ROLE" => self.parse_show_instance_role()?,
                    "TO ROLE" | "OF ROLE" if self.check_next(TokenType::Exclamation) => {
                        self.parse_show_instance_role()?
                    }
                    "TO APPLICATION ROLE" if self.match_token(TokenType::Dot) => {
                        self.parse_show_name()?
                    }
                    _ => self.parse_show_name()?,
                });
            }
            show.scope_kind = Some(clause);
        }
        if show.scope_kind.as_deref() == Some("TO SHARE") && self.match_token(TokenType::In) {
            self.expect_alter_words(&[&["APPLICATION", "PACKAGE"]])?;
            if self.check(TokenType::QuotedIdentifier) {
                return Err(
                    self.parse_error("Snowflake takes an unquoted application package name")
                );
            }
            show.target = Some(self.parse_show_name()?);
            return Ok(Expression::Show(Box::new(show)));
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
    fn parse_show_grants_on(&mut self, mut show: Show) -> Result<(String, Show)> {
        if self.match_identifier(ACCOUNT) {
            if self.is_limit_name() {
                show.scope = Some(self.parse_show_name()?);
            }
            return Ok((ON_ACCOUNT.to_owned(), show));
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
        show.scope = Some(self.parse_show_name()?);
        if kind.is_empty() {
            return Ok(("ON".to_owned(), show));
        }
        if self.check(TokenType::LParen) {
            show.privileges = self.parse_function_param_types()?;
        }
        Ok((format!("ON {}", kind.join(" ")), show))
    }

    /// The grantee after SHOW GRANTS TO: a grantee kind, a bare name, or a class instance role, which names its class as the target.
    fn parse_show_grants_to(&mut self, mut show: Show) -> Result<(String, Show)> {
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
            return Ok((format!("TO {}", words.join(" ")), show));
        }
        show.target = Some(self.parse_show_name()?);
        if self.is_statement_end_at(0) {
            show.scope = show.target.take();
            return Ok(("TO".to_owned(), show));
        }
        self.expect_alter_words(&[&["ROLE"]])?;
        show.scope = Some(self.parse_show_instance_role()?);
        Ok(("TO INSTANCE ROLE".to_owned(), show))
    }

    /// `<instance_name>!<role_name>`, as a pair.
    fn parse_show_instance_role(&mut self) -> Result<Expression> {
        let instance = self.parse_show_name()?;
        self.expect_token(TokenType::Exclamation)?;
        let role = Expression::Identifier(self.parse_simple_name()?);
        Ok(Expression::Tuple(Box::new(
            sqlquack_syntax::expressions::types::models::Tuple {
                expressions: vec![instance, role],
            },
        )))
    }
}
