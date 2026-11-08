//! Statements: dispatch on the leading words, control of flow, variables, transactions and `EXEC`.

use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::statements::models::{
    Commit, Declare, DeclareItem, ExecuteParameter, ExecuteStatement, Rollback, ScriptingBlock,
    ScriptingBranch, ScriptingElse, ScriptingIf, ScriptingLoopKeyword, ScriptingSql,
    ScriptingStatement, ScriptingWhile, SetItem, SetStatement, Transaction, TryCatch, Use,
};
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_any_name, is_name};
use crate::parse::_helpers::tree::nodes::{binary, column_from, paren, var};
use crate::parse::_helpers::values::primary::variable;
use crate::parse::constants::{MAX_DEPTH, STATISTICS_SWITCH, UNREAD_STATEMENTS};
use crate::scan::models::{Token, TokenKind};

/// Statements that must be the first of their batch and take the rest of it as their body.
pub(crate) const BATCH_FIRST: [&str; 7] = [
    "VIEW",
    "FUNCTION",
    "PROCEDURE",
    "PROC",
    "TRIGGER",
    "SCHEMA",
    "DEFAULT",
];

/// The `SET` options that take `ON` or `OFF`.
const SET_SWITCHES: [&str; 22] = [
    "ANSI_DEFAULTS",
    "ANSI_NULL_DFLT_OFF",
    "ANSI_NULL_DFLT_ON",
    "ANSI_NULLS",
    "ANSI_PADDING",
    "ANSI_WARNINGS",
    "ARITHABORT",
    "ARITHIGNORE",
    "CONCAT_NULL_YIELDS_NULL",
    "CURSOR_CLOSE_ON_COMMIT",
    "FMTONLY",
    "FORCEPLAN",
    "IMPLICIT_TRANSACTIONS",
    "NOCOUNT",
    "NOEXEC",
    "NUMERIC_ROUNDABORT",
    "PARSEONLY",
    "QUOTED_IDENTIFIER",
    "REMOTE_PROC_TRANSACTIONS",
    "SHOWPLAN_ALL",
    "STATISTICS",
    "XACT_ABORT",
];
/// Words that can follow a cursor's name in `DECLARE name ... CURSOR`.
const CURSOR_WORDS: [&str; 14] = [
    "CURSOR",
    "INSENSITIVE",
    "SCROLL",
    "FORWARD_ONLY",
    "LOCAL",
    "GLOBAL",
    "STATIC",
    "KEYSET",
    "DYNAMIC",
    "FAST_FORWARD",
    "READ_ONLY",
    "SCROLL_LOCKS",
    "OPTIMISTIC",
    "TYPE_WARNING",
];

fn sql(statement: Expression) -> ScriptingStatement {
    ScriptingStatement::Sql(ScriptingSql {
        span: SourceSpan(None),
        statement,
    })
}

fn block(body: Vec<ScriptingStatement>) -> ScriptingBlock {
    ScriptingBlock {
        span: SourceSpan(None),
        declarations: Vec::new(),
        body,
        exception_handlers: Vec::new(),
        label: None,
    }
}

impl<'sql> Parser<'sql> {
    /// Every statement of the batch; a batch-first `CREATE` is read only as the first.
    pub(crate) fn batch(&mut self) -> Parsing<Vec<Expression>> {
        let mut statements: Vec<Expression> = Vec::new();
        loop {
            while self.eat_char(';') {}
            if self.at_end() {
                return Ok(statements);
            }
            if self.batch_first_at() {
                if !statements.is_empty() {
                    return Err(self.batch_first_error());
                }
                let start: usize = self.offset();
                let statement: Expression = match self.kw("ALTER") {
                    true => self.alter_statement()?,
                    false => self.create_statement()?,
                };
                statements.push(self.within_depth(statement, start)?);
                while self.eat_char(';') {}
                if !self.at_end() && !self.gapped() {
                    return Err(self.error());
                }
                return Ok(statements);
            }
            let start: usize = self.offset();
            let statement: Expression = self.statement()?;
            statements.push(self.within_depth(statement, start)?);
        }
    }

    /// `statement`, or the gap `nesting_depth` from `start` when its tree is deeper than the limit.
    fn within_depth(&mut self, statement: Expression, start: usize) -> Parsing<Expression> {
        match statement.tree_depth() > MAX_DEPTH {
            true => Err(self.gap("nesting_depth", start)),
            false => Ok(statement),
        }
    }

