//! Compound statements (`compoundBody`), read as the shared scripting tree and kept as the form `compound`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::scripting::main::scripting_sql::scripting_sql;
use sqlquack_syntax::expressions::scripting::models::{
    ScriptingBlock, ScriptingBranch, ScriptingCase, ScriptingCondition, ScriptingConditionValue,
    ScriptingCursor, ScriptingCursorSource, ScriptingDeclaration, ScriptingDiagnostics,
    ScriptingDiagnosticsItem, ScriptingElse, ScriptingFetch, ScriptingFor, ScriptingForIteration,
    ScriptingHandler, ScriptingHandlerAction, ScriptingIf, ScriptingJump, ScriptingJumpKeyword,
    ScriptingLoop, ScriptingLoopKeyword, ScriptingName, ScriptingOpen, ScriptingRepeat,
    ScriptingSignal, ScriptingSignalSet, ScriptingSqlstate, ScriptingStatement, ScriptingWhile,
};
use sqlquack_syntax::tokens::models::SourceSpan;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::forms::is_integer;
use crate::parse::_helpers::tree::nodes::{
    alias, joined, literal, null, qualifier_and_name, too_deep_to_keep,
};
use crate::parse::constants::RESIGNAL;

/// The words a labelled statement may start with.
const LABELLED: [&str; 5] = ["BEGIN", "LOOP", "WHILE", "REPEAT", "FOR"];

/// `condition_info_item`, what `GET DIAGNOSTICS CONDITION 1` reads.
const CONDITION_ITEMS: [&str; 5] = [
    "MESSAGE_TEXT",
    "RETURNED_SQLSTATE",
    "MESSAGE_ARGUMENTS",
    "CONDITION_IDENTIFIER",
    "LINE_NUMBER",
];

/// `statement_info_item`, what `GET DIAGNOSTICS` reads without `CONDITION`.
const STATEMENT_ITEMS: [&str; 2] = ["TRANSACTION_ACTIVE", "ROW_COUNT"];

/// The items `SIGNAL ... SET` assigns.
const SIGNAL_ITEMS: [&str; 2] = ["MESSAGE_ARGUMENTS", "MESSAGE_TEXT"];

/// Words after `SET` that keep their own statement inside a compound rather than naming a variable.
const SET_KEYWORDS: [&str; 9] = [
    "VAR",
    "VARIABLE",
    "TIME",
    "PATH",
    "CATALOG",
    "QUERY_TAGS",
    "ROLE",
    "RECIPIENT",
    "COLLATION",
];

/// Documented statements a compound also reads as scripting nodes, once their documented form accepts them.
const CHECKED_STATEMENTS: [&str; 5] = ["OPEN", "FETCH", "CLOSE", "SIGNAL", "RESIGNAL"];

