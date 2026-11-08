//! `tvf`, `tvf_argument` and `table_clause` (googlesql.tm): table-valued function calls.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::{
    NamedArgSeparator, NamedArgument, TableArgument,
};

use crate::parse::_helpers::commands::alters::named;
use crate::parse::_helpers::commands::from::aliased;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier, is_path_word, is_word};
use crate::parse::_helpers::tree::nodes::{
    binary, column_from, function, null, subquery, table, tuple, var,
};
use crate::scan::models::Token;

/// `prefix this`, a `MODEL`, `CONNECTION`, `GRAPH`, `DESCRIPTOR` or `TABLE` argument.
fn prefixed(prefix: &str, this: Expression) -> Expression {
    Expression::TableArgument(Box::new(TableArgument {
        prefix: prefix.to_owned(),
        this,
    }))
}

impl Parser<'_> {
    /// A `tvf`'s name: a path, or `IF` when a call follows.
    pub(crate) fn tvf_name(&mut self) -> Parsing<Vec<Identifier>> {
        if self.kw("IF") && self.char_at(1, '(') {
            return Ok(vec![self.name(|token: &Token| is_word(token, "IF"))?]);
        }
        self.multipart_identifier()
    }

    /// `tvf`: `name ( tvf_argument, ... )`, with none of a scalar call's modifiers.
    pub(crate) fn tvf(&mut self, name: Vec<Identifier>) -> Parsing<Expression> {
        self.expect_char('(')?;
        let args: Vec<Expression> = match self.is_char(')') {
            true => Vec::new(),
            false => self.comma_list(|parser| parser.deeper(|parser| parser.tvf_argument()))?,
        };
        self.expect_char(')')?;
        Ok(Expression::Function(Box::new(function(name, args))))
    }

    /// `pipe_call`'s target: `tvf [as_alias]`.
    pub(crate) fn aliased_tvf(&mut self) -> Parsing<Expression> {
        let name: Vec<Identifier> = self.tvf_name()?;
        let call: Expression = self.tvf(name)?;
        let alias = self.table_alias()?;
        Ok(aliased(call, alias))
    }

    /// `tvf_argument`: a named argument, a keyword-led argument, or an expression.
    fn tvf_argument(&mut self) -> Parsing<Expression> {
        if (self.is(is_identifier) || self.kw("PROTO")) && self.op_at(1, "=>") {
            let name: Identifier = self
                .advance()
                .map(|token| crate::parse::_helpers::tree::cursor::identifier(&token))?;
            self.at += 1;
            let value: Expression = match self.prefixed_argument(true)? {
                Some(value) => value,
                None => self.expression()?,
            };
            return Ok(Expression::NamedArgument(Box::new(NamedArgument {
                name,
                value,
                separator: NamedArgSeparator::DArrow,
            })));
        }
        if let Some(argument) = self.prefixed_argument(false)? {
            return Ok(argument);
        }
        let value: Expression = self.expression()?;
        if matches!(value, Expression::Lambda(_)) {
            return Err(self.error());
        }
        Ok(value)
    }

    /// A `tvf_argument` or a named value (after `name =>`) that starts with a keyword, when present.
    fn prefixed_argument(&mut self, after_name: bool) -> Parsing<Option<Expression>> {
        let start: usize = self.offset();
        if self.kw_in(0, &["MODEL", "GRAPH"])
            && self.is_at(1, is_path_word)
            && (!after_name || self.kw("MODEL"))
        {
            let prefix: String = self.word_at(0).unwrap_or_default().to_ascii_uppercase();
            self.at += 1;
            let path: Vec<Identifier> = self.multipart_identifier()?;
            return Ok(Some(prefixed(
                &prefix,
                Expression::Table(Box::new(table(path))),
            )));
        }
        if !after_name && self.kw("CONNECTION") {
            if self.kw_at(1, "DEFAULT") {
                self.at += 2;
                return Ok(Some(prefixed("CONNECTION", var("DEFAULT"))));
            }
            if self.char_at(1, '(') && self.is_at(2, is_identifier) && self.char_at(3, '=') {
                self.at += 1;
                let entries: Vec<Expression> = self.parenthesized(|parser| {
                    let key: Identifier = parser.ident()?;
                    parser.expect_char('=')?;
                    Ok(binary(Expression::Eq, named(key), parser.expression()?))
                })?;
                return Ok(Some(prefixed("CONNECTION", tuple(entries))));
            }
            if self.is_at(1, is_path_word) {
                self.at += 1;
                let path: Vec<Identifier> = self.multipart_identifier()?;
                return Ok(Some(prefixed(
                    "CONNECTION",
                    Expression::Table(Box::new(table(path))),
                )));
            }
        }
        if !after_name && self.kw("DESCRIPTOR") && self.char_at(1, '(') {
            self.at += 1;
            let columns: Vec<Identifier> = self.identifier_list()?;
            return Ok(Some(prefixed(
                "DESCRIPTOR",
                tuple(
                    columns
                        .into_iter()
                        .map(|name| column_from(vec![name]))
                        .collect(),
                ),
            )));
        }
        if self.kw("COLUMNS") && self.char_at(1, '(') {
            let name: Identifier = self.ident()?;
            self.expect_char('(')?;
            let columns: Expression = self.expression()?;
            self.expect_char(')')?;
            return Ok(Some(Expression::Function(Box::new(function(
                vec![name],
                vec![columns],
            )))));
        }
        if self.kw("TABLE") && (self.char_at(1, '(') || self.is_at(1, is_identifier)) {
            return self.table_clause().map(Some);
        }
        let reference: bool =
            after_name && self.kw_in(0, &["FUNCTION", "SEQUENCE"]) && self.is_at(1, is_path_word);
        if reference {
            self.at += 1;
            self.multipart_identifier()?;
            self.gap("function_or_sequence_argument", start);
            return Ok(Some(null()));
        }
        if self.kw("INPUT")
            && self.kw_at(1, "TABLE")
            && (self.char_at(2, ',') || self.char_at(2, ')'))
        {
            self.at += 2;
            self.gap("input_table_argument", start);
            return Ok(Some(null()));
        }
        Ok(None)
    }

    /// `table_clause`: `TABLE target` or `TABLE ( target_or_query ... )`; partitioning is a gap.
    pub(crate) fn table_clause(&mut self) -> Parsing<Expression> {
        self.expect_kw("TABLE")?;
        if !self.eat_char('(') {
            let target: Expression = self.table_clause_target()?;
            return Ok(prefixed("TABLE", target));
        }
        let target: Expression = match self.is_char('(') && self.starts_query_at(1) {
            true => {
                self.at += 1;
                let query: Expression = self.query()?;
                self.expect_char(')')?;
                subquery(query, None, false)
            }
            false => self.table_clause_target()?,
        };
        let start: usize = self.offset();
        let partitioned: bool = self.kw("PARTITION") && self.kw_at(1, "BY");
        if partitioned {
            self.at += 2;
            self.comma_list(|parser| parser.expression())?;
        }
        let ordered: bool = self.kw("ORDER") && self.kw_at(1, "BY");
        if ordered {
            self.at += 2;
            self.comma_list(|parser| parser.sort_item())?;
        }
        if partitioned || ordered {
            self.gap("table_argument_partitioning", start);
        }
        self.expect_char(')')?;
        Ok(prefixed("TABLE", target))
    }

    /// `table_clause_target`: a table path, or a `tvf [as_alias] [pivot_or_unpivot_clause]`.
    fn table_clause_target(&mut self) -> Parsing<Expression> {
        let name: Vec<Identifier> = self.tvf_name()?;
        if !self.is_char('(') {
            return Ok(Expression::Table(Box::new(table(name))));
        }
        let call: Expression = self.tvf(name)?;
        let alias = self.table_alias()?;
        let item: Expression = aliased(call, alias);
        match self.kw_in(0, &["PIVOT", "UNPIVOT"]) {
            true => self.pivot_or_unpivot(item),
            false => Ok(item),
        }
    }
}
