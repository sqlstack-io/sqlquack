//! `CreateFunctionStmt` (gram.y): `CREATE [OR REPLACE] FUNCTION | PROCEDURE`, options kept in order.

use sqlquack_syntax::expressions::columns::models::ColumnDef;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::schema_objects::models::{
    CreateFunction, CreateProcedure, FunctionBody, FunctionParameter, FunctionSecurity,
    ParameterMode,
};
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::{DataType, Expression};

use crate::parse::_helpers::commands::tables::unsupported;
use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, is_col_id, is_non_reserved_word, is_type_function_name,
};
use crate::parse::_helpers::tree::nodes::{boolean, literal, name_text, table, tuple};
use crate::parse::_helpers::values::expressions::Flavor;
use crate::parse::main::format_type::format_type;

/// `createfunc_opt_item` keywords with no value, as `common_func_opt_item` spells them.
const BARE_OPTIONS: [&str; 6] = [
    "strict",
    "immutable",
    "stable",
    "volatile",
    "leakproof",
    "window",
];

/// `set_rest_more` forms other than `generic_set`.
const SPECIAL_SETTINGS: [&str; 8] = [
    "time",
    "catalog",
    "schema",
    "names",
    "role",
    "session",
    "xml",
    "transaction",
];

/// The words that begin a `transaction_mode_item`.
const TRANSACTION_MODE_WORDS: [&str; 4] = ["isolation", "read", "deferrable", "not"];

/// The option `SET TRANSACTION transaction_mode_list` sets, a `VAR_SET_MULTI`.
const TRANSACTION_OPTION: &str = "SET TRANSACTION";

/// The option `SET SESSION CHARACTERISTICS AS TRANSACTION transaction_mode_list` sets.
const SESSION_CHARACTERISTICS_OPTION: &str = "SET SESSION CHARACTERISTICS";

/// `iso_level` spellings, with the value PostgreSQL gives each.
const ISOLATION_LEVELS: [(&[&str], &str); 4] = [
    (&["read", "uncommitted"], "read uncommitted"),
    (&["read", "committed"], "read committed"),
    (&["repeatable", "read"], "repeatable read"),
    (&["serializable"], "serializable"),
];

/// `reset_rest` special forms: their words and the setting PostgreSQL names.
const SPECIAL_RESETS: [(&[&str], &str); 3] = [
    (&["time", "zone"], "timezone"),
    (
        &["transaction", "isolation", "level"],
        "transaction_isolation",
    ),
    (&["session", "authorization"], "session_authorization"),
];

/// The `opt_interval` field words.
const INTERVAL_FIELDS: [&str; 6] = ["year", "month", "day", "hour", "minute", "second"];

/// `createfunc_opt_item` keywords `common_func_opt_item` lacks, which `ALTER FUNCTION` refuses.
const CREATE_ONLY_OPTIONS: [&str; 4] = ["as", "language", "transform", "window"];

/// The option `SET TRANSACTION SNAPSHOT` sets, a `VAR_SET_MULTI` no generic setting spells.
const SNAPSHOT_OPTION: &str = "SET TRANSACTION SNAPSHOT";

/// The option `STRICT` and `RETURNS NULL ON NULL INPUT` both set, as PostgreSQL's raw tree has it.
const STRICT_OPTION: &str = "STRICT";

/// The option `CALLED ON NULL INPUT` sets.
const CALLED_OPTION: &str = "CALLED ON NULL INPUT";

/// The option `[EXTERNAL] SECURITY DEFINER` sets.
const DEFINER_OPTION: &str = "SECURITY DEFINER";

/// The option `[EXTERNAL] SECURITY INVOKER` sets.
const INVOKER_OPTION: &str = "SECURITY INVOKER";

/// The option `NOT LEAKPROOF` sets.
const NOT_LEAKPROOF_OPTION: &str = "NOT LEAKPROOF";

/// The option `LANGUAGE` sets.
const LANGUAGE_OPTION: &str = "LANGUAGE";

