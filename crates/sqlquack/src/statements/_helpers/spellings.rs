//! Keys for the spellings and call shapes a construct key alone does not tell apart.

use std::collections::BTreeSet;

use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::operators::models::ParameterStyle;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{Dialect, Expression, ExpressionWalk, TokenKind, tokenize};

use crate::statements::constants::{CALL, CALL_BARE, CALL_NAMED, CALL_OVER, SPELLING};

/// The spelling keys of the tokens in `sql`: operator spellings and string quoting.
pub(crate) fn token_spellings(sql: &str) -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = BTreeSet::new();
    let Ok(tokens) = tokenize(sql, Dialect::Snowflake) else {
        return keys;
    };
    let significant: Vec<(TokenType, String)> = tokens
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::Significant(kind) => Some((*kind, token.text(sql).to_ascii_uppercase())),
            _ => None,
        })
        .collect();
    for (position, (kind, text)) in significant.iter().enumerate() {
        let opens_call = significant
            .get(position + 1)
            .is_some_and(|(next, _)| *next == TokenType::LParen);
        let spelling = match (kind, text.as_str()) {
            (TokenType::Neq, "<>") => "neq_angle",
            (TokenType::Neq, "!=") => "neq_bang",
            (TokenType::Except, "MINUS") => "minus",
            (TokenType::Except, "EXCEPT") => "except",
            (TokenType::RLike, "REGEXP") if !opens_call => "regexp",
            (TokenType::RLike, "RLIKE") if !opens_call => "rlike",
            (TokenType::DollarString, _) => "dollar_string",
            (TokenType::String, _) => "quoted_string",
            _ => continue,
        };
        keys.insert(format!("{SPELLING}{spelling}"));
    }
    keys
}

/// The spelling and call-shape keys of the nodes under `statement`.
pub(crate) fn tree_spellings(statement: &Expression) -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = BTreeSet::new();
    let mut pending: Vec<(&Expression, bool)> = vec![(statement, false)];
    while let Some((node, windowed)) = pending.pop() {
        let window_call = match node {
            Expression::WindowFunction(window) => Some(&window.this),
            Expression::WithinGroup(within) if windowed => Some(&within.this),
            _ => None,
        };
        if let Some(key) = node_key(node, windowed) {
            keys.insert(key);
        }
        pending.extend(node.children().into_iter().map(|child| {
            (
                child,
                window_call.is_some_and(|call| std::ptr::eq(call, child)),
            )
        }));
    }
    keys
}

/// The key one node adds: its call shape (qualified or not), CASE form, subscript, literal or parameter.
fn node_key(node: &Expression, windowed: bool) -> Option<String> {
    let spelling = match node {
        Expression::Function(function) => {
            return Some(call_key(
                &function.name,
                &function.args,
                windowed,
                function.no_parens,
            ));
        }
        Expression::AggregateFunction(function) => {
            return Some(call_key(&function.name, &function.args, windowed, false));
        }
        Expression::MethodCall(call) => {
            let Expression::Column(path) = &call.this else {
                return None;
            };
            let name = path
                .table
                .iter()
                .chain([&path.name, &call.method])
                .map(|part| part.name.as_str())
                .collect::<Vec<_>>()
                .join(".");
            return Some(call_key(&name, &call.args, windowed, false));
        }
        Expression::Case(case) if case.operand.is_some() => "case_simple",
        Expression::Case(_) => "case_searched",
        Expression::Subscript(subscript) => match &subscript.index {
            Expression::Literal(literal) if matches!(**literal, Literal::String(_)) => {
                "subscript_key"
            }
            _ => "subscript_index",
        },
        Expression::Literal(literal) => match **literal {
            Literal::Number(_) => "number",
            Literal::Date(_) | Literal::Time(_) | Literal::Timestamp(_) | Literal::Datetime(_) => {
                "datetime_literal"
            }
            _ => return None,
        },
        Expression::Parameter(parameter) => match (&parameter.style, &parameter.name) {
            (ParameterStyle::Dollar, Some(_)) => "session_variable",
            (ParameterStyle::Dollar, None) => "positional_column",
            _ => "bind_variable",
        },
        _ => return None,
    };
    Some(format!("{SPELLING}{spelling}"))
}

/// A call's shape key: over a window, with named arguments, without parentheses, or plain.
fn call_key(name: &str, args: &[Expression], windowed: bool, bare: bool) -> String {
    let named = args
        .iter()
        .any(|argument| matches!(argument, Expression::NamedArgument(_)));
    let prefix = match (windowed, named, bare) {
        (true, _, _) => CALL_OVER,
        (false, true, _) => CALL_NAMED,
        (false, false, true) => CALL_BARE,
        (false, false, false) => CALL,
    };
    format!("{prefix}{}", name.to_ascii_uppercase())
}
