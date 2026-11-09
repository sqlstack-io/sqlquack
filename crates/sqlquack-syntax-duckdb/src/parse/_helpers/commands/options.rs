//! `GenericCopyOptionList` (copy.gram) and `WithList` (create_index.gram), with their checks.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::queries::models::Ordered;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_reserved_word};
use crate::parse::_helpers::tree::nodes::{null, tuple, var};
use crate::parse::constants::{
    MODIFIERS_ONLY_IN_ORDER_BY, OIDS, ORDER_BY_OPTION, PARTITION_PATH_ONE, PARTITION_PATH_OPTION,
    PARTITION_PATH_VALUE,
};
use crate::scan::models::TokenKind;

/// A `RelOption` value that is `NULL` or `NONE`, as the tree keeps it.
const NULL_WORD: &str = "NULL";
const NONE_WORD: &str = "NONE";

/// One `GenericCopyOption`: its name, and its value when it has one.
pub(crate) struct CopyOption {
    pub(crate) name: Identifier,
    pub(crate) value: Option<CopyValue>,
    /// The value as written: an expression, or a tuple of a sort list's items.
    pub(crate) expression: Option<Expression>,
}

/// What a `GenericCopyOption`'s value holds once the transformer has read it.
pub(crate) enum CopyValue {
    /// Values the option keeps as constants: a literal, a column name, `*`, or `'t'`/`'f'` cast.
    Constant { null: bool },
    /// An expression the option keeps unevaluated: a call, a parameter, a cast, or a list.
    Expression,
}

impl Parser<'_> {
    /// `GenericCopyOptionList`: `(name [value], ...)`, each name once.
    pub(crate) fn generic_copy_options(&mut self) -> Parsing<Vec<CopyOption>> {
        let start: usize = self.offset();
        let options: Vec<CopyOption> = self.parenthesized(|parser| parser.generic_copy_option())?;
        for (index, option) in options.iter().enumerate() {
            let name: String = option.name.name.to_lowercase();
            let repeated: bool = options[..index]
                .iter()
                .any(|earlier| earlier.name.name.eq_ignore_ascii_case(&name));
            if repeated {
                let message: String = format!("Unexpected duplicate option \"{name}\"");
                self.refuse_later(&message, start);
            }
        }
        Ok(options)
    }

    /// `GenericCopyOption`: `name [(sort items) | expression]`.
    fn generic_copy_option(&mut self) -> Parsing<CopyOption> {
        let start: usize = self.offset();
        let name: Identifier = self.name(is_col_label)?;
        self.copy_option_value(name, start)
    }

    /// `GenericCopyOptionValue?` after an option's name, with the transformer's checks.
    pub(crate) fn copy_option_value(
        &mut self,
        name: Identifier,
        start: usize,
    ) -> Parsing<CopyOption> {
        let path: bool = name.name.eq_ignore_ascii_case(PARTITION_PATH_OPTION);
        if self.terminal_at(0) {
            if path {
                self.refuse_later(PARTITION_PATH_VALUE, start);
            }
            return Ok(CopyOption {
                name,
                value: None,
                expression: None,
            });
        }
        if self.is_char('(')
            && let Ok(items) =
                self.attempt(|parser| parser.parenthesized(|inner| inner.sort_item()))
        {
            return self.sorted_option(name, path, items, start);
        }
        let expression: Expression = self.expression()?;
        let value: CopyValue = copy_value(&expression);
        Ok(CopyOption {
            name,
            value: Some(value),
            expression: Some(expression),
        })
    }

    /// A copy option whose value is a parenthesised list of sort items.
    fn sorted_option(
        &self,
        name: Identifier,
        path: bool,
        items: Vec<Ordered>,
        start: usize,
    ) -> Parsing<CopyOption> {
        let modified: bool = items
            .iter()
            .any(|item| item.desc || item.explicit_asc || item.nulls_first.is_some());
        let ordering: bool = name.name.eq_ignore_ascii_case(ORDER_BY_OPTION);
        let value: CopyValue = match (path, modified, items.len()) {
            (_, false, 1) => copy_value(&items[0].this),
            (true, _, _) => return Err(self.error_with(PARTITION_PATH_ONE, start, false)),
            (false, true, _) if !ordering => {
                return Err(self.error_with(MODIFIERS_ONLY_IN_ORDER_BY, start, false));
            }
            _ => CopyValue::Expression,
        };
        let expression: Expression = match (modified, items.len()) {
            (false, 1) => items[0].this.clone(),
            _ => tuple(
                items
                    .into_iter()
                    .map(|item| Expression::Ordered(Box::new(item)))
                    .collect(),
            ),
        };
        Ok(CopyOption {
            name,
            value: Some(value),
            expression: Some(expression),
        })
    }

    /// `WithList`: `WITH (options)`, or `WITH {WITH | WITHOUT} OIDS`, which is not implemented.
    pub(crate) fn with_list(&mut self) -> Parsing<Vec<RelOption>> {
        let start: usize = self.offset();
        self.expect_kw("WITH")?;
        if self.kw_in(0, &["WITH", "WITHOUT"]) {
            self.at += 1;
            self.expect_kw("OIDS")?;
            return Err(self.error_with(OIDS, start, false));
        }
        self.rel_options()
    }

    /// `RelOptionList`: `(name[.name...] [= value], ...)`, each name once.
    pub(crate) fn rel_options(&mut self) -> Parsing<Vec<RelOption>> {
        let start: usize = self.offset();
        let options: Vec<RelOption> = self.parenthesized(|parser| parser.rel_option())?;
        for (index, option) in options.iter().enumerate() {
            let name: &str = option.name.as_str();
            if options[..index]
                .iter()
                .any(|earlier| earlier.name.eq_ignore_ascii_case(name))
            {
                let message: String = format!("Duplicate option \"{name}\" in WITH clause");
                self.refuse_later(&message, start);
            }
        }
        Ok(options)
    }

    /// `RelOption`: `name[.name...] [= DefArg]` or `'name' [= DefArg]`.
    fn rel_option(&mut self) -> Parsing<RelOption> {
        let mut parts: Vec<String> = Vec::new();
        match self.string_at(0) {
            true => parts.push(self.advance()?.value),
            false => {
                parts.push(self.name(is_col_label)?.name);
                while self.eat_char('.') {
                    parts.push(self.name(is_col_label)?.name);
                }
            }
        }
        let name: String = parts.join(".");
        if !self.eat_char('=') {
            return Ok(RelOption {
                name,
                value: RelValue::Absent,
                expression: null(),
                text: NULL_WORD.to_owned(),
            });
        }
        let first: usize = self.at;
        let (value, expression) = self.def_arg()?;
        let text: String =
            self.sql[self.tokens[first].start..self.tokens[self.at - 1].end].to_owned();
        Ok(RelOption {
            name,
            value,
            expression,
            text,
        })
    }

    /// `DefArg`: `NULL`, a string, a number or `NONE` alone, else an expression or a reserved word.
    fn def_arg(&mut self) -> Parsing<(RelValue, Expression)> {
        if self.kw("NULL") {
            self.at += 1;
            return Ok((RelValue::Null, null()));
        }
        if self.kw("NONE") {
            self.at += 1;
            return Ok((RelValue::Null, var(NONE_WORD)));
        }
        let alone: bool = matches!(
            self.peek().map(|token| token.kind),
            Some(TokenKind::Number) | Some(TokenKind::String(_))
        );
        if alone {
            return Ok((RelValue::Other, self.single_expression()?));
        }
        match self.attempt(|parser| parser.expression()) {
            Ok(value) => Ok((rel_value(&value), value)),
            Err((error, _)) => match self.is(is_reserved_word) {
                true => {
                    let word: String = self.advance()?.value.to_ascii_uppercase();
                    Ok((RelValue::Other, var(&word)))
                }
                false => Err(error),
            },
        }
    }
}

