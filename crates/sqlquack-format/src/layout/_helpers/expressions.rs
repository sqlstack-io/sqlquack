//! Laying out expressions: brackets that fit stay on their line, others break, outermost first.

use sqlquack_syntax::tokens::models::TokenType;

use crate::layout::_helpers::words::{
    conditions, forced, items, kind_at, opens_query, spaced, step, width, window_orders,
};
use crate::layout::constants::LINE_WIDTH;
use crate::layout::models::Printer;

impl Printer<'_, '_> {
    /// Write `from..to`, breaking each bracket that holds a query or CASE, or does not fit.
    pub(crate) fn expression(&mut self, from: usize, to: usize) {
        let mut index = from;
        while index < to {
            let next = step(self.words, index);
            let closes_inside = next <= to && next - 1 > index;
            match self.words.words[index].kind {
                Some(TokenType::LParen) if closes_inside => self.bracket(index, next - 1),
                Some(TokenType::Case) if closes_inside => self.case(index, next - 1),
                _ => self.word(index),
            }
            index = next;
        }
    }

    /// Whether `from..=to` fits on the current line and holds nothing that must break.
    pub(crate) fn fits(&self, from: usize, to: usize) -> bool {
        let space = usize::from(!self.output.line_start && spaced(self.words, from));
        let width = self.output.column + space + width(self.words, from, to);
        width <= LINE_WIDTH && !forced(self.words, from, to)
    }

    /// Write the bracket from `open` to `close`: on the line when it fits, else over lines.
    fn bracket(&mut self, open: usize, close: usize) {
        let level = self.output.level;
        if self.fits(open, close) {
            return self.flat(open, close + 1);
        }
        self.word(open);
        let case = kind_at(self.words, open + 1) == Some(TokenType::Case);
        if case && step(self.words, open + 1) == close {
            self.case(open + 1, close - 1);
            return self.word(close);
        }
        let window = matches!(
            kind_at(self.words, open + 1),
            Some(TokenType::Partition | TokenType::Order)
        );
        match (opens_query(self.words, open), window) {
            (true, _) => {
                self.newline(level + 1);
                self.query(open + 1, close, level + 1);
            }
            (false, true) => {
                self.newline(level + 1);
                let orders = window_orders(self.words, open + 1, close);
                self.condition_lines(open + 1, close, (&orders, level + 1));
            }
            (false, false) => {
                for (start, end, comma) in items(self.words, open + 1, close) {
                    self.newline(level + 1);
                    self.expression(start, end);
                    comma.into_iter().for_each(|comma| self.word(comma));
                }
            }
        }
        self.newline(level);
        self.word(close);
    }

    /// Write the CASE from `case` to `end`: on the line when it fits, else one branch per line.
    fn case(&mut self, case: usize, end: usize) {
        if self.fits(case, end) {
            return self.flat(case, end + 1);
        }
        let level = self.output.level;
        self.word(case);
        let mut start = case + 1;
        let mut index = start;
        while index < end {
            let branch = matches!(
                self.words.words[index].kind,
                Some(TokenType::When | TokenType::Then | TokenType::Else)
            );
            if branch {
                self.branch(start, index, level + 1);
                self.newline(level + 1);
                start = index;
            }
            index = step(self.words, index);
        }
        self.branch(start, end, level + 1);
        self.newline(level);
        self.word(end);
    }

    /// Write a CASE branch `from..to`, breaking before its `AND` and `OR` when it is too wide.
    fn branch(&mut self, from: usize, to: usize, level: usize) {
        let joins = match from < to && self.fits(from, to - 1) {
            true => Vec::new(),
            false => conditions(self.words, from + 1, to),
        };
        self.condition_lines(from, to, (&joins, level));
    }
}
