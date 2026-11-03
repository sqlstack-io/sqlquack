//! Snowflake Scripting syntax: blocks, declarations and control flow, never execution.

use serde::Deserialize;
use serde::Serialize;

use crate::expressions::names::models::Identifier;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::tokens::models::SourceSpan;

/// `[DECLARE ...] BEGIN ... [EXCEPTION ...] END [label]`, with every member in written order.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingBlock {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub declarations: Vec<ScriptingDeclaration>,
    pub body: Vec<ScriptingStatement>,
    pub exception_handlers: Vec<ScriptingExceptionHandler>,
    pub label: Option<Identifier>,
}

/// One DECLARE or LET member; LET-only and DECLARE-only forms are separated by the parser.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingDeclaration {
    Variable(ScriptingVariable),
    Cursor(ScriptingCursor),
    Resultset(ScriptingResultset),
    Exception(ScriptingException),
}

/// `name [type] [{DEFAULT | :=} expression]`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingVariable {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    pub data_type: Option<DataType>,
    pub default: Option<ScriptingDefault>,
}

/// The written assignment spelling and its value.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingDefault {
    pub operator: ScriptingAssignmentOperator,
    pub value: Expression,
}

#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingAssignmentOperator {
    Default,
    ColonEquals,
}

/// `name CURSOR FOR {query | resultset}`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingCursor {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    pub source: ScriptingCursorSource,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingCursorSource {
    Query(Expression),
    Resultset(Identifier),
}

/// `name RESULTSET [{DEFAULT | :=} [ASYNC] (statement)]`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingResultset {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    pub default: Option<ScriptingResultsetDefault>,
}

/// The statement is a child job only when ASYNC is written; it is never run here.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingResultsetDefault {
    pub operator: ScriptingAssignmentOperator,
    pub asynchronous: bool,
    pub statement: Expression,
}

/// `name EXCEPTION [(number, 'message')]`; omitted values are not defaulted.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingException {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    pub number: Option<Expression>,
    pub message: Option<Expression>,
}

/// One body member, terminated by its own semicolon.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingStatement {
    Sql(ScriptingSql),
    Block(ScriptingBlock),
    Let(ScriptingLet),
    Assignment(ScriptingAssignment),
    If(ScriptingIf),
    Case(ScriptingCase),
    For(ScriptingFor),
    While(ScriptingWhile),
    Repeat(ScriptingRepeat),
    Loop(ScriptingLoop),
    Break(ScriptingJump),
    Continue(ScriptingJump),
    Open(ScriptingOpen),
    Fetch(ScriptingFetch),
    Close(ScriptingName),
    Await(ScriptingAwait),
    Cancel(ScriptingName),
    Null(ScriptingNull),
    Raise(ScriptingRaise),
    Return(ScriptingReturn),
}

/// An ordinary SQL statement inside a block.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingSql {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub statement: Expression,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingLet {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub declaration: ScriptingDeclaration,
}

/// `name := expression`, without LET.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingAssignment {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub target: Identifier,
    pub value: Expression,
}

/// `IF (c) THEN ... [ELSEIF (c) THEN ...] [ELSE ...] END IF`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingIf {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub branches: Vec<ScriptingBranch>,
    pub else_branch: Option<ScriptingElse>,
}

/// A condition or WHEN value and the statements it guards.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingBranch {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub condition: Expression,
    pub body: Vec<ScriptingStatement>,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingElse {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub body: Vec<ScriptingStatement>,
}

/// Simple (with operand) or searched CASE statement; `END CASE` versus bare `END` is kept.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingCase {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub operand: Option<Expression>,
    pub branches: Vec<ScriptingBranch>,
    pub else_branch: Option<ScriptingElse>,
    pub end_case: bool,
}

/// `FOR name IN {source | [REVERSE] start TO end} {DO | LOOP} ... END {FOR | LOOP} [label]`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingFor {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub variable: Identifier,
    pub iteration: ScriptingForIteration,
    pub keyword: ScriptingLoopKeyword,
    pub body: Vec<ScriptingStatement>,
    pub label: Option<Identifier>,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingForIteration {
    /// A cursor or RESULTSET name; which one is not resolved here.
    Rows(Identifier),
    Range(ScriptingRange),
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingRange {
    pub reverse: bool,
    pub start: Expression,
    pub end: Expression,
}

/// The body keyword, which fixes the closing `END FOR`/`END WHILE` or `END LOOP`.
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingLoopKeyword {
    Do,
    Loop,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingWhile {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub condition: Expression,
    pub keyword: ScriptingLoopKeyword,
    pub body: Vec<ScriptingStatement>,
    pub label: Option<Identifier>,
}

/// `REPEAT ... UNTIL (c) END REPEAT [label]`: the condition follows the body.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingRepeat {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub body: Vec<ScriptingStatement>,
    pub condition: Expression,
    pub label: Option<Identifier>,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingLoop {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub body: Vec<ScriptingStatement>,
    pub label: Option<Identifier>,
}

/// BREAK/EXIT or CONTINUE/ITERATE, keeping the synonym written.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingJump {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub keyword: ScriptingJumpKeyword,
    pub label: Option<Identifier>,
}

#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingJumpKeyword {
    Break,
    Exit,
    Continue,
    Iterate,
}

/// `OPEN cursor [USING (bind, ...)]`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingOpen {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub cursor: Identifier,
    pub using: Option<Vec<Expression>>,
}

/// `FETCH cursor INTO variable, ...`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingFetch {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub cursor: Identifier,
    pub targets: Vec<Identifier>,
}

/// A statement whose only operand is a cursor or RESULTSET name: CLOSE and CANCEL.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingName {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingAwait {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub target: ScriptingAwaitTarget,
}

/// Unquoted `ALL`; a quoted `"ALL"` names a RESULTSET.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingAwaitTarget {
    All,
    Resultset(Identifier),
}

/// The no-op `NULL;` statement, distinct from a NULL value.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingNull {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// `RAISE [exception]`; without a name it re-raises inside a handler.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingRaise {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub exception: Option<Identifier>,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingReturn {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub value: ScriptingReturnValue,
}

/// A scalar expression, or `TABLE(...)` returning the rows its argument names.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingReturnValue {
    Scalar(Expression),
    Table(Expression),
}

/// `WHEN {name [OR name ...] | OTHER} [EXIT | CONTINUE] THEN ...`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingExceptionHandler {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub exceptions: ScriptingHandledExceptions,
    /// Absent when not written; EXIT is Snowflake's default, never filled in here.
    pub action: Option<ScriptingHandlerAction>,
    pub body: Vec<ScriptingStatement>,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingHandledExceptions {
    Named(Vec<Identifier>),
    Other,
}

#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingHandlerAction {
    Exit,
    Continue,
}
