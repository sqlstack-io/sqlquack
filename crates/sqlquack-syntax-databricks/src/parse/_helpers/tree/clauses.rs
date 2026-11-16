//! The tree of a statement checked against a syntax pattern: its keyword clauses in order.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::operators::models::{NamedArgSeparator, NamedArgument};
use sqlquack_syntax::expressions::statements::models::UtilityStatement;
use sqlquack_syntax::expressions::tables::models::Property;
use sqlquack_syntax::expressions::types::models::Tuple;

use crate::parse::_helpers::tree::nodes::{binary, column_from, table, var};
use crate::parse::constants::FLAG_PHRASES;

/// One element a pattern matched, in the order written.
#[derive(Clone, Debug)]
pub(crate) enum Part {
    /// A keyword of the pattern.
    Word(&'static str),
    /// Punctuation of the pattern, such as `(`, `,` or `=`.
    Punct(&'static str),
    /// A slot's value.
    Value(Expression),
    /// A slot whose value holds a construct the shared tree lacks.
    Opaque,
    /// The start of an optional part, a choice, a named rule or one item of a repeated part.
    Begin(Group),
    /// The end of the part `Begin` started.
    End,
}

/// What a `Begin` part starts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Group {
    /// An optional part or a choice.
    #[default]
    Choice,
    /// A named rule.
    Rule,
    /// One item of a repeated part, a value of the clause it is in.
    Item,
}

/// The statement `parts` spell: its first `lead` and next required words, then its clauses.
pub(crate) fn documented_tree(lead: usize, parts: Vec<Part>) -> Expression {
    let mut kind: Vec<&'static str> = Vec::new();
    let mut rest = parts.into_iter().peekable();
    let mut unclosed: usize = 0;
    while kind.len() < lead {
        match rest.next() {
            Some(Part::Word(word)) => kind.push(word),
            Some(Part::Begin(_)) => unclosed += 1,
            Some(Part::End) => unclosed = unclosed.saturating_sub(1),
            Some(_) => {}
            None => break,
        }
    }
    let mut depth: usize = 0;
    let parts: Vec<Part> = rest
        .filter(|part| match part {
            Part::Begin(_) => {
                depth += 1;
                true
            }
            Part::End if depth == 0 && unclosed > 0 => {
                unclosed -= 1;
                false
            }
            Part::End => {
                depth = depth.saturating_sub(1);
                true
            }
            _ => true,
        })
        .collect();
    let required: usize = parts
        .iter()
        .take_while(|part| matches!(part, Part::Word(_)))
        .count();
    kind.extend(parts.iter().take(required).filter_map(|part| match part {
        Part::Word(word) => Some(*word),
        _ => None,
    }));
    let parts: &[Part] = &parts[required..];
    let (operands, _): (Vec<Expression>, usize) = clauses(parts, 0);
    Expression::UtilityStatement(Box::new(UtilityStatement {
        kind: kind.join(" "),
        options: Vec::new(),
        operands,
    }))
}

/// The clause being read: its keywords, its values, and an operator before the next value.
#[derive(Default)]
struct Clause {
    words: Vec<&'static str>,
    values: Vec<Expression>,
    joiner: Option<&'static str>,
}

impl Clause {
    /// Adds `value`, first ending a flag such as `IF EXISTS` before it, which it returns.
    fn push(&mut self, value: Expression) -> Vec<Expression> {
        let mut ended: Vec<Expression> = Vec::new();
        if self.values.is_empty() && FLAG_PHRASES.contains(&self.words.join(" ").as_str()) {
            ended = self.flush();
        }
        let joined: Option<Expression> = match (self.joiner.take(), self.values.pop()) {
            (Some(joiner), Some(last)) => Some(join(joiner, last, value.clone())),
            (_, Some(last)) => {
                self.values.push(last);
                None
            }
            (_, None) => None,
        };
        self.values.push(joined.unwrap_or(value));
        ended
    }

    /// Ends the clause: a property of its keywords and values, or its values when it has none.
    fn flush(&mut self) -> Vec<Expression> {
        let words: Vec<&'static str> = std::mem::take(&mut self.words);
        let mut values: Vec<Expression> = std::mem::take(&mut self.values);
        self.joiner = None;
        if words.is_empty() {
            return values;
        }
        let value: Option<Box<Expression>> = match values.len() {
            0 => None,
            1 => values.pop().map(Box::new),
            _ => Some(Box::new(Expression::Tuple(Box::new(Tuple {
                expressions: values,
            })))),
        };
        vec![Expression::Property(Box::new(Property {
            this: Box::new(var(&words.join(" "))),
            value,
        }))]
    }
}

/// `last` and `value` joined by `=`, `=>` or `.`, or listed with any other operator between them.
fn join(joiner: &str, last: Expression, value: Expression) -> Expression {
    match (joiner, last, value) {
        ("=", last, value) => binary(Expression::Eq, last, value),
        ("=>", Expression::Identifier(name), value) => {
            Expression::NamedArgument(Box::new(NamedArgument {
                name,
                value,
                separator: NamedArgSeparator::DArrow,
            }))
        }
        (".", Expression::Identifier(first), Expression::Identifier(second)) => {
            Expression::Table(Box::new(table(vec![first, second])))
        }
        (".", Expression::Identifier(first), Expression::Table(table)) => {
            let mut parts = vec![first];
            parts.extend(table.catalog);
            parts.extend(table.schema);
            parts.push(table.name);
            column_from(parts)
        }
        (joiner, last, value) => Expression::Tuple(Box::new(Tuple {
            expressions: vec![last, var(joiner), value],
        })),
    }
}

/// The clauses of `parts` from `at` to a closing parenthesis or item end, and where they end.
fn clauses(parts: &[Part], mut at: usize) -> (Vec<Expression>, usize) {
    let mut out: Vec<Expression> = Vec::new();
    let mut clause: Clause = Clause::default();
    let mut opened: Vec<(bool, Group)> = Vec::new();
    while let Some(part) = parts.get(at) {
        at += 1;
        match part {
            Part::Word(word) => {
                if !clause.values.is_empty() {
                    out.extend(clause.flush());
                }
                clause.words.push(word);
            }
            Part::Value(value) => out.extend(clause.push(value.clone())),
            Part::Punct("(") => {
                let (inner, end): (Vec<Expression>, usize) = clauses(parts, at);
                at = end;
                out.extend(clause.push(Expression::Tuple(Box::new(Tuple { expressions: inner }))));
            }
            Part::Punct(")") => break,
            Part::Punct(",") => {}
            Part::Punct(joiner) => clause.joiner = Some(joiner),
            Part::Begin(Group::Item) => {
                let (mut inner, end): (Vec<Expression>, usize) = clauses(parts, at);
                at = end;
                let item: Expression = match inner.len() {
                    1 => inner.remove(0),
                    _ => Expression::Tuple(Box::new(Tuple { expressions: inner })),
                };
                out.extend(clause.push(item));
            }
            Part::Begin(group) => opened.push((clause.words.is_empty(), *group)),
            Part::End => {
                let Some((starts_clause, group)) = opened.pop() else {
                    break;
                };
                let complete: bool =
                    !clause.values.is_empty() && (starts_clause || group == Group::Rule);
                let flag: bool = starts_clause
                    && clause.values.is_empty()
                    && !clause.words.is_empty()
                    && (group == Group::Rule
                        || matches!(parts.get(at), Some(Part::Begin(Group::Choice))));
                if complete || flag {
                    out.extend(clause.flush());
                }
            }
            Part::Opaque => {}
        }
    }
    out.extend(clause.flush());
    (out, at)
}
