//! Snowflake Scripting blocks and their members, as documented; unknown forms fail rather than pass.

use crate::expressions::names::models::{Identifier, Literal};
use crate::expressions::statements::models::{
    ScriptingAssignment, ScriptingAssignmentOperator, ScriptingAwait, ScriptingAwaitTarget,
    ScriptingBlock, ScriptingBranch, ScriptingCase, ScriptingCursor, ScriptingCursorSource,
    ScriptingDeclaration, ScriptingDefault, ScriptingElse, ScriptingException,
    ScriptingExceptionHandler, ScriptingFetch, ScriptingFor, ScriptingForIteration,
    ScriptingHandledExceptions, ScriptingHandlerAction, ScriptingIf, ScriptingJump,
    ScriptingJumpKeyword, ScriptingLet, ScriptingLoop, ScriptingLoopKeyword, ScriptingName,
    ScriptingNull, ScriptingOpen, ScriptingProcedure, ScriptingRaise, ScriptingRange,
    ScriptingRepeat, ScriptingResultset, ScriptingResultsetDefault, ScriptingReturn,
    ScriptingReturnValue, ScriptingSql, ScriptingStatement, ScriptingVariable, ScriptingWhile,
};
use crate::expressions::tree::models::Expression;
use crate::parser::models::Parser;
use crate::tokens::errors::Error;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// Words that end a statement list; the enclosing construct decides which one it accepts.
const BODY_TERMINATORS: [&str; 6] = ["END", "ELSE", "ELSEIF", "WHEN", "UNTIL", "EXCEPTION"];

/// Unquoted words Snowflake refuses as scripting labels and names, as recorded.
const RESERVED_NAMES: [&str; 62] = [
    "ALL",
    "ALTER",
    "AND",
    "ANY",
    "AS",
    "BEGIN",
    "BETWEEN",
    "BY",
    "CHECK",
    "COLUMN",
    "CONNECT",
    "CREATE",
    "CURRENT",
    "DELETE",
    "DISTINCT",
    "DROP",
    "ELSE",
    "EXISTS",
    "FOLLOWING",
    "FOR",
    "FROM",
    "GRANT",
    "GROUP",
    "HAVING",
    "ILIKE",
    "IN",
    "INCREMENT",
    "INSERT",
    "INTERSECT",
    "INTO",
    "IS",
    "LIKE",
    "MINUS",
    "NOT",
    "NULL",
    "OF",
    "ON",
    "OR",
    "ORDER",
    "QUALIFY",
    "REGEXP",
    "REVOKE",
    "RLIKE",
    "ROW",
    "ROWS",
    "SAMPLE",
    "SELECT",
    "SET",
    "SOME",
    "START",
    "TABLE",
    "TABLESAMPLE",
    "THEN",
    "TO",
    "TRIGGER",
    "UNION",
    "UNIQUE",
    "UPDATE",
    "VALUES",
    "WHENEVER",
    "WHERE",
    "WITH",
];

/// Words that keep `BEGIN` a transaction statement rather than a block.
const TRANSACTION_CONTINUATIONS: [&str; 10] = [
    "TRANSACTION",
    "TRAN",
    "WORK",
    "DEFERRED",
    "IMMEDIATE",
    "EXCLUSIVE",
    "READ",
    "ISOLATION",
    "TRY",
    "CATCH",
];

impl Parser {
    /// Whether this `BEGIN` opens a block: not followed by a semicolon, the end or a transaction form.
    pub(crate) fn is_scripting_begin(&self) -> bool {
        let Some(next) = self.peek_nth(1) else {
            return false;
        };
        if next.token_type == TokenType::Semicolon {
            return false;
        }
        let unquoted = !Self::text_match_excludes_token(next.token_type);
        let continues_transaction = unquoted
            && TRANSACTION_CONTINUATIONS
                .iter()
                .any(|word| next.text.eq_ignore_ascii_case(word));
        let names_transaction = unquoted
            && next.text.eq_ignore_ascii_case("NAME")
            && self.peek_nth(2).is_some_and(|name| {
                name.token_type != TokenType::ColonEq
                    && self
                        .peek_nth(3)
                        .is_none_or(|end| end.token_type == TokenType::Semicolon)
            });
        !continues_transaction && !names_transaction
    }

