//! Laying out statements and queries: each clause starts a line, at its query's indent.

use sqlquack_syntax::tokens::models::TokenType;

use crate::layout::_helpers::words::{conditions, items, step, text_is};
use crate::layout::constants::SELECT_ITEMS_BROKEN;
use crate::layout::models::{Clause, ClauseKind, Printer, Words};

impl Printer<'_, '_> {
    /// Write every statement, separated by one empty line.
    pub(crate) fn statements(&mut self) {
        let count = self.words.words.len();
        let (mut start, mut index) = (0, 0);
        while index < count {
            if self.words.words[index].kind == Some(TokenType::Semicolon) {
                self.query(start, index, 0);
                self.word(index);
                start = index + 1;
                if start < count {
                    self.blank_line(0);
                }
            }
            index = step(self.words, index);
        }
        self.query(start, count, 0);
    }

    /// Write the query `from..to`, its clauses at indent `level`; the first on the current line.
    pub(crate) fn query(&mut self, from: usize, to: usize, level: usize) {
        let clauses: Vec<Clause> = clauses(self.words, from, to);
        for (position, clause) in clauses.iter().enumerate() {
            match position
                .checked_sub(1)
                .map(|previous| clauses[previous].kind)
            {
                Some(ClauseKind::With) => self.blank_line(level),
                Some(_) => self.newline(level),
                None => {}
            }
            self.flat(clause.start, clause.body);
            self.body(clause, level);
        }
    }

    /// Write what follows a clause's keyword.
    fn body(&mut self, clause: &Clause, level: usize) {
        let (from, to) = (clause.body, clause.end);
        match clause.kind {
            ClauseKind::Prefix | ClauseKind::Inline => self.expression(from, to),
            ClauseKind::SetOperation => {
                if from < to {
                    self.newline(level);
                }
                self.expression(from, to);
            }
            ClauseKind::With => {
                let ctes = items(self.words, from, to);
                for (position, (start, end, comma)) in ctes.into_iter().enumerate() {
                    if position > 0 {
                        self.blank_line(level);
                    }
                    self.expression(start, end);
                    comma.into_iter().for_each(|comma| self.word(comma));
                }
            }
            ClauseKind::Select | ClauseKind::ListBelow => {
                let items = items(self.words, from, to);
                let single = clause.kind == ClauseKind::Select && items.len() < SELECT_ITEMS_BROKEN;
                if single && (from == to || self.fits(from, to - 1)) {
                    return self.expression(from, to);
                }
                for (start, end, comma) in items {
                    self.newline(level + 1);
                    self.expression(start, end);
                    comma.into_iter().for_each(|comma| self.word(comma));
                }
            }
            ClauseKind::Condition => {
                let joins = conditions(self.words, from, to);
                if !joins.is_empty() {
                    self.newline(level + 1);
                }
                self.condition_lines(from, to, (&joins, level + 1));
            }
            ClauseKind::Join => {
                let mut on = from;
                while on < to && self.words.words[on].kind != Some(TokenType::On) {
                    on = step(self.words, on);
                }
                self.expression(from, on);
                if on < to {
                    self.newline(level + 1);
                    let joins = conditions(self.words, on + 1, to);
                    self.condition_lines(on, to, (&joins, level + 1));
                }
            }
        }
    }

    /// Write `from..to`, starting a line at `level` before each of its joining `AND` and `OR`.
    pub(crate) fn condition_lines(
        &mut self,
        from: usize,
        to: usize,
        (joins, level): (&[usize], usize),
    ) {
        let mut start = from;
        for &join in joins {
            self.expression(start, join);
            self.newline(level);
            start = join;
        }
        self.expression(start, to);
    }
}

/// The clauses of the query `from..to`.
fn clauses(words: &Words<'_>, from: usize, to: usize) -> Vec<Clause> {
    let mut clauses: Vec<Clause> = Vec::new();
    let mut index = from;
    while index < to {
        let Some((kind, body)) = clause_at(words, index) else {
            index = step(words, index);
            continue;
        };
        match clauses.last_mut() {
            Some(last) => last.end = index,
            None if index > from => clauses.push(Clause {
                kind: ClauseKind::Prefix,
                start: from,
                body: from,
                end: index,
            }),
            None => {}
        }
        let body = body.min(to);
        clauses.push(Clause {
            kind,
            start: index,
            body,
            end: to,
        });
        index = body;
    }
    if clauses.is_empty() {
        clauses.push(Clause {
            kind: ClauseKind::Prefix,
            start: from,
            body: from,
            end: to,
        });
    }
    clauses
}

/// The clause a keyword at `index` starts, and where its body begins.
fn clause_at(words: &Words<'_>, index: usize) -> Option<(ClauseKind, usize)> {
    let kind_of = |index: usize| words.words.get(index).and_then(|word| word.kind);
    let kind = kind_of(index)?;
    let clause = match kind {
        TokenType::Select => {
            let mut body = index + 1;
            while matches!(kind_of(body), Some(TokenType::Distinct | TokenType::All)) {
                body += 1;
            }
            if kind_of(body) == Some(TokenType::On) && kind_of(body + 1) == Some(TokenType::LParen)
            {
                body = step(words, body + 1);
            }
            (ClauseKind::Select, body)
        }
        TokenType::With => {
            let recursive = kind_of(index + 1) == Some(TokenType::Recursive);
            (ClauseKind::With, index + 1 + usize::from(recursive))
        }
        TokenType::From | TokenType::Limit | TokenType::Offset => (ClauseKind::Inline, index + 1),
        TokenType::Where | TokenType::Having | TokenType::Qualify => {
            (ClauseKind::Condition, index + 1)
        }
        TokenType::Group | TokenType::Order if kind_of(index + 1) == Some(TokenType::By) => {
            (ClauseKind::ListBelow, index + 2)
        }
        TokenType::Window => (ClauseKind::ListBelow, index + 1),
        TokenType::Union | TokenType::Intersect | TokenType::Except => {
            if index
                .checked_sub(1)
                .is_some_and(|previous| kind_of(previous) == Some(TokenType::Star))
            {
                return None;
            }
            let mut body = index + 1;
            if matches!(kind_of(body), Some(TokenType::All | TokenType::Distinct)) {
                body += 1;
            }
            if kind_of(body) == Some(TokenType::By) && text_is(words, body + 1, "NAME") {
                body += 2;
            }
            (ClauseKind::SetOperation, body)
        }
        _ => {
            let mut join = index;
            while join_modifier(words, join) {
                join += 1;
            }
            if kind_of(join) != Some(TokenType::Join) {
                return None;
            }
            (ClauseKind::Join, join + 1)
        }
    };
    Some(clause)
}

/// Whether the word at `index` can come before `JOIN` in a join's keyword, such as `LEFT`.
fn join_modifier(words: &Words<'_>, index: usize) -> bool {
    matches!(
        words.words.get(index).and_then(|word| word.kind),
        Some(
            TokenType::Natural
                | TokenType::Left
                | TokenType::Right
                | TokenType::Full
                | TokenType::Inner
                | TokenType::Outer
                | TokenType::Cross
                | TokenType::Semi
                | TokenType::Anti
                | TokenType::Positional
        )
    ) || text_is(words, index, "ASOF")
}
