//! Databricks' `MATCH_RECOGNIZE` clause on a relation.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::queries::models::{
    MatchRecognize, MatchRecognizeAfter, MatchRecognizeMeasure, MatchRecognizeRows, Ordered,
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
        self.row_pattern()?;
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
            define: Some(define),
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

    /// `row_pattern`: alternatives of sequences of terms; an empty pattern consumes nothing.
    fn row_pattern(&mut self) -> Parsing<()> {
        self.deeper(|parser| {
            if parser.is_char(')') {
                return Ok(());
            }
            loop {
                parser.pattern_term()?;
                while !(parser.is_char('|') || parser.is_char(')') || parser.is_char(',')) {
                    parser.pattern_term()?;
                }
                if !parser.eat_char('|') {
                    return Ok(());
                }
            }
        })
    }

    /// One pattern term and its quantifier.
    fn pattern_term(&mut self) -> Parsing<()> {
        if self.kw("PERMUTE") && self.char_at(1, '(') {
            self.at += 2;
            loop {
                if self.is_char(')') || self.is_char(',') {
                    return Err(self.error());
                }
                self.row_pattern()?;
                if !self.eat_char(',') {
                    break;
                }
            }
            self.expect_char(')')?;
        } else if self.eat_char('(') {
            self.row_pattern()?;
            self.expect_char(')')?;
        } else if !(self.eat_char('^') || self.eat_char('$')) {
            self.ident()?;
        }
        self.pattern_quantifier()
    }

    /// `pattern_quantifier`: `*`, `+`, `?`, `{n}`, `{min, [max]}` or `{, max}`, when written.
    fn pattern_quantifier(&mut self) -> Parsing<()> {
        if self.eat_char('*') || self.eat_char('+') || self.eat_char('?') {
            return Ok(());
        }
        if !self.eat_char('{') {
            return Ok(());
        }
        let min: bool = self.is(is_integer);
        if min {
            self.at += 1;
        }
        if self.eat_char(',') {
            if self.is(is_integer) {
                self.at += 1;
            } else if !min {
                return Err(self.error());
            }
        } else if !min {
            return Err(self.error());
        }
        self.expect_char('}')
    }
}
