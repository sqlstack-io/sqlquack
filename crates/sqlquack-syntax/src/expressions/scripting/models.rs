//! Data models of Snowflake Scripting and Databricks SQL scripting blocks.

use crate::tokens::models::SourceSpan;
use serde::Deserialize;
use serde::Serialize;

use crate::expressions::names::models::Identifier;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::StructField;

/// `[DECLARE ...] BEGIN ... [EXCEPTION ...] END [label]`, with every member in written order.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingBlock {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub declarations: Vec<ScriptingDeclaration>,
    pub body: Vec<ScriptingStatement>,
    pub exception_handlers: Vec<ScriptingExceptionHandler>,
    pub label: Option<Identifier>,
    /// Databricks: the label written before the statement, `label :`; `label` is the one after its END.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub begin_label: Option<Identifier>,
    /// Databricks: `ATOMIC` (true) or `NOT ATOMIC` (false) after BEGIN, when written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atomic: Option<bool>,
}

/// One DECLARE or LET member; LET-only and DECLARE-only forms are separated by the parser.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingDeclaration {
    Variable(ScriptingVariable),
    Cursor(ScriptingCursor),
    Resultset(ScriptingResultset),
    Exception(ScriptingException),
    Procedure(Box<ScriptingProcedure>),
    /// Databricks: `DECLARE {CONTINUE | EXIT} HANDLER`, written in a block's body.
    Handler(Box<ScriptingHandler>),
    /// Databricks: `DECLARE name CONDITION`, written in a block's body.
    Condition(ScriptingCondition),
}

/// `name [type] [{DEFAULT | :=} expression]`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingVariable {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    pub data_type: Option<DataType>,
    pub default: Option<ScriptingDefault>,
}

/// The written assignment spelling and its value.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingDefault {
    pub operator: ScriptingAssignmentOperator,
    pub value: Expression,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingAssignmentOperator {
    Default,
    ColonEquals,
}

/// `name CURSOR FOR {query | resultset}`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingCursor {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    pub source: ScriptingCursorSource,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingCursorSource {
    Query(Expression),
    Resultset(Identifier),
}

/// `name RESULTSET [{DEFAULT | :=} [ASYNC] (statement)]`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingResultset {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    pub default: Option<ScriptingResultsetDefault>,
}

/// The statement is a child job only when ASYNC is written; it is never run here.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingResultsetDefault {
    pub operator: ScriptingAssignmentOperator,
    pub asynchronous: bool,
    pub statement: Expression,
}

/// `name EXCEPTION [(number, 'message')]`; omitted values are not defaulted.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingException {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    pub number: Option<Expression>,
    pub message: Option<Expression>,
}

/// `name PROCEDURE ( [<arg> <type>, ...] ) RETURNS { <type> | TABLE ( ... ) } AS <block>`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingProcedure {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    pub parameters: Vec<crate::expressions::schema_objects::models::FunctionParameter>,
    pub return_type: Option<DataType>,
    pub returns_table: Option<Vec<StructField>>,
    pub body: ScriptingBlock,
}

/// One body member, terminated by its own semicolon.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingStatement {
    Sql(ScriptingSql),
    Block(ScriptingBlock),
    Let(Box<ScriptingLet>),
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
    /// Databricks: a cursor, handler or condition declared among the body's statements.
    Declare(ScriptingDeclaration),
    /// Databricks: SIGNAL or RESIGNAL.
    Signal(ScriptingSignal),
    /// Databricks: GET DIAGNOSTICS.
    Diagnostics(ScriptingDiagnostics),
}

/// An ordinary SQL statement inside a block.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingSql {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub statement: Expression,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingLet {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub declaration: ScriptingDeclaration,
}

/// `name := expression`, without LET.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingAssignment {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub target: Identifier,
    pub value: Expression,
}

/// `IF (c) THEN ... [ELSEIF (c) THEN ...] [ELSE ...] END IF`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingIf {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub branches: Vec<ScriptingBranch>,
    pub else_branch: Option<ScriptingElse>,
}

/// A condition or WHEN value and the statements it guards.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingBranch {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub condition: Expression,
    pub body: Vec<ScriptingStatement>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingElse {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub body: Vec<ScriptingStatement>,
}

/// Simple (with operand) or searched CASE statement; `END CASE` versus bare `END` is kept.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingCase {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub operand: Option<Expression>,
    pub branches: Vec<ScriptingBranch>,
    pub else_branch: Option<ScriptingElse>,
    pub end_case: bool,
}

/// `FOR name IN {source | [REVERSE] start TO end} {DO | LOOP} ... END {FOR | LOOP} [label]`, or Databricks' `FOR [name AS] query DO`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingFor {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub variable: Option<Identifier>,
    pub iteration: ScriptingForIteration,
    pub keyword: ScriptingLoopKeyword,
    pub body: Vec<ScriptingStatement>,
    pub label: Option<Identifier>,
    /// Databricks: the label written before the statement, `label :`; `label` is the one after its END.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub begin_label: Option<Identifier>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingForIteration {
    /// A cursor or RESULTSET name; which one is not resolved here.
    Rows(Identifier),
    Range(Box<ScriptingRange>),
    /// Databricks: the rows of a query.
    Query(Expression),
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingRange {
    pub reverse: bool,
    pub start: Expression,
    pub end: Expression,
}

