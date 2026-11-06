//! `CommentStmt` (gram.y): `COMMENT ON` an object, named as its kind's grammar names it.

use sqlquack_syntax::expressions::columns::models::ColumnDef;
use sqlquack_syntax::expressions::functions::models::{Operator, WithinGroup};
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::DotAccess;
use sqlquack_syntax::expressions::statements::models::{Comment, Var, Variadic};
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::{DataType, Expression};

use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, identifier, is_col_id, is_col_label, is_type_function_name,
};
use crate::parse::_helpers::tree::nodes::{function, null, ordered, star};
use crate::parse::_helpers::values::expressions::is_operator_token;
use crate::scan::models::{Token, TokenKind};

/// `object_type_any_name`: kinds named by `any_name`, longest spelling first.
const ANY_NAME_KINDS: [&[&str]; 13] = [
    &["materialized", "view"],
    &["foreign", "table"],
    &["text", "search", "parser"],
    &["text", "search", "dictionary"],
    &["text", "search", "template"],
    &["text", "search", "configuration"],
    &["table"],
    &["sequence"],
    &["view"],
    &["index"],
    &["collation"],
    &["conversion"],
    &["statistics"],
];

/// `object_type_name` with `drop_type_name`: kinds named by one `name`, longest spelling first.
const NAME_KINDS: [&[&str]; 13] = [
    &["access", "method"],
    &["event", "trigger"],
    &["foreign", "data", "wrapper"],
    &["procedural", "language"],
    &["extension"],
    &["language"],
    &["publication"],
    &["schema"],
    &["server"],
    &["database"],
    &["role"],
    &["subscription"],
    &["tablespace"],
];

/// `object_type_name_on_any_name`: kinds named by `name ON any_name`.
const ON_TABLE_KINDS: [&str; 3] = ["policy", "rule", "trigger"];

/// Kinds named by `function_with_argtypes`.
const ROUTINE_KINDS: [&str; 3] = ["function", "procedure", "routine"];

/// `arg_class` keywords; a mode other than `VARIADIC` pairs with its argument in a tuple.
const ARGUMENT_MODES: [&str; 4] = ["variadic", IN_MODE, OUT_MODE, INOUT_MODE];

/// The `arg_class` that `OUT` may follow, as `IN OUT`, which means `INOUT`.
const IN_MODE: &str = "in";

const OUT_MODE: &str = "out";

const INOUT_MODE: &str = "inout";

/// `VARIADIC` as `argument_class` gives it, which becomes a Variadic node.
const VARIADIC_MODE: &str = "VARIADIC";

/// `a.b.c` as an identifier with field accesses.
fn dotted(parts: Vec<Identifier>) -> Expression {
    let mut parts = parts.into_iter();
    let first: Identifier = parts.next().unwrap_or_else(Identifier::empty);
    parts.fold(Expression::Identifier(first), |this, field| {
        Expression::Dot(Box::new(DotAccess {
            this,
            field,
            inferred_type: None,
        }))
    })
}

fn tuple(expressions: Vec<Expression>) -> Expression {
    Expression::Tuple(Box::new(Tuple { expressions }))
}

fn kind_text(words: &[&str]) -> String {
    words.join(" ").to_uppercase()
}

