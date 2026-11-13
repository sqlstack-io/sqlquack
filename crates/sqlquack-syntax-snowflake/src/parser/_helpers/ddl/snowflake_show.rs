//! Snowflake SHOW: a documented kind, then the clauses that kind takes, each with its own operand.

use crate::parser::constants::ACCOUNT;
use crate::parser::constants::ON_ACCOUNT;
use crate::parser::constants::USER;
use crate::parser::constants::{
    SHARED_SHOW_KINDS, SNOWFLAKE_GRANT_PRIVILEGES, SNOWFLAKE_SHOW_CONTAINERS,
    SNOWFLAKE_SHOW_EXTRA_CONTAINERS, SNOWFLAKE_SHOW_GENERAL_CONTAINERS, SNOWFLAKE_SHOW_KINDS,
    SNOWFLAKE_SHOW_REQUIRED_CONTAINERS,
};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::queries::models::Limit;
use sqlquack_syntax::expressions::statements::models::Show;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The filters SHOW AVAILABLE LISTINGS takes after LIMIT, each `= TRUE`, in their documented order.
const LISTING_FILTERS: &[&str] = &["IS_IMPORTED", "IS_ORGANIZATION", "IS_SHARED_WITH_ME"];

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
        if matches!(kind.as_str(), "SERVICES") && self.match_text_seq(&["EXCLUDE", "JOBS"]) {
            show.this.push_str(" EXCLUDE JOBS");
        }
        if matches!(kind.as_str(), "SERVICES") && self.match_text_seq(&["OF", "TYPE"]) {
            show.this.push_str(" OF TYPE ");
            loop {
                if !(self.is_identifier_token() || self.check_keyword()) {
                    return Err(self.parse_error("Expected a service type after OF TYPE"));
                }
                show.this
                    .push_str(&self.advance_text()?.to_ascii_uppercase());
                if !self.match_token(TokenType::Comma) {
                    break;
                }
                show.this.push_str(", ");
            }
        }
        if matches!(kind.as_str(), "OPENFLOW DATA PLANE INTEGRATIONS") {
            return Ok(Some(Expression::Show(Box::new(show))));
        }
        if kind.starts_with("DELEGATED AUTHORIZATIONS ") {
            if kind.ends_with(" USER") && self.is_statement_end_at(0) {
                self.current -= 1;
                show.this = "DELEGATED AUTHORIZATIONS BY".to_owned();
            }
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
        let show = self.parse_show_on_user(&kind, show)?;
        let show = self.parse_show_in(&kind, show)?;
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

    /// `IN <container> [ <name> ]`, which some kinds require and VARIABLES never takes.
    fn parse_show_in(&mut self, kind: &str, mut show: Show) -> Result<Show> {
        let required = SNOWFLAKE_SHOW_REQUIRED_CONTAINERS
            .iter()
            .find(|(name, _)| *name == kind)
            .map(|(_, containers)| *containers);
        let takes_in = !matches!(kind, "VARIABLES");
        if show.scope_kind.is_none()
            && takes_in
            && (required.is_some() || self.check(TokenType::In))
        {
            self.expect_token(TokenType::In)?;
            let (container, name) = self.parse_show_container(kind)?;
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
        Ok(show)
    }

    /// `[ STARTS WITH '<prefix>' ]`, then ROOT ONLY for tasks, then `[ LIMIT <rows> [ FROM '<name>' ] ]`.
    fn parse_show_starts_and_limit(&mut self, kind: &str, mut show: Show) -> Result<Show> {
        if self.match_identifier("STARTS") {
            self.expect_token(TokenType::With)?;
            show.starts_with = Some(self.parse_show_string("STARTS WITH")?);
        }
        if matches!(kind, "TASKS") && self.match_text_seq(&["ROOT", "ONLY"]) {
            show.root_only = true;
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

    /// SHOW AVAILABLE LISTINGS' `[ IS_IMPORTED = TRUE ] [ IS_ORGANIZATION = TRUE ] [ IS_SHARED_WITH_ME = TRUE ]` after LIMIT.
    pub(crate) fn parse_show_listing_filters(&mut self, kind: &str) -> Result<Vec<String>> {
        let mut filters: Vec<String> = Vec::new();
        if !self.is_snowflake() || kind != crate::parser::constants::AVAILABLE_LISTINGS {
            return Ok(filters);
        }
        for filter in LISTING_FILTERS {
            if self.match_text_seq(&[filter]) {
                self.expect_token(TokenType::Eq)?;
                self.expect_token(TokenType::True)?;
                filters.push((*filter).to_owned());
            }
        }
        Ok(filters)
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

    /// Whether the scope after IN ends `offset` tokens ahead: at the end of the statement, LIKE, LIMIT or STARTS WITH.
    fn ends_show_scope_at(&self, offset: usize) -> bool {
        self.peek_nth(offset).is_none_or(|token| {
            matches!(
                token.token_type,
                TokenType::Eof | TokenType::Semicolon | TokenType::Like | TokenType::Limit
            ) || (matches!(token.token_type, TokenType::Identifier | TokenType::Var)
                && token.text.eq_ignore_ascii_case("STARTS"))
        })
    }

    /// The container after IN and its name, which ACCOUNT, DATABASE or SCHEMA may lack; a lone APPLICATION, CLASS, DATASET, GATEWAY, GRAPH, LISTING or MODEL is one.
    fn parse_show_container(&mut self, kind: &str) -> Result<(Option<String>, Option<Expression>)> {
        let Some(words) = SNOWFLAKE_SHOW_CONTAINERS
            .iter()
            .find(|container| self.check_text_seq(container))
        else {
            return Ok((None, Some(self.parse_show_name()?)));
        };
        let container = words.join(" ");
        let groups = matches!(kind, "FAILOVER GROUPS" | "REPLICATION GROUPS");
        let optional = match container.as_str() {
            "ACCOUNT" => !groups,
            "DATABASE" | "SCHEMA" => true,
            "APPLICATION" => !matches!(
                kind,
                "PRIVILEGES" | "REFERENCES" | "TELEMETRY EVENT DEFINITIONS"
            ),
            "TABLE" | "VIEW" => true,
            "REPLICATION GROUP" | "FAILOVER GROUP" => matches!(kind, "DATABASES" | "SHARES"),
            "SERVICE" => matches!(kind, "ROLES" | "ENDPOINTS"),
            "CLASS" => matches!(kind, "ROLES" | "FUNCTIONS" | "PROCEDURES"),
            "ORGANIZATION PROFILE" | "BACKUP SET" | "SNAPSHOT SET" => true,
            _ => false,
        };
        let lone_name = match container.as_str() {
            "APPLICATION" | "CLASS" => optional,
            "DATASET" | "GATEWAY" | "GRAPH" | "LISTING" | "MODEL" => true,
            _ => false,
        };
        if lone_name && self.ends_show_scope_at(1) {
            return Ok((None, Some(self.parse_show_name()?)));
        }
        self.match_text_seq(words);
        let ends = self.ends_show_scope_at(0);
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

    /// `WITH PRIVILEGES <privilege> [ , ... ]` with documented privileges, or `WITH PRIMARY <account>.<database>`.
    fn parse_show_with(&mut self, kind: &str, mut show: Show) -> Result<Show> {
        if matches!(kind, "REPLICATION DATABASES") && self.match_text_seq(&["PRIMARY"]) {
            show.target = Some(self.parse_show_name()?);
            return Ok(show);
        }
        if !self.match_identifier("PRIVILEGES") {
            return Err(self.parse_error("Expected PRIVILEGES after WITH"));
        }
        loop {
            let found = SNOWFLAKE_GRANT_PRIVILEGES
                .iter()
                .map(|name| name.split(' ').collect::<Vec<_>>())
                .filter(|parts| {
                    self.check_text_seq(parts)
                        && (self.peek_nth(parts.len()).is_some_and(|token| {
                            matches!(
                                token.token_type,
                                TokenType::Comma | TokenType::RParen | TokenType::DArrow
                            )
                        }) || self.is_statement_end_at(parts.len()))
                })
                .max_by_key(Vec::len);
            let Some(parts) = found else {
                return Err(self.parse_error("Expected a privilege"));
            };
            self.match_text_seq(&parts);
            show.privileges.push(parts.join(" "));
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
        show.scope_kind = Some(match self.match_grantee_kind(kinds) {
            Some(words) => format!("TO {}", words.join(" ")),
            None => "TO".to_owned(),
        });
        show.scope = Some(self.parse_show_name()?);
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
            match self.match_grantee_kind(kinds) {
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
            if show.scope.is_none() && clause != ON_ACCOUNT {
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
        if let Some(words) = self.match_grantee_kind(GRANTEES) {
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

    /// The first grantee kind a name follows; a kind ending the statement is its last word as the name, as `TO v1`.
    fn match_grantee_kind<'k>(&mut self, kinds: &[&'k [&'k str]]) -> Option<&'k [&'k str]> {
        let words = *kinds
            .iter()
            .find(|words| self.check_text_seq(words) && !self.is_statement_end_at(words.len()))?;
        self.match_text_seq(words);
        Some(words)
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
