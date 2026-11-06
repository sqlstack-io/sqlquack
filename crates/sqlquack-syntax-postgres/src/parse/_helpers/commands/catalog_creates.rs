//! The `CREATE` statements of catalog objects (gram.y), from access methods to transforms, as `CreateObject`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};

use crate::parse::_helpers::commands::alters::name_from;
use crate::parse::_helpers::commands::creates::PERSISTENCES;
use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::commands::tables::{clause, name_value, unsupported};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id, is_col_label};
use crate::parse::_helpers::tree::nodes::{function, paren, table, tuple};

/// The catalog kinds created here, longest spelling first, with their kind text and the modifier the spelling gives.
const CATALOG_KINDS: [(&[&str], &str, Option<&str>); 33] = [
    (&["access", "method"], ACCESS_METHOD, None),
    (&["cast"], CAST, None),
    (&["collation"], COLLATION, None),
    (&["default", "conversion"], CONVERSION, Some(DEFAULT)),
    (&["conversion"], CONVERSION, None),
    (&["domain"], DOMAIN, None),
    (&["event", "trigger"], EVENT_TRIGGER, None),
    (&["extension"], EXTENSION, None),
    (&["foreign", "data", "wrapper"], FOREIGN_DATA_WRAPPER, None),
    (
        &["trusted", "procedural", "language"],
        LANGUAGE,
        Some(TRUSTED),
    ),
    (&["trusted", "language"], LANGUAGE, Some(TRUSTED)),
    (&["procedural", "language"], LANGUAGE, None),
    (&["language"], LANGUAGE, None),
    (&["operator", "class"], OPERATOR_CLASS, None),
    (&["operator", "family"], OPERATOR_FAMILY, None),
    (&["operator"], OPERATOR, None),
    (&["publication"], PUBLICATION, None),
    (&["rule"], RULE, None),
    (&["schema"], SCHEMA, None),
    (&["server"], SERVER, None),
    (&["statistics"], STATISTICS, None),
    (&["subscription"], SUBSCRIPTION, None),
    (&["tablespace"], TABLESPACE, None),
    (&["text", "search", "parser"], "TEXT SEARCH PARSER", None),
    (
        &["text", "search", "dictionary"],
        "TEXT SEARCH DICTIONARY",
        None,
    ),
    (
        &["text", "search", "template"],
        "TEXT SEARCH TEMPLATE",
        None,
    ),
    (
        &["text", "search", "configuration"],
        "TEXT SEARCH CONFIGURATION",
        None,
    ),
    (&["transform"], TRANSFORM, None),
    (&["user", "mapping"], USER_MAPPING, None),
    (&["user"], USER, None),
    (&["role"], ROLE, None),
    (&["group"], GROUP, None),
    (&["policy"], POLICY, None),
];

const ACCESS_METHOD: &str = "ACCESS METHOD";
const CAST: &str = "CAST";
const COLLATION: &str = "COLLATION";
const CONVERSION: &str = "CONVERSION";
const DOMAIN: &str = "DOMAIN";
const EVENT_TRIGGER: &str = "EVENT TRIGGER";
const EXTENSION: &str = "EXTENSION";
const FOREIGN_DATA_WRAPPER: &str = "FOREIGN DATA WRAPPER";
const LANGUAGE: &str = "LANGUAGE";
const OPERATOR_CLASS: &str = "OPERATOR CLASS";
const OPERATOR_FAMILY: &str = "OPERATOR FAMILY";
const OPERATOR: &str = "OPERATOR";
const PUBLICATION: &str = "PUBLICATION";
const RULE: &str = "RULE";
const SCHEMA: &str = "SCHEMA";
const SERVER: &str = "SERVER";
const STATISTICS: &str = "STATISTICS";
const SUBSCRIPTION: &str = "SUBSCRIPTION";
const TABLESPACE: &str = "TABLESPACE";
const TRANSFORM: &str = "TRANSFORM";
const USER_MAPPING: &str = "USER MAPPING";
const USER: &str = "USER";
const ROLE: &str = "ROLE";
const GROUP: &str = "GROUP";
const POLICY: &str = "POLICY";