    /// Whether `CREATE [OR ALTER] kind` or `ALTER kind` of a batch-first kind is next.
    pub(crate) fn batch_first_at(&self) -> bool {
        let offset: usize = match (self.word_at(0).as_deref(), self.kw_at(1, "OR")) {
            (Some("CREATE"), true) if self.kw_at(2, "ALTER") => 3,
            (Some("CREATE" | "ALTER"), _) => 1,
            _ => return false,
        };
        let kind: Option<String> = self.word_at(offset);
        let alter_schema: bool = self.kw("ALTER") && kind.as_deref() == Some("SCHEMA");
        kind.is_some_and(|kind| BATCH_FIRST.contains(&kind.as_str())) && !alter_schema
    }

    /// The engine's error 111 for a batch-first statement after another statement.
    fn batch_first_error(&self) -> crate::parse::errors::ParseError {
        let kind: String = (1..4)
            .filter_map(|offset| self.word_at(offset))
            .find(|word| BATCH_FIRST.contains(&word.as_str()))
            .unwrap_or_default();
        let head: String = self.word_at(0).unwrap_or_default();
        self.error_with(&format!(
            "'{head} {kind}' must be the first statement in a query batch."
        ))
    }

    /// One statement, with a `;` after it if one is written.
    pub(crate) fn statement(&mut self) -> Parsing<Expression> {
        let statement: Expression = self.deeper(|parser| parser.statement_body())?;
        Ok(statement)
    }

    fn statement_body(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        if self.batch_first_at() {
            return Err(self.error_at(self.at + 1));
        }
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        if token.kind == TokenKind::Char('(') {
            return self.query_statement();
        }
        if token.kind != TokenKind::Word {
            return Err(self.error());
        }
        if self.char_at(1, ':') {
            return Err(self.gap("label", start));
        }
        let word: String = token.value.to_ascii_uppercase();
        match word.as_str() {
            "SELECT" => self.query_statement(),
            "WITH" => {
                let with = self.with_clause()?;
                match with {
                    Some(with) => self.dml_with(with),
                    None => Err(self.error()),
                }
            }
            "INSERT" => self.insert_statement(),
            "UPDATE" if !self.char_at(1, '(') => self.update_statement(),
            "DELETE" => self.delete_statement(),
            "MERGE" => self.merge_statement(),
            "CREATE" => self.create_statement(),
            "ALTER" => self.alter_statement(),
            "DROP" => self.drop_statement(),
            "TRUNCATE" => self.truncate_statement(),
            "IF" => self.if_statement(),
            "WHILE" => self.while_statement(),
            "BEGIN" => self.begin_statement(),
            "COMMIT" => self.commit_statement(),
            "ROLLBACK" => self.rollback_statement(),
            "SAVE" => Err(self.gap("save_transaction", start)),
            "DECLARE" => self.declare_statement(),
            "SET" => self.set_statement(),
            "EXEC" | "EXECUTE" => self.execute_statement(),
            "USE" => {
                self.at += 1;
                let name: Identifier = self
                    .expect_token(is_any_name)
                    .map(|token| identifier(&token))?;
                Ok(Expression::Use(Box::new(Use {
                    kind: None,
                    this: name,
                    roles: Vec::new(),
                    parts: Vec::new(),
                })))
            }
            "RETURN" => {
                self.at += 1;
                let value: Expression = match self.statement_ends() {
                    true => var("RETURN"),
                    false => self.scalar()?,
                };
                Ok(Expression::ReturnStmt(Box::new(value)))
            }
            "BREAK" | "CONTINUE" => Err(self.gap("loop_jump", start)),
            "PRINT" => Err(self.gap("print", start)),
            "THROW" => Err(self.gap("throw", start)),
            "RAISERROR" => Err(self.gap("raiserror", start)),
            word if UNREAD_STATEMENTS.contains(&word) => Err(self.gap("statement", start)),
            "END" if self.kw_at(1, "CONVERSATION") => Err(self.gap("statement", start)),
            _ if self.at == 0 && is_name(&token) => Err(self.gap("implicit_exec", start)),
            _ => Err(self.error()),
        }
    }

    /// Whether the statement ends here: the input's end, a `;`, or a word that starts another.
    fn statement_ends(&self) -> bool {
        self.at_end()
            || self.is_char(';')
            || self.kw_in(
                0,
                &[
                    "END", "ELSE", "SELECT", "INSERT", "UPDATE", "DELETE", "IF", "BEGIN", "SET",
                    "DECLARE", "EXEC", "EXECUTE", "RETURN", "COMMIT", "ROLLBACK",
                ],
            )
    }

