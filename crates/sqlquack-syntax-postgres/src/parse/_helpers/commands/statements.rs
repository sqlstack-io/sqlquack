//! `stmt` (gram.y): the statements read, including `TRUNCATE` and `COPY`, and the opaque rest.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::statements::models::{
    CopyParameter, CopyStmt, Schema, Truncate, TruncateIdentity, TruncateTableEntry, TruncateTarget,
};

use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, is_col_id, is_col_label, is_non_reserved_word,
};
use crate::parse::_helpers::tree::nodes::{
    boolean, column, command, function, literal, star, subquery, table,
};
use crate::parse::constants::{STATEMENT_STARTS, STATEMENT_STARTS_MORE};
use crate::scan::models::TokenKind;

impl Parser<'_> {
    /// One statement: a query or DML statement, or any other statement as an opaque gap.
    pub(crate) fn statement(&mut self) -> Parsing<Expression> {
        if self.is_char('(') {
            return self.preparable_statement();
        }
        match self.keyword_at(0) {
            Some(
                "select" | "values" | "table" | "with" | "insert" | "update" | "delete" | "merge",
            ) => self.preparable_statement(),
            Some("truncate") => self.truncate_statement(),
            Some("copy") => self.copy_statement(),
            Some("comment") => self.comment_statement(),
            Some(word)
                if STATEMENT_STARTS.contains(&word) || STATEMENT_STARTS_MORE.contains(&word) =>
            {
                let start: usize = self.offset();
                self.skip_statement_rest()?;
                if self.is_char(')') {
                    return Err(self.error());
                }
                self.gap(word, start);
                Ok(command(""))
            }
            _ => Err(self.error()),
        }
    }
}

/// The call a `PROGRAM 'command'` target becomes, so it differs from a file.
const PROGRAM_TARGET: &str = "program";

/// `opt_program copy_file_name`: the file, and whether `PROGRAM` names a command.
struct CopyFile {
    file: Expression,
    program: bool,
}

fn parameter(name: &str, value: Option<Expression>, values: Vec<Expression>) -> CopyParameter {
    CopyParameter {
        name: name.to_owned(),
        value,
        values,
        eq: false,
    }
}

fn string(text: &str) -> Expression {
    literal(Literal::String(text.to_owned()))
}

