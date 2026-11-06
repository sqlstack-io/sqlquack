//! The utility statements of gram.y other than sessions and transactions, as `UtilityStatement`, `PrepareStatement` and `ExecuteStatement`.

use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::statements::models::{ExecuteStatement, PrepareStatement};
use sqlquack_syntax::{DataType, Expression};

use crate::parse::_helpers::commands::alters::name_from;
use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::commands::sessions::utility;
use crate::parse::_helpers::commands::tables::clause;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_col_id};
use crate::parse::_helpers::tree::nodes::{literal, star, table, tuple};
use crate::scan::models::TokenKind;
use sqlquack_syntax::expressions::names::models::Literal;

/// The `utility_option_name` both spellings of `ANALYZE` give.
const ANALYZE_OPTION: &str = "analyze";

/// `VACUUM`'s keyword options, in the only order gram.y takes them.
const VACUUM_FLAGS: [&str; 4] = ["full", "freeze", "verbose", ANALYZE_OPTION];

/// `cursor_options` words, longest first, with the flag each sets.
const CURSOR_FLAGS: [(&[&str], &str); 5] = [
    (&["no", "scroll"], "NO SCROLL"),
    (&["scroll"], "SCROLL"),
    (&["binary"], "BINARY"),
    (&["asensitive"], "ASENSITIVE"),
    (&["insensitive"], "INSENSITIVE"),
];

/// The flag `WITH HOLD` sets.
const HOLD: &str = "HOLD";

/// `fetch_args` directions that take no count, with the direction and count each gives.
const FIXED_DIRECTIONS: [(&str, &str, i64); 4] = [
    ("next", FORWARD, 1),
    ("prior", BACKWARD, 1),
    ("first", ABSOLUTE, 1),
    ("last", ABSOLUTE, -1),
];

const FORWARD: &str = "FORWARD";
const BACKWARD: &str = "BACKWARD";
const ABSOLUTE: &str = "ABSOLUTE";
const RELATIVE: &str = "RELATIVE";

/// The count `ALL` gives (`FETCH_ALL`).
const ALL: &str = "ALL";

/// `REINDEX` targets with the kind each names, and whether its name is a relation, a name, or optional.
const REINDEX_TARGETS: [(&str, &str, Reindexed); 5] = [
    ("index", "INDEX", Reindexed::Relation),
    ("table", "TABLE", Reindexed::Relation),
    ("schema", "SCHEMA", Reindexed::Name),
    ("system", "SYSTEM", Reindexed::Optional),
    ("database", "DATABASE", Reindexed::Optional),
];

/// What a `REINDEX` target names.
#[derive(Clone, Copy, PartialEq)]
enum Reindexed {
    Relation,
    Name,
    Optional,
}

/// The statements `EXPLAIN` takes beyond a `PreparableStmt`.
const EXPLAINED_STARTS: [&str; 4] = ["declare", "create", "refresh", "execute"];

/// One argument-less `DefElem` option, as `(name)`.
fn flag(name: &str) -> Expression {
    tuple(vec![Expression::Identifier(Identifier::new(name))])
}

/// An integer value.
fn integer(value: i64) -> Expression {
    literal(Literal::Number(value.to_string()))
}