/// The body keyword, which fixes the closing `END FOR`/`END WHILE` or `END LOOP`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingLoopKeyword {
    Do,
    Loop,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingWhile {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub condition: Expression,
    pub keyword: ScriptingLoopKeyword,
    pub body: Vec<ScriptingStatement>,
    pub label: Option<Identifier>,
    /// Databricks: the label written before the statement, `label :`; `label` is the one after its END.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub begin_label: Option<Identifier>,
}

/// `REPEAT ... UNTIL (c) END REPEAT [label]`: the condition follows the body.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingRepeat {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub body: Vec<ScriptingStatement>,
    pub condition: Expression,
    pub label: Option<Identifier>,
    /// Databricks: the label written before the statement, `label :`; `label` is the one after its END.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub begin_label: Option<Identifier>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingLoop {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub body: Vec<ScriptingStatement>,
    pub label: Option<Identifier>,
    /// Databricks: the label written before the statement, `label :`; `label` is the one after its END.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub begin_label: Option<Identifier>,
}

/// BREAK/EXIT/LEAVE or CONTINUE/ITERATE, keeping the synonym written.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingJump {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub keyword: ScriptingJumpKeyword,
    pub label: Option<Identifier>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingJumpKeyword {
    Break,
    Exit,
    Continue,
    Iterate,
    /// Databricks' BREAK.
    Leave,
}

/// `OPEN cursor [USING (bind, ...)]`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingOpen {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub cursor: Identifier,
    /// Databricks: the parts written before the cursor's name, such as a label
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cursor_qualifier: Vec<Identifier>,
    pub using: Option<Vec<Expression>>,
    /// Databricks: the USING list written without parentheses.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub using_bare: bool,
}

/// `FETCH cursor INTO variable, ...`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingFetch {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub cursor: Identifier,
    /// Databricks: the parts written before the cursor's name, such as a label
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cursor_qualifier: Vec<Identifier>,
    pub targets: Vec<Identifier>,
    /// Databricks: `FROM` or `NEXT FROM` before the cursor, as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
}

/// A statement whose only operand is a cursor or RESULTSET name: CLOSE and CANCEL.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingName {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    /// Databricks: the parts written before the name, such as a label
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub qualifier: Vec<Identifier>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingAwait {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub target: ScriptingAwaitTarget,
}

/// Unquoted `ALL`; a quoted `"ALL"` names a RESULTSET.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingAwaitTarget {
    All,
    Resultset(Identifier),
}

/// The no-op `NULL;` statement, distinct from a NULL value.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingNull {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// `RAISE [exception]`; without a name it re-raises inside a handler.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingRaise {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub exception: Option<Identifier>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingReturn {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    /// Absent for a bare `RETURN;`, which returns NULL.
    pub value: Option<ScriptingReturnValue>,
}

/// A scalar expression, or `TABLE(...)` returning the rows its argument names.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingReturnValue {
    Scalar(Expression),
    Table(Expression),
}

/// `WHEN {name [OR name ...] | OTHER} [EXIT | CONTINUE] THEN ...`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingExceptionHandler {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub exceptions: ScriptingHandledExceptions,
    /// Absent when not written; EXIT is Snowflake's default, never filled in here.
    pub action: Option<ScriptingHandlerAction>,
    pub body: Vec<ScriptingStatement>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingHandledExceptions {
    Named(Vec<Identifier>),
    Other,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingHandlerAction {
    Exit,
    Continue,
}

/// `DECLARE {CONTINUE | EXIT} HANDLER FOR condition, ... body` (Databricks): a block or one statement.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingHandler {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub action: ScriptingHandlerAction,
    pub conditions: Vec<ScriptingConditionValue>,
    pub body: ScriptingStatement,
}

/// `DECLARE name CONDITION [FOR SQLSTATE [VALUE] 'state']` (Databricks).
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingCondition {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    pub name: Identifier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sqlstate: Option<ScriptingSqlstate>,
}

/// `SQLSTATE [VALUE] 'state'`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingSqlstate {
    pub state: String,
    /// VALUE written before the state.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub value_keyword: bool,
}

/// A condition a handler catches or a SIGNAL raises.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingConditionValue {
    Sqlstate(ScriptingSqlstate),
    Sqlexception,
    NotFound,
    /// A declared condition's name; a quoted `SQLEXCEPTION` is a name too.
    Named(Identifier),
}

/// `SIGNAL condition [SET item = value]`, or `RESIGNAL [condition] [SET item = value]` (Databricks).
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingSignal {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub resignal: bool,
    pub condition: Option<ScriptingConditionValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<ScriptingSignalSet>,
}

/// `SET {MESSAGE_ARGUMENTS | MESSAGE_TEXT} = value`, the item upper-cased.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingSignalSet {
    pub item: String,
    pub value: Expression,
}

/// `GET DIAGNOSTICS [CONDITION number] target = item, ...` (Databricks).
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingDiagnostics {
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<Expression>,
    pub items: Vec<ScriptingDiagnosticsItem>,
}

/// One `target = item`, the item upper-cased.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptingDiagnosticsItem {
    pub target: Identifier,
    pub item: String,
}
