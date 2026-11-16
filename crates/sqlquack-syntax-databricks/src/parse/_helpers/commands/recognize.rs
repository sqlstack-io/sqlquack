//! Databricks' `MATCH_RECOGNIZE` clause on a relation.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::queries::models::{
    MatchRecognize, MatchRecognizeAfter, MatchRecognizeMeasure, MatchRecognizeRows, Ordered,
    RowPattern, RowPatternQuantifier,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::alias;
use crate::scan::models::{NumberKind, Token, TokenKind};

fn is_integer(token: &Token) -> bool {
    token.kind == TokenKind::Number(NumberKind::Integer)
}

impl Parser<'_> {
    /// `MATCH_RECOGNIZE ( ... )` over `input`, and its alias.
    pub(crate) fn match_recognize(&mut self, input: Expression) -> Parsing<Expression> {
        self.expect_kw("MATCH_RECOGNIZE")?;
        self.expect_char('(')?;
        let partition_by: Option<Vec<Expression>> = match self.eat_kws(&["PARTITION", "BY"]) {
            true => Some(self.comma_list(|parser| parser.expression())?),
            false => None,
        };
        let order_by: Option<Vec<Ordered>> = match self.eat_kws(&["ORDER", "BY"]) {
            true => Some(self.comma_list(|parser| parser.sort_item())?),
            false => None,
        };
        let measures: Option<Vec<MatchRecognizeMeasure>> = match self.eat_kw("MEASURES") {
            true => Some(self.comma_list(|parser| parser.measure())?),
            false => None,
        };
        let rows: Option<MatchRecognizeRows> = if self.eat_kws(&["ONE", "ROW", "PER", "MATCH"]) {
            Some(MatchRecognizeRows::OneRowPerMatch)
        } else if self.eat_kws(&["ALL", "ROWS", "PER", "MATCH"]) {
            match self.eat_kws(&["SHOW", "EMPTY", "MATCHES"]) {
                true => Some(MatchRecognizeRows::AllRowsPerMatchShowEmptyMatches),
                false => Some(MatchRecognizeRows::AllRowsPerMatch),
            }
        } else {
            None
        };
        let after: Option<MatchRecognizeAfter> = match self.eat_kw("AFTER") {
            true => {
                for word in ["MATCH", "SKIP", "PAST", "LAST", "ROW"] {
                    self.expect_kw(word)?;
                }
                Some(MatchRecognizeAfter::PastLastRow)
            }
            false => None,
        };
        self.expect_kw("PATTERN")?;
        self.expect_char('(')?;
        let first: usize = self.at;
        let row_pattern: RowPattern = self.row_pattern()?;
        let pattern: String = match self.at > first {
            true => self.sql[self.tokens[first].start..self.tokens[self.at - 1].end].to_owned(),
            false => return Err(self.error()),
        };
        self.expect_char(')')?;
        self.expect_kw("DEFINE")?;
        let define: Vec<(Identifier, Expression)> = self.comma_list(|parser| {
            let name: Identifier = parser.ident()?;
            parser.expect_kw("AS")?;
            Ok((name, parser.expression()?))
        })?;
        self.expect_char(')')?;
        let alias_explicit_as: bool = self.kw("AS");
        let name: Option<Identifier> = self.pivot_alias()?;
        Ok(Expression::MatchRecognize(Box::new(MatchRecognize {
            options: Vec::new(),
            this: Some(Box::new(input)),
            partition_by,
            order_by,
            measures,
            rows,
            after,
            pattern: Some(pattern),
            row_pattern: Some(row_pattern),
            define: Some(define),
            define_semantics: Vec::new(),
            alias_explicit_as,
            alias: name,
        })))
    }

    /// `expression AS name`: a measure must be aliased with `AS`.
    fn measure(&mut self) -> Parsing<MatchRecognizeMeasure> {
        let start: usize = self.offset();
        let this: Expression = self.expression()?;
        if !self.eat_kw("AS") {
            return Err(self.error_with(
                "[MATCH_RECOGNIZE_MEASURES_MUST_BE_ALIASED] MATCH_RECOGNIZE MEASURES expression \
                 must be aliased with the AS keyword",
                start,
                true,
            ));
        }
        let name: Identifier = self.ident()?;
        Ok(MatchRecognizeMeasure {
            this: alias(this, name, true),
            window_frame: None,
        })
    }

    /// `row_pattern`: alternatives of sequences of quantified terms; an empty pattern is an empty sequence.
    fn row_pattern(&mut self) -> Parsing<RowPattern> {
        self.deeper(|parser| {
            if parser.is_char(')') {
                return Ok(RowPattern::Sequence(Vec::new()));
            }
            let mut alternatives: Vec<RowPattern> = Vec::new();
            loop {
                let mut terms: Vec<RowPattern> = vec![parser.pattern_term()?];
                while !(parser.is_char('|') || parser.is_char(')') || parser.is_char(',')) {
                    terms.push(parser.pattern_term()?);
                }
                alternatives.push(match terms.len() {
                    1 => terms.remove(0),
                    _ => RowPattern::Sequence(terms),
                });
                if !parser.eat_char('|') {
                    break;
                }
            }
            Ok(match alternatives.len() {
                1 => alternatives.remove(0),
                _ => RowPattern::Alternation(alternatives),
            })
        })
    }

    /// One pattern term with its quantifier, when written.
    fn pattern_term(&mut self) -> Parsing<RowPattern> {
        let term: RowPattern = if self.kw("PERMUTE") && self.char_at(1, '(') {
            self.at += 2;
            let mut members: Vec<RowPattern> = Vec::new();
            loop {
                if self.is_char(')') || self.is_char(',') {
                    return Err(self.error());
                }
                members.push(self.row_pattern()?);
                if !self.eat_char(',') {
                    break;
                }
            }
            self.expect_char(')')?;
            RowPattern::Permute(members)
        } else if self.eat_char('(') {
            let inner: RowPattern = self.row_pattern()?;
            self.expect_char(')')?;
            RowPattern::Group(Box::new(inner))
        } else if self.eat_char('^') {
            RowPattern::Start
        } else if self.eat_char('$') {
            RowPattern::End
        } else {
            RowPattern::Symbol(self.ident()?)
        };
        Ok(match self.pattern_quantifier()? {
            Some(quantifier) => RowPattern::Quantified {
                this: Box::new(term),
                quantifier,
                reluctant: false,
            },
            None => term,
        })
    }

    /// `pattern_quantifier`: `*`, `+`, `?`, `{n}`, `{min, [max]}` or `{, max}`, when written.
    fn pattern_quantifier(&mut self) -> Parsing<Option<RowPatternQuantifier>> {
        if self.eat_char('*') {
            return Ok(Some(RowPatternQuantifier::ZeroOrMore));
        }
        if self.eat_char('+') {
            return Ok(Some(RowPatternQuantifier::OneOrMore));
        }
        if self.eat_char('?') {
            return Ok(Some(RowPatternQuantifier::ZeroOrOne));
        }
        if !self.eat_char('{') {
            return Ok(None);
        }
        let min: Option<u64> = self.pattern_bound()?;
        let quantifier: RowPatternQuantifier = match self.eat_char(',') {
            true => {
                let max: Option<u64> = self.pattern_bound()?;
                if min.is_none() && max.is_none() {
                    return Err(self.error());
                }
                RowPatternQuantifier::Between { min, max }
            }
            false => RowPatternQuantifier::Exactly(min.ok_or_else(|| self.error())?),
        };
        self.expect_char('}')?;
        Ok(Some(quantifier))
    }

    /// An integer bound of a quantifier, when written.
    fn pattern_bound(&mut self) -> Parsing<Option<u64>> {
        if !self.is(is_integer) {
            return Ok(None);
        }
        let bound: u64 = self.advance()?.value.parse().map_err(|_| self.error())?;
        Ok(Some(bound))
    }
}