const DEFAULT: &str = "DEFAULT";
const TRUSTED: &str = "TRUSTED";

/// The kinds gram.y lets `OR REPLACE` precede.
const REPLACEABLE_KINDS: [&str; 3] = [LANGUAGE, RULE, TRANSFORM];

/// The kinds `CREATE` makes inside `CREATE SCHEMA` besides tables (`schema_stmt`).
const SCHEMA_ELEMENT_KINDS: [&str; 4] = ["INDEX", "SEQUENCE", "TRIGGER", "VIEW"];

/// The `RuleStmt` events.
const RULE_EVENTS: [&str; 4] = ["select", "update", "delete", "insert"];

/// The definition element `CREATE COLLATION ... FROM` makes.
const COLLATION_FROM: &str = "from";

/// The gap a `GRANT` inside `CREATE SCHEMA` leaves until grants parse.
const GRANT_GAP: &str = "grant";

/// gram.y's error for `CREATE EXTENSION ... FROM` (0A000).
const EXTENSION_FROM: &str = "CREATE EXTENSION ... FROM is no longer supported";

/// gram.y's error for schema elements after `IF NOT EXISTS` (0A000).
const SCHEMA_ELEMENTS: &str = "CREATE SCHEMA IF NOT EXISTS cannot include schema elements";

/// Whether a `VERSION` clause is `VERSION NULL`, which `CREATE SERVER` stores as no version.
fn is_null_version(version: &Expression) -> bool {
    matches!(version, Expression::Tuple(clause) if matches!(clause.expressions.get(1), Some(Expression::Null(_))))
}

/// The node a created catalog object becomes.
fn object(node: CreateObject) -> Expression {
    Expression::CreateObject(Box::new(node))
}