    /// A complete block starting at `DECLARE` or `BEGIN`.
    pub(crate) fn parse_scripting_block(&mut self) -> Result<Expression> {
        self.with_parser_depth(Self::parse_scripting_block_members)
            .map(|block| Expression::ScriptingBlock(Box::new(block)))
    }

    fn parse_scripting_block_members(&mut self) -> Result<ScriptingBlock> {
        let start = self.current;
        let mut declarations: Vec<ScriptingDeclaration> = Vec::new();
        if self.match_token(TokenType::Declare) {
            while !self.check(TokenType::Begin) {
                declarations.push(self.parse_scripting_declaration(false)?);
                self.expect_token(TokenType::Semicolon)?;
            }
            if declarations.is_empty() {
                return Err(self.parse_error("Expected a declaration after DECLARE"));
            }
        }
        self.expect_token(TokenType::Begin)?;
        let body = self.parse_scripting_body()?;
        let mut exception_handlers: Vec<ScriptingExceptionHandler> = Vec::new();
        if self.match_text_seq(&["EXCEPTION"]) {
            exception_handlers = self.parse_scripting_handlers()?;
        }
        self.expect_scripting_words(&["END"])?;
        let label = self.parse_scripting_label()?;
        Ok(ScriptingBlock {
            span: self.source_range(start),
            declarations,
            body,
            exception_handlers,
            label,
        })
    }

    /// One DECLARE member, or with `is_let` the documented LET forms.
    fn parse_scripting_declaration(&mut self, is_let: bool) -> Result<ScriptingDeclaration> {
        let start = self.current;
        let name = self.expect_scripting_name()?;
        if self.match_text_seq(&["CURSOR", "FOR"]) {
            let source = match self.peek_nth(1) {
                Some(end)
                    if self.is_identifier_token() && end.token_type == TokenType::Semicolon =>
                {
                    ScriptingCursorSource::Resultset(self.expect_identifier_with_quoted()?)
                }
                _ => ScriptingCursorSource::Query(self.parse_scripting_sql_statement()?),
            };
            return Ok(ScriptingDeclaration::Cursor(ScriptingCursor {
                span: self.source_range(start),
                name,
                source,
            }));
        }
        if self.match_text_seq(&["RESULTSET"]) {
            let default = match self.parse_scripting_operator() {
                Some(operator) => {
                    let asynchronous = self.match_text_seq(&["ASYNC"]);
                    self.expect_token(TokenType::LParen)?;
                    let statement = self.parse_scripting_sql_statement()?;
                    self.expect_token(TokenType::RParen)?;
                    Some(ScriptingResultsetDefault {
                        operator,
                        asynchronous,
                        statement,
                    })
                }
                None => None,
            };
            return Ok(ScriptingDeclaration::Resultset(ScriptingResultset {
                span: self.source_range(start),
                name,
                default,
            }));
        }
        if !is_let && self.match_text_seq(&["EXCEPTION"]) {
            return self.parse_scripting_exception(start, name);
        }
        if !is_let && self.match_text_seq(&["PROCEDURE"]) {
            let parameters = self.parse_anonymous_procedure_parameters()?;
            let (return_type, returns_table) = self.parse_procedure_returns()?;
            self.expect_token(TokenType::As)?;
            let body = self.with_parser_depth(Self::parse_scripting_block_members)?;
            return Ok(ScriptingDeclaration::Procedure(Box::new(
                ScriptingProcedure {
                    span: self.source_range(start),
                    name,
                    parameters,
                    return_type,
                    returns_table,
                    body,
                },
            )));
        }
        if self.check_text_seq(&["PROCEDURE"]) {
            return Err(self.parse_error("Expected DECLARE before a nested procedure"));
        }
        let data_type = if self.check(TokenType::ColonEq)
            || self.check_text_seq(&["DEFAULT"])
            || self.check(TokenType::Semicolon)
        {
            None
        } else {
            Some(self.parse_data_type()?)
        };
        let default = match self.parse_scripting_operator() {
            Some(operator) => Some(ScriptingDefault {
                operator,
                value: self.parse_scripting_value()?,
            }),
            None if data_type.is_none() && is_let => {
                return Err(self.parse_error("Expected a type or a value for the variable"));
            }
            None => None,
        };
        Ok(ScriptingDeclaration::Variable(ScriptingVariable {
            span: self.source_range(start),
            name,
            data_type,
            default,
        }))
    }

