//! Parser ddl: Snowflake CREATE VIEW and CREATE MATERIALIZED VIEW, checked against their grammar.

use crate::parser::_helpers::ddl::snowflake_semantic_view::{part, tuple};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::schema_objects::models::{CreateView, ViewColumn};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The words that make a view temporary, longest first.
const VIEW_SCOPES: [&[&str]; 9] = [
    &["LOCAL", "TEMPORARY"],
    &["LOCAL", "TEMP"],
    &["LOCAL", "VOLATILE"],
    &["GLOBAL", "TEMPORARY"],
    &["GLOBAL", "TEMP"],
    &["GLOBAL", "VOLATILE"],
    &["TEMPORARY"],
    &["TEMP"],
    &["VOLATILE"],
];

/// What a COPY = ( ... ) list may name.
const VIEW_COPY_KINDS: [&[&str]; 3] = [&["COMMENTS"], &["TAGS"], &["GRANTS"]];

/// The view kinds whose clauses differ: a view, a materialized view, and CREATE OR ALTER or DEFINE of a view.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewKind {
    View,
    Materialized,
    OrAlter,
}

impl Parser {
    /// A Snowflake CREATE [ OR REPLACE | OR ALTER ] [ SECURE ] [ <scope> ] [ RECURSIVE ] VIEW, or [ SECURE ] [ INTERACTIVE ] MATERIALIZED VIEW; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_view(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() {
            return Ok(None);
        }
        let start = self.current;
        let secure = self.match_text_seq(&["SECURE"]);
        let scope = VIEW_SCOPES
            .iter()
            .find(|words| self.match_text_seq(words))
            .map(|words| words.join(" "));
        let interactive = self.match_text_seq(&["INTERACTIVE"]);
        let recursive = self.match_text_seq(&["RECURSIVE"]);
        let materialized = self.match_token(TokenType::Materialized);
        if !self.check(TokenType::View) {
            self.current = start;
            return Ok(None);
        }
        if materialized && (scope.is_some() || recursive || or_alter) {
            return Err(self.parse_error("Expected VIEW; a materialized view is not temporary, recursive or altered in place"));
        }
        if recursive && scope.is_some() {
            return Err(self.parse_error(
                "Expected VIEW; Snowflake refuses RECURSIVE after a temporary scope",
            ));
        }
        if interactive && !materialized {
            return Err(self.parse_error("Expected MATERIALIZED VIEW after INTERACTIVE"));
        }
        self.skip();
        let kind = match (materialized, or_alter) {
            (true, _) => ViewKind::Materialized,
            (false, true) => ViewKind::OrAlter,
            (false, false) => ViewKind::View,
        };
        let if_not_exists = kind != ViewKind::OrAlter && self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_iceberg_object_name()?;
        let temporary = scope.is_some();
        let mut properties: Vec<Expression> = [scope, recursive.then(|| "RECURSIVE".to_owned())]
            .into_iter()
            .chain([interactive.then(|| "INTERACTIVE".to_owned())])
            .flatten()
            .map(|word| Expression::Identifier(Identifier::new(word)))
            .collect();
        let mut copy_grants = false;
        if kind == ViewKind::Materialized {
            copy_grants = self.match_text_seq(&["COPY", "GRANTS"]);
            if self.check(TokenType::Copy) && self.check_next(TokenType::Eq) {
                self.skip();
                self.skip();
                properties.push(part("COPY", self.parse_view_copy_list()?));
            }
        }
        let mut columns = match self.check(TokenType::LParen) {
            true => self.parse_snowflake_view_columns()?,
            false => Vec::new(),
        };
        while !self.is_at_end() && !self.check(TokenType::As) && !self.check(TokenType::Semicolon) {
            if kind == ViewKind::View && columns.is_empty() && self.check(TokenType::LParen) {
                columns = self.parse_snowflake_view_columns()?;
                continue;
            }
            match self.parse_snowflake_view_clause(kind)? {
                Some(clause) => properties.push(clause),
                None => copy_grants = true,
            }
        }
        if !self.match_token(TokenType::As) {
            return Err(self.parse_error("Expected AS and the view's query"));
        }
        let query_parenthesized = self.check(TokenType::LParen);
        let query = self.parse_create_view_query(query_parenthesized)?;
        let mut view = CreateView::new(String::new(), query);
        view.name = name;
        view.columns = columns;
        view.or_replace = or_replace;
        view.or_alter = or_alter;
        view.if_not_exists = if_not_exists;
        view.materialized = materialized;
        view.temporary = temporary;
        view.secure = secure;
        view.query_parenthesized = query_parenthesized;
        view.copy_grants = copy_grants;
        view.table_properties = properties;
        Ok(Some(Expression::CreateView(Box::new(view))))
    }

    /// `( <column> [ <option> ... ] [ , ... ] )`, which needs a column.
    fn parse_snowflake_view_columns(&mut self) -> Result<Vec<ViewColumn>> {
        self.expect_token(TokenType::LParen)?;
        let mut columns: Vec<ViewColumn> = Vec::new();
        loop {
            let mut column = ViewColumn::new(String::new());
            column.name = self.parse_external_name()?;
            while !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
                if self.match_token(TokenType::Comment) {
                    self.match_token(TokenType::Eq);
                    column.comment = Some(self.expect_string()?);
                    continue;
                }
                let with = self.match_token(TokenType::With);
                let option = match self.check_text_seq(&["TAG"]) {
                    true => {
                        self.skip();
                        part("TAG", self.parse_view_tags()?)
                    }
                    false => self.parse_view_policy(&[&["MASKING"], &["PROJECTION"]], with)?,
                };
                column.options.push(option);
            }
            columns.push(column);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(columns)
    }

    /// One view-level clause before AS; None for COPY GRANTS, which the view node holds itself.
    fn parse_snowflake_view_clause(&mut self, kind: ViewKind) -> Result<Option<Expression>> {
        if kind == ViewKind::View && self.check(TokenType::Copy) {
            self.skip();
            if self.match_text_seq(&["GRANTS"]) {
                return Ok(None);
            }
            if self.match_text_seq(&["TAGS"]) {
                return Ok(Some(Expression::Identifier(Identifier::new("COPY TAGS"))));
            }
            self.expect_token(TokenType::Eq)?;
            return Ok(Some(part("COPY", self.parse_view_copy_list()?)));
        }
        let keys: &[&str] = match kind {
            ViewKind::Materialized => &["COMMENT"],
            _ => &["COMMENT", "CHANGE_TRACKING"],
        };
        for key in keys {
            if self.check_text_seq(&[key]) && self.check_next(TokenType::Eq) {
                self.skip();
                self.skip();
                return Ok(Some(part(key, self.parse_external_scalar()?)));
            }
        }
        if kind == ViewKind::OrAlter {
            return Err(self.parse_error("Expected CHANGE_TRACKING, COMMENT or AS"));
        }
        if kind == ViewKind::Materialized && self.match_text_seq(&["CLUSTER", "BY"]) {
            self.expect_token(TokenType::LParen)?;
            let keys = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Some(part("CLUSTER BY", tuple(keys))));
        }
        let with = self.match_token(TokenType::With);
        if self.match_text_seq(&["TAG"]) {
            return Ok(Some(part("TAG", self.parse_view_tags()?)));
        }
        if with && self.match_text_seq(&["CONTACT"]) {
            return Ok(Some(part("CONTACT", self.parse_view_contacts()?)));
        }
        let policies: &[&[&str]] = match kind {
            ViewKind::Materialized => &[&["ROW", "ACCESS"], &["AGGREGATION"]],
            _ => &[&["ROW", "ACCESS"], &["AGGREGATION"], &["JOIN"]],
        };
        self.parse_view_policy(policies, with).map(Some)
    }

    /// `<kind> POLICY <policy>` and what that kind takes after it: USING ( ... ) for masking, ON ( ... ) for row access, ENTITY KEY ( ... ) for aggregation, ALLOWED JOIN KEYS ( ... ) for join.
    fn parse_view_policy(&mut self, kinds: &[&[&str]], with: bool) -> Result<Expression> {
        let Some(kind) = kinds.iter().find(|words| self.match_text_seq(words)) else {
            let expected = match with {
                true => "a policy, TAG or CONTACT after WITH",
                false => "a view clause or AS",
            };
            return Err(self.parse_error(format!("Expected {expected}")));
        };
        if !self.match_text_seq(&["POLICY"]) {
            return Err(self.parse_error("Expected POLICY"));
        }
        let mut parts = vec![Expression::Table(Box::new(
            self.parse_iceberg_object_name()?,
        ))];
        let (columns, empty): (Option<&[&str]>, bool) = match kind[0] {
            "MASKING" => (self.check(TokenType::Using).then_some(&["USING"]), false),
            "ROW" => (Some(&["ON"]), true),
            "AGGREGATION" => (
                self.check_text_seq(&["ENTITY"])
                    .then_some(&["ENTITY", "KEY"]),
                false,
            ),
            "JOIN" => (
                self.check_text_seq(&["ALLOWED"])
                    .then_some(&["ALLOWED", "JOIN", "KEYS"]),
                false,
            ),
            _ => (None, false),
        };
        if let Some(words) = columns {
            if !self.match_text_seq(words) {
                return Err(self.parse_error(format!("Expected {}", words.join(" "))));
            }
            parts.push(part(&words.join(" "), self.parse_view_names(empty)?));
        }
        Ok(part(&format!("{} POLICY", kind.join(" ")), tuple(parts)))
    }

    /// `( <name> [ , ... ] )`, which needs a name unless `empty`: Snowflake accepts a row access policy ON ( ).
    fn parse_view_names(&mut self, empty: bool) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut names: Vec<Expression> = Vec::new();
        while !empty || !self.check(TokenType::RParen) {
            names.push(Expression::Identifier(self.parse_external_name()?));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(tuple(names))
    }

    /// `( <tag> = <value> [ , ... ] )` after TAG.
    fn parse_view_tags(&mut self) -> Result<Expression> {
        let tags = self.parse_external_tags()?;
        Ok(tuple(
            tags.into_iter().filter_map(|tag| tag.value).collect(),
        ))
    }

    /// `( <purpose> = <contact> [ , ... ] )` after CONTACT.
    fn parse_view_contacts(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut contacts: Vec<Expression> = Vec::new();
        loop {
            let purpose = self.parse_external_name()?;
            self.expect_token(TokenType::Eq)?;
            let contact = self.parse_external_name()?;
            contacts.push(Expression::PropertyEQ(Box::new(BinaryOp::new(
                Expression::Identifier(purpose),
                Expression::Identifier(contact),
            ))));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(tuple(contacts))
    }

    /// `( [ <kind> [ , ... ] ] )` after COPY =, each kind COMMENTS, TAGS or GRANTS.
    fn parse_view_copy_list(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut kinds: Vec<Expression> = Vec::new();
        while !self.check(TokenType::RParen) || !kinds.is_empty() {
            let kind = self.expect_alter_words(&VIEW_COPY_KINDS)?;
            kinds.push(Expression::Identifier(Identifier::new(kind)));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(tuple(kinds))
    }
}
