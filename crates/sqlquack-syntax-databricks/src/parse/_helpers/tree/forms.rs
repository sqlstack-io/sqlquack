//! Statements and clauses checked against a syntax pattern in the reference's notation.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::types::models::Tuple;

use crate::parse::_helpers::commands::alters::partition;
use crate::parse::_helpers::commands::documented::rule;
use crate::parse::_helpers::tree::clauses::{Group, Part, documented_tree};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_word};
use crate::parse::_helpers::tree::nodes::{literal, table};
use crate::parse::constants::COMMA_SEPARATED_REPEAT;
use crate::parse::errors::ParseError;
use crate::scan::models::{NumberKind, Token, TokenKind};

/// A pattern, parsed.
enum Node {
    Word(&'static str),
    Punct(&'static str),
    Slot(&'static str),
    Seq(Vec<Node>),
    Alt(Vec<Node>),
    Opt(Box<Node>),
    Many(Box<Node>, bool),
}

/// A position after a match and the parts matched on the way there.
type Path = (usize, Vec<Part>);

/// Each path's parts extended by `part`.
fn with_part(paths: Vec<Path>, part: &Part) -> Vec<Path> {
    paths
        .into_iter()
        .map(|(at, mut parts)| {
            parts.push(part.clone());
            (at, parts)
        })
        .collect()
}

/// Parses the alternatives from `words[at]` to a closing bracket or the end, and where they end.
fn alternatives(words: &[&'static str], at: usize) -> (Node, usize) {
    let (first, mut at): (Node, usize) = sequence(words, at);
    let mut choices: Vec<Node> = vec![first];
    while words.get(at) == Some(&"|") {
        let (choice, end): (Node, usize) = sequence(words, at + 1);
        choices.push(choice);
        at = end;
    }
    let node: Node = match choices.len() {
        1 => choices.remove(0),
        _ => Node::Alt(choices),
    };
    (node, at)
}

/// Parses a sequence from `words[at]` to a `|`, a closing bracket or the end, and where it ends.
fn sequence(words: &[&'static str], mut at: usize) -> (Node, usize) {
    let mut items: Vec<Node> = Vec::new();
    while let Some(word) = words.get(at).copied() {
        let item: Node = match word {
            "|" | "]" | "}" => break,
            "..." | ",..." => {
                at += 1;
                let last: Node = items.pop().unwrap_or(Node::Seq(Vec::new()));
                items.push(Node::Many(Box::new(last), word == COMMA_SEPARATED_REPEAT));
                continue;
            }
            "[" | "{" => {
                let (inner, end): (Node, usize) = alternatives(words, at + 1);
                at = end + 1;
                items.push(match word {
                    "[" => Node::Opt(Box::new(inner)),
                    _ => inner,
                });
                continue;
            }
            _ if word.starts_with(|c: char| c.is_ascii_uppercase()) => Node::Word(word),
            _ if word.starts_with(|c: char| c.is_ascii_lowercase()) => Node::Slot(word),
            _ => Node::Punct(word),
        };
        at += 1;
        items.push(item);
    }
    let node: Node = match items.len() {
        1 => items.remove(0),
        _ => Node::Seq(items),
    };
    (node, at)
}

/// Parses `pattern`.
fn compile(pattern: &'static str) -> Node {
    let words: Vec<&'static str> = pattern.split_whitespace().collect();
    alternatives(&words, 0).0
}

pub(crate) fn is_integer(token: &Token) -> bool {
    token.kind == TokenKind::Number(NumberKind::Integer)
}

fn is_number(token: &Token) -> bool {
    matches!(token.kind, TokenKind::Number(_))
}

fn is_string(token: &Token) -> bool {
    matches!(token.kind, TokenKind::String { .. })
}

/// The furthest failure seen while matching.
struct Furthest {
    error: Option<ParseError>,
}

impl Furthest {
    fn record(&mut self, error: ParseError) {
        if self
            .error
            .as_ref()
            .is_none_or(|best| error.position > best.position)
        {
            self.error = Some(error);
        }
    }
}

impl Parser<'_> {
    /// The position after `slot` from `at` and the part it matched, recording a failure.
    fn slot(&mut self, slot: &str, at: usize, furthest: &mut Furthest) -> Option<(usize, Part)> {
        self.at = at;
        let (gaps, opaque) = (self.gaps.len(), self.opaque);
        let result: Result<Expression, (ParseError, usize)> =
            self.attempt(|parser| parser.slot_value(slot));
        match result {
            Ok(value) => {
                let part: Part = match self.gaps.len() > gaps || (self.opaque && !opaque) {
                    true => Part::Opaque,
                    false => Part::Value(value),
                };
                self.gaps.truncate(gaps);
                self.opaque = opaque;
                Some((self.at, part))
            }
            Err((error, _)) => {
                furthest.record(error);
                None
            }
        }
    }

    /// The value of `slot` from here.
    pub(crate) fn slot_value(&mut self, slot: &str) -> Parsing<Expression> {
        let tuple =
            |expressions: Vec<Expression>| Expression::Tuple(Box::new(Tuple { expressions }));
        Ok(match slot {
            "name" => Expression::Table(Box::new(table(self.multipart_identifier()?))),
            "id" => Expression::Identifier(self.ident()?),
            "table" => Expression::Table(Box::new(self.table_reference()?)),
            "str" => self.string_literal()?,
            "int" => self.number(false, is_integer)?,
            "num" => self.number(true, is_number)?,
            "literal" => self.literal_value()?,
            "expr" => self.expression()?,
            "query" => self.query()?,
            "type" => Expression::DataType(self.data_type()?),
            "props" | "keys" => {
                let at: usize = self.at;
                match slot {
                    "props" => drop(self.valued_property_list()?),
                    _ => drop(self.key_property_list()?),
                }
                self.at = at;
                tuple(self.property_list()?)
            }
            "plist" => tuple(self.property_list()?),
            "partition" => partition(self.partition_spec()?),
            "columns" => tuple(
                self.identifier_list()?
                    .into_iter()
                    .map(Expression::Identifier)
                    .collect(),
            ),
            "statement" => self.statement()?,
            "column" => Expression::ColumnDef(Box::new(self.column_definition()?)),
            "table_constraint" => {
                self.table_constraint()?;
                let start: usize = self.offset();
                self.gap("table_constraint", start);
                Expression::Null(sqlquack_syntax::expressions::names::models::Null)
            }
            _ => return Err(self.error_with(slot, self.offset(), true)),
        })
    }

    /// Records the syntax error at token `at` as a failure.
    fn miss(&mut self, at: usize, furthest: &mut Furthest) {
        self.at = at;
        furthest.record(self.error());
    }

    /// The paths after `node` from any of `from`, one per position, the first that reached it.
    fn follow(&mut self, node: &Node, from: Vec<Path>, furthest: &mut Furthest) -> Vec<Path> {
        let mut to: Vec<Path> = match node {
            Node::Word(word) => {
                let mut to: Vec<Path> = Vec::new();
                for (at, mut parts) in from {
                    match self
                        .tokens
                        .get(at)
                        .is_some_and(|token| is_word(token, word))
                    {
                        true => {
                            parts.push(Part::Word(word));
                            to.push((at + 1, parts));
                        }
                        false => self.miss(at, furthest),
                    }
                }
                to
            }
            Node::Punct(text) => {
                let mut to: Vec<Path> = Vec::new();
                for (at, mut parts) in from {
                    let found: bool = self.tokens.get(at).is_some_and(|token| match token.kind {
                        TokenKind::Char(character) => {
                            text.len() == 1 && text.starts_with(character)
                        }
                        TokenKind::Op => token.value == *text,
                        _ => false,
                    });
                    match found {
                        true => {
                            parts.push(Part::Punct(text));
                            to.push((at + 1, parts));
                        }
                        false => self.miss(at, furthest),
                    }
                }
                to
            }
            Node::Slot(slot) => match rule(slot) {
                Some(pattern) => {
                    let inner: Vec<Path> = self.follow(
                        &compile(pattern),
                        with_part(from, &Part::Begin(Group::Rule)),
                        furthest,
                    );
                    with_part(inner, &Part::End)
                }
                None => from
                    .into_iter()
                    .filter_map(|(at, mut parts)| {
                        let (end, part) = self.slot(slot, at, furthest)?;
                        parts.push(part);
                        Some((end, parts))
                    })
                    .collect(),
            },
            Node::Seq(items) => {
                let mut at: Vec<Path> = from;
                for item in items {
                    if at.is_empty() {
                        break;
                    }
                    at = self.follow(item, at, furthest);
                }
                at
            }
            Node::Alt(choices) => {
                let from: Vec<Path> = with_part(from, &Part::Begin(Group::Choice));
                let mut to: Vec<Path> = Vec::new();
                for choice in choices {
                    to.extend(self.follow(choice, from.clone(), furthest));
                }
                with_part(to, &Part::End)
            }
            Node::Opt(inner) => {
                let taken: Vec<Path> = self.follow(
                    inner,
                    with_part(from.clone(), &Part::Begin(Group::Choice)),
                    furthest,
                );
                let mut to: Vec<Path> = with_part(taken, &Part::End);
                to.extend(from);
                to
            }
            Node::Many(inner, comma) => {
                let mut to: Vec<Path> = Vec::new();
                let mut at: Vec<Path> = self.item(inner, from, furthest);
                while !at.is_empty() {
                    at.retain(|(position, _)| !to.iter().any(|(seen, _)| seen == position));
                    to.extend(at.iter().cloned());
                    let next: Vec<Path> = match comma {
                        true => at
                            .into_iter()
                            .filter(|(position, _)| {
                                self.tokens
                                    .get(*position)
                                    .is_some_and(|token| token.kind == TokenKind::Char(','))
                            })
                            .map(|(position, mut parts)| {
                                parts.push(Part::Punct(","));
                                (position + 1, parts)
                            })
                            .collect(),
                        false => at,
                    };
                    at = match next.is_empty() {
                        true => next,
                        false => self.item(inner, next, furthest),
                    };
                }
                to
            }
        };
        to.sort_by_key(|(at, _)| *at);
        to.dedup_by_key(|(at, _)| *at);
        to
    }

    /// The paths after one item of a repeated part from any of `from`, marked as one item.
    fn item(&mut self, inner: &Node, from: Vec<Path>, furthest: &mut Furthest) -> Vec<Path> {
        let to: Vec<Path> =
            self.follow(inner, with_part(from, &Part::Begin(Group::Item)), furthest);
        with_part(to, &Part::End)
    }

    /// An integer, or with `signed` a number after an optional `-`, as written.
    fn number(&mut self, signed: bool, kind: fn(&Token) -> bool) -> Parsing<Expression> {
        let negative: bool = signed && self.eat_char('-');
        if !self.is(kind) {
            return Err(self.error());
        }
        let token: Token = self.advance()?;
        Ok(literal(Literal::Number(match negative {
            true => format!("-{}", token.value),
            false => token.value,
        })))
    }

    /// `literal`: a number, with an optional sign, a string, a boolean or `NULL`.
    fn literal_value(&mut self) -> Parsing<Expression> {
        if self.is_char('+') {
            self.at += 1;
            return self.number(false, is_number);
        }
        if self.is_char('-') || self.is(is_number) {
            return self.number(true, is_number);
        }
        if self.is(is_string) {
            return self.string_literal();
        }
        match self.kw_in(0, &["TRUE", "FALSE", "NULL"]) {
            true => self.primary_expression(),
            false => Err(self.error()),
        }
    }

    /// What `pattern` matched from here to the end of the statement, where the parser then ends.
    pub(crate) fn form(&mut self, pattern: &'static str) -> Parsing<Vec<Part>> {
        let node: Node = compile(pattern);
        let (gaps, opaque) = (self.gaps.len(), self.opaque);
        let start: usize = self.at;
        let mut furthest: Furthest = Furthest { error: None };
        let ends: Vec<Path> = self.follow(&node, vec![(start, Vec::new())], &mut furthest);
        self.gaps.truncate(gaps);
        self.opaque = opaque;
        let end: Option<usize> = ends.iter().position(|(at, _)| {
            self.tokens
                .get(*at)
                .is_none_or(|token| token.kind == TokenKind::Char(';'))
        });
        if let Some(end) = end {
            let (at, parts): Path = ends.into_iter().nth(end).unwrap_or_default();
            self.at = at;
            return Ok(parts);
        }
        for (at, _) in ends {
            self.miss(at, &mut furthest);
        }
        self.at = start;
        Err(furthest.error.unwrap_or_else(|| self.error()))
    }

    /// The tree of the statement `patterns` match, its kind from `lead` words, or the gap `construct`.
    pub(crate) fn checked_form(
        &mut self,
        construct: &'static str,
        start: usize,
        lead: usize,
        patterns: &[&'static str],
    ) -> Parsing<Expression> {
        let at: usize = self.at;
        let mut furthest: Option<ParseError> = None;
        for pattern in patterns {
            self.at = at;
            match self.form(pattern) {
                Ok(parts) if parts.iter().any(|part| matches!(part, Part::Opaque)) => {
                    self.gap(construct, start);
                    return Ok(Expression::Null(
                        sqlquack_syntax::expressions::names::models::Null,
                    ));
                }
                Ok(parts) => return Ok(documented_tree(lead, parts)),
                Err(error) => {
                    if furthest
                        .as_ref()
                        .is_none_or(|best| error.position > best.position)
                    {
                        furthest = Some(error);
                    }
                }
            }
        }
        self.at = at;
        Err(furthest.unwrap_or_else(|| self.error()))
    }
}
