//! googlesql.tm's `CREATE` kinds beyond `TABLE` and `VIEW`, as `CreateObject` or `CreateSchema`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, CreateSchema};

use crate::parse::_helpers::commands::alters::{clause, named, path_operand};
use crate::parse::_helpers::commands::creates::Head;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{table, tuple, var};
use crate::parse::constants::{GENERIC_ENTITY_TYPES, INDEX_KIND, SORT_DIRECTIONS, TABLE_WORD};

/// The determinism levels a function may declare.
const DETERMINISM_LEVELS: [&str; 3] = ["IMMUTABLE", "STABLE", "VOLATILE"];

/// The words that begin a table function's declaration options.
const TVF_OPTION_WORDS: [&str; 3] = ["LANGUAGE", "WITH", "OPTIONS"];

/// The object a `CREATE` makes: its kind, whether `IF NOT EXISTS` is written, and its name.
struct CreateTarget<'kind> {
    kind: &'kind str,
    if_not_exists: bool,
    name: TableRef,
}

/// A `CreateObject` of `target` after `head`, with the clauses `operands`.
fn created(head: &Head, target: CreateTarget, operands: Vec<Expression>) -> Expression {
    Expression::CreateObject(Box::new(CreateObject {
        kind: target.kind.to_owned(),
        modifier: head.scope.map(str::to_owned),
        or_replace: head.or_replace,
        or_alter: false,
        if_not_exists: target.if_not_exists,
        name: target.name,
        source: String::new(),
        operands,
        properties: Vec::new(),
    }))
}

