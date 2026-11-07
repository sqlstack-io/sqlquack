//! `TransactionStmt`, `VariableSetStmt`, `VariableResetStmt`, `VariableShowStmt`, `ConstraintsSetStmt`, `LockStmt` and `DiscardStmt` (gram.y), as `UtilityStatement`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::statements::models::UtilityStatement;

use crate::parse::_helpers::commands::alters::name_from;
use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id};
use crate::parse::_helpers::tree::nodes::{table, tuple};

/// The kind `ABORT` and `ROLLBACK` give.
const ROLLBACK: &str = "ROLLBACK";

/// The kind `END` and `COMMIT` give.
const COMMIT: &str = "COMMIT";

/// The kind of `SET` and `RESET`, which both make a `VariableSetStmt`.
const SET: &str = "SET";

/// `lock_type` spellings, longest first, with the mode each names.
const LOCK_MODES: [(&[&str], &str); 8] = [
    (&["share", "update", "exclusive"], "SHARE UPDATE EXCLUSIVE"),
    (&["share", "row", "exclusive"], "SHARE ROW EXCLUSIVE"),
    (&["access", "share"], "ACCESS SHARE"),
    (&["access", "exclusive"], ACCESS_EXCLUSIVE),
    (&["row", "share"], "ROW SHARE"),
    (&["row", "exclusive"], "ROW EXCLUSIVE"),
    (&["share"], "SHARE"),
    (&["exclusive"], "EXCLUSIVE"),
];

/// The lock mode `LOCK` takes without `IN ... MODE`.
const ACCESS_EXCLUSIVE: &str = "ACCESS EXCLUSIVE";

/// `DISCARD` targets with the target each names; `TEMPORARY` is `TEMP`.
const DISCARD_TARGETS: [(&str, &str); 5] = [
    ("all", "ALL"),
    ("temp", "TEMP"),
    ("temporary", "TEMP"),
    ("plans", "PLANS"),
    ("sequences", "SEQUENCES"),
];

/// `VariableShowStmt`'s special forms with the setting each shows.
const SHOW_SPECIALS: [(&[&str], &str); 4] = [
    (&["time", "zone"], "timezone"),
    (
        &["transaction", "isolation", "level"],
        "transaction_isolation",
    ),
    (&["session", "authorization"], "session_authorization"),
    (&["all"], "all"),
];

/// The words after `SET SESSION` that make it `set_rest` itself rather than its scope.
const SESSION_REST: [&str; 2] = ["authorization", "characteristics"];

/// A `UtilityStatement` of `kind`.
pub(crate) fn utility(
    kind: &str,
    options: Vec<Expression>,
    operands: Vec<Expression>,
) -> Expression {
    Expression::UtilityStatement(Box::new(UtilityStatement {
        kind: kind.to_owned(),
        options,
        operands,
    }))
}