impl Parser<'_> {
    /// `TRUNCATE [TABLE] relation_expr, ... [CONTINUE|RESTART IDENTITY] [CASCADE|RESTRICT]`.
    pub(crate) fn truncate_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("truncate")?;
        self.eat_kw("table");
        let mut entries: Vec<TruncateTableEntry> = self.comma_list(|parser| {
            let table = parser.relation()?;
            Ok(TruncateTableEntry { table, star: false })
        })?;
        let identity: Option<TruncateIdentity> = match self.keyword_at(0) {
            Some("continue") => Some(TruncateIdentity::Continue),
            Some("restart") => Some(TruncateIdentity::Restart),
            _ => None,
        };
        if identity.is_some() {
            self.at += 1;
            self.expect_kw("identity")?;
        }
        let cascade: bool = self.eat_kw("cascade");
        let restrict: bool = !cascade && self.eat_kw("restrict");
        let first: TruncateTableEntry = entries.remove(0);
        Ok(Expression::Truncate(Box::new(Truncate {
            target: TruncateTarget::Table,
            if_exists: false,
            table: first.table,
            on_cluster: None,
            cascade,
            extra_tables: entries,
            identity,
            restrict,
            partition: None,
            error_table: false,
        })))
    }

    /// `COPY [BINARY] table [(columns)] FROM|TO ...` or `COPY ( statement ) TO ...`.
    pub(crate) fn copy_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("copy")?;
        if self.eat_char('(') {
            let query: Expression = self.preparable_statement()?;
            self.expect_char(')')?;
            self.expect_kw("to")?;
            let file: CopyFile = self.copy_file(start)?;
            let (params, with_wrapped) = self.copy_options()?;
            return Ok(copy(
                subquery(query, None, false),
                false,
                file,
                (params, with_wrapped),
            ));
        }
        let binary: bool = self.eat_kw("binary");
        let relation: Expression = Expression::Table(Box::new(table(self.table_name()?)));
        let target: Expression = match self.is_char('(') {
            true => Expression::Schema(Box::new(Schema {
                this: Some(Box::new(relation)),
                expressions: self
                    .name_list()?
                    .into_iter()
                    .map(|name| column(None, name))
                    .collect(),
            })),
            false => relation,
        };
        let from: bool = match self.keyword_at(0) {
            Some("from") => true,
            Some("to") => false,
            _ => return Err(self.error()),
        };
        self.at += 1;
        let file: CopyFile = self.copy_file(start)?;
        let mut params: Vec<CopyParameter> = Vec::new();
        if binary {
            params.push(parameter("binary", None, Vec::new()));
        }
        if self.eat_kw("using") || self.kw("delimiters") {
            self.expect_kw("delimiters")?;
            params.push(parameter("delimiter", Some(self.sconst()?), Vec::new()));
        }
        let (options, with_wrapped) = self.copy_options()?;
        params.extend(options);
        let where_start: usize = self.offset();
        if self.eat_kw("where") {
            self.a_expr()?;
            if !from {
                return Err(self.error_with("WHERE clause not allowed with COPY TO", where_start));
            }
            self.gap("copy_where", where_start);
        }
        Ok(copy(target, from, file, (params, with_wrapped)))
    }

    /// `opt_program copy_file_name`; `PROGRAM` takes a string only.
    fn copy_file(&mut self, start: usize) -> Parsing<CopyFile> {
        let program: bool = self.eat_kw("program");
        let (file, standard) = match self.keyword_at(0) {
            Some(word @ ("stdin" | "stdout")) => {
                self.at += 1;
                (column(None, Identifier::new(word.to_uppercase())), true)
            }
            _ => (self.sconst()?, false),
        };
        if program && standard {
            return Err(self.error_with("STDIN/STDOUT not allowed with PROGRAM", start));
        }
        Ok(CopyFile { file, program })
    }

    /// `opt_with copy_options`: the old option words, or `( name [arg], ... )`.
    fn copy_options(&mut self) -> Parsing<(Vec<CopyParameter>, bool)> {
        self.eat_kw("with");
        if self.is_char('(') {
            return Ok((
                self.parenthesized(|parser| parser.copy_generic_option())?,
                true,
            ));
        }
        let mut params: Vec<CopyParameter> = Vec::new();
        while let Some(param) = self.copy_option_item()? {
            params.push(param);
        }
        Ok((params, false))
    }

    /// `copy_opt_item`, or `None` when no option word is next.
    fn copy_option_item(&mut self) -> Parsing<Option<CopyParameter>> {
        let Some(word) = self.keyword_at(0) else {
            return Ok(None);
        };
        match word {
            "binary" | "freeze" | "csv" | "header" => {
                self.at += 1;
                Ok(Some(parameter(word, None, Vec::new())))
            }
            "delimiter" | "null" | "quote" | "escape" => {
                self.at += 1;
                self.eat_kw("as");
                Ok(Some(parameter(word, Some(self.sconst()?), Vec::new())))
            }
            "encoding" => {
                self.at += 1;
                Ok(Some(parameter(word, Some(self.sconst()?), Vec::new())))
            }
            "force" => {
                self.at += 1;
                let name: &str = match self.keyword_at(0) {
                    Some("quote") => "force_quote",
                    Some("null") => "force_null",
                    Some("not") => {
                        self.at += 1;
                        if !self.kw("null") {
                            return Err(self.error());
                        }
                        "force_not_null"
                    }
                    _ => return Err(self.error()),
                };
                self.at += 1;
                let columns: Vec<Expression> = match self.eat_char('*') {
                    true => vec![star(None)],
                    false => self.comma_list(|parser| Ok(column(None, parser.name(is_col_id)?)))?,
                };
                Ok(Some(parameter(name, None, columns)))
            }
            _ => Ok(None),
        }
    }

    /// `copy_generic_opt_elem`: `ColLabel [copy_generic_opt_arg]`.
    fn copy_generic_option(&mut self) -> Parsing<CopyParameter> {
        let name: String = self.expect_token(is_col_label)?.value;
        if self.is_char(',') || self.is_char(')') {
            return Ok(parameter(&name, None, Vec::new()));
        }
        if self.is_char('(') {
            let values: Vec<Expression> =
                self.parenthesized(|parser| parser.boolean_or_string())?;
            return Ok(parameter(&name, None, values));
        }
        let value: Expression = match self.peek().map(|token| token.kind) {
            Some(TokenKind::Char('*')) => {
                self.at += 1;
                star(None)
            }
            Some(TokenKind::Integer | TokenKind::Numeric | TokenKind::Char('+' | '-')) => {
                self.numeric_only()?
            }
            _ if self.eat_kw("default") => string("default"),
            _ => self.boolean_or_string()?,
        };
        Ok(parameter(&name, Some(value), Vec::new()))
    }

    /// `opt_boolean_or_string`: `TRUE`, `FALSE`, `ON`, a non-reserved word or a string.
    fn boolean_or_string(&mut self) -> Parsing<Expression> {
        match self.keyword_at(0) {
            Some("true") => {
                self.at += 1;
                return Ok(boolean(true));
            }
            Some("false") => {
                self.at += 1;
                return Ok(boolean(false));
            }
            Some("on") => {
                self.at += 1;
                return Ok(string("on"));
            }
            _ => {}
        }
        match self.sconst_at(0) {
            true => self.sconst(),
            false => Ok(string(&self.expect_token(is_non_reserved_word)?.value)),
        }
    }

    /// `NumericOnly`: a number, with an optional sign.
    pub(crate) fn numeric_only(&mut self) -> Parsing<Expression> {
        let sign: &str = match self.peek().map(|token| token.kind) {
            Some(TokenKind::Char('-')) => "-",
            Some(TokenKind::Char('+')) => "",
            _ => return self.unsigned_number(""),
        };
        self.at += 1;
        self.unsigned_number(sign)
    }

    fn unsigned_number(&mut self, sign: &str) -> Parsing<Expression> {
        let token = self.advance()?;
        match token.kind {
            TokenKind::Integer | TokenKind::Numeric => {
                Ok(literal(Literal::Number(format!("{sign}{}", token.value))))
            }
            _ => {
                self.at -= 1;
                Err(self.error())
            }
        }
    }
}

/// The `COPY` node; `options` is the parameters and whether they were parenthesized.
fn copy(
    this: Expression,
    from: bool,
    file: CopyFile,
    options: (Vec<CopyParameter>, bool),
) -> Expression {
    let (params, with_wrapped) = options;
    let target: Expression = match file.program {
        true => Expression::Function(Box::new(function(
            vec![Identifier::new(PROGRAM_TARGET)],
            vec![file.file],
        ))),
        false => file.file,
    };
    Expression::Copy(Box::new(CopyStmt {
        this,
        kind: from,
        files: vec![target],
        params,
        credentials: None,
        is_into: false,
        with_wrapped,
    }))
}