    fn parse_scripting_exception(
        &mut self,
        start: usize,
        name: Identifier,
    ) -> Result<ScriptingDeclaration> {
        let (mut number, mut message) = (None, None);
        if self.match_token(TokenType::LParen) {
            let value = self.parse_expression()?;
            let integer = |expression: &Expression| {
                matches!(expression, Expression::Literal(literal)
                    if matches!(**literal, Literal::Number(ref text)
                        if text.bytes().all(|b| b.is_ascii_digit())))
            };
            let documented = match &value {
                Expression::Neg(negated) => integer(&negated.this),
                other => integer(other),
            };
            if !documented {
                return Err(self.parse_error("Expected an integer exception number"));
            }
            self.expect_token(TokenType::Comma)?;
            if !self.check(TokenType::String) {
                return Err(self.parse_error("Expected a quoted exception message"));
            }
            message = Some(self.parse_expression()?);
            number = Some(value);
            self.expect_token(TokenType::RParen)?;
        }
        Ok(ScriptingDeclaration::Exception(ScriptingException {
            span: self.source_range(start),
            name,
            number,
            message,
        }))
    }

    fn parse_scripting_operator(&mut self) -> Option<ScriptingAssignmentOperator> {
        if self.match_token(TokenType::ColonEq) {
            Some(ScriptingAssignmentOperator::ColonEquals)
        } else if self.match_text_seq(&["DEFAULT"]) {
            Some(ScriptingAssignmentOperator::Default)
        } else {
            None
        }
    }

    /// A scalar value; an ASYNC child job is only structured for RESULTSET declarations.
    fn parse_scripting_value(&mut self) -> Result<Expression> {
        if self.check_text_seq(&["ASYNC"]) && self.check_next(TokenType::LParen) {
            return Err(self
                .parse_error("SCRIPTING_PENDING: ASYNC is parsed only in RESULTSET declarations"));
        }
        self.parse_expression()
    }

    /// An ordinary statement member, spanned like a top-level one; never empty or bare.
    fn parse_scripting_sql_statement(&mut self) -> Result<Expression> {
        if self.is_at_end() || self.check(TokenType::Semicolon) {
            return Err(self.parse_error("Expected a statement"));
        }
        let start = self.current;
        let statement = self.parse_statement()?;
        if !statement.is_statement() {
            return Err(self.parse_error("Expected a statement, not a bare expression"));
        }
        Ok(self.spanned(start, statement))
    }

    /// One or more `statement;` members, stopping before the enclosing construct's keyword.
    fn parse_scripting_body(&mut self) -> Result<Vec<ScriptingStatement>> {
        let mut body: Vec<ScriptingStatement> = Vec::new();
        while !self.is_at_end() && !self.at_scripting_terminator() {
            body.push(self.with_parser_depth(Self::parse_scripting_statement)?);
            self.expect_token(TokenType::Semicolon)?;
        }
        if body.is_empty() {
            return Err(self.parse_error("Expected at least one statement"));
        }
        Ok(body)
    }

    fn at_scripting_terminator(&self) -> bool {
        BODY_TERMINATORS
            .iter()
            .any(|word| self.check_text_seq(&[word]))
    }