/// The option `AS` sets.
const AS_OPTION: &str = "AS";

/// The return marker of `RETURNS SETOF type`.
const SETOF_RETURN: &str = "RETURNS SETOF";

/// The return marker of `RETURNS TABLE (columns)`.
const TABLE_RETURN: &str = "RETURNS TABLE";

/// One `createfunc_opt_item`: its keyword and its values.
struct RoutineOption {
    keyword: String,
    values: Vec<Expression>,
}

fn option(keyword: &str, values: Vec<Expression>) -> RoutineOption {
    RoutineOption {
        keyword: keyword.to_owned(),
        values,
    }
}

pub(crate) fn var(text: &str) -> Expression {
    Expression::Var(Box::new(Var {
        this: text.to_owned(),
    }))
}

fn string(text: String) -> Expression {
    literal(Literal::String(text))
}

/// An option as a tuple of its keyword and values, for `CreateFunction.options`.
fn option_tuple(option: &RoutineOption) -> Expression {
    let mut parts: Vec<Expression> = vec![var(&option.keyword)];
    parts.extend(option.values.iter().cloned());
    tuple(parts)
}

/// A value's canonical text: strings and names quoted, numbers bare, types as `format_type`.
fn value_text(value: &Expression) -> String {
    match value {
        Expression::Literal(literal) => match &**literal {
            Literal::String(text) => format!("'{}'", text.replace('\'', "''")),
            other => other.value_str().to_owned(),
        },
        Expression::Identifier(name) => format!("\"{}\"", name.name.replace('"', "\"\"")),
        Expression::DataType(data_type) => {
            format_type(data_type).unwrap_or_else(|| format!("{data_type:?}"))
        }
        Expression::Var(word) => word.this.clone(),
        other => format!("{other:?}"),
    }
}

/// An option as canonical text, for `CreateProcedure.with_options`.
fn option_text(option: &RoutineOption) -> String {
    let values: Vec<String> = option.values.iter().map(value_text).collect();
    match values.is_empty() {
        true => option.keyword.clone(),
        false => format!("{} {}", option.keyword, values.join(", ")),
    }
}

