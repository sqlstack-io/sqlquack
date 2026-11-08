//! Compound statements (`compoundBody`), checked and kept as the gap `compound`.

use sqlquack_syntax::Expression;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::null;

/// The words a labelled statement may start with.
const LABELLED: [&str; 5] = ["BEGIN", "LOOP", "WHILE", "REPEAT", "FOR"];

impl Parser<'_> {
    /// `BEGIN [NOT ATOMIC] [compoundBody] END [;]`, or the `BEGIN` of a transaction.
    pub(crate) fn begin_statement(&mut self) -> Parsing<Expression> {
        if self.kw_in(1, &["TRANSACTION", "WORK"]) && self.ends_at(2) {
            self.at += 2;
            self.gap("begin", self.tokens[self.at - 2].start);
            return Ok(null());
        }
        let start: usize = self.offset();
        self.compound_block(false)?;
        self.gap("compound", start);
        Ok(null())
    }

    /// `[label :] BEGIN [NOT ATOMIC] [compoundBody] END [label]`; a whole statement takes no labels.
    fn compound_block(&mut self, labelled: bool) -> Parsing<()> {
        self.expect_kw("BEGIN")?;
        if self.kw("NOT") && self.kw_at(1, "ATOMIC") {
            self.at += 2;
        }
        if !self.kw("END") {
            self.compound_body(&["END"])?;
        }
        self.expect_kw("END")?;
        if labelled {
            self.end_label()?;
        }
        Ok(())
    }

    /// `endLabel`, when written.
    fn end_label(&mut self) -> Parsing<()> {
        if !self.ends_at(0) {
            self.ident()?;
        }
        Ok(())
    }

    /// `(compoundStatement ;)+`, up to one of `stops`.
    fn compound_body(&mut self, stops: &[&str]) -> Parsing<()> {
        loop {
            self.compound_statement()?;
            self.expect_char(';')?;
            if self.peek().is_none() || self.kw_in(0, stops) {
                return Ok(());
            }
        }
    }

    /// One `compoundStatement`.
    fn compound_statement(&mut self) -> Parsing<()> {
        let labelled: bool =
            self.is(is_identifier) && self.char_at(1, ':') && self.kw_in(2, &LABELLED);
        if labelled {
            self.at += 2;
        }
        match self.word_at(0).as_deref() {
            Some("BEGIN") => self.compound_block(true),
            Some(word @ ("LOOP" | "WHILE" | "REPEAT" | "FOR")) => self.loop_statement(word),
            _ if labelled => Err(self.error()),
            Some("IF") => self.if_statement(),
            Some("CASE") => self.case_statement(),
            Some("LEAVE" | "ITERATE") => {
                self.at += 1;
                self.ident()?;
                Ok(())
            }
            Some("GET") => Err(self.error()),
            Some("DECLARE") if self.kw_in(1, &["CONTINUE", "EXIT"]) && self.kw_at(2, "HANDLER") => {
                self.at += 3;
                self.expect_kw("FOR")?;
                self.comma_list(|parser| parser.condition_value())?;
                match self.kw("BEGIN") {
                    true => self.compound_block(true),
                    false => self.statement().map(|_| ()),
                }
            }
            Some("DECLARE") if self.is_at(1, is_identifier) && self.kw_at(2, "CONDITION") => {
                self.at += 3;
                if self.eat_kw("FOR") {
                    self.expect_kw("SQLSTATE")?;
                    self.eat_kw("VALUE");
                    self.string_lit()?;
                }
                Ok(())
            }
            _ => self.statement().map(|_| ()),
        }
    }

    /// A `LOOP`, `WHILE`, `REPEAT` or (for any other `word`) `FOR` statement, from `word`.
    fn loop_statement(&mut self, word: &str) -> Parsing<()> {
        self.at += 1;
        match word {
            "LOOP" => {
                self.compound_body(&["END"])?;
                self.expect_kw("END")?;
                self.expect_kw("LOOP")?;
            }
            "WHILE" => {
                self.expression()?;
                self.expect_kw("DO")?;
                self.compound_body(&["END"])?;
                self.expect_kw("END")?;
                self.expect_kw("WHILE")?;
            }
            "REPEAT" => {
                self.compound_body(&["UNTIL"])?;
                self.expect_kw("UNTIL")?;
                self.expression()?;
                self.expect_kw("END")?;
                self.expect_kw("REPEAT")?;
            }
            _ => {
                if self.is(is_identifier) && self.kw_at(1, "AS") {
                    self.at += 2;
                }
                self.query()?;
                self.expect_kw("DO")?;
                self.compound_body(&["END"])?;
                self.expect_kw("END")?;
                self.expect_kw("FOR")?;
            }
        }
        self.end_label()
    }

    /// `IF condition THEN body [ELSEIF condition THEN body]... [ELSE body] END IF`.
    fn if_statement(&mut self) -> Parsing<()> {
        self.at += 1;
        self.expression()?;
        self.expect_kw("THEN")?;
        self.compound_body(&["ELSEIF", "ELSE", "END"])?;
        while self.eat_kw("ELSEIF") {
            self.expression()?;
            self.expect_kw("THEN")?;
            self.compound_body(&["ELSEIF", "ELSE", "END"])?;
        }
        if self.eat_kw("ELSE") {
            self.compound_body(&["END"])?;
        }
        self.expect_kw("END")?;
        self.expect_kw("IF")
    }

    /// `CASE [value] WHEN condition THEN body... [ELSE body] END CASE`.
    fn case_statement(&mut self) -> Parsing<()> {
        self.at += 1;
        if !self.kw("WHEN") {
            self.expression()?;
        }
        self.expect_kw("WHEN")?;
        loop {
            self.expression()?;
            self.expect_kw("THEN")?;
            self.compound_body(&["WHEN", "ELSE", "END"])?;
            if !self.eat_kw("WHEN") {
                break;
            }
        }
        if self.eat_kw("ELSE") {
            self.compound_body(&["END"])?;
        }
        self.expect_kw("END")?;
        self.expect_kw("CASE")
    }

    /// `conditionValue`: `SQLSTATE [VALUE] 'state'`, `SQLEXCEPTION`, `NOT FOUND` or a name.
    fn condition_value(&mut self) -> Parsing<()> {
        if self.kw("SQLSTATE") && (self.string_at(1) || self.kw_at(1, "VALUE")) {
            self.at += 1;
            self.eat_kw("VALUE");
            self.string_lit()?;
        } else if self.kw("NOT") && self.kw_at(1, "FOUND") {
            self.at += 2;
        } else {
            self.multipart_identifier()?;
        }
        Ok(())
    }
}