    fn parse_scripting_statement(&mut self) -> Result<ScriptingStatement> {
        let start = self.current;
        if self.check(TokenType::Declare)
            || (self.check(TokenType::Begin) && self.is_scripting_begin())
        {
            return Ok(ScriptingStatement::Block(
                self.parse_scripting_block_members()?,
            ));
        }
        if self.check(TokenType::ColonEq) {
            return Err(self.parse_error("Expected a variable before :="));
        }
        if self.is_identifier_or_keyword_token() && self.check_next(TokenType::ColonEq) {
            let target = self.expect_identifier_or_keyword_with_quoted()?;
            self.skip();
            let value = self.parse_scripting_value()?;
            return Ok(ScriptingStatement::Assignment(ScriptingAssignment {
                span: self.source_range(start),
                target,
                value,
            }));
        }
        let keyword = if self.current_token_allows_text_match() {
            self.peek_text().to_ascii_uppercase()
        } else {
            String::new()
        };
        match keyword.as_str() {
            "LET" => {
                self.skip();
                let declaration = self.parse_scripting_declaration(true)?;
                Ok(ScriptingStatement::Let(Box::new(ScriptingLet {
                    span: self.source_range(start),
                    declaration,
                })))
            }
            "IF" => self.parse_scripting_if(start),
            "CASE" => self.parse_scripting_case(start),
            "FOR" => self.parse_scripting_for(start),
            "WHILE" => self.parse_scripting_while(start),
            "REPEAT" => self.parse_scripting_repeat(start),
            "LOOP" => self.parse_scripting_loop(start),
            "BREAK" | "EXIT" | "CONTINUE" | "ITERATE" => self.parse_scripting_jump(start, &keyword),
            "OPEN" => self.parse_scripting_open(start),
            "FETCH" => self.parse_scripting_fetch(start),
            "CLOSE" => self
                .parse_scripting_named(start)
                .map(ScriptingStatement::Close),
            "CANCEL" => self
                .parse_scripting_named(start)
                .map(ScriptingStatement::Cancel),
            "AWAIT" => self.parse_scripting_await(start),
            "NULL" if self.check_next(TokenType::Semicolon) => {
                self.skip();
                Ok(ScriptingStatement::Null(ScriptingNull {
                    span: self.source_range(start),
                }))
            }
            "RAISE" => self.parse_scripting_raise(start),
            "RETURN" => self.parse_scripting_return(start),
            _ => {
                let statement = self.parse_scripting_sql_statement()?;
                Ok(ScriptingStatement::Sql(ScriptingSql {
                    span: self.source_range(start),
                    statement,
                }))
            }
        }
    }

    fn parse_scripting_loop(&mut self, start: usize) -> Result<ScriptingStatement> {
        self.skip();
        let body = self.parse_scripting_body()?;
        self.expect_scripting_words(&["END", "LOOP"])?;
        let label = self.parse_scripting_label()?;
        Ok(ScriptingStatement::Loop(ScriptingLoop {
            span: self.source_range(start),
            body,
            label,
        }))
    }

    /// BREAK, EXIT, CONTINUE or ITERATE, with its optional label.
    fn parse_scripting_jump(&mut self, start: usize, keyword: &str) -> Result<ScriptingStatement> {
        self.skip();
        let keyword = match keyword {
            "BREAK" => ScriptingJumpKeyword::Break,
            "EXIT" => ScriptingJumpKeyword::Exit,
            "CONTINUE" => ScriptingJumpKeyword::Continue,
            _ => ScriptingJumpKeyword::Iterate,
        };
        let label = self.parse_scripting_label()?;
        let jump = ScriptingJump {
            span: self.source_range(start),
            keyword,
            label,
        };
        Ok(match keyword {
            ScriptingJumpKeyword::Break | ScriptingJumpKeyword::Exit => {
                ScriptingStatement::Break(jump)
            }
            _ => ScriptingStatement::Continue(jump),
        })
    }

