//! `CommentStmt` (gram.y): `COMMENT ON` an object, named as its kind's grammar names it.

use sqlquack_syntax::expressions::columns::models::ColumnDef;
use sqlquack_syntax::expressions::functions::models::{Operator, WithinGroup};
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::operators::models::DotAccess;
use sqlquack_syntax::expressions::statements::models::{Comment, Var, Variadic};
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::{DataType, Expression};

use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, identifier, is_col_id, is_col_label, is_non_reserved_word,
    is_type_function_name,
};
use crate::parse::_helpers::tree::nodes::{
    function, literal, name_text, null, ordered, star, type_name_text,
};
use crate::parse::_helpers::tree::typenames::setof as setof_type;
use crate::parse::_helpers::values::expressions::is_operator_token;
use crate::parse::constants::{CATALOGUE_SCHEMA, TYPE_KEYWORDS, TYPE_REFERENCE_SUFFIX};
use crate::parse::errors::ParseError;
use crate::parse::main::format_type::format_type;
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

/// Kinds `COMMENT ON` names that `SECURITY LABEL ON` does not.
const COMMENT_ONLY_KINDS: [&str; 7] = [
    "constraint",
    "policy",
    "rule",
    "trigger",
    "operator",
    "cast",
    "transform",
];

/// The prefix of a security label's kind, which keeps it apart from a comment's.
const SECURITY_LABEL_KIND: &str = "SECURITY LABEL";

/// Kinds named by `function_with_argtypes`.
const ROUTINE_KINDS: [&str; 3] = ["function", "procedure", "routine"];

/// `arg_class` keywords; a mode other than `VARIADIC` pairs with its argument in a tuple.
const ARGUMENT_MODES: [&str; 4] = ["variadic", IN_MODE, OUT_MODE, INOUT_MODE];

/// The `arg_class` that `OUT` may follow, as `IN OUT`, which means `INOUT`.
const IN_MODE: &str = "in";

const OUT_MODE: &str = "out";

const INOUT_MODE: &str = "inout";

/// `makeOrderedSetArgs`'s error (0A000) for a `VARIADIC` direct argument without its aggregated twin.
const ORDERED_SET_VARIADIC: &str = "an ordered-set aggregate with a VARIADIC direct argument must have one VARIADIC aggregated argument of the same data type";