impl Parser<'_> {
    /// The catalog kind `CREATE` creates here, with the words up to and including its spelling.
    pub(crate) fn created_catalog(&self) -> Option<(usize, &'static str, Option<&'static str>)> {
        if !self.kw("create") {
            return None;
        }
        let mut offset: usize = 1;
        if self.kw_at(offset, "or") && self.kw_at(offset + 1, "replace") {
            offset += 2;
        }
        if let Some((words, _)) = PERSISTENCES
            .iter()
            .find(|(words, _)| self.words_from(offset, words))
        {
            offset += words.len();
        }
        CATALOG_KINDS
            .iter()
            .find(|(words, kind, _)| {
                self.words_from(offset, words)
                    && (*kind != USER_MAPPING
                        || self.kw_at(offset + 2, "for")
                        || self.kw_at(offset + 2, "if"))
            })
            .map(|(words, kind, modifier)| (offset + words.len(), *kind, *modifier))
    }

    /// `CREATE [OR REPLACE] kind ...` for a catalog kind; a persistence, or `OR REPLACE` where gram.y has none, is refused.
    pub(crate) fn create_catalog(&mut self) -> Parsing<Expression> {
        let base: usize = self.at;
        let Some((end, kind, modifier)) = self.created_catalog() else {
            return Err(self.error());
        };
        self.expect_kw("create")?;
        let or_replace: bool = self.kw("or") && self.kw_at(1, "replace");
        if or_replace {
            self.at += 2;
        }
        if let Some((words, _)) = PERSISTENCES
            .iter()
            .find(|(words, _)| self.words_from(0, words))
        {
            self.at += words.len();
            return Err(self.error());
        }
        if or_replace && !REPLACEABLE_KINDS.contains(&kind) {
            return Err(self.error());
        }
        self.at = base + end;
        let node = CreateObject {
            kind: kind.to_owned(),
            modifier: modifier.map(str::to_owned),
            or_replace,
            or_alter: false,
            if_not_exists: false,
            name: table(Vec::new()),
            source: String::new(),
            operands: Vec::new(),
            properties: Vec::new(),
        };
        match kind {
            ACCESS_METHOD => self.create_access_method(node),
            CAST => self.create_cast(node),
            COLLATION => self.create_collation(node),
            CONVERSION => self.create_conversion(node),
            DOMAIN => self.create_domain(node),
            EVENT_TRIGGER => self.create_event_trigger(node),
            EXTENSION => self.create_extension(node),
            FOREIGN_DATA_WRAPPER => self.create_fdw(node),
            LANGUAGE => self.create_language(node),
            OPERATOR_CLASS => self.create_operator_class(node),
            OPERATOR_FAMILY => self.create_operator_family(node),
            OPERATOR => self.create_operator(node),
            PUBLICATION => self.create_publication(node),
            RULE => self.create_rule(node),
            SCHEMA => self.create_schema(node),
            SERVER => self.create_server(node),
            STATISTICS => self.create_statistics(node),
            SUBSCRIPTION => self.create_subscription(node),
            TABLESPACE => self.create_tablespace(node),
            TRANSFORM => self.create_transform(node),
            USER | ROLE | GROUP => self.create_role(node),
            POLICY => self.create_policy(node),
            USER_MAPPING => self.create_user_mapping(node),
            _ => self.create_text_search(node),
        }
    }

    /// `CreateAmStmt`: `name TYPE {INDEX | TABLE} HANDLER handler_name`.
    fn create_access_method(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(vec![self.name(is_col_id)?]);
        self.expect_kw("type")?;
        let method_type: &str = match self.eat_kw("index") {
            true => "INDEX",
            false => {
                self.expect_kw("table")?;
                "TABLE"
            }
        };
        self.expect_kw("handler")?;
        node.operands = vec![
            clause("TYPE", vec![var(method_type)]),
            clause("HANDLER", vec![name_value(&self.qualified_name()?)]),
        ];
        Ok(object(node))
    }

    /// `CreateCastStmt`: `(source AS target)`, the function, `INOUT` or none, then the context unless explicit.
    fn create_cast(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        self.expect_char('(')?;
        let source: Expression = Expression::DataType(self.typename()?);
        self.expect_kw("as")?;
        let target: Expression = Expression::DataType(self.typename()?);
        self.expect_char(')')?;
        node.operands.push(tuple(vec![source, target]));
        let method: Expression = match self.keyword_at(0) {
            Some("without") => {
                self.at += 1;
                self.expect_kw("function")?;
                clause("WITHOUT FUNCTION", Vec::new())
            }
            Some("with") if self.kw_at(1, "inout") => {
                self.at += 2;
                clause("WITH INOUT", Vec::new())
            }
            _ => {
                self.expect_kw("with")?;
                self.expect_kw("function")?;
                clause("WITH FUNCTION", vec![self.function_with_argtypes()?])
            }
        };
        node.operands.push(method);
        if self.eat_kw("as") {
            let context: &str = match self.eat_kw("implicit") {
                true => "AS IMPLICIT",
                false => {
                    self.expect_kw("assignment")?;
                    "AS ASSIGNMENT"
                }
            };
            node.operands.push(clause(context, Vec::new()));
        }
        Ok(object(node))
    }

    /// `CREATE COLLATION [IF NOT EXISTS] any_name` then a definition, or `FROM any_name` as its one `from` element.
    fn create_collation(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.if_not_exists = self.if_not_exists()?;
        node.name = table(self.qualified_name()?);
        node.properties = match self.eat_kw("from") {
            true => vec![ObjectProperty {
                kind: String::new(),
                name: Some(Identifier::new(COLLATION_FROM)),
                value: Some(name_value(&self.qualified_name()?)),
            }],
            false => self.definition_list()?,
        };
        Ok(object(node))
    }

    /// `CreateConversionStmt`: `any_name FOR 'source' TO 'target' FROM any_name`.
    fn create_conversion(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(self.qualified_name()?);
        self.expect_kw("for")?;
        let source: Expression = self.sconst()?;
        self.expect_kw("to")?;
        let target: Expression = self.sconst()?;
        self.expect_kw("from")?;
        node.operands = vec![
            clause("FOR", vec![source]),
            clause("TO", vec![target]),
            clause("FROM", vec![name_value(&self.qualified_name()?)]),
        ];
        Ok(object(node))
    }

    /// `CreateDomainStmt`: `any_name [AS] Typename ColQualList`, its one `COLLATE` before its constraints, as `SplitColQualList` sorts them.
    fn create_domain(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(self.qualified_name()?);
        self.eat_kw("as");
        node.operands.push(Expression::DataType(self.typename()?));
        let mut collation: Option<Expression> = None;
        let mut constraints: Vec<Expression> = Vec::new();
        loop {
            let start: usize = self.offset();
            let Some((constraint, collate)) = self.column_constraint()? else {
                break;
            };
            match (collate, collation.is_some()) {
                (true, true) => {
                    return Err(self.error_with("multiple COLLATE clauses not allowed", start));
                }
                (true, false) => collation = Some(constraint),
                (false, _) => constraints.push(constraint),
            }
        }
        node.operands.extend(collation);
        node.operands.extend(constraints);
        Ok(object(node))
    }

    /// `CreateEventTrigStmt`: `name ON event [WHEN filter AND ...] EXECUTE {FUNCTION | PROCEDURE} func_name()`.
    fn create_event_trigger(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(vec![self.name(is_col_id)?]);
        self.expect_kw("on")?;
        node.operands
            .push(clause("ON", vec![name_from(self.name(is_col_label)?)]));
        if self.eat_kw("when") {
            let mut filters: Vec<Expression> = Vec::new();
            loop {
                let variable: Expression = name_from(self.name(is_col_id)?);
                self.expect_kw("in")?;
                let values: Vec<Expression> = self.parenthesized(|parser| parser.sconst())?;
                filters.push(tuple(vec![variable, tuple(values)]));
                if !self.eat_kw("and") {
                    break;
                }
            }
            node.operands.push(clause("WHEN", filters));
        }
        self.expect_kw("execute")?;
        if !self.eat_kw("function") {
            self.expect_kw("procedure")?;
        }
        let name: Vec<Identifier> = self.func_name()?;
        self.expect_char('(')?;
        self.expect_char(')')?;
        node.operands.push(clause(
            "EXECUTE",
            vec![Expression::Function(Box::new(function(name, Vec::new())))],
        ));
        Ok(object(node))
    }

    /// `CreateExtensionStmt`: `[IF NOT EXISTS] name [WITH]` then `SCHEMA`, `VERSION` and `CASCADE` in order.
    fn create_extension(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.if_not_exists = self.if_not_exists()?;
        node.name = table(vec![self.name(is_col_id)?]);
        self.eat_kw("with");
        loop {
            let start: usize = self.offset();
            let option: Expression = match self.keyword_at(0) {
                Some("schema") => {
                    self.at += 1;
                    clause("SCHEMA", vec![name_from(self.name(is_col_id)?)])
                }
                Some("version") => {
                    self.at += 1;
                    clause("VERSION", vec![self.word_or_string()?])
                }
                Some("cascade") => {
                    self.at += 1;
                    var("CASCADE")
                }
                Some("from") => {
                    self.at += 1;
                    self.word_or_string()?;
                    return Err(unsupported(EXTENSION_FROM.to_owned(), start));
                }
                _ => return Ok(object(node)),
            };
            node.operands.push(option);
        }
    }

    /// `CreateFdwStmt`: `name`, `[NO] HANDLER` and `[NO] VALIDATOR`, then `OPTIONS`.
    fn create_fdw(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(vec![self.name(is_col_id)?]);
        node.operands = self.fdw_options()?;
        node.operands.extend(self.create_generic_options()?);
        Ok(object(node))
    }

    /// `CreatePLangStmt`; without `HANDLER` gram.y makes it `CREATE EXTENSION`, `OR REPLACE` as `IF NOT EXISTS` and `TRUSTED` dropped.
    fn create_language(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(vec![self.name(is_col_id)?]);
        if !self.eat_kw("handler") {
            node.kind = EXTENSION.to_owned();
            node.modifier = None;
            node.if_not_exists = node.or_replace;
            node.or_replace = false;
            return Ok(object(node));
        }
        node.operands
            .push(clause("HANDLER", vec![name_value(&self.qualified_name()?)]));
        if self.eat_kw("inline") {
            node.operands
                .push(clause("INLINE", vec![name_value(&self.qualified_name()?)]));
        }
        if self.eat_kw("validator") {
            node.operands.push(clause(
                "VALIDATOR",
                vec![name_value(&self.qualified_name()?)],
            ));
        } else if self.kw("no") && self.kw_at(1, "validator") {
            self.at += 2;
        }
        Ok(object(node))
    }

    /// `CreateOpClassStmt`: `any_name [DEFAULT] FOR TYPE Typename USING name [FAMILY any_name] AS opclass_item, ...`.
    fn create_operator_class(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(self.qualified_name()?);
        if self.eat_kw("default") {
            node.modifier = Some(DEFAULT.to_owned());
        }
        self.expect_kw("for")?;
        self.expect_kw("type")?;
        node.operands.push(clause(
            "FOR TYPE",
            vec![Expression::DataType(self.typename()?)],
        ));
        self.expect_kw("using")?;
        node.operands
            .push(clause("USING", vec![name_from(self.name(is_col_id)?)]));
        if self.eat_kw("family") {
            node.operands
                .push(clause("FAMILY", vec![name_value(&self.qualified_name()?)]));
        }
        self.expect_kw("as")?;
        let items: Vec<Expression> = self.comma_list(|parser| parser.operator_class_item())?;
        node.operands.push(clause("AS", items));
        Ok(object(node))
    }

    /// `CreateOpFamilyStmt`: `any_name USING name`.
    fn create_operator_family(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(self.qualified_name()?);
        self.expect_kw("using")?;
        node.operands
            .push(clause("USING", vec![name_from(self.name(is_col_id)?)]));
        Ok(object(node))
    }

    /// `CREATE OPERATOR any_operator definition`, the operator in `operands`.
    fn create_operator(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.operands.push(self.any_operator()?);
        node.properties = self.definition_list()?;
        Ok(object(node))
    }

    /// `CreatePublicationStmt`: `name [FOR ALL TABLES | FOR pub_obj_list] [WITH definition]`.
    fn create_publication(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(vec![self.name(is_col_id)?]);
        if self.eat_kw("for") {
            if self.kw("all") && self.kw_at(1, "tables") {
                self.at += 2;
                node.operands.push(clause("FOR ALL TABLES", Vec::new()));
            } else {
                node.operands
                    .push(clause("FOR", self.publication_object_list()?));
            }
        }
        node.operands.extend(self.with_definition()?);
        Ok(object(node))
    }

    /// `RuleStmt`: `name AS ON event TO qualified_name [WHERE a_expr] DO [INSTEAD | ALSO] actions`; `ALSO` is the default.
    fn create_rule(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(vec![self.name(is_col_id)?]);
        self.expect_kw("as")?;
        self.expect_kw("on")?;
        let Some(event) = RULE_EVENTS.iter().find(|event| self.kw(event)) else {
            return Err(self.error());
        };
        self.at += 1;
        node.operands
            .push(clause("ON", vec![var(&event.to_uppercase())]));
        self.expect_kw("to")?;
        let relation: Expression = Expression::Table(Box::new(table(self.table_name()?)));
        node.operands.push(clause("TO", vec![relation]));
        if self.eat_kw("where") {
            node.operands.push(clause("WHERE", vec![self.a_expr()?]));
        }
        self.expect_kw("do")?;
        if self.eat_kw("instead") {
            node.operands.push(clause("INSTEAD", Vec::new()));
        } else {
            self.eat_kw("also");
        }
        let actions: Vec<Expression> = self.rule_actions()?;
        node.operands.push(clause("DO", actions));
        Ok(object(node))
    }

    /// `RuleActionList`: `NOTHING`, one action, or `( action; ... )` where empty actions drop out.
    fn rule_actions(&mut self) -> Parsing<Vec<Expression>> {
        if self.eat_kw("nothing") {
            return Ok(Vec::new());
        }
        if !self.is_char('(') {
            return Ok(vec![self.rule_action()?]);
        }
        let start: usize = self.at;
        let several = self.attempt(|parser| parser.rule_action_list());
        let ended: bool = self.peek().is_none() || self.is_char(';');
        match several {
            Ok(actions) if ended => Ok(actions),
            _ => {
                self.at = start;
                Ok(vec![self.rule_action()?])
            }
        }
    }

    /// `'(' RuleActionMulti ')'`: actions separated by `;`, empty ones dropped.
    fn rule_action_list(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_char('(')?;
        let mut actions: Vec<Expression> = Vec::new();
        loop {
            if !self.is_char(';') && !self.is_char(')') {
                actions.push(self.rule_action()?);
            }
            if !self.eat_char(';') {
                break;
            }
        }
        self.expect_char(')')?;
        Ok(actions)
    }

    /// `RuleActionStmt`: a `SELECT`, `INSERT`, `UPDATE` or `DELETE`, or `NOTIFY ColId [, 'payload']`.
    fn rule_action(&mut self) -> Parsing<Expression> {
        if self.eat_kw("notify") {
            let mut parts: Vec<Expression> = vec![var("NOTIFY"), name_from(self.name(is_col_id)?)];
            if self.eat_char(',') {
                parts.push(self.sconst()?);
            }
            return Ok(tuple(parts));
        }
        if self.kw("merge") {
            return Err(self.error());
        }
        let start: usize = self.at;
        let action: Expression = self.preparable_statement()?;
        if matches!(action, Expression::Merge(_)) {
            self.at = start;
            return Err(self.error());
        }
        Ok(action)
    }

    /// `CreateSchemaStmt`: `[IF NOT EXISTS]`, a name, `AUTHORIZATION role` or both, then schema elements.
    fn create_schema(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.if_not_exists = self.if_not_exists()?;
        let named: bool = !self.kw("authorization");
        if named {
            node.name = table(vec![self.name(is_col_id)?]);
        }
        if self.eat_kw("authorization") {
            node.operands
                .push(clause("AUTHORIZATION", vec![self.role_spec()?]));
        } else if !named {
            return Err(self.error());
        }
        let elements_at: usize = self.offset();
        let mut elements: Vec<Expression> = Vec::new();
        while self.kw("create") || self.kw("grant") {
            elements.push(self.schema_element()?);
        }
        if node.if_not_exists && !elements.is_empty() {
            return Err(unsupported(SCHEMA_ELEMENTS.to_owned(), elements_at));
        }
        node.operands.extend(elements);
        Ok(object(node))
    }

    /// `schema_stmt`: `CREATE TABLE`, `INDEX`, `SEQUENCE`, `TRIGGER` or `VIEW`, or a `GRANT`, a gap until grants parse.
    fn schema_element(&mut self) -> Parsing<Expression> {
        if self.kw("grant") {
            let start: usize = self.offset();
            self.skip_statement_rest()?;
            self.gap(GRANT_GAP, start);
            return Ok(var("GRANT"));
        }
        if self.creates_table() {
            let created: Expression = self.create_table()?;
            return match &created {
                Expression::CreateTable(node) if node.as_select.is_some() => Err(self.error()),
                _ => Ok(created),
            };
        }
        match self.created_kind() {
            Some(kind) if SCHEMA_ELEMENT_KINDS.contains(&kind) => self.create_kind(kind),
            _ => {
                self.at += 1;
                Err(self.error())
            }
        }
    }

    /// `CreateForeignServerStmt`: `[IF NOT EXISTS] name [TYPE 'type'] [VERSION ...] FOREIGN DATA WRAPPER name [OPTIONS]`.
    fn create_server(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.if_not_exists = self.if_not_exists()?;
        node.name = table(vec![self.name(is_col_id)?]);
        if self.eat_kw("type") {
            node.operands.push(clause("TYPE", vec![self.sconst()?]));
        }
        let version: Vec<Expression> = self.server_version()?;
        node.operands.extend(
            version
                .into_iter()
                .filter(|clause| !is_null_version(clause)),
        );
        self.expect_kw("foreign")?;
        self.expect_kw("data")?;
        self.expect_kw("wrapper")?;
        node.operands.push(clause(
            "FOREIGN DATA WRAPPER",
            vec![name_from(self.name(is_col_id)?)],
        ));
        node.operands.extend(self.create_generic_options()?);
        Ok(object(node))
    }

    /// `CreateStatsStmt`: `[IF NOT EXISTS] [any_name] [(kind, ...)] ON stats_param, ... FROM from_list`.
    fn create_statistics(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.if_not_exists = self.if_not_exists()?;
        if node.if_not_exists || !(self.is_char('(') || self.kw("on")) {
            node.name = table(self.qualified_name()?);
        }
        if self.is_char('(') {
            let kinds: Vec<Identifier> = self.name_list()?;
            node.operands
                .push(tuple(kinds.into_iter().map(name_from).collect()));
        }
        self.expect_kw("on")?;
        let parameters: Vec<Expression> = self.comma_list(|parser| parser.statistics_param())?;
        node.operands.push(clause("ON", parameters));
        self.expect_kw("from")?;
        let mut sources: Vec<Expression> = Vec::new();
        loop {
            sources.push(self.table_ref()?);
            for join in self.joins()? {
                sources.push(Expression::Join(Box::new(join)));
            }
            if !self.eat_char(',') {
                break;
            }
        }
        node.operands.push(clause("FROM", sources));
        Ok(object(node))
    }

    /// `stats_param`: a column, a call without a window, or `( a_expr )`.
    fn statistics_param(&mut self) -> Parsing<Expression> {
        if self.is_char('(') {
            return Ok(paren(self.parenthesized_condition()?));
        }
        if self.is(is_col_id) && !self.char_at(1, '(') && !self.char_at(1, '.') {
            return Ok(name_from(self.name(is_col_id)?));
        }
        self.windowless_call()
    }

    /// `CreateSubscriptionStmt`: `name CONNECTION 'conninfo' PUBLICATION name, ... [WITH definition]`.
    fn create_subscription(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(vec![self.name(is_col_id)?]);
        self.expect_kw("connection")?;
        node.operands
            .push(clause("CONNECTION", vec![self.sconst()?]));
        self.expect_kw("publication")?;
        node.operands
            .push(clause("PUBLICATION", vec![self.bare_name_list()?]));
        node.operands.extend(self.with_definition()?);
        Ok(object(node))
    }

    /// `CreateTableSpaceStmt`: `name [OWNER role] LOCATION 'path' [WITH reloptions]`.
    fn create_tablespace(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(vec![self.name(is_col_id)?]);
        if self.eat_kw("owner") {
            node.operands.push(clause("OWNER", vec![self.role_spec()?]));
        }
        self.expect_kw("location")?;
        node.operands.push(clause("LOCATION", vec![self.sconst()?]));
        node.operands.extend(self.reloptions()?);
        Ok(object(node))
    }

    /// `CREATE TEXT SEARCH {PARSER | DICTIONARY | TEMPLATE | CONFIGURATION} any_name definition`.
    fn create_text_search(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(self.qualified_name()?);
        node.properties = self.definition_list()?;
        Ok(object(node))
    }

    /// `CreateTransformStmt`: `FOR Typename LANGUAGE name (...)`, `FROM SQL` before `TO SQL` as gram.y's list keeps them.
    fn create_transform(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        self.expect_kw("for")?;
        node.operands
            .push(clause("FOR", vec![Expression::DataType(self.typename()?)]));
        self.expect_kw("language")?;
        node.operands
            .push(clause("LANGUAGE", vec![name_from(self.name(is_col_id)?)]));
        self.expect_char('(')?;
        let (first_from, first) = self.transform_element()?;
        let mut elements: Vec<Expression> = vec![first];
        if self.eat_char(',') {
            let second_at: usize = self.at;
            let (second_from, second) = self.transform_element()?;
            if second_from == first_from {
                self.at = second_at;
                return Err(self.error());
            }
            match first_from {
                true => elements.push(second),
                false => elements.insert(0, second),
            }
        }
        self.expect_char(')')?;
        node.operands.extend(elements);
        Ok(object(node))
    }

    /// One `transform_element`: `{FROM | TO} SQL WITH FUNCTION function_with_argtypes`, and whether it is `FROM`.
    fn transform_element(&mut self) -> Parsing<(bool, Expression)> {
        let from: bool = self.eat_kw("from");
        if !from {
            self.expect_kw("to")?;
        }
        self.expect_kw("sql")?;
        self.expect_kw("with")?;
        self.expect_kw("function")?;
        let signature: Expression = self.function_with_argtypes()?;
        let keyword: &str = match from {
            true => "FROM SQL",
            false => "TO SQL",
        };
        Ok((from, clause(keyword, vec![signature])))
    }
}