    fn parse_scripting_open(&mut self, start: usize) -> Result<ScriptingStatement> {
        self.skip();
        let cursor = self.expect_scripting_name()?;
        let using = if self.match_token(TokenType::Using) {
            self.expect_token(TokenType::LParen)?;
            let binds = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            Some(binds)
        } else {
            None
        };
        Ok(ScriptingStatement::Open(ScriptingOpen {
            span: self.source_range(start),
            cursor,
            using,
        }))
    }

    fn parse_scripting_fetch(&mut self, start: usize) -> Result<ScriptingStatement> {
        self.skip();
        let cursor = self.expect_scripting_name()?;
        self.expect_token(TokenType::Into)?;
        let mut targets = vec![self.expect_identifier_or_keyword_with_quoted()?];
        self.reject_reserved_name_part(&targets[0])?;
        while self.match_token(TokenType::Comma) {
            targets.push(self.expect_identifier_or_keyword_with_quoted()?);
            self.reject_reserved_name_part(&targets[targets.len() - 1])?;
        }
        Ok(ScriptingStatement::Fetch(ScriptingFetch {
            span: self.source_range(start),
            cursor,
            targets,
        }))
    }

    /// The cursor or result set name after CLOSE or CANCEL.
    fn parse_scripting_named(&mut self, start: usize) -> Result<ScriptingName> {
        self.skip();
        Ok(ScriptingName {
            name: self.expect_scripting_name()?,
            span: self.source_range(start),
        })
    }

    fn parse_scripting_await(&mut self, start: usize) -> Result<ScriptingStatement> {
        self.skip();
        let target = if self.match_text_seq(&["ALL"]) {
            ScriptingAwaitTarget::All
        } else {
            ScriptingAwaitTarget::Resultset(self.expect_scripting_name()?)
        };
        Ok(ScriptingStatement::Await(ScriptingAwait {
            span: self.source_range(start),
            target,
        }))
    }

    fn parse_scripting_raise(&mut self, start: usize) -> Result<ScriptingStatement> {
        self.skip();
        let exception = if self.check(TokenType::Semicolon) {
            None
        } else {
            Some(self.expect_scripting_name()?)
        };
        Ok(ScriptingStatement::Raise(ScriptingRaise {
            span: self.source_range(start),
            exception,
        }))
    }

    fn parse_scripting_return(&mut self, start: usize) -> Result<ScriptingStatement> {
        self.skip();
        let value = if self.check(TokenType::Semicolon) {
            None
        } else if self.check_text_seq(&["TABLE"]) && self.check_next(TokenType::LParen) {
            self.skip();
            self.skip();
            let rows = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            Some(ScriptingReturnValue::Table(rows))
        } else {
            Some(ScriptingReturnValue::Scalar(self.parse_scripting_value()?))
        };
        Ok(ScriptingStatement::Return(ScriptingReturn {
            span: self.source_range(start),
            value,
        }))
    }

    /// The documented parenthesized condition of IF, ELSEIF, WHILE and UNTIL.
    fn parse_scripting_condition(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let condition = self.parse_expression()?;
        self.expect_token(TokenType::RParen)?;
        Ok(condition)
    }

    fn parse_scripting_branch(
        &mut self,
        start: usize,
        condition: Expression,
    ) -> Result<ScriptingBranch> {
        self.expect_scripting_words(&["THEN"])?;
        let body = self.parse_scripting_body()?;
        Ok(ScriptingBranch {
            span: self.source_range(start),
            condition,
            body,
        })
    }

    fn parse_scripting_else(&mut self) -> Result<Option<ScriptingElse>> {
        let start = self.current;
        if !self.match_text_seq(&["ELSE"]) {
            return Ok(None);
        }
        let body = self.parse_scripting_body()?;
        Ok(Some(ScriptingElse {
            span: self.source_range(start),
            body,
        }))
    }

