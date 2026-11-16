//! The auxiliary statements (SqlBaseParser.g4), such as `DESCRIBE`, `SHOW` and `SET`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::{
    Grant, GrantPrincipal, Privilege, Revoke,
};
use sqlquack_syntax::expressions::statements::models::{
    Cache, Comment, Declare, DeclareItem, Describe, ResetStatement, SetItem, SetStatement, Show,
    Truncate, Uncache, Use, UseKind,
};
use sqlquack_syntax::tokens::models::SourceSpan;

use crate::parse::_helpers::commands::alters::partition;
use crate::parse::_helpers::commands::queries::limit;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    call, column, column_from, dot, joined, null, property, string, table, tuple, utility, var,
};
use crate::parse::constants::{
    ADD_RESOURCE, ADD_RESOURCES, CACHE_SELECT, CATALOG, COMMENT_KINDS, DESCRIBE_KINDS,
    EXPLAIN_MODES, FUNCTIONS, IDENTIFIER_CLAUSE, LIST_RESOURCES, MIN_BACKQUOTED_LENGTH,
    MIN_PATH_PARTS, NAMELESS_SECURABLES, PATH_ELEMENTS, SCHEMA_SCOPE, SECURABLE_KINDS, SET_VERBOSE,
};
use crate::scan::models::TokenKind;

/// `configKeyDef`: the characters a configuration key written without quotes may hold.
fn is_config_key(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_\\.:".contains(character))
}

fn table_expression(parts: Vec<Identifier>) -> Expression {
    Expression::Table(Box::new(table(parts)))
}

fn set_item(name: &str, value: Expression, kind: Option<&str>) -> SetItem {
    SetItem {
        name: Expression::Identifier(Identifier::new(name)),
        value,
        kind: kind.map(str::to_owned),
        no_equals: false,
    }
}

fn set_statement(items: Vec<SetItem>) -> Expression {
    Expression::SetStatement(Box::new(SetStatement { items }))
}

/// `SHOW` of `node` for one partition, which `Show` cannot hold: its parts, then the partition.
fn shown_partition(node: Show, spec: Expression) -> Expression {
    let mut operands: Vec<Expression> = node.target.into_iter().collect();
    if let (Some(keyword), Some(scope)) = (node.scope_keyword, node.scope) {
        operands.push(property(&keyword, Some(scope)));
    }
    if let Some(pattern) = node.like {
        operands.push(property("LIKE", Some(pattern)));
    }
    operands.push(spec);
    utility(format!("SHOW {}", node.this), operands)
}

/// `DESCRIBE` of one column, which `Describe` cannot hold: the table, any partition, the column.
fn described_column(node: Describe, column: Expression) -> Expression {
    let mut words: Vec<&str> = vec![match node.short_form {
        true => "DESC",
        false => "DESCRIBE",
    }];
    words.extend(node.kind.as_deref());
    words.extend(node.extended.then_some("EXTENDED"));
    words.extend(node.formatted.then_some("FORMATTED"));
    let mut operands: Vec<Expression> = vec![node.target];
    operands.extend(node.partition.map(|partition| *partition));
    operands.push(column);
    utility(words.join(" "), operands)
}

