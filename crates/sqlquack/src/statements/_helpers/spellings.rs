//! Keys for the spellings and call shapes a construct key alone does not tell apart.

use std::collections::BTreeSet;

use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_policy::policy::models::EvidenceDialect;
use sqlquack_syntax::expressions::operators::models::{OperatorSpelling, ParameterStyle};
use sqlquack_syntax::expressions::typed_functions::models::TrimPosition;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{Dialect, Expression, ExpressionWalk};
use sqlquack_syntax_snowflake::tokenize;
use sqlquack_tokenize::TokenKind;

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

/// The spelling and call-shape keys of the nodes under `statement`, as read in `dialect`.
pub(crate) fn tree_spellings(statement: &Expression, dialect: EvidenceDialect) -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = BTreeSet::new();
    let mut pending: Vec<(&Expression, bool)> = vec![(statement, false)];
    while let Some((node, windowed)) = pending.pop() {
        let window_call = match node {
            Expression::WindowFunction(window) => Some(&window.this),
            Expression::WithinGroup(within) if windowed => Some(&within.this),
            _ => None,
        };
        let keyword = match dialect {
            EvidenceDialect::Snowflake => None,
            EvidenceDialect::Databricks => keyword_spelling(node),
        };
        if let Some(key) = keyword.or_else(|| node_key(node, windowed)) {
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

/// Databricks: the spelling of a call written with an operator or keywords rather than commas, which
/// keys it in place of its call shape so the comma call's form does not claim it, and TRIM's keyword form.
fn keyword_spelling(node: &Expression) -> Option<String> {
    let spelling: String = match node {
        Expression::Function(function) => match function.written_as {
            Some(OperatorSpelling::ShiftRightUnsigned) => "shift_right_unsigned".to_owned(),
            Some(OperatorSpelling::Div) => "div_operator".to_owned(),
            Some(OperatorSpelling::In) => "position_in".to_owned(),
            Some(OperatorSpelling::Placing) => "overlay_placing".to_owned(),
            _ if function.from_separator => format!("{}_from", function.name.to_ascii_lowercase()),
            _ => return None,
        },
        Expression::Trim(trim) if trim.sql_standard_syntax => {
            let position: &str = match (trim.position_explicit, &trim.position) {
                (false, _) => "",
                (true, TrimPosition::Both) => "both_",
                (true, TrimPosition::Leading) => "leading_",
                (true, TrimPosition::Trailing) => "trailing_",
            };
            let characters: &str = match trim.characters {
                Some(_) => "characters_",
                None => "",
            };
            format!("trim_{position}{characters}from")
        }
        _ => return None,
    };
    Some(format!("{SPELLING}{spelling}"))
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
        Expression::Call(call) if call.name.iter().any(Option::is_none) => {
            let name = call
                .name
                .iter()
                .map(|part| part.as_ref().map_or("", |part| part.name.as_str()))
                .collect::<Vec<_>>()
                .join(".");
            return Some(call_key(&name, &call.arguments, windowed, false));
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
