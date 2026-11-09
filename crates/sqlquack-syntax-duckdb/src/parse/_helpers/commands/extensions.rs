//! `LoadStatement`, `InstallStatement`, `UpdateExtensionsStatement` and `CheckpointStatement`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::Call;
use sqlquack_syntax::expressions::statements::models::Install;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{boolean, null, string};

/// The catalog and schema of the table functions `CHECKPOINT` calls, as the transformer names them.
const SYSTEM_MAIN: [&str; 2] = ["system", "main"];

/// The table function `CHECKPOINT` calls.
const CHECKPOINT: &str = "checkpoint";

/// The table function `FORCE CHECKPOINT` calls.
const FORCE_CHECKPOINT: &str = "force_checkpoint";

impl Parser<'_> {
    /// `FORCE INSTALL ...` or `FORCE CHECKPOINT ...`; `FORCE` starts no other statement.
    pub(crate) fn force_statement(&mut self) -> Parsing<Expression> {
        match self.kw_at(1, "CHECKPOINT") {
            true => self.checkpoint_statement(),
            false => self.install_statement(),
        }
    }

    /// `InstallStatement`: an `Install` tree, or a gap when it loads too or names a version.
    pub(crate) fn install_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        let force: bool = self.eat_kw("FORCE");
        self.expect_kw("INSTALL")?;
        let and_load: bool = self.eat_kw("AND");
        if and_load {
            self.expect_kw("LOAD")?;
        }
        let this: Expression = self.identifier_or_string()?;
        let from: Option<Expression> = self.extension_source()?;
        let versioned: bool = self.eat_kw("VERSION");
        if versioned {
            self.identifier_or_string()?;
        }
        if and_load || versioned {
            self.gap("install_options", start);
            return Ok(null());
        }
        Ok(Expression::Install(Box::new(Install {
            this: Box::new(this),
            from_: from.map(Box::new),
            force: force.then(|| Box::new(boolean(true))),
        })))
    }

    /// `LoadStatement`: `LOAD name [FROM source] [AS alias]`, kept as a gap.
    pub(crate) fn load_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("LOAD")?;
        self.col_id_or_string()?;
        self.extension_source()?;
        if self.eat_kw("AS") {
            self.name(is_identifier)?;
        }
        self.gap("load", start);
        Ok(null())
    }

    /// `UpdateExtensionsStatement`: `UPDATE EXTENSIONS [(name, ...)]`, kept as a gap.
    pub(crate) fn update_extensions_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("UPDATE")?;
        self.expect_kw("EXTENSIONS")?;
        if self.is_char('(') {
            self.parenthesized(|parser| parser.name(is_identifier))?;
        }
        self.gap("update_extensions", start);
        Ok(null())
    }

    /// `CheckpointStatement`: `[FORCE] CHECKPOINT [catalog]`, the `checkpoint` or `force_checkpoint` call DuckDB makes it.
    pub(crate) fn checkpoint_statement(&mut self) -> Parsing<Expression> {
        let force: bool = self.eat_kw("FORCE");
        self.expect_kw("CHECKPOINT")?;
        let arguments: Vec<Expression> = match self.is(is_identifier) {
            true => vec![string(&self.name(is_identifier)?.name)],
            false => Vec::new(),
        };
        let name: &str = match force {
            true => FORCE_CHECKPOINT,
            false => CHECKPOINT,
        };
        Ok(Expression::Call(Box::new(Call {
            source_dialect: None,
            name: SYSTEM_MAIN
                .into_iter()
                .chain([name])
                .map(Identifier::new)
                .collect(),
            member: None,
            arguments,
            asynchronous: false,
            hints: Vec::new(),
            with: None,
            into: None,
        })))
    }

    /// `FromSource`: `FROM` a repository name or a repository URL as a string.
    fn extension_source(&mut self) -> Parsing<Option<Expression>> {
        match self.eat_kw("FROM") {
            true => self.identifier_or_string().map(Some),
            false => Ok(None),
        }
    }

    /// `IdentifierOrStringLiteral`: a name as an identifier, or a string as a string literal.
    fn identifier_or_string(&mut self) -> Parsing<Expression> {
        match self.string_at(0) {
            true => Ok(string(&self.string_lit()?)),
            false => Ok(Expression::Identifier(self.name(is_identifier)?)),
        }
    }
}