    fn parse_scripting_if(&mut self, start: usize) -> Result<ScriptingStatement> {
        let mut branches: Vec<ScriptingBranch> = Vec::new();
        loop {
            let branch_start = self.current;
            self.skip();
            let condition = self.parse_scripting_condition()?;
            branches.push(self.parse_scripting_branch(branch_start, condition)?);
            if !self.check_text_seq(&["ELSEIF"]) {
                break;
            }
        }
        let else_branch = self.parse_scripting_else()?;
        self.expect_scripting_words(&["END", "IF"])?;
        Ok(ScriptingStatement::If(ScriptingIf {
            span: self.source_range(start),
            branches,
            else_branch,
        }))
    }

    fn parse_scripting_case(&mut self, start: usize) -> Result<ScriptingStatement> {
        self.skip();
        let operand = if self.check(TokenType::LParen) {
            let open = self.current;
            let condition = self.parse_scripting_condition()?;
            if self.check_text_seq(&["WHEN"]) {
                Some(condition)
            } else {
                self.current = open;
                Some(self.parse_expression()?)
            }
        } else if self.check_text_seq(&["WHEN"]) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        let mut branches: Vec<ScriptingBranch> = Vec::new();
        while self.check_text_seq(&["WHEN"]) {
            let branch_start = self.current;
            self.skip();
            let condition = self.parse_expression()?;
            branches.push(self.parse_scripting_branch(branch_start, condition)?);
        }
        if branches.is_empty() {
            return Err(self.parse_error("Expected WHEN in CASE statement"));
        }
        let else_branch = self.parse_scripting_else()?;
        self.expect_scripting_words(&["END"])?;
        let end_case = self.match_text_seq(&["CASE"]);
        Ok(ScriptingStatement::Case(ScriptingCase {
            span: self.source_range(start),
            operand,
            branches,
            else_branch,
            end_case,
        }))
    }

    fn parse_scripting_loop_keyword(&mut self) -> Result<ScriptingLoopKeyword> {
        if self.match_text_seq(&["DO"]) {
            Ok(ScriptingLoopKeyword::Do)
        } else if self.match_text_seq(&["LOOP"]) {
            Ok(ScriptingLoopKeyword::Loop)
        } else {
            Err(self.parse_error("Expected DO or LOOP"))
        }
    }

    /// `END FOR`/`END WHILE` after DO and `END LOOP` after LOOP, as documented.
    fn expect_scripting_loop_end(
        &mut self,
        keyword: ScriptingLoopKeyword,
        do_end: &str,
    ) -> Result<Option<Identifier>> {
        let end = match keyword {
            ScriptingLoopKeyword::Do => do_end,
            ScriptingLoopKeyword::Loop => "LOOP",
        };
        self.expect_scripting_words(&["END", end])?;
        self.parse_scripting_label()
    }

    fn parse_scripting_for(&mut self, start: usize) -> Result<ScriptingStatement> {
        self.skip();
        let variable = self.expect_scripting_name()?;
        self.expect_token(TokenType::In)?;
        let rows = self.is_identifier_token()
            && self.peek_nth(1).is_some_and(|next| {
                next.text.eq_ignore_ascii_case("DO") || next.text.eq_ignore_ascii_case("LOOP")
            });
        let iteration = if rows && !self.check_text_seq(&["REVERSE"]) {
            ScriptingForIteration::Rows(self.expect_identifier_with_quoted()?)
        } else {
            let reverse = self.match_text_seq(&["REVERSE"]);
            let range_start = self.parse_expression()?;
            self.expect_scripting_words(&["TO"])?;
            ScriptingForIteration::Range(Box::new(ScriptingRange {
                reverse,
                start: range_start,
                end: self.parse_expression()?,
            }))
        };
        let keyword = self.parse_scripting_loop_keyword()?;
        let body = self.parse_scripting_body()?;
        let label = self.expect_scripting_loop_end(keyword, "FOR")?;
        Ok(ScriptingStatement::For(ScriptingFor {
            span: self.source_range(start),
            variable,
            iteration,
            keyword,
            body,
            label,
        }))
    }

