//! SQL/JSON calls, `JSON_TABLE` and `MERGE_ACTION()` as `KeywordCall`s holding gram.y's raw tree.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;

use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_col_id, is_col_label};
use crate::parse::_helpers::tree::nodes::{keyword_call, tuple};
use crate::parse::_helpers::values::expressions::Flavor;
use crate::parse::constants::{JSON_ENCODINGS, JSON_QUERY_FUNCTION, QUERY_STARTS};
use crate::parse::errors::ParseError;
use crate::scan::models::TokenKind;

/// One way to read the text ahead, tried in turn by `alternatives`.
type Alternative = fn(&mut Parser<'_>) -> Parsing<Expression>;

/// The `FORMAT JSON` wrapper's kind.
const FORMAT_JSON: &str = "FORMAT JSON";

/// The option `ABSENT ON NULL` sets; `NULL ON NULL` and an object's default leave it out.
const ABSENT_ON_NULL: &str = "ABSENT ON NULL";

/// The option a true `json_key_uniqueness_constraint_opt` sets.
const UNIQUE_KEYS: &str = "WITH UNIQUE KEYS";

/// A clause as a tuple led by its keywords.
fn clause(keywords: &str, values: Vec<Expression>) -> Expression {
    tuple(std::iter::once(var(keywords)).chain(values).collect())
}

impl Parser<'_> {
    /// A SQL/JSON call after its name, through its closing parenthesis.
    pub(crate) fn json_call(&mut self, name: &str) -> Parsing<Expression> {
        self.expect_char('(')?;
        let kind: String = name.to_uppercase();
        let node: Expression = match name {
            "json_object" => return self.json_object_arguments(),
            "json_array" => return self.json_array_arguments(),
            "json" => {
                let operand: Expression = self.json_value_expr()?;
                let options: Vec<Expression> = self.json_unique_option();
                keyword_call(&kind, vec![operand], options)
            }
            "json_scalar" => keyword_call(&kind, vec![self.a_expr()?], Vec::new()),
            "json_serialize" => {
                let operand: Expression = self.json_value_expr()?;
                let options: Vec<Expression> = self.json_returning()?.into_iter().collect();
                keyword_call(&kind, vec![operand], options)
            }
            "json_query" | "json_value" | "json_exists" => {
                let (operands, mut options) = self.json_path_head()?;
                match name {
                    "json_exists" => options.extend(self.json_on_error()?),
                    _ => {
                        options.extend(self.json_returning()?);
                        if name == JSON_QUERY_FUNCTION {
                            options.extend(self.json_wrapper()?);
                            options.extend(self.json_quotes()?);
                        }
                        options.extend(self.json_behaviors()?);
                    }
                }
                keyword_call(&kind, operands, options)
            }
            "json_objectagg" => {
                let pair: Expression = self.json_name_and_value()?;
                let mut options: Vec<Expression> = self.json_null_option(false);
                options.extend(self.json_unique_option());
                options.extend(self.json_returning()?);
                keyword_call(&kind, vec![pair], options)
            }
            "merge_action" => keyword_call(&kind, Vec::new(), Vec::new()),
            _ => {
                let operand: Expression = self.json_value_expr()?;
                let mut options: Vec<Expression> = Vec::new();
                if self.kw("order") && self.kw_at(1, "by") {
                    let order: Vec<Expression> = self
                        .sort_clause()?
                        .into_iter()
                        .map(|ordered| Expression::Ordered(Box::new(ordered)))
                        .collect();
                    options.push(clause("ORDER BY", order));
                }
                options.extend(self.json_null_option(true));
                options.extend(self.json_returning()?);
                keyword_call(&kind, vec![operand], options)
            }
        };
        self.expect_char(')')?;
        Ok(node)
    }

    /// `JSON_TABLE ( context, path [AS name] [PASSING ...] COLUMNS (...) [ON ERROR] )`.
    pub(crate) fn json_table(&mut self) -> Parsing<Expression> {
        self.expect_char('(')?;
        let context: Expression = self.json_value_expr()?;
        self.expect_char(',')?;
        let path_start: usize = self.at;
        let path: Expression = self.a_expr()?;
        let constant: bool = self.string_constant_between(path_start);
        let mut options: Vec<Expression> = Vec::new();
        if self.eat_kw("as") {
            options.push(clause(
                "AS",
                vec![Expression::Identifier(self.name(is_col_id)?)],
            ));
        }
        options.extend(self.json_passing()?);
        self.expect_kw("columns")?;
        options.push(clause("COLUMNS", self.json_table_columns()?));
        options.extend(self.json_on_error()?);
        self.expect_char(')')?;
        match constant {
            true => Ok(keyword_call(
                "JSON_TABLE",
                vec![context, unparenthesized(path)],
                options,
            )),
            false => Err(ParseError {
                message: "only string constants are supported in JSON_TABLE path specification"
                    .to_owned(),
                position: self.tokens[path_start].start,
                syntax: false,
            }),
        }
    }

    /// The first alternative that parses; a non-syntax error ends the search, else the furthest.
    fn alternatives(&mut self, choices: &[Alternative]) -> Parsing<Expression> {
        let mut furthest: Option<(ParseError, usize)> = None;
        for choice in choices {
            match self.attempt(|parser| choice(parser)) {
                Ok(node) => return Ok(node),
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

    /// `JSON_OBJECT`'s arguments: none, `RETURNING` alone, pairs, or the old list as `json_object`.
    fn json_object_arguments(&mut self) -> Parsing<Expression> {
        if self.eat_char(')') {
            return Ok(keyword_call("JSON_OBJECT", Vec::new(), Vec::new()));
        }
        if self.kw("returning") {
            let options: Vec<Expression> = self.json_returning()?.into_iter().collect();
            self.expect_char(')')?;
            return Ok(keyword_call("JSON_OBJECT", Vec::new(), options));
        }
        self.alternatives(&[
            |parser| {
                let pairs: Vec<Expression> =
                    parser.comma_list(|parser| parser.json_name_and_value())?;
                let mut options: Vec<Expression> = parser.json_null_option(false);
                options.extend(parser.json_unique_option());
                options.extend(parser.json_returning()?);
                parser.expect_char(')')?;
                Ok(keyword_call("JSON_OBJECT", pairs, options))
            },
            |parser| {
                let args: Vec<Expression> =
                    parser.comma_list(|parser| parser.function_argument())?;
                parser.expect_char(')')?;
                Ok(Expression::Function(Box::new(
                    crate::parse::_helpers::tree::nodes::function(
                        vec![Identifier::new("json_object")],
                        args,
                    ),
                )))
            },
        ])
    }

    /// `JSON_ARRAY`'s arguments: none, `RETURNING` alone, a query, or values, which alone set nulls.
    fn json_array_arguments(&mut self) -> Parsing<Expression> {
        let absent: Expression = var(ABSENT_ON_NULL);
        if self.eat_char(')') {
            return Ok(keyword_call("JSON_ARRAY", Vec::new(), vec![absent]));
        }
        if self.kw("returning") {
            let mut options: Vec<Expression> = vec![absent];
            options.extend(self.json_returning()?);
            self.expect_char(')')?;
            return Ok(keyword_call("JSON_ARRAY", Vec::new(), options));
        }
        let (kind, operands, mut options): (&str, Vec<Expression>, Vec<Expression>) = match self
            .keyword_at(0)
            .is_some_and(|word| QUERY_STARTS.contains(&word))
        {
            true => {
                let query: Expression = self.query()?;
                (
                    "JSON_ARRAY QUERY",
                    vec![self.json_formatted(query)?],
                    vec![absent],
                )
            }
            false => {
                let values: Vec<Expression> = self.comma_list(|parser| parser.json_value_expr())?;
                ("JSON_ARRAY", values, self.json_null_option(true))
            }
        };
        options.extend(self.json_returning()?);
        self.expect_char(')')?;
        Ok(keyword_call(kind, operands, options))
    }

    /// `json_name_and_value`: `c_expr VALUE v` or `a_expr : v`, as `(key, value)`.
    fn json_name_and_value(&mut self) -> Parsing<Expression> {
        let valued: Alternative = |parser| {
            let key: Expression = parser.c_expr(Flavor::B)?;
            parser.expect_kw("value")?;
            Ok(tuple(vec![key, parser.json_value_expr()?]))
        };
        let colon: Alternative = |parser| {
            let key: Expression = parser.a_expr()?;
            parser.expect_char(':')?;
            Ok(tuple(vec![key, parser.json_value_expr()?]))
        };
        self.alternatives(&[valued, colon])
    }

    /// `json_value_expr`: `a_expr [FORMAT JSON [ENCODING name]]`.
    pub(crate) fn json_value_expr(&mut self) -> Parsing<Expression> {
        let value: Expression = self.a_expr()?;
        self.json_formatted(value)
    }

    /// `this` with any `json_format_clause`, as a `FORMAT JSON` call; other encodings are 22023.
    fn json_formatted(&mut self, this: Expression) -> Parsing<Expression> {
        if !(self.kw("format") && self.kw_at(1, "json")) {
            return Ok(this);
        }
        self.at += 2;
        if !self.eat_kw("encoding") {
            return Ok(keyword_call(FORMAT_JSON, vec![this], Vec::new()));
        }
        let position: usize = self.offset();
        let encoding: String = self.name(is_col_id)?.name;
        let folded: String = encoding.to_ascii_lowercase();
        match JSON_ENCODINGS.contains(&folded.as_str()) {
            true => Ok(keyword_call(
                FORMAT_JSON,
                vec![this],
                vec![clause("ENCODING", vec![var(&folded.to_uppercase())])],
            )),
            false => Err(ParseError {
                message: format!("unrecognized JSON encoding: {encoding}"),
                position,
                syntax: false,
            }),
        }
    }

    /// `json_returning_clause_opt`: `RETURNING Typename [FORMAT JSON ...]`, as `(RETURNING, type)`.
    fn json_returning(&mut self) -> Parsing<Option<Expression>> {
        if !self.eat_kw("returning") {
            return Ok(None);
        }
        let to: Expression = Expression::DataType(self.typename()?);
        Ok(Some(clause("RETURNING", vec![self.json_formatted(to)?])))
    }

    /// `json_key_uniqueness_constraint_opt`: whether it is `WITH UNIQUE [KEYS]`.
    pub(crate) fn json_key_uniqueness(&mut self) -> bool {
        let with: bool = self.kw("with");
        if (with || self.kw("without")) && self.kw_at(1, "unique") {
            self.at += 2;
            self.eat_kw("keys");
            return with;
        }
        false
    }

    /// `WITH UNIQUE KEYS` as an option, when the uniqueness constraint is true.
    fn json_unique_option(&mut self) -> Vec<Expression> {
        match self.json_key_uniqueness() {
            true => vec![var(UNIQUE_KEYS)],
            false => Vec::new(),
        }
    }

    /// The null clause as written, or gram.y's default (`absent_by_default`) when none is.
    fn json_null_option(&mut self, absent_by_default: bool) -> Vec<Expression> {
        let mut absent: bool = absent_by_default;
        if (self.kw("null") || self.kw("absent")) && self.kw_at(1, "on") && self.kw_at(2, "null") {
            absent = self.kw("absent");
            self.at += 3;
        }
        match absent {
            true => vec![var(ABSENT_ON_NULL)],
            false => Vec::new(),
        }
    }

    /// The context item, `,`, the path and any `PASSING` of the query functions.
    fn json_path_head(&mut self) -> Parsing<(Vec<Expression>, Vec<Expression>)> {
        let context: Expression = self.json_value_expr()?;
        self.expect_char(',')?;
        let path: Expression = self.a_expr()?;
        Ok((
            vec![context, path],
            self.json_passing()?.into_iter().collect(),
        ))
    }

    /// `PASSING json_value_expr AS ColLabel, ...`, as `(PASSING, (value, name), ...)`.
    fn json_passing(&mut self) -> Parsing<Option<Expression>> {
        if !self.eat_kw("passing") {
            return Ok(None);
        }
        let arguments: Vec<Expression> = self.comma_list(|parser| {
            let value: Expression = parser.json_value_expr()?;
            parser.expect_kw("as")?;
            let name: Identifier = identifier(&parser.expect_token(is_col_label)?);
            Ok(tuple(vec![value, Expression::Identifier(name)]))
        })?;
        Ok(Some(clause("PASSING", arguments)))
    }

    /// `json_wrapper_behavior`, as `(WRAPPER, NONE | UNCONDITIONAL | CONDITIONAL)`; `ARRAY` is noise.
    fn json_wrapper(&mut self) -> Parsing<Option<Expression>> {
        let kind: &str = if self.eat_kw("without") {
            "NONE"
        } else if self.eat_kw("with") {
            match self.eat_kw("conditional") {
                true => "CONDITIONAL",
                false => {
                    self.eat_kw("unconditional");
                    "UNCONDITIONAL"
                }
            }
        } else {
            return Ok(None);
        };
        self.eat_kw("array");
        self.expect_kw("wrapper")?;
        Ok(Some(clause("WRAPPER", vec![var(kind)])))
    }

    /// `json_quotes_clause_opt`, as `(QUOTES, KEEP | OMIT)`; `ON SCALAR STRING` is noise.
    fn json_quotes(&mut self) -> Parsing<Option<Expression>> {
        let kind: &str = match self.keyword_at(0) {
            Some("keep") => "KEEP",
            Some("omit") => "OMIT",
            _ => return Ok(None),
        };
        self.at += 1;
        self.expect_kw("quotes")?;
        if self.eat_kw("on") {
            self.expect_kw("scalar")?;
            self.expect_kw("string")?;
        }
        Ok(Some(clause("QUOTES", vec![var(kind)])))
    }

    /// `json_behavior`, when one is next: `(DEFAULT, a_expr)` or its type; `EMPTY` is `EMPTY ARRAY`.
    fn json_behavior(&mut self) -> Parsing<Option<Expression>> {
        let kind: &str = match self.keyword_at(0) {
            Some("default") => {
                self.at += 1;
                return Ok(Some(clause("DEFAULT", vec![self.a_expr()?])));
            }
            Some("error") => "ERROR",
            Some("null") => "NULL",
            Some("true") => "TRUE",
            Some("false") => "FALSE",
            Some("unknown") => "UNKNOWN",
            Some("empty") => {
                self.at += 1;
                return Ok(Some(var(match self.eat_kw("object") {
                    true => "EMPTY OBJECT",
                    false => {
                        self.eat_kw("array");
                        "EMPTY ARRAY"
                    }
                })));
            }
            _ => return Ok(None),
        };
        self.at += 1;
        Ok(Some(var(kind)))
    }

    /// `json_behavior_clause_opt`: `(ON EMPTY, b)`, `(ON ERROR, b)`, or both in that order.
    fn json_behaviors(&mut self) -> Parsing<Vec<Expression>> {
        let Some(first) = self.json_behavior()? else {
            return Ok(Vec::new());
        };
        self.expect_kw("on")?;
        if self.eat_kw("error") {
            return Ok(vec![clause("ON ERROR", vec![first])]);
        }
        self.expect_kw("empty")?;
        let mut behaviors: Vec<Expression> = vec![clause("ON EMPTY", vec![first])];
        behaviors.extend(self.json_on_error()?);
        Ok(behaviors)
    }

    /// `json_on_error_clause_opt`: `(ON ERROR, b)`.
    fn json_on_error(&mut self) -> Parsing<Option<Expression>> {
        let Some(behavior) = self.json_behavior()? else {
            return Ok(None);
        };
        self.expect_kw("on")?;
        self.expect_kw("error")?;
        Ok(Some(clause("ON ERROR", vec![behavior])))
    }

    /// `( json_table_column_definition, ... )`.
    fn json_table_columns(&mut self) -> Parsing<Vec<Expression>> {
        self.parenthesized(|parser| parser.json_table_column())
    }

    /// `json_table_column_definition`: each column kind as a `KeywordCall` of its name, type and clauses.
    fn json_table_column(&mut self) -> Parsing<Expression> {
        if self.kw("nested") && (self.sconst_at(1) || self.kw_at(1, "path")) {
            self.at += 1;
            self.eat_kw("path");
            let path: Expression = self.sconst()?;
            let mut options: Vec<Expression> = Vec::new();
            if self.eat_kw("as") {
                options.push(clause(
                    "AS",
                    vec![Expression::Identifier(self.name(is_col_id)?)],
                ));
            }
            self.expect_kw("columns")?;
            options.push(clause("COLUMNS", self.json_table_columns()?));
            return Ok(keyword_call("NESTED", vec![path], options));
        }
        let name: Expression = Expression::Identifier(self.name(is_col_id)?);
        if self.kw("for") && self.kw_at(1, "ordinality") {
            self.at += 2;
            return Ok(keyword_call("FOR ORDINALITY", vec![name], Vec::new()));
        }
        let to: Expression = Expression::DataType(self.typename()?);
        if self.eat_kw("exists") {
            let mut options: Vec<Expression> = self.json_column_path()?.into_iter().collect();
            options.extend(self.json_on_error()?);
            return Ok(keyword_call("EXISTS", vec![name, to], options));
        }
        let to: Expression = self.json_formatted(to)?;
        let mut options: Vec<Expression> = self.json_column_path()?.into_iter().collect();
        options.extend(self.json_wrapper()?);
        options.extend(self.json_quotes()?);
        options.extend(self.json_behaviors()?);
        Ok(keyword_call("COLUMN", vec![name, to], options))
    }

    /// `json_table_column_path_clause_opt`: `(PATH, Sconst)`.
    fn json_column_path(&mut self) -> Parsing<Option<Expression>> {
        if !self.eat_kw("path") {
            return Ok(None);
        }
        Ok(Some(clause("PATH", vec![self.sconst()?])))
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

/// The string constant inside any parentheses, which gram.y's raw tree does not keep.
fn unparenthesized(this: Expression) -> Expression {
    match this {
        Expression::Paren(paren) => unparenthesized(paren.this),
        other => other,
    }
}