impl Parser<'_> {
    /// The source text from the current token to the end of the statement, consumed.
    fn statement_text(&mut self) -> Parsing<String> {
        let first: usize = self.at;
        self.skip_statement_rest()?;
        Ok(match (self.tokens.get(first), self.at > first) {
            (Some(from), true) => self.sql[from.start..self.tokens[self.at - 1].end].to_owned(),
            _ => String::new(),
        })
    }

    /// Whether the word `offset` ahead names an object kind: an identifier follows it.
    fn kind_at(&self, offset: usize) -> bool {
        self.is_at(offset + 1, is_identifier) || self.string_at(offset + 1)
    }

    /// `DESC` or `DESCRIBE` of a relation, query, function, procedure, schema or other object.
    pub(crate) fn describe_statement(&mut self) -> Parsing<Expression> {
        let start_at: usize = self.at;
        let short_form: bool = self.kw("DESC");
        self.at += 1;
        let word: Option<String> = self.word_at(0);
        let mut node: Describe = Describe::new(null());
        node.short_form = short_form;
        match word.as_deref() {
            Some("QUERY") => {
                self.at += 1;
                node.target = self.query()?;
                node.kind = Some("QUERY".to_owned());
            }
            Some("SELECT" | "WITH" | "VALUES" | "FROM") if !self.ends_at(1) => {
                node.target = self.query()?;
            }
            None if self.is_char('(') => {
                node.target = self.query()?;
            }
            Some("FUNCTION") if self.kind_at(0) || self.kw_at(1, "EXTENDED") => {
                self.at += 1;
                node.extended = self.eat_kw("EXTENDED");
                node.target = match self.string_at(0) {
                    true => self.string_literal()?,
                    false => table_expression(self.multipart_identifier()?),
                };
                node.kind = Some("FUNCTION".to_owned());
            }
            Some("PROCEDURE") if self.kind_at(0) || self.kw_at(1, "EXTENDED") => {
                self.at += 1;
                node.extended = self.eat_kw("EXTENDED");
                node.target = table_expression(self.multipart_identifier()?);
                node.kind = Some("PROCEDURE".to_owned());
            }
            Some(kind @ ("SCHEMA" | "DATABASE" | "NAMESPACE"))
                if self.kind_at(0) || self.kw_at(1, "EXTENDED") =>
            {
                node.kind = Some(kind.to_owned());
                self.at += 1;
                node.extended = self.eat_kw("EXTENDED");
                node.target = Expression::Identifier(joined(self.multipart_identifier()?));
            }
            Some(style @ ("HISTORY" | "DETAIL")) if self.kind_at(0) => {
                node.style = Some(style.to_owned());
                self.at += 1;
                node.target = match self.string_at(0) {
                    true => self.string_literal()?,
                    false => Expression::Table(Box::new(self.table_reference()?)),
                };
            }
            Some(kind) if DESCRIBE_KINDS.contains(&kind) && self.kind_at(0) => {
                node.kind = Some(kind.to_owned());
                self.at += 1;
                node.extended = self.eat_kw("EXTENDED");
                node.target = match kind == CATALOG && self.string_at(0) {
                    true => self.string_literal()?,
                    false => Expression::Identifier(joined(self.multipart_identifier()?)),
                };
            }
            Some("EXTERNAL" | "STORAGE" | "GOVERNED" | "POLICY" | "MATERIALIZED")
                if self.is_at(1, is_identifier) && self.kind_at(1) =>
            {
                self.at = start_at;
                return self.opaque_statement();
            }
            _ => {
                if self.kw("TABLE") && !self.char_at(1, '.') && !self.ends_at(1) {
                    self.at += 1;
                    node.kind = Some("TABLE".to_owned());
                }
                if self.kw("EXTENDED") && !self.ends_at(1) && !self.char_at(1, '.') {
                    self.at += 1;
                    node.extended = true;
                } else if self.kw("FORMATTED") && !self.ends_at(1) && !self.char_at(1, '.') {
                    self.at += 1;
                    node.formatted = true;
                }
                node.target = Expression::Table(Box::new(self.table_reference()?));
                if self.kw("PARTITION") && self.char_at(1, '(') {
                    node.partition = Some(Box::new(partition(self.partition_spec()?)));
                }
                if self.is(is_identifier) && !(self.kw("AS") && self.kw_at(1, "JSON")) {
                    let mut names = self.multipart_identifier()?.into_iter();
                    let first: Identifier = names.next().unwrap_or_else(Identifier::empty);
                    let column: Expression = names.fold(column(None, first), dot);
                    if self.kw("AS") && self.kw_at(1, "JSON") {
                        return Err(self.error_with(
                            "[UNSUPPORTED_FEATURE.DESC_TABLE_COLUMN_JSON] The feature is not \
                             supported: DESC TABLE COLUMN AS JSON.",
                            self.tokens[start_at].start,
                            true,
                        ));
                    }
                    return Ok(described_column(node, column));
                }
                if self.kw("AS") && self.kw_at(1, "JSON") {
                    self.at += 2;
                    node.as_json = true;
                }
            }
        }
        Ok(Expression::Describe(Box::new(node)))
    }

    /// `FROM` or `IN`, when written.
    fn eat_from_or_in(&mut self) -> Option<String> {
        ["FROM", "IN"]
            .into_iter()
            .find(|word| self.eat_kw(word))
            .map(str::to_owned)
    }

    /// `node` with `[FROM | IN] name` as its scope, when written; a schema may be an `IDENTIFIER (...)`.
    fn show_scope(&mut self, mut node: Show, scope_kind: &str) -> Parsing<Show> {
        if let Some(keyword) = self.eat_from_or_in() {
            node.scope_keyword = Some(keyword);
            node.scope_kind = Some(scope_kind.to_owned());
            node.scope = Some(
                match scope_kind == SCHEMA_SCOPE
                    && self.kw(IDENTIFIER_CLAUSE)
                    && self.char_at(1, '(')
                {
                    true => {
                        self.at += 2;
                        let name: Expression = self.expression()?;
                        self.expect_char(')')?;
                        call(IDENTIFIER_CLAUSE, vec![name])
                    }
                    false => Expression::Identifier(joined(self.multipart_identifier()?)),
                },
            );
        }
        Ok(node)
    }

    /// `node` with `[LIKE] 'pattern'`, when written.
    fn show_pattern(&mut self, mut node: Show) -> Parsing<Show> {
        node.like_keyword = self.eat_kw("LIKE");
        if node.like_keyword || self.string_at(0) {
            node.like = Some(self.string_literal()?);
        }
        Ok(node)
    }

    /// `SHOW` of the objects the shared `Show` holds; Spark refuses any other `SHOW`.
    pub(crate) fn show_statement(&mut self) -> Parsing<Expression> {
        let start_at: usize = self.at;
        self.at += 1;
        let word: String = self.word_at(0).unwrap_or_default();
        let scoped = |parser: &mut Self, this: &str, scope_kind: &str| -> Parsing<Show> {
            let node: Show = parser.show_scope(Show::new(this), scope_kind)?;
            parser.show_pattern(node)
        };
        let node: Show = match word.as_str() {
            "TABLES" if self.kw_at(1, "DROPPED") => {
                self.at += 2;
                let mut node: Show = self.show_scope(Show::new("TABLES DROPPED"), "SCHEMA")?;
                if self.eat_kw("LIMIT") {
                    node.limit = Some(Box::new(limit(self.slot_value("int")?)));
                }
                node
            }
            "TABLES" => {
                self.at += 1;
                scoped(self, "TABLES", "SCHEMA")?
            }
            "TABLE" if self.kw_at(1, "EXTENDED") => return self.show_table_extended(),
            "VIEWS" => {
                self.at += 1;
                scoped(self, "VIEWS", "SCHEMA")?
            }
            "SCHEMAS" | "DATABASES" | "NAMESPACES" => {
                self.at += 1;
                scoped(self, word.as_str(), "CATALOG")?
            }
            "CATALOGS" | "COLLATIONS" => {
                self.at += 1;
                self.show_pattern(Show::new(word.as_str()))?
            }
            "FUNCTIONS" | "USER" | "SYSTEM" | "ALL"
                if word == FUNCTIONS || self.kw_at(1, FUNCTIONS) =>
            {
                self.show_functions(&word)?
            }
            "PROCEDURES" => {
                self.at += 1;
                self.show_scope(Show::new("PROCEDURES"), "SCHEMA")?
            }
            "COLUMNS" => {
                self.at += 1;
                let mut node: Show = Show::new("COLUMNS");
                node.target_keyword = Some(self.eat_from_or_in().ok_or_else(|| self.error())?);
                node.target = Some(Expression::Table(Box::new(self.table_reference()?)));
                if let Some(keyword) = self.eat_from_or_in() {
                    node.scope_keyword = Some(keyword);
                    node.db = Some(Expression::Identifier(joined(self.multipart_identifier()?)));
                }
                node
            }
            "GRANTS" | "GRANT" => self.show_grants()?,
            "TBLPROPERTIES" => self.show_table_properties()?,
            "PARTITIONS" => {
                self.at += 1;
                let mut node: Show = Show::new("PARTITIONS");
                node.target = Some(Expression::Table(Box::new(self.table_reference()?)));
                if self.kw("PARTITION") {
                    let spec: Expression = partition(self.partition_spec()?);
                    return Ok(shown_partition(node, spec));
                }
                node
            }
            "CREATE" if self.kw_at(1, "TABLE") => {
                self.at += 2;
                let mut node: Show = Show::new("CREATE TABLE");
                node.target = Some(Expression::Table(Box::new(self.table_reference()?)));
                let at: usize = self.offset();
                if self.eat_kws(&["AS", "SERDE"]) {
                    self.gap("show_create_table_serde", at);
                }
                node
            }
            "CURRENT" if self.kw_in(1, &["SCHEMA", "DATABASE", "NAMESPACE"]) => {
                self.at += 2;
                Show::new("CURRENT SCHEMA")
            }
            _ if word == FUNCTIONS || self.kw_at(1, FUNCTIONS) => {
                return Err(self.error_with(
                    "[INVALID_SQL_SYNTAX.SHOW_FUNCTIONS_INVALID_SCOPE] Invalid SQL syntax: SHOW \
                     FUNCTIONS takes the scope USER, SYSTEM or ALL.",
                    self.tokens[start_at].start,
                    true,
                ));
            }
            _ => return Err(self.error()),
        };
        Ok(Expression::Show(Box::new(node)))
    }

    /// `SHOW TABLE EXTENDED [scope] LIKE pattern [PARTITION (...)]`.
    fn show_table_extended(&mut self) -> Parsing<Expression> {
        self.at += 2;
        let mut node: Show = self.show_scope(Show::new("TABLE EXTENDED"), "SCHEMA")?;
        self.expect_kw("LIKE")?;
        node.like_keyword = true;
        node.like = Some(self.string_literal()?);
        if self.kw("PARTITION") {
            let spec: Expression = partition(self.partition_spec()?);
            return Ok(shown_partition(node, spec));
        }
        Ok(Expression::Show(Box::new(node)))
    }

    /// `SHOW [USER | SYSTEM | ALL] FUNCTIONS [scope] [[LIKE] pattern]`, from `word`.
    fn show_functions(&mut self, word: &str) -> Parsing<Show> {
        let this: String = match word {
            "FUNCTIONS" => "FUNCTIONS".to_owned(),
            scope => format!("{scope} FUNCTIONS"),
        };
        self.at += 1 + usize::from(word != FUNCTIONS);
        let mut node: Show = self.show_scope(Show::new(this), "SCHEMA")?;
        node.like_keyword = self.eat_kw("LIKE");
        if self.string_at(0) {
            node.like = Some(self.string_literal()?);
        } else if node.like_keyword || self.is(is_identifier) {
            node.like = Some(Expression::Identifier(joined(self.multipart_identifier()?)));
        }
        Ok(node)
    }

    /// `SHOW GRANTS [principal] ON securable` or `SHOW GRANTS TO RECIPIENT name`.
    fn show_grants(&mut self) -> Parsing<Show> {
        self.at += 1;
        let mut node: Show = Show::new("GRANTS");
        if self.kw("TO") && self.kw_at(1, "RECIPIENT") {
            self.at += 2;
            node.scope_kind = Some("TO RECIPIENT".to_owned());
            node.scope = Some(Expression::Identifier(self.ident()?));
        } else {
            if !self.kw("ON") {
                node.for_target = Some(Expression::Identifier(self.ident()?));
            }
            let (kind, name) = self.securable()?;
            node.scope_kind = Some(match kind {
                Some(kind) => format!("ON {kind}"),
                None => "ON".to_owned(),
            });
            if !name.name.is_empty() {
                node.scope = Some(Expression::Identifier(name));
            }
        }
        Ok(node)
    }

    /// `SHOW TBLPROPERTIES table [(key)]`.
    fn show_table_properties(&mut self) -> Parsing<Show> {
        self.at += 1;
        let mut node: Show = Show::new("TBLPROPERTIES");
        node.target = Some(Expression::Table(Box::new(self.table_reference()?)));
        if self.eat_char('(') {
            let key: String = match self.string_at(0) {
                true => self.string_lit()?,
                false => joined(self.multipart_identifier()?).name,
            };
            self.expect_char(')')?;
            node.scope_kind = Some("PROPERTY".to_owned());
            node.scope = Some(string(&key));
        }
        Ok(node)
    }

    /// `SET` of a variable, the time zone, the path, the catalog or a configuration.
    pub(crate) fn set_statement(&mut self) -> Parsing<Expression> {
        let set_at: usize = self.offset();
        self.expect_kw("SET")?;
        if self.kw("ROLE") {
            self.at -= 1;
            return self.opaque_statement();
        }
        if self.kw("TIME") && self.kw_at(1, "ZONE") {
            self.at += 2;
            let value: Expression = if self.kw("INTERVAL") {
                self.primary_expression()?
            } else if self.string_at(0) && self.ends_at(1) {
                self.string_literal()?
            } else if self.kw("LOCAL") && self.ends_at(1) {
                self.at += 1;
                var("LOCAL")
            } else {
                return Err(self.error_with("Invalid time zone displacement value.", set_at, true));
            };
            return Ok(set_statement(vec![set_item("TIME ZONE", value, None)]));
        }
        if self.kw_in(0, &["VAR", "VARIABLE"])
            && (self.is_at(1, is_identifier) || self.char_at(1, '('))
        {
            let keyword: String = self.word_at(0).unwrap_or_default();
            self.at += 1;
            if self.is_char('(') {
                let names: Vec<Expression> =
                    self.parenthesized(|parser| parser.multipart_identifier().map(column_from))?;
                self.expect_char('=')?;
                self.expect_char('(')?;
                let query: Expression = self.query()?;
                self.expect_char(')')?;
                let item: SetItem = SetItem {
                    name: tuple(names),
                    value: query,
                    kind: Some(keyword),
                    no_equals: false,
                };
                return Ok(set_statement(vec![item]));
            }
            let items: Vec<SetItem> = self.comma_list(|parser| {
                let name: Expression = column_from(parser.multipart_identifier()?);
                parser.expect_char('=')?;
                let value: Expression = match parser.kw("DEFAULT") && parser.terminal_at(1) {
                    true => {
                        parser.at += 1;
                        var("DEFAULT")
                    }
                    false => parser.expression()?,
                };
                Ok(SetItem {
                    name,
                    value,
                    kind: Some(keyword.clone()),
                    no_equals: false,
                })
            })?;
            return Ok(set_statement(items));
        }
        if self.kw("PATH") && self.char_at(1, '=') {
            let path = self.attempt(|parser| {
                parser.at += 2;
                let elements: Vec<(Expression, usize)> =
                    parser.comma_list(|parser| match parser.word_at(0) {
                        Some(word)
                            if PATH_ELEMENTS.contains(&word.as_str())
                                && !parser.char_at(1, '.') =>
                        {
                            parser.at += 1;
                            Ok((var(&word), 2))
                        }
                        _ => {
                            let parts: Vec<Identifier> = parser.multipart_identifier()?;
                            let count: usize = parts.len();
                            Ok((Expression::Identifier(joined(parts)), count))
                        }
                    })?;
                match parser.ends_at(0) {
                    true => Ok(elements),
                    false => Err(parser.error()),
                }
            });
            if let Ok(elements) = path {
                if let Some((element, _)) =
                    elements.iter().find(|(_, parts)| *parts < MIN_PATH_PARTS)
                {
                    let name: String = match element {
                        Expression::Identifier(name) => name.name.clone(),
                        _ => String::new(),
                    };
                    return Err(self.error_with(
                        &format!(
                            "[INVALID_SQL_PATH_SCHEMA_REFERENCE] Invalid schema reference in SET \
                             PATH: {name}. Use at least two name parts (catalog.schema)."
                        ),
                        set_at,
                        true,
                    ));
                }
                let elements: Vec<Expression> =
                    elements.into_iter().map(|(element, _)| element).collect();
                return Ok(set_statement(vec![set_item("PATH", tuple(elements), None)]));
            }
        }
        if self.kw("QUERY_TAGS") && !self.ends_at(1) && !self.char_at(1, '[') {
            return Err(self.error_with(
                "[SET_QUERY_TAGS_SYNTAX_INVALID] Invalid QUERY_TAGS syntax.",
                set_at,
                true,
            ));
        }
        if self.kw("QUERY_TAGS") && self.char_at(1, '[') {
            let start: usize = self.tokens[self.at - 1].start;
            self.comma_list(|parser| parser.query_tag(set_at))?;
            self.gap("set", start);
            return Ok(null());
        }
        if self.kw("CATALOG") && !self.ends_at(1) && !self.char_at(1, '=') {
            self.at += 1;
            let value: Expression = match self.is(is_identifier) && self.ends_at(1) {
                true => Expression::Identifier(self.ident()?),
                false => self.expression()?,
            };
            return Ok(set_statement(vec![set_item("CATALOG", value, None)]));
        }
        self.configuration(set_at)
    }

    /// `QUERY_TAGS [ 'key' ] = { 'value' | NULL | UNSET }`; any other spelling is Databricks' own error.
    fn query_tag(&mut self, set_at: usize) -> Parsing<()> {
        let keyed: bool = self.eat_kw("QUERY_TAGS") && self.eat_char('[') && self.string_at(0);
        self.at += usize::from(keyed);
        let valued: bool = keyed
            && self.eat_char(']')
            && self.eat_char('=')
            && (self.string_at(0) || self.kw_in(0, &["NULL", "UNSET"]));
        if !valued {
            return Err(self.error_with(
                "[SET_QUERY_TAGS_SYNTAX_INVALID] Invalid QUERY_TAGS syntax.",
                set_at,
                true,
            ));
        }
        self.at += 1;
        Ok(())
    }

    /// `SET key [= value]`, `SET -v` or `SET`, as Spark matches the text after `SET`.
    fn configuration(&mut self, set_at: usize) -> Parsing<Expression> {
        let quoted_key: bool = self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Quoted)
            && (self.ends_at(1) || self.char_at(1, '='));
        if quoted_key {
            let key: String = self.advance()?.value;
            let value: Expression = match self.eat_char('=') {
                true => var(self.statement_text()?.trim()),
                false => null(),
            };
            let mut item: SetItem = set_item(&key, value, None);
            item.name = Expression::Identifier(Identifier::quoted(key));
            return Ok(set_statement(vec![item]));
        }
        let first: usize = self.at;
        let text: String = self.statement_text()?;
        let text: &str = text.trim().trim_end_matches(';');
        if text.is_empty() {
            return Ok(set_statement(Vec::new()));
        }
        if text == SET_VERBOSE {
            return Ok(set_statement(vec![set_item(SET_VERBOSE, null(), None)]));
        }
        if let Some((key, value)) = text.split_once('=') {
            let key: &str = key.trim_end();
            if is_config_key(key) {
                return Ok(set_statement(vec![set_item(key, var(value.trim()), None)]));
            }
            let value: &str = value.trim();
            let backquoted: bool = value.len() >= MIN_BACKQUOTED_LENGTH
                && value.starts_with('`')
                && value.ends_with('`');
            if backquoted && is_config_key(key.trim()) {
                return Ok(set_statement(vec![set_item(key.trim(), var(value), None)]));
            }
        } else if is_config_key(text) {
            return Ok(set_statement(vec![set_item(text, null(), None)]));
        }
        let end: usize = self.at;
        self.at = first;
        let named: bool = end == first + 2
            && self.kw_in(0, &["RECIPIENT", "COLLATION"])
            && self.is_at(1, is_identifier);
        self.at = end;
        if named {
            self.at = first - 1;
            return self.checked_form("set", set_at, 2, &["SET { RECIPIENT | COLLATION } id"]);
        }
        Err(self.error_with(
            "[INVALID_SET_SYNTAX] Expected format is 'SET', 'SET key', or 'SET key=value'.",
            set_at,
            true,
        ))
    }

    /// `RESET [key]`, as Spark matches the text after `RESET`.
    pub(crate) fn reset_statement(&mut self) -> Parsing<Expression> {
        let reset_at: usize = self.offset();
        self.expect_kw("RESET")?;
        let quoted: bool = self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Quoted)
            && self.ends_at(1);
        let name: Option<Expression> = if quoted {
            Some(Expression::Identifier(Identifier::new(
                self.advance()?.value,
            )))
        } else {
            let text: String = self.statement_text()?;
            let key: &str = text.trim();
            if key.is_empty() {
                None
            } else if is_config_key(key) {
                Some(Expression::Identifier(Identifier::new(key)))
            } else {
                return Err(self.error_with(
                    "[INVALID_RESET_COMMAND_FORMAT] Expected format is 'RESET' or 'RESET key'.",
                    reset_at,
                    true,
                ));
            }
        };
        Ok(Expression::ResetStatement(Box::new(ResetStatement {
            scope: None,
            name,
            all: false,
        })))
    }

    /// `USE [CATALOG | SCHEMA | DATABASE | NAMESPACE] name`.
    pub(crate) fn use_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("USE")?;
        let kind: Option<UseKind> = match self.word_at(0).as_deref() {
            Some("CATALOG") if self.kind_at(0) => Some(UseKind::Catalog),
            Some("SCHEMA") if self.kind_at(0) => Some(UseKind::Schema),
            Some("DATABASE" | "NAMESPACE") if self.kind_at(0) => Some(UseKind::Database),
            _ => None,
        };
        self.at += usize::from(kind.is_some());
        let string_name: bool = kind == Some(UseKind::Catalog) && self.string_at(0);
        let parts: Vec<Identifier> = match string_name {
            true => vec![Identifier::new(self.string_lit()?)],
            false => self.multipart_identifier()?,
        };
        Ok(Expression::Use(Box::new(Use {
            kind,
            this: joined(parts.clone()),
            roles: Vec::new(),
            parts,
            string_name,
        })))
    }

    /// `COMMENT ON kind name IS 'text' | NULL`.
    pub(crate) fn comment_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("COMMENT")?;
        self.expect_kw("ON")?;
        let word: String = self.word_at(0).unwrap_or_default();
        let kind: &str = match word.as_str() {
            kind @ ("DATABASE" | "NAMESPACE") => kind,
            kind if COMMENT_KINDS.contains(&kind) => kind,
            _ => return Err(self.error()),
        };
        self.at += 1;
        let this: Expression = match kind {
            CATALOG if self.string_at(0) => self.string_literal()?,
            _ => {
                let parts: Vec<Identifier> = self.multipart_identifier()?;
                match kind {
                    "TABLE" | "VIEW" | "VOLUME" => table_expression(parts),
                    "COLUMN" => column_from(parts),
                    _ => Expression::Identifier(joined(parts)),
                }
            }
        };
        self.expect_kw("IS")?;
        let expression: Expression = match self.eat_kw("NULL") {
            true => null(),
            false => self.string_literal()?,
        };
        Ok(Expression::Comment(Box::new(Comment {
            this,
            kind: kind.to_owned(),
            expression,
            exists: false,
            materialized: false,
        })))
    }

    /// `TRUNCATE TABLE name [PARTITION (...)]`.
    pub(crate) fn truncate_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("TRUNCATE")?;
        self.expect_kw("TABLE")?;
        let mut node: Truncate = Truncate::new(String::new());
        node.table = self.table_reference()?;
        if self.kw("PARTITION") {
            node.partition = Some(Box::new(partition(self.partition_spec()?)));
        }
        Ok(Expression::Truncate(Box::new(node)))
    }

    /// `EXPLAIN [mode] statement`, as a `Describe` of the statement whose style is `EXPLAIN`.
    pub(crate) fn explain_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("EXPLAIN")?;
        let mode: Option<String> = self
            .word_at(0)
            .filter(|word| EXPLAIN_MODES.contains(&word.as_str()) && !self.ends_at(1));
        self.at += usize::from(mode.is_some());
        let statement: Expression = match self.kw("SET") || self.kw("RESET") {
            true if self.kw("RESET") => self.reset_statement()?,
            true => self.set_statement()?,
            false => self.statement()?,
        };
        let mut node: Describe = Describe::new(statement);
        node.extended = mode.as_deref() == Some("EXTENDED");
        node.formatted = mode.as_deref() == Some("FORMATTED");
        node.style = Some(match mode {
            Some(mode) => format!("EXPLAIN {mode}"),
            None => "EXPLAIN".to_owned(),
        });
        Ok(Expression::Describe(Box::new(node)))
    }

    /// `privilege_types`: `ALL [PRIVILEGES]` or privileges of one or more words, up to `ON`.
    fn privileges(&mut self) -> Parsing<Vec<Privilege>> {
        self.comma_list(|parser| {
            let mut words: Vec<String> = Vec::new();
            while parser.is_at(0, |token| token.kind == TokenKind::Word)
                && !(parser.kw("ON") && !words.is_empty())
            {
                words.push(parser.advance()?.value.to_ascii_uppercase());
            }
            if words.is_empty() {
                return Err(parser.error());
            }
            Ok(Privilege {
                name: words.join(" "),
                columns: Vec::new(),
                class: None,
            })
        })
    }

    /// `ON securable`: its kind, when written, and its name; a catalog may go unnamed.
    fn securable(&mut self) -> Parsing<(Option<String>, Identifier)> {
        self.expect_kw("ON")?;
        let mut kind: Option<String> = None;
        for (words, name_optional) in SECURABLE_KINDS.iter() {
            let matched: bool = words
                .iter()
                .enumerate()
                .all(|(offset, word)| self.kw_at(offset, word));
            let named_by_kind: bool = words.len() == 1
                && self.kw_in(words.len(), &["TO", "FROM"])
                && !self.kw_in(words.len() + 1, &["TO", "FROM"])
                && !self.ends_at(words.len() + 1);
            if matched && !named_by_kind && (*name_optional || self.kind_at(words.len() - 1)) {
                self.at += words.len();
                let joined_kind: String = words.join(" ");
                let nameless: bool = NAMELESS_SECURABLES.contains(&joined_kind.as_str());
                kind = Some(joined_kind);
                if nameless || (*name_optional && !self.is(is_identifier)) {
                    return Ok((kind, Identifier::new(String::new())));
                }
                break;
            }
        }
        Ok((kind, joined(self.multipart_identifier()?)))
    }

    /// The principals after `TO` or `FROM`; a share is granted only to a `RECIPIENT`.
    fn principals(&mut self, kind: Option<&str>) -> Parsing<Vec<GrantPrincipal>> {
        let recipient: bool = kind == Some("SHARE");
        if recipient {
            self.expect_kw("RECIPIENT")?;
        }
        self.comma_list(|parser| {
            Ok(GrantPrincipal {
                name: parser.ident()?,
                is_role: false,
                is_group: false,
                is_share: false,
                kind: recipient.then(|| "RECIPIENT".to_owned()),
            })
        })
    }

    /// `GRANT privileges ON securable TO principal`.
    pub(crate) fn grant_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("GRANT")?;
        if self.kw("ROLE") {
            return Err(self.error_with(
                "Operation not supported: GRANT ROLE",
                self.offset(),
                true,
            ));
        }
        let privileges: Vec<Privilege> = self.privileges()?;
        let (kind, securable) = self.securable()?;
        self.expect_kw("TO")?;
        let principals: Vec<GrantPrincipal> = self.principals(kind.as_deref())?;
        Ok(Expression::Grant(Box::new(Grant {
            caller: None,
            bulk: None,
            container: None,
            objects: Vec::new(),
            granted_by: None,
            options: Vec::new(),
            privileges,
            kind,
            securable: Some(securable),
            function_params: Vec::new(),
            principals,
            grant_option: false,
            as_principal: None,
            current_grants: None,
            class: None,
            service: None,
        })))
    }

    /// `DENY privileges ON securable TO principal`, read as `GRANT` is, as privileges and clauses.
    pub(crate) fn deny_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("DENY")?;
        let mut operands: Vec<Expression> = self
            .privileges()?
            .into_iter()
            .map(|privilege| var(&privilege.name))
            .collect();
        let (kind, securable) = self.securable()?;
        let on: String = match &kind {
            Some(kind) => format!("ON {kind}"),
            None => "ON".to_owned(),
        };
        let named: bool = !securable.name.is_empty();
        operands.push(property(
            &on,
            named.then_some(Expression::Identifier(securable)),
        ));
        self.expect_kw("TO")?;
        let mut principals: Vec<Expression> = self
            .principals(kind.as_deref())?
            .into_iter()
            .map(|principal| Expression::Identifier(principal.name))
            .collect();
        let to: &str = match kind.as_deref() {
            Some("SHARE") => "TO RECIPIENT",
            _ => "TO",
        };
        let principals: Expression = match principals.len() {
            1 => principals.remove(0),
            _ => tuple(principals),
        };
        operands.push(property(to, Some(principals)));
        Ok(utility("DENY".to_owned(), operands))
    }

    /// Spark's `(ADD | LIST) resourceType .*?`: a file, jar or archive type, then string resources.
    pub(crate) fn manage_resource(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        let op: String = self.advance()?.value.to_ascii_uppercase();
        let kind: Option<String> = match self.peek() {
            Some(token) if matches!(token.kind, TokenKind::Word | TokenKind::Quoted) => {
                Some(token.value.to_ascii_lowercase())
            }
            _ => None,
        };
        let Some(kind) = kind else {
            return Err(self.error());
        };
        let known: bool = ["file", "files", "jar", "jars", "archive", "archives"]
            .contains(&kind.as_str())
            && self
                .peek()
                .is_some_and(|token| token.kind == TokenKind::Word);
        if !known {
            return Err(self.error_with(
                &format!("Operation not allowed: {op} with resource type '{kind}'."),
                start,
                true,
            ));
        }
        let from: usize = self.at - 1;
        self.skip_statement_rest()?;
        let (construct, pattern): (&str, &str) = match op == ADD_RESOURCE {
            true => ("add", ADD_RESOURCES),
            false => ("list", LIST_RESOURCES),
        };
        match self.tree_within(from, self.at, 2, pattern) {
            Some(tree) => Ok(tree),
            None => {
                self.gap(construct, start);
                Ok(null())
            }
        }
    }

    /// `REVOKE privileges ON securable FROM principal`.
    pub(crate) fn revoke_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("REVOKE")?;
        if self.kw("ROLE") {
            return Err(self.error_with(
                "Operation not supported: REVOKE ROLE",
                self.offset(),
                true,
            ));
        }
        let privileges: Vec<Privilege> = self.privileges()?;
        let (kind, securable) = self.securable()?;
        self.expect_kw("FROM")?;
        let principals: Vec<GrantPrincipal> = self.principals(kind.as_deref())?;
        Ok(Expression::Revoke(Box::new(Revoke {
            caller: None,
            bulk: None,
            container: None,
            objects: Vec::new(),
            granted_by: None,
            options: Vec::new(),
            privileges,
            kind,
            securable: Some(securable),
            function_params: Vec::new(),
            principals,
            grant_option: false,
            cascade: false,
            restrict: false,
            class: None,
            service: None,
        })))
    }

    /// `DECLARE [OR REPLACE] [VARIABLE] name, ... [type] [DEFAULT | = expression]`.
    pub(crate) fn declare_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("DECLARE")?;
        let replace: bool = self.eat_kws(&["OR", "REPLACE"]);
        let cursor: bool = !replace
            && (self.kw_at(1, "CURSOR")
                || (self.kw_in(1, &["ASENSITIVE", "INSENSITIVE"]) && self.kw_at(2, "CURSOR")));
        if cursor {
            self.ident()?;
            self.eat_kw("ASENSITIVE");
            self.eat_kw("INSENSITIVE");
            self.expect_kw("CURSOR")?;
            self.expect_kw("FOR")?;
            self.query()?;
            if self.eat_kws(&["FOR", "READ"]) {
                self.expect_kw("ONLY")?;
            }
            self.gap("declare_cursor", start);
            return Ok(null());
        }
        if self.kw_in(0, &["VAR", "VARIABLE"]) && self.is_at(1, is_identifier) {
            self.at += 1;
        }
        let mut names: Vec<Expression> = self.comma_list(|parser| {
            Ok(Expression::Identifier(joined(
                parser.multipart_identifier()?,
            )))
        })?;
        let type_first: usize = self.at;
        let kind: Option<String> = match self.kw("DEFAULT") || self.is_char('=') || self.ends_at(0)
        {
            true => None,
            false => {
                self.data_type()?;
                let from: usize = self.tokens[type_first].start;
                Some(self.sql[from..self.tokens[self.at - 1].end].to_owned())
            }
        };
        let equals: bool = self.is_char('=');
        let default: Option<Box<Expression>> = match self.eat_kw("DEFAULT") || self.eat_char('=') {
            true => Some(Box::new(self.expression()?)),
            false => None,
        };
        if kind.is_none() && default.is_none() {
            return Err(self.error_with(
                "[INVALID_SQL_SYNTAX.VARIABLE_TYPE_OR_DEFAULT_REQUIRED] The definition of a \
                 SQL variable requires either a datatype or a DEFAULT clause",
                start,
                false,
            ));
        }
        let this: Expression = names.remove(0);
        let item: DeclareItem = DeclareItem {
            this: Box::new(this),
            kind,
            default,
            has_as: false,
            additional_names: names,
            span: SourceSpan::default(),
            equals,
        };
        Ok(Expression::Declare(Box::new(Declare {
            expressions: vec![Expression::DeclareItem(Box::new(item))],
            replace,
        })))
    }

    /// `CACHE [LAZY] TABLE name ...`, or `CACHE SELECT` as documented, else its text as written.
    pub(crate) fn cache_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("CACHE")?;
        if self.kw("SELECT") {
            let first: usize = self.at;
            self.skip_statement_rest()?;
            if let Some(at) = (first..self.at).find(|at| {
                crate::parse::_helpers::tree::cursor::is_word(&self.tokens[*at], "FROM")
                    && self
                        .tokens
                        .get(at + 1)
                        .is_some_and(|token| matches!(token.kind, TokenKind::String { .. }))
            }) {
                self.at = at + 1;
                return Err(self.error());
            }
            if let Some(tree) = self.tree_within(first - 1, self.at, 2, CACHE_SELECT) {
                return Ok(tree);
            }
            let text: Vec<Expression> = (self.at > first + 1)
                .then(|| {
                    string(&self.sql[self.tokens[first + 1].start..self.tokens[self.at - 1].end])
                })
                .into_iter()
                .collect();
            return Ok(utility("CACHE SELECT".to_owned(), text));
        }
        let lazy: bool = self.eat_kw("LAZY");
        self.expect_kw("TABLE")?;
        let table: Identifier = joined(self.multipart_identifier()?);
        let options: Vec<(Expression, Expression)> = match self.eat_kw("OPTIONS") {
            true => self.valued_property_list()?,
            false => Vec::new(),
        };
        let as_query: bool = self.eat_kw("AS");
        let query: Option<Expression> = match as_query || self.starts_query_at(0) {
            true => Some(self.query()?),
            false => None,
        };
        Ok(Expression::Cache(Box::new(Cache {
            table,
            lazy,
            options,
            query,
            query_as: as_query,
        })))
    }

    /// `UNCACHE TABLE [IF EXISTS] name`.
    pub(crate) fn uncache_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("UNCACHE")?;
        self.expect_kw("TABLE")?;
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        let table: Identifier = joined(self.multipart_identifier()?);
        Ok(Expression::Uncache(Box::new(Uncache { table, if_exists })))
    }
}
