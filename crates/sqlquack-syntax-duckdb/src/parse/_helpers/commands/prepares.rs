//! `PrepareStatement`, `ExecuteStatement`, `DeallocateStatement` and `CallStatement`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::Call;
use sqlquack_syntax::expressions::statements::models::{
    CopyStmt, ExecuteStatement, Pragma, PrepareStatement,
};
use sqlquack_syntax::expressions::tables::models::DropObject;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::table;
use crate::parse::constants::{PREPARABLE_STARTS, PREPARED_STATEMENT_KIND, STATEMENT_TYPES};

impl Parser<'_> {
    /// `PrepareStatement`: `PREPARE name [(types)] AS statement`, a statement DuckDB can prepare.
    pub(crate) fn prepare_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("PREPARE")?;
        let name: Identifier = self.col_id_or_string()?;
        let parameter_types = match self.is_char('(') {
            true => self.parenthesized(|parser| parser.data_type())?,
            false => Vec::new(),
        };
        self.expect_kw("AS")?;
        let kind: Option<&'static str> = self.statement_type();
        let statement: Expression = self.statement()?;
        let kind: Option<&'static str> = match (&statement, kind) {
            (Expression::Merge(_), _) => Some("MERGE_INTO_STATEMENT"),
            (Expression::Pragma(pragma), _) => Some(pragma_type(pragma)),
            (Expression::Copy(copy), _) => copy_type(copy),
            (_, kind) => kind,
        };
        if let (Some(kind), true) = (kind, parameter_types.is_empty()) {
            self.refuse_later(&format!("{kind} is not a preparable statement"), start);
        }
        Ok(Expression::Prepare(Box::new(PrepareStatement {
            name,
            parameter_types,
            statement,
        })))
    }

    /// The statement type DuckDB gives the statement ahead when it cannot be prepared, by its first words.
    fn statement_type(&self) -> Option<&'static str> {
        let word: String = self.word_at(0)?;
        match word.as_str() {
            "UPDATE" if self.kw_at(1, "EXTENSIONS") && !self.kw_at(2, "SET") => {
                Some("UPDATE_EXTENSIONS_STATEMENT")
            }
            "FORCE" if self.kw_at(1, "CHECKPOINT") => Some("CALL_STATEMENT"),
            "FORCE" => Some("LOAD_STATEMENT"),
            _ if PREPARABLE_STARTS.contains(&word.as_str()) => None,
            _ => STATEMENT_TYPES
                .iter()
                .find(|(start, _)| *start == word)
                .map(|(_, kind)| *kind),
        }
    }

    /// `ExecuteStatement`: `EXECUTE name [(arguments)]`.
    pub(crate) fn execute_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("EXECUTE")?;
        let name: Identifier = self.col_id_or_string()?;
        let arguments: Vec<Expression> = match self.is_char('(') {
            true => self.table_function_arguments()?,
            false => Vec::new(),
        };
        Ok(Expression::Execute(Box::new(ExecuteStatement {
            this: Expression::Identifier(name),
            return_status: None,
            parameters: Vec::new(),
            arguments,
            prepared: true,
            suffix: None,
        })))
    }

    /// `DeallocateStatement`: `DEALLOCATE [PREPARE] name`, the drop of a prepared statement.
    pub(crate) fn deallocate_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("DEALLOCATE")?;
        self.eat_kw("PREPARE");
        let name: Identifier = self.col_id_or_string()?;
        Ok(Expression::DropObject(Box::new(DropObject {
            kind: PREPARED_STATEMENT_KIND.to_owned(),
            name: table(vec![name]),
            if_exists: false,
            cascade: false,
            restrict: false,
            force: false,
            objects: Vec::new(),
            concurrently: false,
        })))
    }

    /// `CallStatement`: `CALL [catalog.][schema.]function(arguments)`.
    pub(crate) fn call_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("CALL")?;
        let Some((name, arguments)) = self.table_function_call()? else {
            return Err(self.error());
        };
        Ok(Expression::Call(Box::new(Call {
            source_dialect: None,
            name,
            member: None,
            arguments,
            asynchronous: false,
            hints: Vec::new(),
            with: None,
            into: None,
        })))
    }
}

/// `PRAGMA name = value` is a SET statement to DuckDB, any other pragma a PRAGMA statement.
fn pragma_type(pragma: &Pragma) -> &'static str {
    match pragma.use_assignment_syntax {
        true => "SET_STATEMENT",
        false => "PRAGMA_STATEMENT",
    }
}

/// A copy of a table or query can be prepared; `COPY FROM DATABASE` is a pragma, or with a flag its own type.
fn copy_type(copy: &CopyStmt) -> Option<&'static str> {
    match (&copy.this, copy.params.is_empty()) {
        (Expression::Identifier(_), true) => Some("PRAGMA_STATEMENT"),
        (Expression::Identifier(_), false) => Some("COPY_DATABASE_STATEMENT"),
        _ => None,
    }
}
