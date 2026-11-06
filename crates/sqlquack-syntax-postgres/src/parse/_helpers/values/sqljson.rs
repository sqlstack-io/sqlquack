//! SQL/JSON (gram.y): constructors, query functions, aggregates and `JSON_TABLE`, checked only.

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id, is_col_label};
use crate::parse::_helpers::values::expressions::Flavor;
use crate::parse::constants::{JSON_ENCODINGS, QUERY_STARTS};
use crate::parse::errors::ParseError;
use crate::scan::models::TokenKind;

/// One way to read the text ahead, tried in turn by `alternatives`.
type Alternative = fn(&mut Parser<'_>) -> Parsing<()>;

impl Parser<'_> {
    /// A SQL/JSON call after its name, through its closing parenthesis.
    pub(crate) fn json_call(&mut self, name: &str) -> Parsing<()> {
        self.expect_char('(')?;
        match name {
            "json_object" => return self.json_object_arguments(),
            "json_array" => return self.json_array_arguments(),
            "json" => {
                self.json_value_expr()?;
                self.json_key_uniqueness();
            }
            "json_scalar" => {
                self.a_expr()?;
            }
            "json_serialize" => {
                self.json_value_expr()?;
                self.json_returning()?;
            }
            "json_query" => {
                self.json_path_head()?;
                self.json_returning()?;
                self.json_wrapper()?;
                self.json_quotes()?;
                self.json_behaviors()?;
            }
            "json_exists" => {
                self.json_path_head()?;
                self.json_on_error()?;
            }
            "json_value" => {
                self.json_path_head()?;
                self.json_returning()?;
                self.json_behaviors()?;
            }
            "json_objectagg" => {
                self.json_name_and_value()?;
                self.json_null_clause();
                self.json_key_uniqueness();
                self.json_returning()?;
            }
            "json_arrayagg" => {
                self.json_value_expr()?;
                if self.kw("order") && self.kw_at(1, "by") {
                    self.sort_clause()?;
                }
                self.json_null_clause();
                self.json_returning()?;
            }
            _ => {}
        }
        self.expect_char(')')
    }

    /// `JSON_TABLE ( context, path [AS name] [PASSING ...] COLUMNS (...) [ON ERROR] )`.
    pub(crate) fn json_table(&mut self) -> Parsing<()> {
        self.expect_char('(')?;
        self.json_value_expr()?;
        self.expect_char(',')?;
        let path_start: usize = self.at;
        self.a_expr()?;
        let constant: bool = self.string_constant_between(path_start);
        if self.eat_kw("as") {
            self.name(is_col_id)?;
        }
        self.json_passing()?;
        self.expect_kw("columns")?;
        self.parenthesized(|parser| parser.json_table_column())?;
        self.json_on_error()?;
        self.expect_char(')')?;
        match constant {
            true => Ok(()),
            false => Err(ParseError {
                message: "only string constants are supported in JSON_TABLE path specification"
                    .to_owned(),
                position: self.tokens[path_start].start,
                syntax: false,
            }),
        }
    }

    /// The first alternative that parses; a non-syntax error ends the search, else the furthest.
    fn alternatives(&mut self, choices: &[Alternative]) -> Parsing<()> {
        let mut furthest: Option<(ParseError, usize)> = None;
        for choice in choices {
            match self.attempt(|parser| choice(parser)) {
                Ok(()) => return Ok(()),
                Err((error, _)) if !error.syntax => return Err(error),
                Err((error, reached)) => {
                    if furthest.as_ref().is_none_or(|(_, best)| reached > *best) {
                        furthest = Some((error, reached));
                    }
                }
            }
        }
        Err(furthest.map_or_else(|| self.error(), |(error, _)| error))
    }

    /// `JSON_OBJECT`'s arguments: none, `RETURNING` alone, key-value pairs, or the old plain list.
    fn json_object_arguments(&mut self) -> Parsing<()> {
        if self.eat_char(')') {
            return Ok(());
        }
        if self.kw("returning") {
            self.json_returning()?;
            return self.expect_char(')');
        }
        self.alternatives(&[
            |parser| {
                parser.comma_list(|parser| parser.json_name_and_value())?;
                parser.json_null_clause();
                parser.json_key_uniqueness();
                parser.json_returning()?;
                parser.expect_char(')')
            },
            |parser| {
                parser.comma_list(|parser| parser.function_argument())?;
                parser.expect_char(')')
            },
        ])
    }

    /// `JSON_ARRAY`'s arguments: none, `RETURNING` alone, a query, or values.
    fn json_array_arguments(&mut self) -> Parsing<()> {
        if self.eat_char(')') {
            return Ok(());
        }
        if self.kw("returning") {
            self.json_returning()?;
            return self.expect_char(')');
        }
        if self
            .keyword_at(0)
            .is_some_and(|word| QUERY_STARTS.contains(&word))
        {
            self.query()?;
            self.json_format()?;
        } else {
            self.comma_list(|parser| parser.json_value_expr())?;
            self.json_null_clause();
        }
        self.json_returning()?;
        self.expect_char(')')
    }

    /// `json_name_and_value`: `KEY c_expr VALUE v`, `c_expr VALUE v` or `a_expr : v`.
    fn json_name_and_value(&mut self) -> Parsing<()> {
        let keyed: Alternative = |parser| {
            parser.expect_kw("key")?;
            parser.c_expr(Flavor::B)?;
            parser.expect_kw("value")?;
            parser.json_value_expr()
        };
        let valued: Alternative = |parser| {
            parser.c_expr(Flavor::B)?;
            parser.expect_kw("value")?;
            parser.json_value_expr()
        };
        let colon: Alternative = |parser| {
            parser.a_expr()?;
            parser.expect_char(':')?;
            parser.json_value_expr()
        };
        self.alternatives(&[keyed, valued, colon])
    }

    /// `json_value_expr`: `a_expr [FORMAT JSON [ENCODING name]]`.
    pub(crate) fn json_value_expr(&mut self) -> Parsing<()> {
        self.a_expr()?;
        self.json_format()
    }

    /// `json_format_clause_opt`; an encoding other than UTF8, UTF16 or UTF32 is refused (22023).
    fn json_format(&mut self) -> Parsing<()> {
        if !(self.kw("format") && self.kw_at(1, "json")) {
            return Ok(());
        }
        self.at += 2;
        if !self.eat_kw("encoding") {
            return Ok(());
        }
        let position: usize = self.offset();
        let encoding: String = self.name(is_col_id)?.name;
        match JSON_ENCODINGS.contains(&encoding.to_ascii_lowercase().as_str()) {
            true => Ok(()),
            false => Err(ParseError {
                message: format!("unrecognized JSON encoding: {encoding}"),
                position,
                syntax: false,
            }),
        }
    }

    /// `json_returning_clause_opt`: `RETURNING Typename [FORMAT JSON ...]`.
    fn json_returning(&mut self) -> Parsing<()> {
        if self.eat_kw("returning") {
            self.typename()?;
            self.json_format()?;
        }
        Ok(())
    }

    /// `json_key_uniqueness_constraint_opt`: `WITH|WITHOUT UNIQUE [KEYS]`.
    pub(crate) fn json_key_uniqueness(&mut self) {
        if (self.kw("with") || self.kw("without")) && self.kw_at(1, "unique") {
            self.at += 2;
            self.eat_kw("keys");
        }
    }

    /// `NULL ON NULL` or `ABSENT ON NULL`.
    fn json_null_clause(&mut self) {
        if (self.kw("null") || self.kw("absent")) && self.kw_at(1, "on") && self.kw_at(2, "null") {
            self.at += 3;
        }
    }

    /// The context item, `,`, the path and any `PASSING` of the query functions.
    fn json_path_head(&mut self) -> Parsing<()> {
        self.json_value_expr()?;
        self.expect_char(',')?;
        self.a_expr()?;
        self.json_passing()
    }

    /// `PASSING json_value_expr AS ColLabel, ...`.
    fn json_passing(&mut self) -> Parsing<()> {
        if self.eat_kw("passing") {
            self.comma_list(|parser| {
                parser.json_value_expr()?;
                parser.expect_kw("as")?;
                parser.expect_token(is_col_label).map(drop)
            })?;
        }
        Ok(())
    }

    /// `json_wrapper_behavior`: `WITHOUT [ARRAY] WRAPPER` or `WITH [UN]CONDITIONAL [ARRAY] WRAPPER`.
    fn json_wrapper(&mut self) -> Parsing<()> {
        if self.eat_kw("without") {
            self.eat_kw("array");
            return self.expect_kw("wrapper");
        }
        if self.eat_kw("with") {
            if !self.eat_kw("conditional") {
                self.eat_kw("unconditional");
            }
            self.eat_kw("array");
            return self.expect_kw("wrapper");
        }
        Ok(())
    }

    /// `json_quotes_clause_opt`: `KEEP|OMIT QUOTES [ON SCALAR STRING]`.
    fn json_quotes(&mut self) -> Parsing<()> {
        if self.eat_kw("keep") || self.eat_kw("omit") {
            self.expect_kw("quotes")?;
            if self.eat_kw("on") {
                self.expect_kw("scalar")?;
                self.expect_kw("string")?;
            }
        }
        Ok(())
    }

    /// `json_behavior`, when one is next: `DEFAULT a_expr`, `ERROR`, `NULL`, `EMPTY [ARRAY]`, ...
    fn json_behavior(&mut self) -> Parsing<bool> {
        match self.keyword_at(0) {
            Some("default") => {
                self.at += 1;
                self.a_expr()?;
            }
            Some("error" | "null" | "true" | "false" | "unknown") => self.at += 1,
            Some("empty") => {
                self.at += 1;
                if !self.eat_kw("array") {
                    self.eat_kw("object");
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `json_behavior_clause_opt`: `b ON EMPTY`, `b ON ERROR`, or `b ON EMPTY b ON ERROR`.
    fn json_behaviors(&mut self) -> Parsing<()> {
        if !self.json_behavior()? {
            return Ok(());
        }
        self.expect_kw("on")?;
        if self.eat_kw("error") {
            return Ok(());
        }
        self.expect_kw("empty")?;
        self.json_on_error()
    }

    /// `json_on_error_clause_opt`: `b ON ERROR`.
    fn json_on_error(&mut self) -> Parsing<()> {
        if self.json_behavior()? {
            self.expect_kw("on")?;
            self.expect_kw("error")?;
        }
        Ok(())
    }

    /// `json_table_column_definition`: ordinality, regular, `EXISTS` and `NESTED` columns.
    fn json_table_column(&mut self) -> Parsing<()> {
        if self.kw("nested") && (self.sconst_at(1) || self.kw_at(1, "path")) {
            self.at += 1;
            self.eat_kw("path");
            self.sconst()?;
            if self.eat_kw("as") {
                self.name(is_col_id)?;
            }
            self.expect_kw("columns")?;
            self.parenthesized(|parser| parser.json_table_column())?;
            return Ok(());
        }
        self.name(is_col_id)?;
        if self.kw("for") && self.kw_at(1, "ordinality") {
            self.at += 2;
            return Ok(());
        }
        self.typename()?;
        if self.eat_kw("exists") {
            self.json_column_path()?;
            return self.json_on_error();
        }
        self.json_format()?;
        self.json_column_path()?;
        self.json_wrapper()?;
        self.json_quotes()?;
        self.json_behaviors()
    }

    /// `json_table_column_path_clause_opt`: `PATH Sconst`.
    fn json_column_path(&mut self) -> Parsing<()> {
        if self.eat_kw("path") {
            self.sconst()?;
        }
        Ok(())
    }

    /// Whether the tokens from `start` to the cursor are one string constant in any parentheses.
    fn string_constant_between(&self, start: usize) -> bool {
        let mut strings: usize = 0;
        let only_parentheses_around: bool =
            self.tokens[start..self.at]
                .iter()
                .all(|token| match token.kind {
                    TokenKind::Char('(' | ')') => true,
                    TokenKind::String | TokenKind::EscapeString | TokenKind::DollarString => {
                        strings += 1;
                        true
                    }
                    _ => false,
                });
        only_parentheses_around && strings == 1
    }
}