    /// The statement after `IF`, `ELSE` or `WHILE`, which may not be a batch-first `CREATE`.
    fn branch(&mut self) -> Parsing<Vec<ScriptingStatement>> {
        while self.eat_char(';') {}
        Ok(vec![sql(self.statement()?)])
    }

    /// `IF condition statement [ELSE statement]`.
    fn if_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("IF")?;
        let condition: Expression = self.condition()?;
        let body: Vec<ScriptingStatement> = self.branch()?;
        let mut look: usize = 0;
        while self.char_at(look, ';') {
            look += 1;
        }
        let else_branch: Option<ScriptingElse> = match self.kw_at(look, "ELSE") {
            true => {
                self.at += look + 1;
                Some(ScriptingElse {
                    span: SourceSpan(None),
                    body: self.branch()?,
                })
            }
            false => None,
        };
        let node: ScriptingIf = ScriptingIf {
            span: SourceSpan(None),
            branches: vec![ScriptingBranch {
                span: SourceSpan(None),
                condition,
                body,
            }],
            else_branch,
        };
        Ok(Expression::ScriptingBlock(Box::new(block(vec![
            ScriptingStatement::If(node),
        ]))))
    }

    /// `WHILE condition statement`.
    fn while_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("WHILE")?;
        let condition: Expression = self.condition()?;
        let body: Vec<ScriptingStatement> = self.branch()?;
        Ok(Expression::ScriptingBlock(Box::new(block(vec![
            ScriptingStatement::While(ScriptingWhile {
                span: SourceSpan(None),
                condition,
                keyword: ScriptingLoopKeyword::Do,
                body,
                label: None,
            }),
        ]))))
    }

    /// The statements of a block up to its `END`, at least one.
    pub(crate) fn block_body(&mut self, end: &[&str]) -> Parsing<Vec<Expression>> {
        let mut statements: Vec<Expression> = Vec::new();
        loop {
            while self.eat_char(';') {}
            if end
                .iter()
                .enumerate()
                .all(|(offset, word)| self.kw_at(offset, word))
            {
                if statements.is_empty() {
                    return Err(self.error());
                }
                self.at += end.len();
                return Ok(statements);
            }
            if self.at_end() {
                return Err(self.error());
            }
            statements.push(self.statement()?);
        }
    }

    /// `BEGIN ... END`, `BEGIN TRY ... END TRY BEGIN CATCH ... END CATCH`, or `BEGIN TRAN`.
    fn begin_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("BEGIN")?;
        match self.word_at(0).as_deref() {
            Some("TRAN" | "TRANSACTION") => {
                self.at += 1;
                let name: Option<Box<Expression>> = self.transaction_name()?;
                if self.kw("WITH") {
                    return Err(self.gap("transaction_mark", start));
                }
                Ok(Expression::Transaction(Box::new(Transaction {
                    this: name,
                    modes: None,
                    mark: None,
                })))
            }
            Some("DISTRIBUTED") => Err(self.gap("distributed_transaction", start)),
            Some("TRY") => {
                self.at += 1;
                let try_body: Vec<Expression> = self.block_body(&["END", "TRY"])?;
                self.expect_kws(&["BEGIN", "CATCH"])?;
                let catch_body: Vec<Expression> = self.catch_body()?;
                Ok(Expression::TryCatch(Box::new(TryCatch {
                    try_body,
                    catch_body: Some(catch_body),
                })))
            }
            Some("DIALOG" | "CONVERSATION") => Err(self.gap("statement", start)),
            _ => {
                let body: Vec<Expression> = self.block_body(&["END"])?;
                Ok(Expression::ScriptingBlock(Box::new(block(
                    body.into_iter().map(sql).collect(),
                ))))
            }
        }
    }

    /// A `CATCH` block may be empty, unlike every other block.
    fn catch_body(&mut self) -> Parsing<Vec<Expression>> {
        let mut statements: Vec<Expression> = Vec::new();
        loop {
            while self.eat_char(';') {}
            if self.kw("END") && self.kw_at(1, "CATCH") {
                self.at += 2;
                return Ok(statements);
            }
            if self.at_end() {
                return Err(self.error());
            }
            statements.push(self.statement()?);
        }
    }

    /// A transaction name or `@variable`, when one follows.
    fn transaction_name(&mut self) -> Parsing<Option<Box<Expression>>> {
        let Some(token) = self.peek().cloned() else {
            return Ok(None);
        };
        Ok(match token.kind {
            TokenKind::Variable => {
                self.at += 1;
                Some(Box::new(variable(&token)))
            }
            TokenKind::Word | TokenKind::Quoted(_)
                if crate::parse::_helpers::tree::cursor::is_name(&token)
                    && !self.statement_ends() =>
            {
                self.at += 1;
                Some(Box::new(column_from(vec![identifier(&token)])))
            }
            _ => None,
        })
    }

    fn commit_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("COMMIT")?;
        let named: bool = self.eat_kw("TRAN") || self.eat_kw("TRANSACTION");
        let this: Option<Box<Expression>> = match named {
            true => self.transaction_name()?,
            false => {
                self.eat_kw("WORK");
                None
            }
        };
        if self.kw("WITH") {
            return Err(self.gap("commit_durability", self.offset()));
        }
        Ok(Expression::Commit(Box::new(Commit {
            chain: None,
            this,
            durability: None,
        })))
    }

    fn rollback_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("ROLLBACK")?;
        let named: bool = self.eat_kw("TRAN") || self.eat_kw("TRANSACTION");
        let this: Option<Box<Expression>> = match named {
            true => self.transaction_name()?,
            false => {
                self.eat_kw("WORK");
                None
            }
        };
        Ok(Expression::Rollback(Box::new(Rollback {
            savepoint: None,
            this,
        })))
    }

    /// `DECLARE @name [AS] type [= value], ...`; cursors and table variables are gaps.
    fn declare_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("DECLARE")?;
        if self.is(is_name) && self.kw_in(1, &CURSOR_WORDS) {
            return Err(self.gap("declare_cursor", start));
        }
        let expressions: Vec<Expression> = self.comma_list(|parser| {
            let name: Token = parser.expect_token(|token| token.kind == TokenKind::Variable)?;
            let has_as: bool = parser.eat_kw("AS");
            if parser.kw_in(0, &["CURSOR", "TABLE"]) {
                return Err(parser.gap("declare_cursor_or_table", start));
            }
            let type_start: usize = parser.offset();
            parser.data_type()?;
            let kind: String = parser.sql[type_start..parser.last_end()].to_owned();
            let default: Option<Box<Expression>> = match parser.eat_char('=') {
                true => Some(Box::new(parser.scalar()?)),
                false => None,
            };
            Ok(Expression::DeclareItem(Box::new(DeclareItem {
                this: Box::new(variable(&name)),
                kind: Some(kind),
                default,
                has_as,
                additional_names: Vec::new(),
            })))
        })?;
        Ok(Expression::Declare(Box::new(Declare {
            expressions,
            replace: false,
        })))
    }

    /// `SET @v = value` (or a compound assignment) and `SET option {ON|OFF}`.
    fn set_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("SET")?;
        if let Some(token) = self
            .peek()
            .cloned()
            .filter(|token| token.kind == TokenKind::Variable)
        {
            self.at += 1;
            let operator: String = match self.peek() {
                Some(next) if next.kind == TokenKind::Char('=') => "=".to_owned(),
                _ if self.assignment_at(0) => self
                    .peek()
                    .map(|next| next.value.clone())
                    .unwrap_or_default(),
                _ => return Err(self.gap("set_variable", start)),
            };
            self.at += 1;
            if self.kw("CURSOR") {
                return Err(self.gap("cursor", start));
            }
            let value: Expression = self.scalar()?;
            return Ok(Expression::SetStatement(Box::new(SetStatement {
                items: vec![SetItem {
                    name: variable(&token),
                    value,
                    kind: (operator != "=").then_some(operator),
                    no_equals: false,
                }],
            })));
        }
        let mut names: Vec<String> = Vec::new();
        loop {
            match self.word_at(0) {
                Some(word) if SET_SWITCHES.contains(&word.as_str()) => {
                    self.at += 1;
                    if word == STATISTICS_SWITCH {
                        return Err(self.gap("set_option", start));
                    }
                    names.push(word);
                }
                _ => return Err(self.gap("set_option", start)),
            }
            if !self.eat_char(',') {
                break;
            }
        }
        let value: String = match self.word_at(0).as_deref() {
            Some(value @ ("ON" | "OFF")) => value.to_owned(),
            _ => return Err(self.error()),
        };
        self.at += 1;
        Ok(Expression::SetStatement(Box::new(SetStatement {
            items: names
                .iter()
                .map(|name| SetItem {
                    name: var(name),
                    value: var(&value),
                    kind: None,
                    no_equals: true,
                })
                .collect(),
        })))
    }

    /// `EXEC [@status =] procedure [args]` or `EXEC ('text' [+ ...])`.
    fn execute_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.at += 1;
        if self.kw("AS") {
            return Err(self.gap("execute_as", start));
        }
        if self.is_char('(') {
            self.at += 1;
            let text: Expression = self.execute_text()?;
            if self.is_char(',') {
                return Err(self.gap("exec_context", start));
            }
            self.expect_char(')')?;
            if self.kw_in(0, &["AS", "AT"]) {
                return Err(self.gap("exec_context", start));
            }
            return Ok(execute(paren(text), None, Vec::new()));
        }
        let mut return_status: Option<String> = None;
        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Variable)
            && self.char_at(1, '=')
        {
            return_status = Some(self.advance()?.value);
            self.at += 1;
        }
        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Variable)
        {
            return Err(self.gap("exec_variable_module", start));
        }
        let name: Vec<Identifier> = self.multipart(4)?;
        if self.is_char(';')
            && self
                .peek_at(1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Number(_)))
        {
            return Err(self.gap("procedure_number", start));
        }
        let mut parameters: Vec<ExecuteParameter> = Vec::new();
        if !self.statement_ends() && !self.kw("WITH") {
            parameters = self.comma_list(|parser| {
                let mut name: String = String::new();
                if parser
                    .peek()
                    .is_some_and(|token| token.kind == TokenKind::Variable)
                    && parser.char_at(1, '=')
                {
                    name = parser.advance()?.value;
                    parser.at += 1;
                }
                let value: Expression = match parser.kw("DEFAULT") {
                    true => {
                        parser.at += 1;
                        var("DEFAULT")
                    }
                    false => parser.execute_argument()?,
                };
                let output: bool = parser.eat_kw("OUTPUT") || parser.eat_kw("OUT");
                Ok(ExecuteParameter {
                    positional: name.is_empty(),
                    name,
                    value,
                    output,
                })
            })?;
        }
        if self.kw("WITH") {
            if !self.is_at(1, |token| token.kind == TokenKind::Word) {
                return Err(self.error());
            }
            return Err(self.gap("exec_options", start));
        }
        Ok(execute(column_from(name), return_status, parameters))
    }

    /// `EXEC`'s dynamic text: string literals and variables joined by `+`, nothing else.
    fn execute_text(&mut self) -> Parsing<Expression> {
        let mut text: Expression = self.execute_text_part()?;
        while self.eat_char('+') {
            let right: Expression = self.execute_text_part()?;
            text = binary(Expression::Add, text, right);
        }
        Ok(text)
    }

    fn execute_text_part(&mut self) -> Parsing<Expression> {
        match self.peek() {
            Some(token) if matches!(token.kind, TokenKind::String { .. } | TokenKind::Variable) => {
                self.primary_value()
            }
            _ => Err(self.error()),
        }
    }

    /// An `EXEC` argument: a constant, a variable, or a bare word read as a string.
    fn execute_argument(&mut self) -> Parsing<Expression> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        match token.kind {
            TokenKind::Word | TokenKind::Quoted(_) if is_name(&token) => {
                self.at += 1;
                Ok(column_from(vec![identifier(&token)]))
            }
            TokenKind::Char('-')
            | TokenKind::Char('+')
            | TokenKind::Number(_)
            | TokenKind::String { .. }
            | TokenKind::Variable
            | TokenKind::Word => {
                let value: Expression = self.primary_or_signed()?;
                Ok(value)
            }
            _ => Err(self.error()),
        }
    }

    fn primary_or_signed(&mut self) -> Parsing<Expression> {
        if self.is_char('-') || self.is_char('+') {
            let negative: bool = self.is_char('-');
            self.at += 1;
            let this: Expression = self.primary()?;
            return Ok(match negative {
                true => Expression::Neg(Box::new(
                    sqlquack_syntax::expressions::operators::models::UnaryOp::new(this),
                )),
                false => this,
            });
        }
        self.primary()
    }
}

fn execute(
    this: Expression,
    return_status: Option<String>,
    parameters: Vec<ExecuteParameter>,
) -> Expression {
    Expression::Execute(Box::new(ExecuteStatement {
        this,
        return_status,
        parameters,
        arguments: Vec::new(),
        prepared: false,
        suffix: None,
    }))
}