impl Parser<'_> {
    /// The `CREATE` kinds after the head; `None` when the kind is not one of them.
    pub(crate) fn create_object(&mut self, head: &Head) -> Option<Parsing<Expression>> {
        let word: String = self.word_at(0)?;
        let scoped: bool = matches!(
            word.as_str(),
            "AGGREGATE" | "FUNCTION" | "PROCEDURE" | "EXTERNAL" | "MODEL" | "CONSTANT"
        ) || (word == TABLE_WORD && self.kw_at(1, "FUNCTION"))
            || (!head.materialized && self.generic_entity_at());
        if head.scope.is_some() && !scoped {
            return Some(Err(self.error()));
        }
        Some(match word.as_str() {
            "AGGREGATE" | "FUNCTION" => self.create_function(head),
            "TABLE" if self.kw_at(1, "FUNCTION") => self.create_table_function(head),
            "PROCEDURE" => self.create_procedure(head),
            "SCHEMA" => self.create_schema(head),
            "EXTERNAL" => self.create_external(head),
            "CONNECTION" => self.create_connection(head),
            "SNAPSHOT" => self.create_snapshot(head),
            "UNIQUE" | "SEARCH" | "VECTOR" | "AI" | "INDEX" => self.create_index(head),
            "ROW" => self.create_row_access_policy(head),
            "DATA_POLICY" => self.create_data_policy(head),
            "MODEL" => self.create_model(head),
            "CONSTANT" => self.create_constant(head),
            "DATABASE" if !head.or_replace => self.create_database(head),
            _ if self.generic_entity_at() => self.create_entity(head),
            _ => return None,
        })
    }

    /// Whether a generic entity type is at the cursor.
    fn generic_entity_at(&self) -> bool {
        self.word_at(0)
            .is_some_and(|word| GENERIC_ENTITY_TYPES.contains(&word.as_str()))
    }

    /// `OPTIONS (...)` as a clause, when written.
    fn options_clause(&mut self) -> Parsing<Option<Expression>> {
        if self.kw("OPTIONS") && self.char_at(1, '(') {
            self.at += 1;
            return Ok(Some(clause("OPTIONS", self.options_list()?)));
        }
        Ok(None)
    }

    /// `OPTIONS (...)` as a clause, which the statement requires.
    fn required_options(&mut self) -> Parsing<Expression> {
        self.expect_kw("OPTIONS")?;
        Ok(clause("OPTIONS", self.options_list()?))
    }

    /// `WITH CONNECTION path | DEFAULT | ( key = value, ... )` as a clause, when written.
    pub(crate) fn connection_clause(&mut self) -> Parsing<Option<Expression>> {
        if !(self.kw("WITH") && self.kw_at(1, "CONNECTION")) {
            return Ok(None);
        }
        self.at += 2;
        let connection: Vec<Expression> = if self.eat_kw("DEFAULT") {
            vec![var("DEFAULT")]
        } else if self.is_char('(') {
            self.parenthesized(|parser| {
                let key: Identifier = parser.ident()?;
                parser.expect_char('=')?;
                Ok(crate::parse::_helpers::tree::nodes::binary(
                    Expression::Eq,
                    named(key),
                    parser.expression()?,
                ))
            })?
        } else {
            vec![path_operand(&self.table_path()?)]
        };
        Ok(Some(clause("WITH CONNECTION", connection)))
    }

    /// `REMOTE [WITH CONNECTION ...]` or `WITH CONNECTION ...` as clauses, none when not written.
    fn remote_clause(&mut self) -> Parsing<Vec<Expression>> {
        let mut clauses: Vec<Expression> = Vec::new();
        if self.eat_kw("REMOTE") {
            clauses.push(clause("REMOTE", Vec::new()));
        }
        clauses.extend(self.connection_clause()?);
        Ok(clauses)
    }

    /// `( column | constraint, ... )`, a table's elements, with the trailing commas allowed.
    pub(crate) fn table_elements(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_char('(')?;
        let mut elements: Vec<Expression> = Vec::new();
        if self.eat_char(')') {
            return Ok(elements);
        }
        loop {
            elements.push(self.table_element()?);
            if !self.eat_char(',') {
                break;
            }
            while self.eat_char(',') {}
            if self.is_char(')') {
                break;
            }
        }
        self.expect_char(')')?;
        Ok(elements)
    }

    /// A column definition or a table constraint.
    fn table_element(&mut self) -> Parsing<Expression> {
        if self.kw("PRIMARY") && self.kw_at(1, "KEY") {
            return self.primary_key_spec();
        }
        if self.kw_in(0, &["CONSTRAINT", "FOREIGN", "CHECK"]) {
            return self.table_constraint_spec();
        }
        Ok(Expression::ColumnDef(Box::new(
            self.table_column_definition()?,
        )))
    }

    /// `[AGGREGATE] FUNCTION`: its signature and attributes, then `OPTIONS` and body in any order.
    fn create_function(&mut self, head: &Head) -> Parsing<Expression> {
        let aggregate: bool = self.eat_kw("AGGREGATE");
        self.expect_kw("FUNCTION")?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        let mut operands: Vec<Expression> = vec![clause("PARAMETERS", self.function_parameters()?)];
        if self.eat_kw("RETURNS") {
            if self.kw("ANY") {
                return Err(self.error_with(
                    "Syntax error: Templated types are not allowed in the RETURNS clause",
                    self.offset(),
                    true,
                ));
            }
            operands.push(clause(
                "RETURNS",
                vec![Expression::DataType(self.data_type()?)],
            ));
        }
        operands.extend(self.sql_security()?);
        if self.eat_kw("DETERMINISTIC") {
            operands.push(clause("DETERMINISTIC", Vec::new()));
        } else if self.eat_kws(&["NOT", "DETERMINISTIC"]) {
            operands.push(clause("NOT DETERMINISTIC", Vec::new()));
        } else if let Some(level) = self
            .word_at(0)
            .filter(|word| DETERMINISM_LEVELS.contains(&word.as_str()))
        {
            self.at += 1;
            operands.push(clause(&level, Vec::new()));
        }
        if self.kw("LANGUAGE") {
            operands.push(self.language_clause()?);
            operands.extend(self.remote_clause()?);
        } else {
            let remote: Vec<Expression> = self.remote_clause()?;
            let remote_written: bool = !remote.is_empty();
            operands.extend(remote);
            if remote_written && self.kw("LANGUAGE") {
                operands.push(self.language_clause()?);
            }
        }
        if self.kw("OPTIONS") {
            operands.extend(self.options_clause()?);
            if self.kw("AS") {
                operands.push(self.function_body()?);
            }
        } else if self.kw("AS") {
            operands.push(self.function_body()?);
            operands.extend(self.options_clause()?);
        }
        let kind: &str = match aggregate {
            true => "AGGREGATE FUNCTION",
            false => "FUNCTION",
        };
        Ok(created(
            head,
            CreateTarget {
                kind,
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// `SQL SECURITY INVOKER | DEFINER` as a clause, when written.
    fn sql_security(&mut self) -> Parsing<Option<Expression>> {
        if !self.eat_kws(&["SQL", "SECURITY"]) {
            return Ok(None);
        }
        match self.eat_kw("INVOKER") {
            true => Ok(Some(clause("SQL SECURITY INVOKER", Vec::new()))),
            false => {
                self.expect_kw("DEFINER")?;
                Ok(Some(clause("SQL SECURITY DEFINER", Vec::new())))
            }
        }
    }

    /// `LANGUAGE name` as a clause.
    fn language_clause(&mut self) -> Parsing<Expression> {
        self.expect_kw("LANGUAGE")?;
        Ok(clause("LANGUAGE", vec![named(self.ident()?)]))
    }

    /// `AS ( expression )` or `AS string`: a function's body. A query is no body.
    fn function_body(&mut self) -> Parsing<Expression> {
        self.expect_kw("AS")?;
        if self.string_at(0) {
            return Ok(clause("AS", vec![self.string_literal()?]));
        }
        self.expect_char('(')?;
        if self.kw("SELECT") {
            return Err(self.error_with(
                "The body of each CREATE FUNCTION statement is an expression, not a query; to \
                 use a query as an expression, the query must be wrapped with additional \
                 parentheses to make it a scalar subquery expression",
                self.offset(),
                true,
            ));
        }
        let body: Expression = self.expression()?;
        self.expect_char(')')?;
        Ok(clause("AS", vec![body]))
    }

    /// `TABLE FUNCTION`: its signature, declaration options, and `AS query` or `AS string`.
    fn create_table_function(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kws(&["TABLE", "FUNCTION"])?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        let mut operands: Vec<Expression> = Vec::new();
        if self.is_char('(') {
            operands.push(clause("PARAMETERS", self.function_parameters()?));
        }
        if self.eat_kw("RETURNS") {
            operands.push(clause(
                "RETURNS",
                vec![Expression::DataType(self.type_or_tvf_schema()?)],
            ));
        }
        operands.extend(self.sql_security()?);
        while self.kw_in(0, &TVF_OPTION_WORDS) {
            match self.word_at(0).as_deref() {
                Some("LANGUAGE") => operands.push(self.language_clause()?),
                Some("WITH") if self.kw_at(1, "CONNECTION") => {
                    operands.extend(self.connection_clause()?)
                }
                Some("OPTIONS") if self.char_at(1, '(') => operands.extend(self.options_clause()?),
                _ => break,
            }
        }
        if self.eat_kw("AS") {
            let body: Expression = match self.string_at(0) {
                true => self.string_literal()?,
                false => self.query()?,
            };
            operands.push(clause("AS", vec![body]));
        }
        Ok(created(
            head,
            CreateTarget {
                kind: "TABLE FUNCTION",
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// `PROCEDURE`: its signature and options, then a `BEGIN ... END` block or `LANGUAGE name`.
    fn create_procedure(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kw("PROCEDURE")?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        let mut operands: Vec<Expression> =
            vec![clause("PARAMETERS", self.procedure_parameters()?)];
        if self.eat_kws(&["EXTERNAL", "SECURITY"]) {
            match self.eat_kw("INVOKER") {
                true => operands.push(clause("EXTERNAL SECURITY INVOKER", Vec::new())),
                false => {
                    self.expect_kw("DEFINER")?;
                    operands.push(clause("EXTERNAL SECURITY DEFINER", Vec::new()));
                }
            }
        }
        operands.extend(self.connection_clause()?);
        operands.extend(self.options_clause()?);
        if self.kw("BEGIN") {
            operands.extend(self.begin_end_block()?);
        } else {
            operands.push(self.language_clause()?);
            if self.eat_kw("AS") {
                operands.push(clause("AS", vec![self.string_literal()?]));
            }
        }
        Ok(created(
            head,
            CreateTarget {
                kind: "PROCEDURE",
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// `BEGIN ... [EXCEPTION WHEN ERROR THEN ...] END` as a `BEGIN` clause and an exception clause.
    fn begin_end_block(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_kw("BEGIN")?;
        let mut clauses: Vec<Expression> = vec![clause("BEGIN", self.block_statements()?)];
        if self.eat_kw("EXCEPTION") {
            self.expect_kws(&["WHEN", "ERROR", "THEN"])?;
            clauses.push(clause(
                "EXCEPTION WHEN ERROR THEN",
                self.block_statements()?,
            ));
        }
        self.expect_kw("END")?;
        Ok(clauses)
    }

    /// The statements of a block up to `END` or `EXCEPTION`, each ended by `;`.
    fn block_statements(&mut self) -> Parsing<Vec<Expression>> {
        let mut statements: Vec<Expression> = Vec::new();
        while !(self.kw("END") || self.kw("EXCEPTION") || self.ends_at(0) && self.peek().is_none())
        {
            statements.push(self.statement()?);
            self.expect_char(';')?;
        }
        Ok(statements)
    }

    /// `SCHEMA [IF NOT EXISTS] path [DEFAULT COLLATE string] [OPTIONS (...)]`.
    fn create_schema(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kw("SCHEMA")?;
        let mut node: CreateSchema = CreateSchema::new(String::new());
        node.or_replace = head.or_replace;
        node.if_not_exists = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        node.name = self.multipart_identifier()?;
        if self.eat_kws(&["DEFAULT", "COLLATE"]) {
            node.properties
                .push(clause("DEFAULT COLLATE", vec![self.string_or_parameter()?]));
        }
        node.properties.extend(self.options_clause()?);
        Ok(Expression::CreateSchema(Box::new(node)))
    }

    /// `EXTERNAL SCHEMA` and `EXTERNAL TABLE`; `EXTERNAL TABLE FUNCTION` is refused.
    fn create_external(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kw("EXTERNAL")?;
        if self.eat_kw("SCHEMA") {
            let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
            let name: TableRef = self.table_path()?;
            let mut operands: Vec<Expression> = Vec::new();
            operands.extend(self.connection_clause()?);
            operands.push(self.required_options()?);
            return Ok(created(
                head,
                CreateTarget {
                    kind: "EXTERNAL SCHEMA",
                    if_not_exists,
                    name,
                },
                operands,
            ));
        }
        self.expect_kw("TABLE")?;
        if self.kw("FUNCTION") {
            return Err(self.error_with(
                "Syntax error: CREATE EXTERNAL TABLE FUNCTION is not supported",
                self.offset(),
                true,
            ));
        }
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        let mut operands: Vec<Expression> = Vec::new();
        if self.is_char('(') {
            operands.push(clause("COLUMNS", self.table_elements()?));
        }
        if self.eat_kw("LIKE") {
            operands.push(clause("LIKE", vec![path_operand(&self.table_path()?)]));
        }
        if self.eat_kws(&["DEFAULT", "COLLATE"]) {
            operands.push(clause("DEFAULT COLLATE", vec![self.string_or_parameter()?]));
        }
        if self.eat_kws(&["WITH", "PARTITION", "COLUMNS"]) {
            let columns: Vec<Expression> = match self.is_char('(') {
                true => self.table_elements()?,
                false => Vec::new(),
            };
            operands.push(clause("WITH PARTITION COLUMNS", columns));
        }
        operands.extend(self.connection_clause()?);
        operands.push(self.required_options()?);
        Ok(created(
            head,
            CreateTarget {
                kind: "EXTERNAL TABLE",
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// `CONNECTION [IF NOT EXISTS] path [OPTIONS (...)]`.
    fn create_connection(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kw("CONNECTION")?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        let mut operands: Vec<Expression> = Vec::new();
        operands.extend(self.options_clause()?);
        Ok(created(
            head,
            CreateTarget {
                kind: "CONNECTION",
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// `SNAPSHOT TABLE | SCHEMA [IF NOT EXISTS] path CLONE source [OPTIONS (...)]`.
    fn create_snapshot(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kw("SNAPSHOT")?;
        let kind_at: usize = self.offset();
        let kind: &str = match self.eat_kw("TABLE") {
            true => "SNAPSHOT TABLE",
            false => match self.schema_object_kind() {
                Some("SCHEMA") => "SNAPSHOT SCHEMA",
                Some(other) => {
                    return Err(self.error_with(
                        &format!("CREATE SNAPSHOT {other} is not supported"),
                        kind_at,
                        true,
                    ));
                }
                None => return Err(self.error()),
            },
        };
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        self.expect_kw("CLONE")?;
        let mut operands: Vec<Expression> = vec![clause("CLONE", self.clone_source()?)];
        operands.extend(self.options_clause()?);
        Ok(created(
            head,
            CreateTarget {
                kind,
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// `clone_data_source`: `path [FOR SYSTEM_TIME AS OF expr] [WHERE expr]`, as operands.
    pub(crate) fn clone_source(&mut self) -> Parsing<Vec<Expression>> {
        let mut operands: Vec<Expression> = vec![path_operand(&self.table_path()?)];
        if self.eat_kw("FOR") {
            if !self.eat_kws(&["SYSTEM", "TIME"]) {
                self.expect_kw("SYSTEM_TIME")?;
            }
            self.expect_kws(&["AS", "OF"])?;
            operands.push(clause("FOR SYSTEM_TIME AS OF", vec![self.expression()?]));
        }
        if self.eat_kw("WHERE") {
            operands.push(clause("WHERE", vec![self.expression()?]));
        }
        Ok(operands)
    }

    /// `[UNIQUE] [SEARCH | VECTOR | AI] INDEX path ON path`, its keys and its suffix.
    fn create_index(&mut self, head: &Head) -> Parsing<Expression> {
        let unique: bool = self.eat_kw("UNIQUE");
        let kind: &str = match self.word_at(0).as_deref() {
            Some("SEARCH") => "SEARCH INDEX",
            Some("VECTOR") => "VECTOR INDEX",
            Some("AI") => "AI INDEX",
            _ => "INDEX",
        };
        if kind != INDEX_KIND {
            self.at += 1;
        }
        self.expect_kw("INDEX")?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        self.expect_kw("ON")?;
        let mut on: Vec<Expression> = vec![path_operand(&self.table_path()?)];
        if self.eat_kw("AS") || (self.is(is_identifier) && !self.kw("UNNEST")) {
            on.push(clause("AS", vec![named(self.ident()?)]));
        }
        let mut operands: Vec<Expression> = vec![clause("ON", on)];
        if unique {
            operands.insert(0, clause("UNIQUE", Vec::new()));
        }
        while self.kw("UNNEST") && self.char_at(1, '(') {
            let start: usize = self.offset();
            self.at += 2;
            self.expression()?;
            self.expect_char(')')?;
            if self.eat_kw("AS") || self.is(is_identifier) {
                self.ident()?;
            }
            if self.eat_kws(&["WITH", "OFFSET"]) && (self.eat_kw("AS") || self.is(is_identifier)) {
                self.ident()?;
            }
            self.gap("index_unnest", start);
        }
        self.expect_char('(')?;
        if self.eat_kws(&["ALL", "COLUMNS"]) {
            let mut all: Vec<Expression> = Vec::new();
            if self.eat_kws(&["WITH", "COLUMN", "OPTIONS"]) {
                all = self.parenthesized(|parser| parser.index_key())?;
            }
            operands.push(clause("ALL COLUMNS", all));
            self.expect_char(')')?;
        } else if self.eat_kws(&["AUTO", "COLUMNS"]) {
            operands.push(clause("AUTO COLUMNS", Vec::new()));
            self.expect_char(')')?;
        } else {
            let keys: Vec<Expression> = self.comma_list(|parser| parser.index_key())?;
            self.expect_char(')')?;
            operands.push(clause("KEYS", keys));
        }
        if self.eat_kw("STORING") {
            operands.push(clause(
                "STORING",
                self.parenthesized(|parser| parser.expression())?,
            ));
        }
        if self.kw("PARTITION") && self.kw_at(1, "BY") {
            self.at += 2;
            operands.push(clause(
                "PARTITION BY",
                self.comma_list(|parser| parser.expression())?,
            ));
        }
        operands.extend(self.connection_clause()?);
        operands.extend(self.options_clause()?);
        Ok(created(
            head,
            CreateTarget {
                kind,
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// `column_ordering_and_options_expr`: an expression and its ordering, bare when it has none.
    fn index_key(&mut self) -> Parsing<Expression> {
        let key: Expression = self.expression()?;
        let mut parts: Vec<Expression> = vec![key];
        if self.eat_kw("COLLATE") {
            parts.push(clause("COLLATE", vec![self.string_or_parameter()?]));
        }
        if let Some(direction) = self
            .word_at(0)
            .filter(|word| SORT_DIRECTIONS.contains(&word.as_str()))
        {
            self.at += 1;
            parts.push(var(&direction));
        }
        if self.kw("NULLS") && self.kw_in(1, &["FIRST", "LAST"]) {
            let order: String = format!("NULLS {}", self.word_at(1).unwrap_or_default());
            self.at += 2;
            parts.push(var(&order));
        }
        if self.kw("OPTIONS") && self.char_at(1, '(') {
            self.at += 1;
            parts.push(clause("OPTIONS", self.options_list()?));
        }
        Ok(match parts.len() {
            1 => parts.remove(0),
            _ => tuple(parts),
        })
    }

    /// `ROW [ACCESS] POLICY [name] ON path`, its grantees and `[FILTER] USING (expr)`.
    fn create_row_access_policy(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kw("ROW")?;
        self.eat_kw("ACCESS");
        self.expect_kw("POLICY")?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = match self.kw("ON") {
            true => table(Vec::new()),
            false => table(vec![self.ident()?]),
        };
        self.expect_kw("ON")?;
        let mut operands: Vec<Expression> =
            vec![clause("ON", vec![path_operand(&self.table_path()?)])];
        if self.eat_kws(&["GRANT", "TO"]) {
            operands.push(clause("GRANT TO", self.grantee_list()?));
        } else if self.eat_kw("TO") {
            operands.push(clause(
                "GRANT TO",
                self.comma_list(|parser| parser.string_or_parameter())?,
            ));
        }
        self.eat_kw("FILTER");
        self.expect_kw("USING")?;
        self.expect_char('(')?;
        let filter: Expression = self.expression()?;
        self.expect_char(')')?;
        operands.push(clause("FILTER USING", vec![filter]));
        Ok(created(
            head,
            CreateTarget {
                kind: "ROW ACCESS POLICY",
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// `AS json | string`, a generic entity's body, as a clause, when written.
    fn entity_body(&mut self) -> Parsing<Option<Expression>> {
        if !self.eat_kw("AS") {
            return Ok(None);
        }
        let body: Expression = match self.kw("JSON") && self.string_at(1) {
            true => self.primary_expression()?,
            false => self.string_literal()?,
        };
        Ok(Some(clause("AS", vec![body])))
    }

    /// `DATA_POLICY [IF NOT EXISTS] path [OPTIONS (...)] [AS body] [WITH CONDITION (expr)]`.
    fn create_data_policy(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kw("DATA_POLICY")?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        let mut operands: Vec<Expression> = Vec::new();
        operands.extend(self.options_clause()?);
        operands.extend(self.entity_body()?);
        if self.eat_kws(&["WITH", "CONDITION"]) {
            self.expect_char('(')?;
            let condition: Expression = self.expression()?;
            self.expect_char(')')?;
            operands.push(clause("WITH CONDITION", vec![condition]));
        }
        Ok(created(
            head,
            CreateTarget {
                kind: "DATA_POLICY",
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// A generic entity: `type [IF NOT EXISTS] path [OPTIONS (...)] [AS body]`.
    fn create_entity(&mut self, head: &Head) -> Parsing<Expression> {
        let Some(entity) = self.generic_entity_type() else {
            return Err(self.error());
        };
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        let mut operands: Vec<Expression> = Vec::new();
        operands.extend(self.options_clause()?);
        operands.extend(self.entity_body()?);
        Ok(created(
            head,
            CreateTarget {
                kind: &entity,
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// `MODEL path`: its columns, `TRANSFORM`, connection, `OPTIONS` and training queries.
    fn create_model(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kw("MODEL")?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        let mut operands: Vec<Expression> = Vec::new();
        if self.eat_kw("INPUT") {
            operands.push(clause("INPUT", self.table_elements()?));
            self.expect_kw("OUTPUT")?;
            operands.push(clause("OUTPUT", self.table_elements()?));
        }
        if self.eat_kw("TRANSFORM") {
            self.expect_char('(')?;
            let mut items: Vec<Expression> = vec![self.named_expression()?];
            while self.eat_char(',') {
                if self.is_char(')') {
                    break;
                }
                items.push(self.named_expression()?);
            }
            self.expect_char(')')?;
            operands.push(clause("TRANSFORM", items));
        }
        operands.extend(self.remote_clause()?);
        operands.extend(self.options_clause()?);
        if self.eat_kw("AS") {
            let aliased: bool =
                self.is_char('(') && self.is_at(1, is_identifier) && self.kw_at(2, "AS");
            match aliased {
                true => {
                    let queries: Vec<Expression> = self.parenthesized(|parser| {
                        let alias: Identifier = parser.ident()?;
                        parser.expect_kw("AS")?;
                        parser.expect_char('(')?;
                        let query: Expression = parser.query()?;
                        parser.expect_char(')')?;
                        Ok(tuple(vec![named(alias), query]))
                    })?;
                    operands.push(clause("AS", queries));
                }
                false => operands.push(clause("AS", vec![self.query()?])),
            }
        }
        Ok(created(
            head,
            CreateTarget {
                kind: "MODEL",
                if_not_exists,
                name,
            },
            operands,
        ))
    }

    /// `CONSTANT [IF NOT EXISTS] path = expression`.
    fn create_constant(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kw("CONSTANT")?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        self.expect_char('=')?;
        let value: Expression = self.expression()?;
        Ok(created(
            head,
            CreateTarget {
                kind: "CONSTANT",
                if_not_exists,
                name,
            },
            vec![clause("=", vec![value])],
        ))
    }

    /// `DATABASE path [OPTIONS (...)]`, which takes no `OR REPLACE` or `IF NOT EXISTS`.
    fn create_database(&mut self, head: &Head) -> Parsing<Expression> {
        self.expect_kw("DATABASE")?;
        let name: TableRef = self.table_path()?;
        let mut operands: Vec<Expression> = Vec::new();
        operands.extend(self.options_clause()?);
        Ok(created(
            head,
            CreateTarget {
                kind: "DATABASE",
                if_not_exists: false,
                name,
            },
            operands,
        ))
    }
}