impl Parser<'_> {
    /// Whether `CREATE [OR REPLACE] FUNCTION | PROCEDURE` comes next.
    pub(crate) fn creates_routine(&self) -> bool {
        let skip: usize = match self.kw_at(1, "or") && self.kw_at(2, "replace") {
            true => 3,
            false => 1,
        };
        self.kw("create") && (self.kw_at(skip, "function") || self.kw_at(skip, "procedure"))
    }

    /// `CREATE [OR REPLACE] FUNCTION | PROCEDURE func_name func_args_with_defaults ...`.
    pub(crate) fn create_routine(&mut self) -> Parsing<Expression> {
        self.expect_kw("create")?;
        let or_replace: bool = self.eat_kw("or");
        if or_replace {
            self.expect_kw("replace")?;
        }
        let procedure: bool = self.eat_kw("procedure");
        if !procedure {
            self.expect_kw("function")?;
        }
        let name = table(self.func_name()?);
        self.expect_char('(')?;
        let parameters: Vec<FunctionParameter> = match self.is_char(')') {
            true => Vec::new(),
            false => self.comma_list(|parser| parser.function_parameter())?,
        };
        self.expect_char(')')?;
        let mut options: Vec<RoutineOption> = Vec::new();
        let mut return_type: Option<DataType> = None;
        if !procedure && self.kw("returns") && !self.kw_at(1, "null") {
            self.at += 1;
            match self.routine_return()? {
                Ok(data_type) => return_type = Some(data_type),
                Err(marker) => options.push(marker),
            }
        }
        while let Some(item) = self.routine_option()? {
            options.push(item);
        }
        let body: Option<FunctionBody> = match self.routine_body()? {
            Some(body) => Some(body),
            None => options
                .iter()
                .rev()
                .find(|item| item.keyword == AS_OPTION)
                .map(|item| match item.values.as_slice() {
                    [Expression::Literal(text)] => {
                        FunctionBody::StringLiteral(text.value_str().to_owned())
                    }
                    values => FunctionBody::Expression(tuple(values.to_vec())),
                }),
        };
        let language: Option<String> = options
            .iter()
            .rev()
            .find(|item| item.keyword == LANGUAGE_OPTION)
            .and_then(|item| item.values.first())
            .map(|value| match value {
                Expression::Literal(text) => text.value_str().to_owned(),
                other => value_text(other),
            });
        let security: Option<FunctionSecurity> =
            options
                .iter()
                .rev()
                .find_map(|item| match item.keyword.as_str() {
                    DEFINER_OPTION => Some(FunctionSecurity::Definer),
                    INVOKER_OPTION => Some(FunctionSecurity::Invoker),
                    _ => None,
                });
        if procedure {
            let mut node: CreateProcedure = CreateProcedure::new(String::new());
            node.name = name;
            node.parameters = parameters;
            node.body = body;
            node.or_replace = or_replace;
            node.language = language;
            node.security = security;
            node.with_options = options.iter().map(option_text).collect();
            return Ok(Expression::CreateProcedure(Box::new(node)));
        }
        let mut node: CreateFunction = CreateFunction::new(String::new());
        node.name = name;
        node.parameters = parameters;
        node.return_type = return_type;
        node.body = body;
        node.or_replace = or_replace;
        node.language = language;
        node.security = security;
        node.returns_null_on_null_input =
            options
                .iter()
                .rev()
                .find_map(|item| match item.keyword.as_str() {
                    STRICT_OPTION => Some(true),
                    CALLED_OPTION => Some(false),
                    _ => None,
                });
        node.options = options.iter().map(option_tuple).collect();
        Ok(Expression::CreateFunction(Box::new(node)))
    }

    /// `func_arg_with_default`: `func_arg [DEFAULT | = a_expr]`.
    fn function_parameter(&mut self) -> Parsing<FunctionParameter> {
        let leading: Option<String> = self.argument_class();
        let named = self.attempt(|parser| {
            let name: Identifier = parser.name(is_type_function_name)?;
            let trailing: Option<String> = match leading {
                Some(_) => None,
                None => parser.argument_class(),
            };
            let data_type: DataType = parser.func_type()?;
            match parser.is_char(',')
                || parser.is_char(')')
                || parser.is_char('=')
                || parser.kw("default")
            {
                true => Ok((name, trailing, data_type)),
                false => Err(parser.error()),
            }
        });
        let (name, mode, data_type) = match named {
            Ok((name, trailing, data_type)) => (Some(name), leading.or(trailing), data_type),
            Err(_) => (None, leading, self.func_type()?),
        };
        let default: Option<Expression> = match self.eat_kw("default") || self.eat_char('=') {
            true => Some(self.a_expr()?),
            false => None,
        };
        Ok(FunctionParameter {
            name,
            data_type,
            mode: mode.as_deref().map(|mode| match mode {
                "IN" => ParameterMode::In,
                "OUT" => ParameterMode::Out,
                "INOUT" => ParameterMode::InOut,
                _ => ParameterMode::Variadic,
            }),
            default,
            mode_text: None,
        })
    }

    /// `func_return`, or `TABLE (columns)`: a type, or the marker option a shared type lacks.
    fn routine_return(&mut self) -> Parsing<Result<DataType, RoutineOption>> {
        if self.kw("table") && self.char_at(1, '(') {
            self.at += 1;
            let columns: Vec<Expression> = self.parenthesized(|parser| {
                let name: Identifier = parser.name(is_type_function_name)?;
                let data_type: DataType = parser.func_type()?;
                let mut column: ColumnDef = ColumnDef::new(name.name.clone(), data_type);
                column.name = name;
                Ok(Expression::ColumnDef(Box::new(column)))
            })?;
            return Ok(Err(option(TABLE_RETURN, columns)));
        }
        if self.eat_kw("setof") {
            let data_type: DataType = self.func_type()?;
            return Ok(Err(option(
                SETOF_RETURN,
                vec![Expression::DataType(data_type)],
            )));
        }
        Ok(Ok(self.func_type()?))
    }

    /// `NonReservedWord_or_Sconst` as a string.
    pub(crate) fn word_or_string(&mut self) -> Parsing<Expression> {
        if self.sconst_at(0) {
            return self.sconst();
        }
        let word = self.expect_token(is_non_reserved_word)?;
        Ok(string(
            crate::parse::_helpers::tree::cursor::identifier(&word).name,
        ))
    }

    /// One `createfunc_opt_item`, or `None` when none comes next.
    fn routine_option(&mut self) -> Parsing<Option<RoutineOption>> {
        let Some(word) = self.keyword_at(0) else {
            return Ok(None);
        };
        let item: RoutineOption = match word {
            _ if BARE_OPTIONS.contains(&word) => {
                self.at += 1;
                option(&word.to_uppercase(), Vec::new())
            }
            "not" if self.kw_at(1, "leakproof") => {
                self.at += 2;
                option(NOT_LEAKPROOF_OPTION, Vec::new())
            }
            "called" => {
                self.at += 1;
                self.expect_kw("on")?;
                self.expect_kw("null")?;
                self.expect_kw("input")?;
                option(CALLED_OPTION, Vec::new())
            }
            "returns" if self.kw_at(1, "null") => {
                self.at += 2;
                self.expect_kw("on")?;
                self.expect_kw("null")?;
                self.expect_kw("input")?;
                option(STRICT_OPTION, Vec::new())
            }
            "external" | "security" => {
                if self.eat_kw("external") {
                    self.expect_kw("security")?;
                } else {
                    self.at += 1;
                }
                match self.eat_kw("definer") {
                    true => option(DEFINER_OPTION, Vec::new()),
                    false => {
                        self.expect_kw("invoker")?;
                        option(INVOKER_OPTION, Vec::new())
                    }
                }
            }
            "cost" | "rows" => {
                self.at += 1;
                option(&word.to_uppercase(), vec![self.numeric_only()?])
            }
            "support" => {
                self.at += 1;
                let name: Vec<Identifier> = self.qualified_name()?;
                option(
                    "SUPPORT",
                    vec![Expression::Identifier(Identifier::new(name_text(&name)))],
                )
            }
            "parallel" => {
                self.at += 1;
                option(
                    "PARALLEL",
                    vec![Expression::Identifier(self.name(is_col_id)?)],
                )
            }
            "language" => {
                self.at += 1;
                option(LANGUAGE_OPTION, vec![self.word_or_string()?])
            }
            "as" => {
                self.at += 1;
                let mut values: Vec<Expression> = vec![self.sconst()?];
                if self.eat_char(',') {
                    values.push(self.sconst()?);
                }
                option(AS_OPTION, values)
            }
            "transform" => {
                self.at += 1;
                let types: Vec<Expression> = self.comma_list(|parser| {
                    parser.expect_kw("for")?;
                    parser.expect_kw("type")?;
                    Ok(Expression::DataType(parser.typename()?))
                })?;
                option("TRANSFORM", types)
            }
            "set" => {
                self.at += 1;
                self.setting()?
            }
            "reset" => {
                self.at += 1;
                match self.eat_kw("all") {
                    true => option("RESET ALL", Vec::new()),
                    false => option("RESET", vec![self.reset_name()?]),
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(item))
    }

    /// `var_name`: `ColId [. ColId]...`, as one name.
    pub(crate) fn setting_name(&mut self) -> Parsing<Expression> {
        let mut parts: Vec<Identifier> = vec![self.name(is_col_id)?];
        while self.eat_char('.') {
            parts.push(self.name(is_col_id)?);
        }
        Ok(Expression::Identifier(Identifier::new(name_text(&parts))))
    }

    /// `set_rest_more` after `SET`: `generic_set`, `var_name FROM CURRENT` or a special form.
    fn setting(&mut self) -> Parsing<RoutineOption> {
        let generic: bool = self.char_at(1, '=')
            || self.char_at(1, '.')
            || self.kw_at(1, "to")
            || self.kw_at(1, "from");
        if !generic
            && self
                .keyword_at(0)
                .is_some_and(|word| SPECIAL_SETTINGS.contains(&word))
        {
            return self.special_setting();
        }
        let name: Expression = self.setting_name()?;
        if self.eat_kw("from") {
            self.expect_kw("current")?;
            return Ok(option("SET FROM CURRENT", vec![name]));
        }
        if !(self.eat_kw("to") || self.eat_char('=')) {
            return Err(self.error());
        }
        if self.eat_kw("default") {
            return Ok(option("SET DEFAULT", vec![name]));
        }
        let mut values: Vec<Expression> = vec![name];
        values.extend(self.comma_list(|parser| parser.setting_value())?);
        Ok(option("SET", values))
    }

    /// `var_value`: `opt_boolean_or_string` or `NumericOnly`.
    fn setting_value(&mut self) -> Parsing<Expression> {
        let number: bool = self.peek().is_some_and(|token| {
            matches!(
                token.kind,
                crate::scan::models::TokenKind::Integer
                    | crate::scan::models::TokenKind::Numeric
                    | crate::scan::models::TokenKind::Char('+' | '-')
            )
        });
        match number {
            true => self.numeric_only(),
            false => self.boolean_or_string(),
        }
    }

    /// A special `set_rest_more` form, as the generic setting PostgreSQL's raw tree makes of it.
    fn special_setting(&mut self) -> Parsing<RoutineOption> {
        let word: &str = self.keyword_at(0).unwrap_or_default();
        self.at += 1;
        let (name, value): (&str, Option<Expression>) = match word {
            "time" => {
                self.expect_kw("zone")?;
                ("timezone", self.zone_value()?)
            }
            "catalog" => {
                let start: usize = self.offset();
                self.sconst()?;
                return Err(unsupported(
                    "current database cannot be changed".to_owned(),
                    start,
                ));
            }
            "schema" => ("search_path", Some(self.sconst()?)),
            "names" => match self.sconst_at(0) {
                true => ("client_encoding", Some(self.sconst()?)),
                false => {
                    self.eat_kw("default");
                    ("client_encoding", None)
                }
            },
            "role" => ("role", Some(self.word_or_string()?)),
            "session" => {
                self.expect_kw("authorization")?;
                match self.eat_kw("default") {
                    true => ("session_authorization", None),
                    false => ("session_authorization", Some(self.word_or_string()?)),
                }
            }
            "transaction" => {
                self.expect_kw("snapshot")?;
                return Ok(option(SNAPSHOT_OPTION, vec![self.sconst()?]));
            }
            _ => {
                self.expect_kw("option")?;
                let mode: &str = match self.eat_kw("document") {
                    true => "DOCUMENT",
                    false => {
                        self.expect_kw("content")?;
                        "CONTENT"
                    }
                };
                ("xmloption", Some(string(mode.to_owned())))
            }
        };
        let name: Expression = Expression::Identifier(Identifier::new(name));
        Ok(match value {
            Some(value) => option("SET", vec![name, value]),
            None => option("SET DEFAULT", vec![name]),
        })
    }

    /// `reset_rest` other than `ALL`: a `var_name`, or a special form as the name PostgreSQL gives it.
    fn reset_name(&mut self) -> Parsing<Expression> {
        let special: Option<&(&[&str], &str)> = SPECIAL_RESETS
            .iter()
            .find(|(words, _)| self.kw(words[0]) && self.kw_at(1, words[1]));
        let Some((words, name)) = special else {
            return self.setting_name();
        };
        for word in words.iter() {
            self.expect_kw(word)?;
        }
        Ok(Expression::Identifier(Identifier::new(*name)))
    }

    /// Whether `INTERVAL 'text'` here ends in fields beyond `HOUR` and `MINUTE`, which gram.y refuses.
    fn zone_interval_refused(&self) -> bool {
        let fielded: bool = self.sconst_at(1)
            && self
                .keyword_at(2)
                .is_some_and(|word| INTERVAL_FIELDS.contains(&word));
        let hour_or_minute: bool = (self.kw_at(2, "hour") || self.kw_at(2, "minute"))
            && (!self.kw_at(3, "to") || self.kw_at(4, "minute"));
        fielded && !hour_or_minute
    }

    /// `zone_value`: a string, an identifier, an interval or a number; `None` for `DEFAULT` or `LOCAL`.
    fn zone_value(&mut self) -> Parsing<Option<Expression>> {
        if self.eat_kw("default") || self.eat_kw("local") {
            return Ok(None);
        }
        if self.kw("interval") {
            if self.zone_interval_refused() {
                self.at += 2;
                return Err(self.error_with(
                    "time zone interval must be HOUR or HOUR TO MINUTE",
                    self.offset(),
                ));
            }
            return Ok(Some(self.c_expr(Flavor::B)?));
        }
        if self.sconst_at(0) {
            return Ok(Some(self.sconst()?));
        }
        let identifier: bool = self.peek().is_some_and(|token| {
            matches!(
                token.kind,
                crate::scan::models::TokenKind::Ident | crate::scan::models::TokenKind::QuotedIdent
            )
        });
        match identifier {
            true => {
                let token = self.advance()?;
                Ok(Some(string(
                    crate::parse::_helpers::tree::cursor::identifier(&token).name,
                )))
            }
            false => Ok(Some(self.numeric_only()?)),
        }
    }

    /// `SET set_rest` or `RESET reset_rest` as one option tuple; `set_rest` adds the transaction forms.
    pub(crate) fn set_reset_clause(&mut self) -> Parsing<Expression> {
        let modes: bool = self.kw("set")
            && self.kw_at(1, "transaction")
            && TRANSACTION_MODE_WORDS
                .iter()
                .any(|word| self.kw_at(2, word));
        if modes {
            self.at += 2;
            let values: Vec<Expression> = self.transaction_modes()?;
            return Ok(option_tuple(&option(TRANSACTION_OPTION, values)));
        }
        if self.kw("set") && self.kw_at(1, "session") && self.kw_at(2, "characteristics") {
            self.at += 3;
            self.expect_kw("as")?;
            self.expect_kw("transaction")?;
            let values: Vec<Expression> = self.transaction_modes()?;
            return Ok(option_tuple(&option(
                SESSION_CHARACTERISTICS_OPTION,
                values,
            )));
        }
        if !(self.kw("set") || self.kw("reset")) {
            return Err(self.error());
        }
        match self.routine_option()? {
            Some(item) => Ok(option_tuple(&item)),
            None => Err(self.error()),
        }
    }

    /// `set_rest` after `SET [LOCAL | SESSION]`, as the option tuple `set_reset_clause` makes of it.
    pub(crate) fn set_rest(&mut self) -> Parsing<Expression> {
        let modes: bool = self.kw("transaction")
            && TRANSACTION_MODE_WORDS
                .iter()
                .any(|word| self.kw_at(1, word));
        if modes {
            self.at += 1;
            let values: Vec<Expression> = self.transaction_modes()?;
            return Ok(option_tuple(&option(TRANSACTION_OPTION, values)));
        }
        if self.kw("session") && self.kw_at(1, "characteristics") {
            self.at += 2;
            self.expect_kw("as")?;
            self.expect_kw("transaction")?;
            let values: Vec<Expression> = self.transaction_modes()?;
            return Ok(option_tuple(&option(
                SESSION_CHARACTERISTICS_OPTION,
                values,
            )));
        }
        Ok(option_tuple(&self.setting()?))
    }

    /// `SET generic_set` or `RESET generic_reset` as one option tuple, shaped as `set_reset_clause` shapes them.
    pub(crate) fn generic_set_reset(&mut self) -> Parsing<Expression> {
        if self.eat_kw("reset") {
            let item: RoutineOption = match self.eat_kw("all") {
                true => option("RESET ALL", Vec::new()),
                false => option("RESET", vec![self.setting_name()?]),
            };
            return Ok(option_tuple(&item));
        }
        self.expect_kw("set")?;
        let name: Expression = self.setting_name()?;
        if !(self.eat_kw("to") || self.eat_char('=')) {
            return Err(self.error());
        }
        if self.eat_kw("default") {
            return Ok(option_tuple(&option("SET DEFAULT", vec![name])));
        }
        let mut values: Vec<Expression> = vec![name];
        values.extend(self.comma_list(|parser| parser.setting_value())?);
        Ok(option_tuple(&option("SET", values)))
    }

    /// `transaction_mode_list`: modes, with or without commas, each as its setting and value.
    pub(crate) fn transaction_modes(&mut self) -> Parsing<Vec<Expression>> {
        let mut modes: Vec<Expression> = vec![self.transaction_mode()?];
        loop {
            let comma: bool = self.eat_char(',');
            let next: bool = TRANSACTION_MODE_WORDS.iter().any(|word| self.kw(word));
            match (comma, next) {
                (true, _) | (false, true) => modes.push(self.transaction_mode()?),
                (false, false) => return Ok(modes),
            }
        }
    }

    /// One `transaction_mode_item`, as `(setting, value)`.
    fn transaction_mode(&mut self) -> Parsing<Expression> {
        let (name, value): (&str, Expression) = match self.keyword_at(0) {
            Some("isolation") => {
                self.at += 1;
                self.expect_kw("level")?;
                let level: Option<&(&[&str], &str)> = ISOLATION_LEVELS
                    .iter()
                    .find(|(words, _)| self.words_at(words));
                let Some((words, level)) = level else {
                    return Err(self.error());
                };
                self.at += words.len();
                ("transaction_isolation", string((*level).to_owned()))
            }
            Some("read") => {
                self.at += 1;
                let only: bool = self.eat_kw("only");
                if !only {
                    self.expect_kw("write")?;
                }
                ("transaction_read_only", boolean(only))
            }
            Some("not") => {
                self.at += 1;
                self.expect_kw("deferrable")?;
                ("transaction_deferrable", boolean(false))
            }
            _ => {
                self.expect_kw("deferrable")?;
                ("transaction_deferrable", boolean(true))
            }
        };
        Ok(tuple(vec![
            Expression::Identifier(Identifier::new(name)),
            value,
        ]))
    }

    /// `alterfunc_opt_list`: one or more `common_func_opt_item`, as option tuples.
    pub(crate) fn common_routine_options(&mut self) -> Parsing<Vec<Expression>> {
        let mut options: Vec<Expression> = Vec::new();
        while !self
            .keyword_at(0)
            .is_some_and(|word| CREATE_ONLY_OPTIONS.contains(&word))
        {
            let Some(item) = self.routine_option()? else {
                break;
            };
            options.push(option_tuple(&item));
        }
        match options.is_empty() {
            true => Err(self.error()),
            false => Ok(options),
        }
    }

    /// `opt_routine_body`: `RETURN a_expr`, or `BEGIN ATOMIC stmt; ... END`.
    fn routine_body(&mut self) -> Parsing<Option<FunctionBody>> {
        if self.eat_kw("return") {
            return Ok(Some(FunctionBody::Return(self.a_expr()?)));
        }
        if !(self.kw("begin") && self.kw_at(1, "atomic")) {
            return Ok(None);
        }
        self.at += 2;
        let mut statements: Vec<Expression> = Vec::new();
        while !self.kw("end") {
            if self.eat_char(';') {
                continue;
            }
            let statement: Expression = match self.eat_kw("return") {
                true => Expression::ReturnStmt(Box::new(self.a_expr()?)),
                false => self.deeper(|parser| parser.statement())?,
            };
            statements.push(statement);
            self.expect_char(';')?;
        }
        self.at += 1;
        Ok(Some(FunctionBody::Statements(statements)))
    }
}