impl Parser<'_> {
    /// `COMMENT ON object IS Sconst | NULL`.
    pub(crate) fn comment_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("comment")?;
        self.expect_kw("on")?;
        let (kind, this): (String, Expression) = self.commented_object()?;
        self.expect_kw("is")?;
        let expression: Expression = match self.eat_kw("null") {
            true => null(),
            false => self.sconst()?,
        };
        Ok(Expression::Comment(Box::new(Comment {
            this,
            materialized: kind.starts_with("MATERIALIZED"),
            kind,
            expression,
            exists: false,
        })))
    }

    /// Whether the keywords `words` come next.
    fn words_at(&self, words: &[&str]) -> bool {
        words
            .iter()
            .enumerate()
            .all(|(offset, word)| self.kw_at(offset, word))
    }

    /// The first of `kinds` whose keywords come next, consumed.
    fn eat_kind(&mut self, kinds: &[&[&str]]) -> Option<String> {
        let words: &[&str] = kinds.iter().find(|words| self.words_at(words))?;
        self.at += words.len();
        Some(kind_text(words))
    }

    /// The object of `COMMENT ON` and its kind.
    fn commented_object(&mut self) -> Parsing<(String, Expression)> {
        if let Some(kind) = self.eat_kind(&ANY_NAME_KINDS) {
            return Ok((kind, dotted(self.qualified_name()?)));
        }
        let operator_set: bool =
            self.words_at(&["operator", "class"]) || self.words_at(&["operator", "family"]);
        if operator_set && !self.char_at(2, '.') {
            let kind: String = kind_text(&["operator", &self.tokens[self.at + 1].value]);
            self.at += 2;
            let name: Expression = dotted(self.qualified_name()?);
            self.expect_kw("using")?;
            let method: Identifier = self.name(is_col_id)?;
            return Ok((kind, tuple(vec![name, Expression::Identifier(method)])));
        }
        if self.words_at(&["large", "object"]) {
            self.at += 2;
            return Ok((kind_text(&["large", "object"]), self.numeric_only()?));
        }
        if let Some(kind) = self.eat_kind(&NAME_KINDS) {
            return Ok((kind, Expression::Identifier(self.name(is_col_id)?)));
        }
        let Some(word) = self.keyword_at(0) else {
            return Err(self.error());
        };
        self.at += 1;
        let kind: String = word.to_uppercase();
        let this: Expression = match word {
            "column" => dotted(self.qualified_name()?),
            "type" | "domain" => Expression::DataType(self.typename()?),
            "aggregate" => self.aggregate_with_argtypes()?,
            "operator" => self.operator_with_argtypes()?,
            "constraint" => {
                let name: Identifier = self.name(is_col_id)?;
                self.expect_kw("on")?;
                let target: Expression = match self.eat_kw("domain") {
                    true => Expression::DataType(DataType::Custom {
                        name: self.qualified_name_text()?,
                    }),
                    false => dotted(self.qualified_name()?),
                };
                tuple(vec![Expression::Identifier(name), target])
            }
            "transform" => {
                self.expect_kw("for")?;
                let data_type: DataType = self.typename()?;
                self.expect_kw("language")?;
                let language: Identifier = self.name(is_col_id)?;
                tuple(vec![
                    Expression::DataType(data_type),
                    Expression::Identifier(language),
                ])
            }
            "cast" => {
                self.expect_char('(')?;
                let source: DataType = self.typename()?;
                self.expect_kw("as")?;
                let target: DataType = self.typename()?;
                self.expect_char(')')?;
                tuple(vec![
                    Expression::DataType(source),
                    Expression::DataType(target),
                ])
            }
            _ if ON_TABLE_KINDS.contains(&word) => {
                let name: Identifier = self.name(is_col_id)?;
                self.expect_kw("on")?;
                tuple(vec![
                    Expression::Identifier(name),
                    dotted(self.qualified_name()?),
                ])
            }
            _ if ROUTINE_KINDS.contains(&word) => self.function_with_argtypes()?,
            _ => {
                self.at -= 1;
                return Err(self.error());
            }
        };
        Ok((kind, this))
    }

    /// `any_name` as one dotted text, for a name read as a type.
    fn qualified_name_text(&mut self) -> Parsing<String> {
        Ok(self
            .qualified_name()?
            .iter()
            .map(Identifier::to_string)
            .collect::<Vec<String>>()
            .join("."))
    }

    /// `func_name`: a `type_function_name`, or a `ColId` with `.ColLabel` parts.
    fn func_name(&mut self) -> Parsing<Vec<Identifier>> {
        let qualified: bool = self.char_at(1, '.');
        match qualified {
            true => self.qualified_name(),
            false => Ok(vec![self.name(is_type_function_name)?]),
        }
    }

    /// `function_with_argtypes`: a name with `func_args`, or a bare name.
    fn function_with_argtypes(&mut self) -> Parsing<Expression> {
        let bare: bool = !self.char_at(1, '.') && !self.char_at(1, '(');
        if bare {
            let token: Token =
                self.expect_token(|token| is_col_id(token) || is_type_function_name(token))?;
            return Ok(Expression::Identifier(identifier(&token)));
        }
        let name: Vec<Identifier> = self.func_name()?;
        if !self.is_char('(') {
            return Ok(dotted(name));
        }
        self.expect_char('(')?;
        let args: Vec<Expression> = match self.is_char(')') {
            true => Vec::new(),
            false => self.comma_list(|parser| parser.func_arg())?,
        };
        self.expect_char(')')?;
        Ok(Expression::Function(Box::new(function(name, args))))
    }

    /// `aggregate_with_argtypes`: `func_name aggr_args`.
    fn aggregate_with_argtypes(&mut self) -> Parsing<Expression> {
        let name: Vec<Identifier> = self.func_name()?;
        self.expect_char('(')?;
        if self.eat_char('*') {
            self.expect_char(')')?;
            return Ok(Expression::Function(Box::new(function(
                name,
                vec![star(None)],
            ))));
        }
        let direct: Vec<Expression> = match self.kw("order") {
            true => Vec::new(),
            false => self.comma_list(|parser| parser.func_arg())?,
        };
        let call: Expression = Expression::Function(Box::new(function(name, direct)));
        if !self.eat_kw("order") {
            self.expect_char(')')?;
            return Ok(call);
        }
        self.expect_kw("by")?;
        let sorted: Vec<Expression> = self.comma_list(|parser| parser.func_arg())?;
        self.expect_char(')')?;
        Ok(Expression::WithinGroup(Box::new(WithinGroup {
            this: call,
            order_by: sorted
                .into_iter()
                .map(|argument| ordered(argument, false, false, None))
                .collect(),
        })))
    }

    /// `operator_with_argtypes`: `any_operator ( left , right )`, where one side may be `NONE`.
    fn operator_with_argtypes(&mut self) -> Parsing<Expression> {
        let mut name: String = String::new();
        while self.is(is_col_id) && self.char_at(1, '.') {
            name.push_str(&self.advance()?.value);
            name.push('.');
            self.at += 1;
        }
        let operator: Token = self.expect_token(is_operator_token)?;
        name.push_str(&match operator.kind {
            TokenKind::LessEquals => "<=".to_owned(),
            TokenKind::GreaterEquals => ">=".to_owned(),
            TokenKind::NotEquals => "<>".to_owned(),
            TokenKind::Char(character) => character.to_string(),
            _ => operator.value.clone(),
        });
        self.expect_char('(')?;
        let left: Option<DataType> = self.operator_side()?;
        let right: Option<DataType> = match self.eat_char(',') {
            true => self.operator_side()?,
            false => None,
        };
        self.expect_char(')')?;
        let side = |side: Option<DataType>| Box::new(side.map_or_else(null, Expression::DataType));
        Ok(Expression::Operator(Box::new(Operator {
            this: side(left),
            operator: Some(Box::new(Expression::Identifier(Identifier::new(name)))),
            expression: side(right),
            comments: Vec::new(),
        })))
    }

    /// One `oper_argtypes` side: a `Typename`, or `NONE` for no operand.
    fn operator_side(&mut self) -> Parsing<Option<DataType>> {
        match self.eat_kw("none") {
            true => Ok(None),
            false => self.typename().map(Some),
        }
    }

    /// `func_arg`: `[arg_class] [param_name] func_type`; a named argument is a column definition.
    fn func_arg(&mut self) -> Parsing<Expression> {
        let leading: Option<String> = self.argument_class();
        let named = self.attempt(|parser| {
            let name: Identifier = parser.name(is_type_function_name)?;
            let trailing: Option<String> = match leading {
                Some(_) => None,
                None => parser.argument_class(),
            };
            let data_type: DataType = parser.func_type()?;
            match parser.is_char(',') || parser.is_char(')') || parser.kw("order") {
                true => Ok((name, trailing, data_type)),
                false => Err(parser.error()),
            }
        });
        let (argument, mode): (Expression, Option<String>) = match named {
            Ok((name, trailing, data_type)) => {
                let mut definition: ColumnDef = ColumnDef::new(name.name.clone(), data_type);
                definition.name = name;
                (
                    Expression::ColumnDef(Box::new(definition)),
                    leading.or(trailing),
                )
            }
            Err(_) => (Expression::DataType(self.func_type()?), leading),
        };
        Ok(match mode.as_deref() {
            None => argument,
            Some(VARIADIC_MODE) => Expression::Variadic(Box::new(Variadic {
                this: Box::new(argument),
            })),
            Some(mode) => tuple(vec![
                Expression::Var(Box::new(Var {
                    this: mode.to_owned(),
                })),
                argument,
            ]),
        })
    }

    /// An `arg_class`, if next: `VARIADIC`, `IN`, `OUT`, `INOUT`, or `IN OUT` as `INOUT`.
    fn argument_class(&mut self) -> Option<String> {
        let mode: &str = ARGUMENT_MODES.iter().find(|mode| self.kw(mode))?;
        self.at += 1;
        Some(match mode == IN_MODE && self.eat_kw(OUT_MODE) {
            true => INOUT_MODE.to_uppercase(),
            false => mode.to_uppercase(),
        })
    }

    /// `func_type`: a `Typename`, or `[SETOF] name.attr...%TYPE`, which is a gap.
    fn func_type(&mut self) -> Parsing<DataType> {
        let start: usize = self.offset();
        let setof: usize = usize::from(self.kw("setof"));
        let referenced: bool = self.peek_at(setof).is_some_and(is_type_function_name)
            && self
                .peek_at(setof + 1)
                .is_some_and(|token| token.kind == TokenKind::Char('.'));
        if referenced {
            let reference = self.attempt(|parser| {
                parser.at += setof;
                parser.at += 1;
                while parser.eat_char('.') {
                    parser.expect_token(is_col_label)?;
                }
                parser.expect_char('%')?;
                parser.expect_kw("type")
            });
            if let Ok(()) = reference {
                self.gap("type_reference", start);
                return Ok(DataType::Custom {
                    name: self.sql[start..self.tokens[self.at - 1].end].to_owned(),
                });
            }
        }
        self.typename()
    }
}