/// One `RelOption`: its dotted name, its value's class, its value and the value as written.
pub(crate) struct RelOption {
    pub(crate) name: String,
    pub(crate) value: RelValue,
    pub(crate) expression: Expression,
    pub(crate) text: String,
}

/// What a `RelOption`'s value is to the transformer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelValue {
    /// No `= value` at all.
    Absent,
    Null,
    Boolean,
    /// Any other constant: a number, a string or a reserved word.
    Other,
    NonConstant,
}

/// The class of an expression given as a `RelOption` value.
fn rel_value(expression: &Expression) -> RelValue {
    match expression {
        Expression::Paren(paren) => rel_value(&paren.this),
        Expression::Null(_) => RelValue::Null,
        Expression::Boolean(_) => RelValue::Boolean,
        _ if is_constant(expression) => RelValue::Other,
        _ => RelValue::NonConstant,
    }
}

/// Whether `expression` is a constant once parentheses and a minus on a number fold away.
pub(crate) fn is_constant(expression: &Expression) -> bool {
    match expression {
        Expression::Paren(paren) => is_constant(&paren.this),
        Expression::Neg(unary) => is_number(&unary.this),
        Expression::Literal(_) | Expression::Boolean(_) | Expression::Null(_) => true,
        _ => false,
    }
}

/// Whether `expression` is a number literal, perhaps in parentheses or negated.
pub(crate) fn is_number(expression: &Expression) -> bool {
    match expression {
        Expression::Paren(paren) => is_number(&paren.this),
        Expression::Neg(unary) => is_number(&unary.this),
        Expression::Literal(literal) => matches!(**literal, Literal::Number(_)),
        _ => false,
    }
}

/// Whether `expression` is the string `'t'` or `'f'`, which a cast in a copy option reads as a boolean.
fn is_boolean_letter(expression: &Expression) -> bool {
    matches!(expression, Expression::Literal(literal)
        if matches!(&**literal, Literal::String(text) if text == "t" || text == "f"))
}

/// The value a copy option keeps; an expression the transformer cannot keep is DuckDB's later refusal.
fn copy_value(expression: &Expression) -> CopyValue {
    match expression {
        Expression::Paren(paren) => copy_value(&paren.this),
        Expression::Null(_) => CopyValue::Constant { null: true },
        Expression::Literal(_)
        | Expression::Boolean(_)
        | Expression::Column(_)
        | Expression::Star(_) => CopyValue::Constant { null: false },
        Expression::Neg(unary) if is_number(&unary.this) => CopyValue::Constant { null: false },
        Expression::Cast(cast) if is_boolean_letter(&cast.this) => {
            CopyValue::Constant { null: false }
        }
        _ => CopyValue::Expression,
    }
}