    fn parse_scripting_while(&mut self, start: usize) -> Result<ScriptingStatement> {
        self.skip();
        let condition = self.parse_scripting_condition()?;
        let keyword = self.parse_scripting_loop_keyword()?;
        let body = self.parse_scripting_body()?;
        let label = self.expect_scripting_loop_end(keyword, "WHILE")?;
        Ok(ScriptingStatement::While(ScriptingWhile {
            span: self.source_range(start),
            condition,
            keyword,
            body,
            label,
        }))
    }

    fn parse_scripting_repeat(&mut self, start: usize) -> Result<ScriptingStatement> {
        self.skip();
        let body = self.parse_scripting_body()?;
        self.expect_scripting_words(&["UNTIL"])?;
        let condition = self.parse_scripting_condition()?;
        self.expect_scripting_words(&["END", "REPEAT"])?;
        let label = self.parse_scripting_label()?;
        Ok(ScriptingStatement::Repeat(ScriptingRepeat {
            span: self.source_range(start),
            body,
            condition,
            label,
        }))
    }

    /// Ordered WHEN clauses; OTHER may appear once, last.
    fn parse_scripting_handlers(&mut self) -> Result<Vec<ScriptingExceptionHandler>> {
        let mut handlers: Vec<ScriptingExceptionHandler> = Vec::new();
        while self.check_text_seq(&["WHEN"]) {
            if handlers
                .last()
                .is_some_and(|handler| handler.exceptions == ScriptingHandledExceptions::Other)
            {
                return Err(self.parse_error("WHEN OTHER must be the last exception handler"));
            }
            let start = self.current;
            self.skip();
            let exceptions = if self.match_text_seq(&["OTHER"]) {
                ScriptingHandledExceptions::Other
            } else {
                let mut names = vec![self.expect_scripting_name()?];
                while self.match_token(TokenType::Or) {
                    names.push(self.expect_scripting_name()?);
                }
                ScriptingHandledExceptions::Named(names)
            };
            let action = if self.match_text_seq(&["EXIT"]) {
                Some(ScriptingHandlerAction::Exit)
            } else if self.match_text_seq(&["CONTINUE"]) {
                Some(ScriptingHandlerAction::Continue)
            } else {
                None
            };
            self.expect_scripting_words(&["THEN"])?;
            let body = self.parse_scripting_body()?;
            handlers.push(ScriptingExceptionHandler {
                span: self.source_range(start),
                exceptions,
                action,
                body,
            });
        }
        if handlers.is_empty() {
            return Err(self.parse_error("Expected WHEN after EXCEPTION"));
        }
        Ok(handlers)
    }

    /// An optional label, written directly before the statement's semicolon.
    fn parse_scripting_label(&mut self) -> Result<Option<Identifier>> {
        if self.is_at_end()
            || self.check(TokenType::Semicolon)
            || (self.check(TokenType::DArrow) && self.applies_snowflake_context_rules())
        {
            return Ok(None);
        }
        self.expect_scripting_name().map(Some)
    }

    /// A label, variable, cursor, result set or exception name; unquoted reserved words fail.
    fn expect_scripting_name(&mut self) -> Result<Identifier> {
        if self.current_token_allows_text_match()
            && RESERVED_NAMES
                .iter()
                .any(|word| self.peek_text().eq_ignore_ascii_case(word))
        {
            return Err(self.parse_error("Expected a name, not a reserved word"));
        }
        self.expect_identifier_or_keyword_with_quoted()
    }

    /// Unquoted keywords in order, so a quoted `"END"` never closes a construct.
    fn expect_scripting_words(&mut self, words: &[&str]) -> Result<()> {
        if self.match_text_seq(words) {
            Ok(())
        } else {
            Err(Error::parse(
                format!("Expected {}", words.join(" ")),
                if self.is_at_end() {
                    self.previous().span
                } else {
                    self.peek().span
                },
            ))
        }
    }
}