impl Parser<'_> {
    /// `TransactionStmt` or `TransactionStmtLegacy`.
    pub(crate) fn transaction_statement(&mut self) -> Parsing<Expression> {
        let word: &str = self.keyword_at(0).unwrap_or_default();
        self.at += 1;
        match word {
            "begin" => {
                self.opt_transaction();
                Ok(utility(
                    "BEGIN",
                    self.transaction_modes_or_empty()?,
                    Vec::new(),
                ))
            }
            "start" => {
                self.expect_kw("transaction")?;
                Ok(utility(
                    "START TRANSACTION",
                    self.transaction_modes_or_empty()?,
                    Vec::new(),
                ))
            }
            "savepoint" => Ok(utility("SAVEPOINT", Vec::new(), vec![self.savepoint()?])),
            "release" => {
                self.eat_noise_kw("savepoint");
                Ok(utility("RELEASE", Vec::new(), vec![self.savepoint()?]))
            }
            "prepare" => {
                self.expect_kw("transaction")?;
                Ok(utility(
                    "PREPARE TRANSACTION",
                    Vec::new(),
                    vec![self.sconst()?],
                ))
            }
            "commit" | "rollback" if self.eat_kw("prepared") => {
                let kind: String = format!("{} PREPARED", word.to_uppercase());
                Ok(utility(&kind, Vec::new(), vec![self.sconst()?]))
            }
            _ => self.transaction_end(word),
        }
    }

    /// `ABORT`, `END`, `COMMIT` or `ROLLBACK` `[WORK | TRANSACTION]`, then `TO [SAVEPOINT] name` or `[AND [NO] CHAIN]`.
    fn transaction_end(&mut self, word: &str) -> Parsing<Expression> {
        let (kind, savepoints): (&str, bool) = match word {
            "rollback" => (ROLLBACK, true),
            "abort" => (ROLLBACK, false),
            _ => (COMMIT, false),
        };
        self.opt_transaction();
        if savepoints && self.eat_kw("to") {
            self.eat_noise_kw("savepoint");
            return Ok(utility("ROLLBACK TO", Vec::new(), vec![self.savepoint()?]));
        }
        let mut options: Vec<Expression> = Vec::new();
        if self.eat_kw("and") {
            let chained: bool = !self.eat_kw("no");
            self.expect_kw("chain")?;
            if chained {
                options.push(tuple(vec![var("CHAIN")]));
            }
        }
        Ok(utility(kind, options, Vec::new()))
    }

    /// `opt_transaction`: `WORK` or `TRANSACTION`, which change nothing.
    fn opt_transaction(&mut self) {
        if !self.eat_kw("work") {
            self.eat_kw("transaction");
        }
    }

    /// `transaction_mode_list_or_empty`.
    fn transaction_modes_or_empty(&mut self) -> Parsing<Vec<Expression>> {
        let starts: bool = ["isolation", "read", "deferrable", "not"]
            .iter()
            .any(|word| self.kw(word));
        match starts {
            true => self.transaction_modes(),
            false => Ok(Vec::new()),
        }
    }

    /// A savepoint's `ColId`.
    fn savepoint(&mut self) -> Parsing<Expression> {
        Ok(name_from(self.name(is_col_id)?))
    }

    /// `VariableSetStmt` or `VariableResetStmt`: `SET [LOCAL | SESSION] set_rest` or `RESET reset_rest`.
    pub(crate) fn set_statement(&mut self) -> Parsing<Expression> {
        if self.kw("reset") {
            return Ok(utility(SET, Vec::new(), vec![self.set_reset_clause()?]));
        }
        self.expect_kw("set")?;
        let mut options: Vec<Expression> = Vec::new();
        if self.eat_kw("local") {
            options.push(tuple(vec![var("LOCAL")]));
        } else if self.kw("session") && !SESSION_REST.iter().any(|word| self.kw_at(1, word)) {
            self.at += 1;
        }
        Ok(utility(SET, options, vec![self.set_rest()?]))
    }

    /// `VariableShowStmt`: the setting `SHOW` names, as `var_name` or a special form's setting.
    pub(crate) fn show_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("show")?;
        let special: Option<&(&[&str], &str)> =
            SHOW_SPECIALS.iter().find(|(words, _)| self.words_at(words));
        let name: Expression = match special {
            Some((words, name)) => {
                self.at += words.len();
                Expression::Identifier(Identifier::new(*name))
            }
            None => self.setting_name()?,
        };
        Ok(utility("SHOW", Vec::new(), vec![name]))
    }

    /// `ConstraintsSetStmt`: `SET CONSTRAINTS {ALL | qualified_name, ...} {DEFERRED | IMMEDIATE}`.
    pub(crate) fn set_constraints(&mut self) -> Parsing<Expression> {
        self.at += 2;
        let constraints: Expression = match self.eat_kw("all") {
            true => var("ALL"),
            false => tuple(self.comma_list(|parser| {
                Ok(Expression::Table(Box::new(table(parser.table_name()?))))
            })?),
        };
        let mode: &str = match self.eat_kw("deferred") {
            true => "DEFERRED",
            false => {
                self.expect_kw("immediate")?;
                "IMMEDIATE"
            }
        };
        Ok(utility(
            "SET CONSTRAINTS",
            Vec::new(),
            vec![constraints, var(mode)],
        ))
    }

    /// `LockStmt`: `LOCK [TABLE] relation_expr, ... [IN lock_type MODE] [NOWAIT]`, the mode written out.
    pub(crate) fn lock_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("lock")?;
        self.eat_kw("table");
        let relations: Vec<Expression> =
            self.comma_list(|parser| Ok(Expression::Table(Box::new(parser.relation()?))))?;
        let mut mode: &str = ACCESS_EXCLUSIVE;
        if self.eat_kw("in") {
            let Some((words, named)) = LOCK_MODES.iter().find(|(words, _)| self.words_at(words))
            else {
                return Err(self.error());
            };
            self.at += words.len();
            self.expect_kw("mode")?;
            mode = named;
        }
        let options: Vec<Expression> = match self.eat_kw("nowait") {
            true => vec![tuple(vec![var("NOWAIT")])],
            false => Vec::new(),
        };
        Ok(utility("LOCK", options, vec![tuple(relations), var(mode)]))
    }

    /// `DiscardStmt`: `DISCARD {ALL | TEMP | TEMPORARY | PLANS | SEQUENCES}`.
    pub(crate) fn discard_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("discard")?;
        let Some((_, target)) = DISCARD_TARGETS.iter().find(|(word, _)| self.kw(word)) else {
            return Err(self.error());
        };
        self.at += 1;
        Ok(utility("DISCARD", Vec::new(), vec![var(target)]))
    }
}