impl Parser<'_> {
    /// `BEGIN [NOT ATOMIC] [compoundBody] END`, the form `compound`, or its gap for an IDENTIFIER-clause label or a tree too deep to keep.
    pub(crate) fn begin_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        let first: usize = self.at;
        let block: ScriptingBlock = self.compound_block(None, false)?;
        let end: usize = self.at;
        let block: Expression = Expression::ScriptingBlock(Box::new(block));
        if too_deep_to_keep(&block) {
            self.gap("compound", start);
            return Ok(null());
        }
        let clause: bool = (first..end).any(|token| {
            self.at = token;
            self.label_clause()
        });
        self.at = end;
        match clause {
            true => self.gap("compound", start),
            false => self.treed("compound", start),
        }
        Ok(block)
    }

    /// Whether `IDENTIFIER ( 'label' ) :` and a labelled statement's first word follow.
    fn label_clause(&self) -> bool {
        self.kw("IDENTIFIER")
            && self.char_at(1, '(')
            && self.string_at(2)
            && self.char_at(3, ')')
            && self.char_at(4, ':')
            && self.kw_in(5, &LABELLED)
    }

    /// `[label :] BEGIN [[NOT] ATOMIC] [compoundBody] END [label]`; a whole statement takes no end label.
    fn compound_block(
        &mut self,
        begin_label: Option<Identifier>,
        labelled: bool,
    ) -> Parsing<ScriptingBlock> {
        self.expect_kw("BEGIN")?;
        let atomic: Option<bool> = if self.kw("NOT") && self.kw_at(1, "ATOMIC") {
            self.at += 2;
            Some(false)
        } else if self.kw("ATOMIC") && !self.char_at(1, ':') {
            self.at += 1;
            Some(true)
        } else {
            None
        };
        let body: Vec<ScriptingStatement> = match self.kw("END") {
            true => Vec::new(),
            false => self.compound_body(&["END"])?,
        };
        self.expect_kw("END")?;
        let label: Option<Identifier> = match labelled {
            true => self.end_label()?,
            false => None,
        };
        Ok(ScriptingBlock {
            span: SourceSpan::default(),
            declarations: Vec::new(),
            body,
            exception_handlers: Vec::new(),
            label,
            begin_label,
            atomic,
        })
    }

    /// `endLabel`, when written.
    fn end_label(&mut self) -> Parsing<Option<Identifier>> {
        match self.ends_at(0) {
            true => Ok(None),
            false => self.ident().map(Some),
        }
    }

    /// `(compoundStatement ;)+`, up to one of `stops`.
    fn compound_body(&mut self, stops: &[&str]) -> Parsing<Vec<ScriptingStatement>> {
        let mut body: Vec<ScriptingStatement> = Vec::new();
        loop {
            body.push(self.compound_statement()?);
            self.expect_char(';')?;
            if self.peek().is_none() || self.kw_in(0, stops) {
                return Ok(body);
            }
        }
    }

    /// One `compoundStatement`; a label written as an `IDENTIFIER` clause is left out of the tree.
    fn compound_statement(&mut self) -> Parsing<ScriptingStatement> {
        let clause: bool = self.label_clause();
        let named: bool =
            self.is(is_identifier) && self.char_at(1, ':') && self.kw_in(2, &LABELLED);
        let label: Option<Identifier> = if clause {
            self.at += 5;
            None
        } else if named {
            let label: Identifier = self.name(is_identifier)?;
            self.at += 1;
            Some(label)
        } else {
            None
        };
        match self.word_at(0).as_deref() {
            Some("BEGIN") => Ok(ScriptingStatement::Block(self.compound_block(label, true)?)),
            Some(word @ ("LOOP" | "WHILE" | "REPEAT" | "FOR")) => self.loop_statement(word, label),
            _ if clause || named => Err(self.error()),
            Some("IF") => self.if_statement(),
            Some("CASE") => self.case_statement(),
            Some(word @ ("LEAVE" | "ITERATE")) => {
                let keyword: ScriptingJumpKeyword = match word {
                    "LEAVE" => ScriptingJumpKeyword::Leave,
                    _ => ScriptingJumpKeyword::Iterate,
                };
                self.at += 1;
                let jump: ScriptingJump = ScriptingJump {
                    span: SourceSpan::default(),
                    keyword,
                    label: Some(self.ident()?),
                };
                Ok(match keyword {
                    ScriptingJumpKeyword::Leave => ScriptingStatement::Break(jump),
                    _ => ScriptingStatement::Continue(jump),
                })
            }
            Some("GET") if self.kw_at(1, "DIAGNOSTICS") => self.diagnostics_statement(),
            Some("GET") => Err(self.error()),
            Some("DECLARE") if self.kw_in(1, &["CONTINUE", "EXIT"]) && self.kw_at(2, "HANDLER") => {
                self.handler_declaration()
            }
            Some("DECLARE") if self.is_at(1, is_identifier) && self.kw_at(2, "CONDITION") => {
                self.condition_declaration()
            }
            Some("DECLARE")
                if self.kw_at(2, "CURSOR")
                    || (self.kw_in(2, &["ASENSITIVE", "INSENSITIVE"])
                        && self.kw_at(3, "CURSOR")) =>
            {
                self.deeper(|parser| parser.cursor_declaration())
            }
            _ => self.script_statement(),
        }
    }

    /// A statement or, as `SET name = value` is inside a script, a variable assignment like `SET VAR`'s.
    fn script_statement(&mut self) -> Parsing<ScriptingStatement> {
        if !self.kw("SET") || self.kw_in(1, &SET_KEYWORDS) {
            return self.simple_statement();
        }
        let assignment = self.attempt(|parser| {
            parser.at += 1;
            let statement: Expression = parser.variable_assignments(None)?;
            match parser.ends_at(0) {
                true => Ok(statement),
                false => Err(parser.error()),
            }
        });
        match assignment {
            Ok(statement) => Ok(scripting_sql(statement)),
            Err(_) => self.simple_statement(),
        }
    }

    /// A statement that is no control statement: an ordinary statement, or a cursor or signal statement.
    fn simple_statement(&mut self) -> Parsing<ScriptingStatement> {
        match self.word_at(0) {
            Some(word) if CHECKED_STATEMENTS.contains(&word.as_str()) => {
                self.checked_statement(&word)
            }
            _ => Ok(scripting_sql(self.statement()?)),
        }
    }

    /// `word`'s statement, checked by its documented form, then read as its scripting node if that ends there too.
    fn checked_statement(&mut self, word: &str) -> Parsing<ScriptingStatement> {
        let first: usize = self.at;
        let gaps: usize = self.gaps.len();
        let statement: Expression = self.statement()?;
        let end: usize = self.at;
        if self.gaps.len() > gaps {
            return Ok(scripting_sql(statement));
        }
        self.at = first;
        let read: Parsing<ScriptingStatement> = match word {
            "OPEN" => self.open_cursor(),
            "FETCH" => self.fetch_cursor(),
            "CLOSE" => {
                self.at += 1;
                self.multipart_identifier().map(|parts| {
                    let (qualifier, name) = qualifier_and_name(parts);
                    ScriptingStatement::Close(ScriptingName {
                        span: SourceSpan::default(),
                        name,
                        qualifier,
                    })
                })
            }
            _ => self.signal_statement(word == RESIGNAL),
        };
        match read {
            Ok(node) if self.at == end => Ok(node),
            _ => {
                self.at = end;
                Ok(scripting_sql(statement))
            }
        }
    }

    /// `OPEN name [USING { ( item, ... ) | item, ... }]`, each item `expr [[AS] id]`; a failed parenthesized read falls back to the bare list.
    fn open_cursor(&mut self) -> Parsing<ScriptingStatement> {
        self.at += 1;
        let (cursor_qualifier, cursor) = qualifier_and_name(self.multipart_identifier()?);
        let mut using: Option<Vec<Expression>> = None;
        let mut using_bare: bool = false;
        if self.eat_kw("USING") {
            let at: usize = self.at;
            let parenthesized: Option<Vec<Expression>> = match self.eat_char('(') {
                true => match self.comma_list(|parser| parser.using_item()) {
                    Ok(items) if self.eat_char(')') && self.ends_at(0) => Some(items),
                    Ok(_) | Err(_) => None,
                },
                false => None,
            };
            using = Some(match parenthesized {
                Some(items) => items,
                None => {
                    self.at = at;
                    using_bare = true;
                    self.comma_list(|parser| parser.using_item())?
                }
            });
        }
        Ok(ScriptingStatement::Open(ScriptingOpen {
            span: SourceSpan::default(),
            cursor,
            cursor_qualifier,
            using,
            using_bare,
        }))
    }

    /// `expr [[AS] id]`: a word after the value names it, `AS` itself included.
    pub(crate) fn using_item(&mut self) -> Parsing<Expression> {
        let value: Expression = self.expression()?;
        if self.kw("AS") && self.is_at(1, is_identifier) {
            self.at += 1;
            return Ok(alias(value, self.ident()?, true));
        }
        match self.is(is_identifier) {
            true => Ok(alias(value, self.ident()?, false)),
            false => Ok(value),
        }
    }

    /// `FETCH [[NEXT] FROM] name INTO name, ...`.
    fn fetch_cursor(&mut self) -> Parsing<ScriptingStatement> {
        self.at += 1;
        let keyword: Option<String> = if self.kw("NEXT") && self.kw_at(1, "FROM") {
            self.at += 2;
            Some("NEXT FROM".to_owned())
        } else if self.kw("FROM") && !self.kw_at(1, "INTO") {
            self.at += 1;
            Some("FROM".to_owned())
        } else {
            None
        };
        let (cursor_qualifier, cursor) = qualifier_and_name(self.multipart_identifier()?);
        self.expect_kw("INTO")?;
        let targets: Vec<Identifier> =
            self.comma_list(|parser| parser.multipart_identifier().map(joined))?;
        Ok(ScriptingStatement::Fetch(ScriptingFetch {
            span: SourceSpan::default(),
            cursor,
            cursor_qualifier,
            targets,
            keyword,
        }))
    }

    /// `SIGNAL { SQLSTATE [VALUE] 'state' | name } [SET item = value]`; RESIGNAL's condition may be left out.
    fn signal_statement(&mut self, resignal: bool) -> Parsing<ScriptingStatement> {
        self.at += 1;
        let set_follows: bool =
            self.kw("SET") && self.kw_in(1, &SIGNAL_ITEMS) && self.char_at(2, '=');
        let condition: Option<ScriptingConditionValue> =
            match resignal && (self.ends_at(0) || set_follows) {
                true => None,
                false => Some(match self.sqlstate_follows() {
                    true => ScriptingConditionValue::Sqlstate(self.sqlstate()?),
                    false => ScriptingConditionValue::Named(joined(self.multipart_identifier()?)),
                }),
            };
        let set: Option<ScriptingSignalSet> = match self.eat_kw("SET") {
            true => {
                let item: String = match self.word_at(0) {
                    Some(word) if SIGNAL_ITEMS.contains(&word.as_str()) => word,
                    _ => return Err(self.error()),
                };
                self.at += 1;
                self.expect_char('=')?;
                Some(ScriptingSignalSet {
                    item,
                    value: self.expression()?,
                })
            }
            false => None,
        };
        Ok(ScriptingStatement::Signal(ScriptingSignal {
            span: SourceSpan::default(),
            resignal,
            condition,
            set,
        }))
    }

    /// `DECLARE {CONTINUE | EXIT} HANDLER FOR condition, ... {block | statement}`.
    fn handler_declaration(&mut self) -> Parsing<ScriptingStatement> {
        let action: ScriptingHandlerAction = match self.kw_at(1, "EXIT") {
            true => ScriptingHandlerAction::Exit,
            false => ScriptingHandlerAction::Continue,
        };
        self.at += 3;
        self.expect_kw("FOR")?;
        let conditions: Vec<ScriptingConditionValue> =
            self.comma_list(|parser| parser.condition_value())?;
        let body: ScriptingStatement = match self.kw("BEGIN") {
            true => ScriptingStatement::Block(self.compound_block(None, true)?),
            false => self.script_statement()?,
        };
        Ok(ScriptingStatement::Declare(ScriptingDeclaration::Handler(
            Box::new(ScriptingHandler {
                span: SourceSpan::default(),
                action,
                conditions,
                body,
            }),
        )))
    }

    /// `DECLARE name CONDITION [FOR SQLSTATE [VALUE] 'state']`.
    fn condition_declaration(&mut self) -> Parsing<ScriptingStatement> {
        self.at += 1;
        let name: Identifier = self.name(is_identifier)?;
        self.at += 1;
        let sqlstate: Option<ScriptingSqlstate> = match self.eat_kw("FOR") {
            true => {
                self.expect_kw("SQLSTATE")?;
                let value_keyword: bool = self.eat_kw("VALUE");
                let (state, state_written) = self.written_string()?;
                Some(ScriptingSqlstate {
                    state_written,
                    state,
                    value_keyword,
                })
            }
            false => None,
        };
        Ok(ScriptingStatement::Declare(
            ScriptingDeclaration::Condition(ScriptingCondition {
                span: SourceSpan::default(),
                name,
                sqlstate,
            }),
        ))
    }

    /// `DECLARE name [ASENSITIVE] [INSENSITIVE] CURSOR FOR query [FOR READ ONLY]`: the form `compound_cursor`, or a gap with the optional words.
    fn cursor_declaration(&mut self) -> Parsing<ScriptingStatement> {
        let start: usize = self.offset();
        self.at += 1;
        let name: Identifier = self.ident()?;
        let asensitive: bool = self.eat_kw("ASENSITIVE");
        let insensitive: bool = self.eat_kw("INSENSITIVE");
        self.expect_kw("CURSOR")?;
        self.expect_kw("FOR")?;
        let query: Expression = self.query()?;
        let read_only: bool = self.eat_kws(&["FOR", "READ"]);
        if read_only {
            self.expect_kw("ONLY")?;
        }
        match asensitive || insensitive || read_only {
            true => self.gap("declare_cursor", start),
            false => self.treed("compound_cursor", start),
        }
        Ok(ScriptingStatement::Declare(ScriptingDeclaration::Cursor(
            ScriptingCursor {
                span: SourceSpan::default(),
                name,
                source: ScriptingCursorSource::Query(query),
            },
        )))
    }

    /// A `LOOP`, `WHILE`, `REPEAT` or (for any other `word`) `FOR` statement, from `word`.
    fn loop_statement(
        &mut self,
        word: &str,
        begin_label: Option<Identifier>,
    ) -> Parsing<ScriptingStatement> {
        self.at += 1;
        Ok(match word {
            "LOOP" => {
                let body: Vec<ScriptingStatement> = self.compound_body(&["END"])?;
                self.expect_kw("END")?;
                self.expect_kw("LOOP")?;
                ScriptingStatement::Loop(ScriptingLoop {
                    span: SourceSpan::default(),
                    body,
                    label: self.end_label()?,
                    begin_label,
                })
            }
            "WHILE" => {
                let condition: Expression = self.expression()?;
                self.expect_kw("DO")?;
                let body: Vec<ScriptingStatement> = self.compound_body(&["END"])?;
                self.expect_kw("END")?;
                self.expect_kw("WHILE")?;
                ScriptingStatement::While(ScriptingWhile {
                    span: SourceSpan::default(),
                    condition,
                    keyword: ScriptingLoopKeyword::Do,
                    body,
                    label: self.end_label()?,
                    begin_label,
                })
            }
            "REPEAT" => {
                let body: Vec<ScriptingStatement> = self.compound_body(&["UNTIL"])?;
                self.expect_kw("UNTIL")?;
                let condition: Expression = self.expression()?;
                self.expect_kw("END")?;
                self.expect_kw("REPEAT")?;
                ScriptingStatement::Repeat(ScriptingRepeat {
                    span: SourceSpan::default(),
                    body,
                    condition,
                    label: self.end_label()?,
                    begin_label,
                })
            }
            _ => self.for_statement(begin_label)?,
        })
    }

    /// `FOR [name AS] query DO body END FOR [label]`, after `FOR`.
    fn for_statement(&mut self, begin_label: Option<Identifier>) -> Parsing<ScriptingStatement> {
        let variable: Option<Identifier> = match self.is(is_identifier) && self.kw_at(1, "AS") {
            true => {
                let name: Identifier = self.name(is_identifier)?;
                self.at += 1;
                Some(name)
            }
            false => None,
        };
        let outer: bool = std::mem::replace(&mut self.for_query, true);
        let query: Parsing<Expression> = self.query();
        self.for_query = outer;
        let query: Expression = query?;
        self.expect_kw("DO")?;
        let body: Vec<ScriptingStatement> = self.compound_body(&["END"])?;
        self.expect_kw("END")?;
        self.expect_kw("FOR")?;
        Ok(ScriptingStatement::For(ScriptingFor {
            span: SourceSpan::default(),
            variable,
            iteration: ScriptingForIteration::Query(query),
            keyword: ScriptingLoopKeyword::Do,
            body,
            label: self.end_label()?,
            begin_label,
        }))
    }

    /// `GET DIAGNOSTICS [CONDITION 1] { variable = item } [, ...]`, each item from its closed list.
    fn diagnostics_statement(&mut self) -> Parsing<ScriptingStatement> {
        self.at += 2;
        let (condition, items): (Option<Expression>, &[&str]) = match self.eat_kw("CONDITION") {
            true => {
                let number: Expression =
                    literal(Literal::Number(self.expect_token(is_integer)?.value));
                (Some(number), &CONDITION_ITEMS)
            }
            false => (None, &STATEMENT_ITEMS),
        };
        let items: Vec<ScriptingDiagnosticsItem> = self.comma_list(|parser| {
            let target: Identifier = parser.ident()?;
            parser.expect_char('=')?;
            match parser.word_at(0) {
                Some(item) if items.contains(&item.as_str()) => {
                    parser.at += 1;
                    Ok(ScriptingDiagnosticsItem { target, item })
                }
                _ => Err(parser.error()),
            }
        })?;
        Ok(ScriptingStatement::Diagnostics(ScriptingDiagnostics {
            span: SourceSpan::default(),
            condition,
            items,
        }))
    }

    /// `IF condition THEN body [ELSEIF condition THEN body]... [ELSE body] END IF`.
    fn if_statement(&mut self) -> Parsing<ScriptingStatement> {
        self.at += 1;
        let mut branches: Vec<ScriptingBranch> = vec![self.branch(&["ELSEIF", "ELSE", "END"])?];
        while self.eat_kw("ELSEIF") {
            branches.push(self.branch(&["ELSEIF", "ELSE", "END"])?);
        }
        let else_branch: Option<ScriptingElse> = self.else_branch()?;
        self.expect_kw("END")?;
        self.expect_kw("IF")?;
        Ok(ScriptingStatement::If(ScriptingIf {
            span: SourceSpan::default(),
            branches,
            else_branch,
        }))
    }

    /// `CASE [value] WHEN condition THEN body... [ELSE body] END CASE`.
    fn case_statement(&mut self) -> Parsing<ScriptingStatement> {
        self.at += 1;
        let operand: Option<Expression> = match self.kw("WHEN") {
            true => None,
            false => Some(self.expression()?),
        };
        self.expect_kw("WHEN")?;
        let mut branches: Vec<ScriptingBranch> = vec![self.branch(&["WHEN", "ELSE", "END"])?];
        while self.eat_kw("WHEN") {
            branches.push(self.branch(&["WHEN", "ELSE", "END"])?);
        }
        let else_branch: Option<ScriptingElse> = self.else_branch()?;
        self.expect_kw("END")?;
        self.expect_kw("CASE")?;
        Ok(ScriptingStatement::Case(ScriptingCase {
            span: SourceSpan::default(),
            operand,
            branches,
            else_branch,
            end_case: true,
        }))
    }

    /// `condition THEN body`, the body up to one of `stops`.
    fn branch(&mut self, stops: &[&str]) -> Parsing<ScriptingBranch> {
        let condition: Expression = self.expression()?;
        self.expect_kw("THEN")?;
        Ok(ScriptingBranch {
            span: SourceSpan::default(),
            condition,
            body: self.compound_body(stops)?,
        })
    }

    /// `ELSE body`, when written.
    fn else_branch(&mut self) -> Parsing<Option<ScriptingElse>> {
        match self.eat_kw("ELSE") {
            true => Ok(Some(ScriptingElse {
                span: SourceSpan::default(),
                body: self.compound_body(&["END"])?,
            })),
            false => Ok(None),
        }
    }

    /// Whether `SQLSTATE [VALUE] 'state'` follows, rather than a condition named SQLSTATE.
    fn sqlstate_follows(&self) -> bool {
        self.kw("SQLSTATE") && (self.string_at(1) || self.kw_at(1, "VALUE"))
    }

    /// `SQLSTATE [VALUE] 'state'`.
    fn sqlstate(&mut self) -> Parsing<ScriptingSqlstate> {
        self.at += 1;
        let value_keyword: bool = self.eat_kw("VALUE");
        let (state, state_written) = self.written_string()?;
        Ok(ScriptingSqlstate {
            state_written,
            state,
            value_keyword,
        })
    }

    /// `conditionValue`: `SQLSTATE [VALUE] 'state'`, `SQLEXCEPTION`, `NOT FOUND` or a name.
    fn condition_value(&mut self) -> Parsing<ScriptingConditionValue> {
        if self.sqlstate_follows() {
            return Ok(ScriptingConditionValue::Sqlstate(self.sqlstate()?));
        }
        if self.kw("NOT") && self.kw_at(1, "FOUND") {
            self.at += 2;
            return Ok(ScriptingConditionValue::NotFound);
        }
        let name: Vec<Identifier> = self.multipart_identifier()?;
        Ok(match name.as_slice() {
            [word] if !word.quoted && word.name.eq_ignore_ascii_case("SQLEXCEPTION") => {
                ScriptingConditionValue::Sqlexception
            }
            _ => ScriptingConditionValue::Named(joined(name)),
        })
    }
}