/// `aggr_arg`'s error (0A000) for an `OUT` or `INOUT` argument.
const AGGREGATE_OUTPUT: &str = "aggregates cannot have output arguments";

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

    /// `SECURITY LABEL [FOR p] ON object IS label`: a Comment of kind `SECURITY LABEL …` on (p, object).
    pub(crate) fn security_label_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("security")?;
        self.expect_kw("label")?;
        let provider: Expression = match self.eat_kw("for") {
            true => match self.sconst_at(0) {
                true => self.sconst()?,
                false => {
                    let word: Token = self.expect_token(is_non_reserved_word)?;
                    literal(Literal::String(identifier(&word).name))
                }
            },
            false => null(),
        };
        self.expect_kw("on")?;
        if self
            .keyword_at(0)
            .is_some_and(|word| COMMENT_ONLY_KINDS.contains(&word))
        {
            return Err(self.error());
        }
        let (kind, this): (String, Expression) = self.commented_object()?;
        self.expect_kw("is")?;
        let expression: Expression = match self.eat_kw("null") {
            true => null(),
            false => self.sconst()?,
        };
        Ok(Expression::Comment(Box::new(Comment {
            this: tuple(vec![provider, this]),
            materialized: kind.starts_with("MATERIALIZED"),
            kind: format!("{SECURITY_LABEL_KIND} {kind}"),
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
        Ok(type_name_text(&self.qualified_name()?))
    }

    /// `func_name`: a `type_function_name`, or a `ColId` with `.ColLabel` parts.
    pub(crate) fn func_name(&mut self) -> Parsing<Vec<Identifier>> {
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
        self.aggregate_arguments(name)
    }

    /// `aggr_arg`: a `func_arg` without an `OUT` or `INOUT` mode (0A000).
    fn aggregate_argument(&mut self) -> Parsing<(Expression, Option<String>)> {
        let start: usize = self.offset();
        let from: usize = self.at;
        let argument: Expression = self.func_arg()?;
        let output: bool = match &argument {
            Expression::Tuple(pair) => matches!(
                pair.expressions.first(),
                Some(Expression::Var(mode)) if mode.this != IN_MODE.to_uppercase()
            ),
            _ => false,
        };
        match output {
            true => Err(ParseError {
                message: AGGREGATE_OUTPUT.to_owned(),
                position: start,
                syntax: false,
            }),
            false => {
                let key: Option<String> = match matches!(argument, Expression::Variadic(_)) {
                    true => Some(self.variadic_type_key(from)?),
                    false => None,
                };
                Ok((argument, key))
            }
        }
    }

    /// The `VARIADIC` argument's type from token `from` to here, keyed as `equal()` compares raw type names.
    fn variadic_type_key(&mut self, from: usize) -> Parsing<String> {
        let (to, gaps, opaque) = (self.at, self.gaps.len(), self.opaque);
        self.at = from + 1;
        let named = self.attempt(|parser| {
            parser.name(is_type_function_name)?;
            let start: usize = parser.at;
            let data_type: DataType = parser.func_type()?;
            match parser.at == to {
                true => Ok((start, data_type)),
                false => Err(parser.error()),
            }
        });
        let (start, data_type) = match named {
            Ok(typed) => typed,
            Err(_) => (self.at, self.func_type()?),
        };
        self.at = to;
        self.gaps.truncate(gaps);
        self.opaque = opaque;
        let text: String = format_type(&data_type).unwrap_or_else(|| format!("{data_type:?}"));
        let system: bool = self.tokens.get(start).is_some_and(|token| {
            matches!(token.kind, TokenKind::Keyword(keyword) if TYPE_KEYWORDS.contains(&keyword.name))
        });
        let names: Vec<&str> = self.tokens[start..to]
            .iter()
            .take_while(|token| {
                !matches!(token.kind, TokenKind::Char('(' | '[' | '%'))
                    && !matches!(token.kind, TokenKind::Keyword(keyword) if keyword.name == "array")
            })
            .map(|token| token.value.as_str())
            .collect();
        Ok(match (system, names.as_slice()) {
            (true, _) | (_, [CATALOGUE_SCHEMA, ".", _]) => format!("system {text}"),
            _ => format!("{} {text}", names.join(" ")),
        })
    }

    /// `aggr_args` as a call of `name`, with `makeOrderedSetArgs`'s check of a `VARIADIC` direct argument.
    pub(crate) fn aggregate_arguments(&mut self, name: Vec<Identifier>) -> Parsing<Expression> {
        self.expect_char('(')?;
        if self.eat_char('*') {
            self.expect_char(')')?;
            return Ok(Expression::Function(Box::new(function(
                name,
                vec![star(None)],
            ))));
        }
        let direct: Vec<(Expression, Option<String>)> = match self.kw("order") {
            true => Vec::new(),
            false => self.comma_list(|parser| parser.aggregate_argument())?,
        };
        let direct_key: Option<String> = direct.last().and_then(|(_, key)| key.clone());
        let direct: Vec<Expression> = direct.into_iter().map(|(argument, _)| argument).collect();
        let call: Expression = Expression::Function(Box::new(function(name, direct)));
        if !self.eat_kw("order") {
            self.expect_char(')')?;
            return Ok(call);
        }
        self.expect_kw("by")?;
        let sorted_start: usize = self.offset();
        let sorted: Vec<(Expression, Option<String>)> =
            self.comma_list(|parser| parser.aggregate_argument())?;
        self.expect_char(')')?;
        if let Some(direct_key) = direct_key {
            let matching: bool = match sorted.as_slice() {
                [(_, Some(key))] => *key == direct_key,
                _ => false,
            };
            if !matching {
                return Err(ParseError {
                    message: ORDERED_SET_VARIADIC.to_owned(),
                    position: sorted_start,
                    syntax: false,
                });
            }
        }
        Ok(Expression::WithinGroup(Box::new(WithinGroup {
            this: call,
            order_by: sorted
                .into_iter()
                .map(|(argument, _)| ordered(argument, false, false, None))
                .collect(),
        })))
    }

    /// `operator_with_argtypes`: `any_operator oper_argtypes`, a dotted schema quoted as for calls.
    fn operator_with_argtypes(&mut self) -> Parsing<Expression> {
        let mut parts: Vec<Identifier> = Vec::new();
        while self.is(is_col_id) && self.char_at(1, '.') {
            parts.push(self.name(is_col_id)?);
            self.at += 1;
        }
        let operator: Token = self.expect_token(is_operator_token)?;
        parts.push(Identifier::new(match operator.kind {
            TokenKind::LessEquals => "<=".to_owned(),
            TokenKind::GreaterEquals => ">=".to_owned(),
            TokenKind::NotEquals => "<>".to_owned(),
            TokenKind::Char(character) => character.to_string(),
            _ => operator.value.clone(),
        }));
        self.expect_char('(')?;
        let left: Option<DataType> = self.operator_side()?;
        self.expect_char(',')?;
        let right: Option<DataType> = match left {
            Some(_) => self.operator_side()?,
            None => Some(self.typename()?),
        };
        self.expect_char(')')?;
        let side = |side: Option<DataType>| Box::new(side.map_or_else(null, Expression::DataType));
        Ok(Expression::Operator(Box::new(Operator {
            this: side(left),
            operator: Some(Box::new(Expression::Identifier(Identifier::new(
                name_text(&parts),
            )))),
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
    pub(crate) fn argument_class(&mut self) -> Option<String> {
        let mode: &str = ARGUMENT_MODES.iter().find(|mode| self.kw(mode))?;
        self.at += 1;
        Some(match mode == IN_MODE && self.eat_kw(OUT_MODE) {
            true => INOUT_MODE.to_uppercase(),
            false => mode.to_uppercase(),
        })
    }

    /// `func_type`: a `Typename`, or `[SETOF] name.attr...%TYPE` as its named reference.
    pub(crate) fn func_type(&mut self) -> Parsing<DataType> {
        let setof: usize = usize::from(self.kw("setof"));
        let referenced: bool = self.peek_at(setof).is_some_and(is_type_function_name)
            && self
                .peek_at(setof + 1)
                .is_some_and(|token| token.kind == TokenKind::Char('.'));
        if referenced {
            let reference = self.attempt(|parser| {
                parser.at += setof;
                let mut parts: Vec<Identifier> = vec![parser.name(is_type_function_name)?];
                while parser.eat_char('.') {
                    parts.push(parser.name(is_col_label)?);
                }
                parser.expect_char('%')?;
                parser.expect_kw("type")?;
                Ok(DataType::Custom {
                    name: format!("{}{TYPE_REFERENCE_SUFFIX}", type_name_text(&parts)),
                })
            });
            if let Ok(data_type) = reference {
                return Ok(match setof {
                    0 => data_type,
                    _ => setof_type(data_type),
                });
            }
        }
        self.typename()
    }
}