impl Parser<'_> {
    /// `CheckPointStmt`.
    pub(crate) fn checkpoint_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("checkpoint")?;
        Ok(utility("CHECKPOINT", Vec::new(), Vec::new()))
    }

    /// `utility_option_list` in parentheses: each `(name)` or `(name, value)`, in order.
    fn utility_options(&mut self) -> Parsing<Vec<Expression>> {
        self.parenthesized(|parser| {
            let name: String = match parser.keyword_at(0) {
                Some("analyze" | "analyse") => {
                    parser.at += 1;
                    ANALYZE_OPTION.to_owned()
                }
                Some("format") => {
                    parser.at += 1;
                    "format".to_owned()
                }
                _ => {
                    let token = parser
                        .expect_token(crate::parse::_helpers::tree::cursor::is_non_reserved_word)?;
                    identifier(&token).name
                }
            };
            let mut parts: Vec<Expression> = vec![Expression::Identifier(Identifier::new(name))];
            let ends: bool = parser.is_char(',') || parser.is_char(')');
            if !ends {
                parts.push(match parser.peek().map(|token| token.kind) {
                    Some(TokenKind::Integer | TokenKind::Numeric | TokenKind::Char('+' | '-')) => {
                        parser.numeric_only()?
                    }
                    _ => parser.boolean_or_string()?,
                });
            }
            Ok(tuple(parts))
        })
    }

    /// `ClusterStmt`: options, then a table and its index; `CLUSTER index ON table` names them the other way round.
    pub(crate) fn cluster_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("cluster")?;
        let mut options: Vec<Expression> = Vec::new();
        let parenthesized: bool = self.is_char('(');
        if parenthesized {
            options = self.utility_options()?;
        } else if self.eat_kw("verbose") {
            options.push(flag("verbose"));
        }
        let mut operands: Vec<Expression> = Vec::new();
        if self.peek().is_some_and(is_col_id) {
            let start: usize = self.at;
            let old: bool = !parenthesized && self.kw_at(1, "on");
            if old {
                let index: Identifier = self.name(is_col_id)?;
                self.at += 1;
                operands.push(Expression::Table(Box::new(table(self.table_name()?))));
                operands.push(clause("USING", vec![name_from(index)]));
                return Ok(utility("CLUSTER", options, operands));
            }
            self.at = start;
            operands.push(Expression::Table(Box::new(table(self.table_name()?))));
            if self.eat_kw("using") {
                operands.push(clause("USING", vec![name_from(self.name(is_col_id)?)]));
            }
        }
        Ok(utility("CLUSTER", options, operands))
    }

    /// `VacuumStmt` (`analyze` false) or `AnalyzeStmt`: options, then relations with their columns.
    pub(crate) fn vacuum_statement(&mut self, analyze: bool) -> Parsing<Expression> {
        self.at += 1;
        let mut options: Vec<Expression> = Vec::new();
        if self.is_char('(') {
            options = self.utility_options()?;
        } else {
            let flags: &[&str] = match analyze {
                true => &VACUUM_FLAGS[2..3],
                false => &VACUUM_FLAGS,
            };
            for word in flags {
                let found: bool = match *word == ANALYZE_OPTION {
                    true => self.eat_kw("analyze") || self.eat_kw("analyse"),
                    false => self.eat_kw(word),
                };
                if found {
                    options.push(flag(word));
                }
            }
        }
        let mut relations: Vec<Expression> = Vec::new();
        if self.peek().is_some_and(is_col_id) || self.kw("only") {
            relations = self.comma_list(|parser| {
                let relation: Expression = Expression::Table(Box::new(parser.relation()?));
                match parser.is_char('(') {
                    true => {
                        let columns: Vec<Expression> = parser
                            .parenthesized(|parser| Ok(name_from(parser.name(is_col_id)?)))?;
                        Ok(tuple(vec![relation, tuple(columns)]))
                    }
                    false => Ok(relation),
                }
            })?;
        }
        let kind: &str = match analyze {
            true => "ANALYZE",
            false => "VACUUM",
        };
        Ok(utility(kind, options, relations))
    }

    /// `ReindexStmt`: options with `CONCURRENTLY` appended, as gram.y appends it, then the target and its name.
    pub(crate) fn reindex_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("reindex")?;
        let mut options: Vec<Expression> = match self.is_char('(') {
            true => self.utility_options()?,
            false => Vec::new(),
        };
        let Some((_, kind, named)) = REINDEX_TARGETS.iter().find(|(word, _, _)| self.kw(word))
        else {
            return Err(self.error());
        };
        self.at += 1;
        if self.eat_kw("concurrently") {
            options.push(flag("concurrently"));
        }
        let mut operands: Vec<Expression> = vec![var(kind)];
        match named {
            Reindexed::Relation => {
                operands.push(Expression::Table(Box::new(table(self.table_name()?))));
            }
            Reindexed::Name => operands.push(name_from(self.name(is_col_id)?)),
            Reindexed::Optional => {
                if self.peek().is_some_and(is_col_id) {
                    operands.push(name_from(self.name(is_col_id)?));
                }
            }
        }
        Ok(utility("REINDEX", options, operands))
    }

    /// `ListenStmt`, `NotifyStmt` and `UnlistenStmt`: a channel, a payload, or every channel.
    pub(crate) fn channel_statement(&mut self) -> Parsing<Expression> {
        let word: &str = self.keyword_at(0).unwrap_or_default();
        self.at += 1;
        let kind: String = word.to_uppercase();
        let (every, payload): (bool, bool) = match word {
            "unlisten" => (true, false),
            "notify" => (false, true),
            _ => (false, false),
        };
        if every && self.eat_char('*') {
            return Ok(utility(&kind, Vec::new(), vec![star(None)]));
        }
        let mut operands: Vec<Expression> = vec![name_from(self.name(is_col_id)?)];
        if payload && self.eat_char(',') {
            operands.push(self.sconst()?);
        }
        Ok(utility(&kind, Vec::new(), operands))
    }

    /// `LoadStmt`: `LOAD 'file'`.
    pub(crate) fn load_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("load")?;
        Ok(utility("LOAD", Vec::new(), vec![self.sconst()?]))
    }

    /// A cursor or prepared statement's `name`, or `ALL` where `all` allows it.
    fn name_or_all(&mut self, all: bool) -> Parsing<Expression> {
        if all && self.eat_kw("all") {
            return Ok(var(ALL));
        }
        Ok(name_from(self.name(is_col_id)?))
    }

    /// `ClosePortalStmt`: `CLOSE {name | ALL}`.
    pub(crate) fn close_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("close")?;
        Ok(utility("CLOSE", Vec::new(), vec![self.name_or_all(true)?]))
    }

    /// `DeallocateStmt`: `DEALLOCATE [PREPARE] {name | ALL}`.
    pub(crate) fn deallocate_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("deallocate")?;
        self.eat_kw("prepare");
        Ok(utility(
            "DEALLOCATE",
            Vec::new(),
            vec![self.name_or_all(true)?],
        ))
    }

    /// `DeclareCursorStmt`: the cursor's flags as a set, then its name and `SelectStmt`.
    pub(crate) fn declare_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("declare")?;
        let name: Expression = name_from(self.name(is_col_id)?);
        let mut flags: Vec<&str> = Vec::new();
        while let Some((words, set)) = CURSOR_FLAGS.iter().find(|(words, _)| self.words_at(words)) {
            self.at += words.len();
            flags.push(set);
        }
        self.expect_kw("cursor")?;
        if self.kw("with") && self.kw_at(1, "hold") {
            self.at += 2;
            flags.push(HOLD);
        } else if self.kw("without") {
            self.at += 1;
            self.expect_kw("hold")?;
        }
        self.expect_kw("for")?;
        let start: usize = self.at;
        let query: Expression = self.preparable_statement()?;
        if matches!(
            query,
            Expression::Insert(_)
                | Expression::Update(_)
                | Expression::Delete(_)
                | Expression::Merge(_)
        ) {
            self.at = start;
            return Err(self.error());
        }
        flags.sort_unstable();
        flags.dedup();
        let options: Vec<Expression> = flags.into_iter().map(|set| tuple(vec![var(set)])).collect();
        Ok(utility("DECLARE CURSOR", options, vec![name, query]))
    }

    /// `FetchStmt`: `FETCH` or `MOVE` with the direction and count gram.y gives its `fetch_args`, then the cursor.
    pub(crate) fn fetch_statement(&mut self) -> Parsing<Expression> {
        let kind: String = self.keyword_at(0).unwrap_or_default().to_uppercase();
        self.at += 1;
        let (direction, count): (&str, Expression) = self.fetch_direction()?;
        if self.kw("from") || self.kw("in") {
            self.at += 1;
        }
        let cursor: Expression = name_from(self.name(is_col_id)?);
        Ok(utility(
            &kind,
            Vec::new(),
            vec![var(direction), count, cursor],
        ))
    }

    /// The direction and count before a cursor name, `FORWARD 1` when none is written.
    fn fetch_direction(&mut self) -> Parsing<(&'static str, Expression)> {
        let cursor_only: bool = self.peek_at(1).is_none() || self.char_at(1, ';');
        if cursor_only {
            return Ok((FORWARD, integer(1)));
        }
        if let Some((_, direction, count)) =
            FIXED_DIRECTIONS.iter().find(|(word, _, _)| self.kw(word))
        {
            self.at += 1;
            return Ok((direction, integer(*count)));
        }
        if self.kw("absolute") || self.kw("relative") {
            let direction: &str = match self.kw("absolute") {
                true => ABSOLUTE,
                false => RELATIVE,
            };
            self.at += 1;
            return Ok((direction, self.signed_iconst()?));
        }
        if self.eat_kw("all") {
            return Ok((FORWARD, var(ALL)));
        }
        if self.kw("forward") || self.kw("backward") {
            let direction: &str = match self.kw("forward") {
                true => FORWARD,
                false => BACKWARD,
            };
            self.at += 1;
            if self.eat_kw("all") {
                return Ok((direction, var(ALL)));
            }
            let counted: bool = matches!(
                self.peek().map(|token| token.kind),
                Some(TokenKind::Integer | TokenKind::Char('+' | '-'))
            );
            return match counted {
                true => Ok((direction, self.signed_iconst()?)),
                false => Ok((direction, integer(1))),
            };
        }
        let counted: bool = matches!(
            self.peek().map(|token| token.kind),
            Some(TokenKind::Integer | TokenKind::Char('+' | '-'))
        );
        match counted {
            true => Ok((FORWARD, self.signed_iconst()?)),
            false => Ok((FORWARD, integer(1))),
        }
    }

    /// `DoStmt`: its code and `LANGUAGE`, in the order written.
    pub(crate) fn do_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("do")?;
        let mut operands: Vec<Expression> = Vec::new();
        loop {
            if self.eat_kw("language") {
                operands.push(clause("LANGUAGE", vec![self.word_or_string()?]));
            } else if self.sconst_at(0) {
                operands.push(self.sconst()?);
            } else {
                break;
            }
        }
        match operands.is_empty() {
            true => Err(self.error()),
            false => Ok(utility("DO", Vec::new(), operands)),
        }
    }

    /// `ExplainStmt`: options, as gram.y lists the keyword forms, then the explained statement.
    pub(crate) fn explain_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("explain")?;
        let mut options: Vec<Expression> = Vec::new();
        if self.is_char('(') {
            options = self.utility_options()?;
        } else if self.eat_kw("analyze") || self.eat_kw("analyse") {
            options.push(flag(ANALYZE_OPTION));
            if self.eat_kw("verbose") {
                options.push(flag("verbose"));
            }
        } else if self.eat_kw("verbose") {
            options.push(flag("verbose"));
        }
        let start: usize = self.at;
        let statement: Expression = match self.keyword_at(0) {
            Some(word) if EXPLAINED_STARTS.contains(&word) => {
                let statement: Expression = self.statement()?;
                let explainable: bool = match &statement {
                    Expression::CreateTable(node) => node.as_select.is_some(),
                    Expression::UtilityStatement(node) => {
                        matches!(
                            node.kind.as_str(),
                            "DECLARE CURSOR" | "REFRESH MATERIALIZED VIEW"
                        )
                    }
                    Expression::CreateView(node) => node.materialized,
                    Expression::Execute(_) => true,
                    _ => false,
                };
                if !explainable {
                    self.at = start;
                    return Err(self.error());
                }
                statement
            }
            _ => self.preparable_statement()?,
        };
        Ok(utility("EXPLAIN", options, vec![statement]))
    }

    /// `CallStmt`: `CALL func_application`, the call whole.
    pub(crate) fn call_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("call")?;
        let name: Vec<Identifier> = self.func_name()?;
        let call: Expression = self.function_call(name, true)?;
        Ok(utility("CALL", Vec::new(), vec![call]))
    }

    /// `PrepareStmt`: `PREPARE name [( type, ... )] AS PreparableStmt`.
    pub(crate) fn prepare_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("prepare")?;
        let name: Identifier = self.name(is_col_id)?;
        let parameter_types: Vec<DataType> = match self.is_char('(') {
            true => self.parenthesized(|parser| parser.typename())?,
            false => Vec::new(),
        };
        self.expect_kw("as")?;
        let statement: Expression = self.preparable_statement()?;
        Ok(Expression::Prepare(Box::new(PrepareStatement {
            name,
            parameter_types,
            statement,
        })))
    }

    /// `ExecuteStmt`: `EXECUTE name [( expr, ... )]`.
    pub(crate) fn execute_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("execute")?;
        let name: Expression = name_from(self.name(is_col_id)?);
        let arguments: Vec<Expression> = match self.is_char('(') {
            true => self.parenthesized(|parser| parser.a_expr())?,
            false => Vec::new(),
        };
        Ok(Expression::Execute(Box::new(ExecuteStatement {
            this: name,
            return_status: None,
            parameters: Vec::new(),
            arguments,
            prepared: true,
            suffix: None,
        })))
    }

    /// `RefreshMatViewStmt`: `[CONCURRENTLY] name [WITH [NO] DATA]`; only `WITH NO DATA` differs from the default.
    pub(crate) fn refresh_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("refresh")?;
        self.expect_kw("materialized")?;
        self.expect_kw("view")?;
        let mut options: Vec<Expression> = Vec::new();
        if self.eat_kw("concurrently") {
            options.push(tuple(vec![var("CONCURRENTLY")]));
        }
        let view: Expression = Expression::Table(Box::new(table(self.table_name()?)));
        if self.eat_kw("with") {
            if self.eat_kw("no") {
                options.push(tuple(vec![var("NO DATA")]));
            }
            self.expect_kw("data")?;
        }
        Ok(utility("REFRESH MATERIALIZED VIEW", options, vec![view]))
    }

    /// `ImportForeignSchemaStmt`: the remote schema, `LIMIT TO` or `EXCEPT` tables, the server, the local schema and options.
    pub(crate) fn import_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("import")?;
        self.expect_kw("foreign")?;
        self.expect_kw("schema")?;
        let mut operands: Vec<Expression> = vec![name_from(self.name(is_col_id)?)];
        let qualification: Option<&str> = match self.keyword_at(0) {
            Some("limit") => {
                self.at += 1;
                self.expect_kw("to")?;
                Some("LIMIT TO")
            }
            Some("except") => {
                self.at += 1;
                Some("EXCEPT")
            }
            _ => None,
        };
        if let Some(qualification) = qualification {
            let tables: Vec<Expression> =
                self.parenthesized(|parser| Ok(Expression::Table(Box::new(parser.relation()?))))?;
            operands.push(clause(qualification, vec![tuple(tables)]));
        }
        self.expect_kw("from")?;
        self.expect_kw("server")?;
        operands.push(clause("SERVER", vec![name_from(self.name(is_col_id)?)]));
        self.expect_kw("into")?;
        operands.push(clause("INTO", vec![name_from(self.name(is_col_id)?)]));
        operands.extend(self.create_generic_options()?);
        Ok(utility("IMPORT FOREIGN SCHEMA", Vec::new(), operands))
    }

    /// `ReassignOwnedStmt`: `REASSIGN OWNED BY role_list TO RoleSpec`.
    pub(crate) fn reassign_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("reassign")?;
        self.expect_kw("owned")?;
        self.expect_kw("by")?;
        let roles: Expression = self.role_list()?;
        self.expect_kw("to")?;
        let role: Expression = self.role_spec()?;
        Ok(utility(
            "REASSIGN OWNED",
            Vec::new(),
            vec![roles, clause("TO", vec![role])],
        ))
    }
}
