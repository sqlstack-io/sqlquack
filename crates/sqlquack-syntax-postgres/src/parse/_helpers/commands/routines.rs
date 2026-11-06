//! `CreateFunctionStmt` (gram.y): `CREATE [OR REPLACE] FUNCTION | PROCEDURE`, options kept in order.

use sqlquack_syntax::expressions::columns::models::ColumnDef;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::schema_objects::models::{
    CreateFunction, CreateProcedure, FunctionBody, FunctionParameter, FunctionSecurity,
    ParameterMode,
};
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::{DataType, Expression};

use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, is_col_id, is_non_reserved_word, is_type_function_name,
};
use crate::parse::_helpers::tree::nodes::{literal, name_text, table, tuple};
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

/// `set_rest_more` forms other than `generic_set`, each a gap the shared tree cannot hold.
const SPECIAL_SETTINGS: [&str; 7] = [
    "time", "catalog", "schema", "names", "role", "session", "xml",
];

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
    fn word_or_string(&mut self) -> Parsing<Expression> {
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
                    false => option("RESET", vec![self.setting_name()?]),
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(item))
    }

    /// `var_name`: `ColId [. ColId]...`, as one name.
    fn setting_name(&mut self) -> Parsing<Expression> {
        let mut parts: Vec<Identifier> = vec![self.name(is_col_id)?];
        while self.eat_char('.') {
            parts.push(self.name(is_col_id)?);
        }
        Ok(Expression::Identifier(Identifier::new(name_text(&parts))))
    }

    /// `set_rest_more` after `SET`: `generic_set` or `var_name FROM CURRENT`; other forms are gaps.
    fn setting(&mut self) -> Parsing<RoutineOption> {
        let start: usize = self.offset();
        let generic: bool = self.char_at(1, '=')
            || self.char_at(1, '.')
            || self.kw_at(1, "to")
            || self.kw_at(1, "from");
        if !generic
            && self
                .keyword_at(0)
                .is_some_and(|word| SPECIAL_SETTINGS.contains(&word))
        {
            self.special_setting()?;
            self.gap("set_special", start);
            return Ok(option("SET", Vec::new()));
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

    /// The `set_rest_more` forms the shared tree cannot hold, read to their end.
    fn special_setting(&mut self) -> Parsing<()> {
        match self.keyword_at(0).unwrap_or_default() {
            "time" => {
                self.at += 1;
                self.expect_kw("zone")?;
                if self.kw("interval") {
                    self.c_expr(Flavor::B)?;
                } else if !(self.eat_kw("default") || self.eat_kw("local")) {
                    self.setting_value()?;
                }
            }
            "catalog" | "schema" => {
                self.at += 1;
                self.sconst()?;
            }
            "names" => {
                self.at += 1;
                if !self.eat_kw("default") && self.sconst_at(0) {
                    self.sconst()?;
                }
            }
            "role" => {
                self.at += 1;
                self.word_or_string()?;
            }
            "session" => {
                self.at += 1;
                self.expect_kw("authorization")?;
                self.word_or_string()?;
            }
            _ => {
                self.at += 1;
                self.expect_kw("option")?;
                if !self.eat_kw("document") {
                    self.expect_kw("content")?;
                }
            }
        }
        Ok(())
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
                false => self.statement()?,
            };
            statements.push(statement);
            self.expect_char(';')?;
        }
        self.at += 1;
        Ok(Some(FunctionBody::Statements(statements)))
    }
}
