//! Statements and clauses checked against a syntax pattern in the reference's notation.

use sqlquack_syntax::Expression;

use crate::parse::_helpers::commands::documented::rule;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_word};
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

fn is_integer(token: &Token) -> bool {
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
    /// The positions after `slot` from `at`, recording a failure.
    fn slot(&mut self, slot: &str, at: usize, furthest: &mut Furthest) -> Option<usize> {
        self.at = at;
        let result: Result<(), (ParseError, usize)> = self.attempt(|parser| {
            match slot {
                "name" => {
                    parser.multipart_identifier()?;
                }
                "id" => {
                    parser.ident()?;
                }
                "table" => {
                    parser.table_reference()?;
                }
                "str" => {
                    parser.string_lit()?;
                }
                "int" => {
                    parser.expect_token(is_integer)?;
                }
                "num" => {
                    parser.eat_char('-');
                    parser.expect_token(is_number)?;
                }
                "literal" => {
                    parser.literal_value()?;
                }
                "expr" => {
                    parser.expression()?;
                }
                "query" => {
                    parser.query()?;
                }
                "type" => {
                    parser.data_type()?;
                }
                "props" => {
                    parser.valued_property_list()?;
                }
                "keys" => {
                    parser.key_property_list()?;
                }
                "plist" => {
                    parser.property_list()?;
                }
                "partition" => {
                    parser.partition_spec()?;
                }
                "columns" => {
                    parser.identifier_list()?;
                }
                "statement" => {
                    parser.statement()?;
                }
                "column" => {
                    parser.column_definition()?;
                }
                "table_constraint" => {
                    parser.table_constraint()?;
                }
                _ => return Err(parser.error_with(slot, parser.offset(), true)),
            }
            Ok(())
        });
        match result {
            Ok(()) => Some(self.at),
            Err((error, _)) => {
                furthest.record(error);
                None
            }
        }
    }

    /// Records the syntax error at token `at` as a failure.
    fn miss(&mut self, at: usize, furthest: &mut Furthest) {
        self.at = at;
        furthest.record(self.error());
    }

    /// The positions after `node` from any of `from`, sorted and without repeats.
    fn follow(&mut self, node: &Node, from: Vec<usize>, furthest: &mut Furthest) -> Vec<usize> {
        let mut to: Vec<usize> = match node {
            Node::Word(word) => {
                let mut to: Vec<usize> = Vec::new();
                for at in from {
                    match self
                        .tokens
                        .get(at)
                        .is_some_and(|token| is_word(token, word))
                    {
                        true => to.push(at + 1),
                        false => self.miss(at, furthest),
                    }
                }
                to
            }
            Node::Punct(text) => {
                let mut to: Vec<usize> = Vec::new();
                for at in from {
                    let found: bool = self.tokens.get(at).is_some_and(|token| match token.kind {
                        TokenKind::Char(character) => {
                            text.len() == 1 && text.starts_with(character)
                        }
                        TokenKind::Op => token.value == *text,
                        _ => false,
                    });
                    match found {
                        true => to.push(at + 1),
                        false => self.miss(at, furthest),
                    }
                }
                to
            }
            Node::Slot(slot) => match rule(slot) {
                Some(pattern) => self.follow(&compile(pattern), from, furthest),
                None => from
                    .into_iter()
                    .filter_map(|at| self.slot(slot, at, furthest))
                    .collect(),
            },
            Node::Seq(items) => {
                let mut at: Vec<usize> = from;
                for item in items {
                    if at.is_empty() {
                        break;
                    }
                    at = self.follow(item, at, furthest);
                }
                at
            }
            Node::Alt(choices) => {
                let mut to: Vec<usize> = Vec::new();
                for choice in choices {
                    to.extend(self.follow(choice, from.clone(), furthest));
                }
                to
            }
            Node::Opt(inner) => {
                let mut to: Vec<usize> = self.follow(inner, from.clone(), furthest);
                to.extend(from);
                to
            }
            Node::Many(inner, comma) => {
                let mut to: Vec<usize> = Vec::new();
                let mut at: Vec<usize> = self.follow(inner, from, furthest);
                while !at.is_empty() {
                    at.retain(|position| !to.contains(position));
                    to.extend(at.iter().copied());
                    let next: Vec<usize> = match comma {
                        true => at
                            .into_iter()
                            .filter(|position| {
                                self.tokens
                                    .get(*position)
                                    .is_some_and(|token| token.kind == TokenKind::Char(','))
                            })
                            .map(|position| position + 1)
                            .collect(),
                        false => at,
                    };
                    at = match next.is_empty() {
                        true => next,
                        false => self.follow(inner, next, furthest),
                    };
                }
                to
            }
        };
        to.sort_unstable();
        to.dedup();
        to
    }

    /// `literal`: a number, with an optional sign, a string, a boolean or `NULL`.
    fn literal_value(&mut self) -> Parsing<()> {
        if self.eat_char('-') || self.eat_char('+') {
            self.expect_token(is_number)?;
            return Ok(());
        }
        if self.is(is_number) || self.is(is_string) {
            self.at += 1;
            while self.is(is_string) {
                self.at += 1;
            }
            return Ok(());
        }
        match self.kw_in(0, &["TRUE", "FALSE", "NULL"]) {
            true => {
                self.at += 1;
                Ok(())
            }
            false => Err(self.error()),
        }
    }

    /// Matches `pattern` from here to the end of the statement; the parser ends there.
    pub(crate) fn form(&mut self, pattern: &'static str) -> Parsing<()> {
        let node: Node = compile(pattern);
        let (gaps, opaque) = (self.gaps.len(), self.opaque);
        let start: usize = self.at;
        let mut furthest: Furthest = Furthest { error: None };
        let ends: Vec<usize> = self.follow(&node, vec![start], &mut furthest);
        self.gaps.truncate(gaps);
        self.opaque = opaque;
        let end: Option<usize> = ends.iter().copied().find(|at| {
            self.tokens
                .get(*at)
                .is_none_or(|token| token.kind == TokenKind::Char(';'))
        });
        if let Some(end) = end {
            self.at = end;
            return Ok(());
        }
        for at in ends {
            self.miss(at, &mut furthest);
        }
        self.at = start;
        Err(furthest.error.unwrap_or_else(|| self.error()))
    }

    /// Checks the statement against `patterns` and keeps it as the gap `construct` from `start`.
    pub(crate) fn checked_form(
        &mut self,
        construct: &'static str,
        start: usize,
        patterns: &[&'static str],
    ) -> Parsing<Expression> {
        let at: usize = self.at;
        let mut furthest: Option<ParseError> = None;
        for pattern in patterns {
            self.at = at;
            match self.form(pattern) {
                Ok(()) => {
                    self.gap(construct, start);
                    return Ok(Expression::Null(
                        sqlquack_syntax::expressions::names::models::Null,
                    ));
                }
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
