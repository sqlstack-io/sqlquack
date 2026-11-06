//! Where lines break: before a token, and after one that asks for a break.

use crate::layout::_helpers::plan::Plan;
use crate::layout::_helpers::tokens::{Token, TokenKind, tokens};
use crate::layout::constants::{
    BY_CLAUSE_WORDS, CLAUSE_WORDS, INDENT, INLINE_WIDTH, JOIN_MODIFIERS, NON_CALL_WORDS,
    SELECT_MODIFIERS, SET_OPERATION_MODIFIERS, SET_OPERATIONS,
};
use crate::layout::models::{Case, Clause, Frame, Walk};

/// `sql` one clause and select item per line, nested queries and long `CASE`s indented by depth.
pub(crate) fn lay_out(sql: &str) -> String {
    let tokens: Vec<Token<'_>> = tokens(sql);
    let plan = Plan::new(&tokens);
    let mut walk = start(plan.width(0, tokens.len().saturating_sub(1)) > INLINE_WIDTH);
    for index in 0..tokens.len() {
        let level = walk.line_break(&tokens, index);
        walk.push_token(&tokens, index, level);
        walk.forced = walk.after_token(&tokens, index, &plan);
    }
    walk.laid_out
}

/// The walk over a statement, which breaks its lines when `broken`.
fn start(broken: bool) -> Walk {
    Walk {
        frames: vec![Frame {
            query: true,
            broken,
            inline: !broken,
            call: false,
            base: 0,
            close: 0,
            clause: Clause::Other,
            item_pending: false,
        }],
        cases: Vec::new(),
        line_indent: 0,
        forced: None,
        laid_out: String::new(),
    }
}

impl Walk {
    /// Appends `tokens[index]`, on a new line at `level` or after its own spacing.
    pub(crate) fn push_token(&mut self, tokens: &[Token<'_>], index: usize, level: Option<usize>) {
        let token = &tokens[index];
        match (index, level) {
            (0, _) => {}
            (_, Some(level)) => {
                self.laid_out.push('\n');
                self.laid_out.push_str(&INDENT.repeat(level));
                self.line_indent = level;
            }
            (_, None) if token.spaced => self.laid_out.push(' '),
            (_, None) => {}
        }
        self.laid_out.push_str(token.text);
    }

    /// The indentation level of the new line `tokens[index]` starts, or `None` to keep its line.
    pub(crate) fn line_break(&mut self, tokens: &[Token<'_>], index: usize) -> Option<usize> {
        if let Some((_, level)) = self.forced.filter(|(at, _)| *at == index) {
            return Some(level);
        }
        let token = &tokens[index];
        let depth = self.frames.len();
        let case = self.cases.last().filter(|case| case.depth == depth);
        let frame = self.frames.last_mut()?;
        if token.is_symbol(")") {
            return (depth > 1 && frame.query && frame.broken).then_some(frame.close);
        }
        if frame.inline {
            return None;
        }
        if let Some(level) = case.and_then(|case| case_break(token, case)) {
            return Some(level);
        }
        if !frame.query || !frame.broken {
            return None;
        }
        if frame.item_pending {
            return frame.first_item_break(tokens, index);
        }
        (starts_clause(tokens, index) || is_set_operation(tokens, index)).then_some(frame.base)
    }

    /// Opens or closes what `tokens[index]` opens or closes, and the break it asks for after it.
    pub(crate) fn after_token(
        &mut self,
        tokens: &[Token<'_>],
        index: usize,
        plan: &Plan,
    ) -> Option<(usize, usize)> {
        let token = &tokens[index];
        match token {
            token if token.is_symbol("(") => self.open_frame(tokens, index, plan),
            token if token.is_symbol(")") => {
                if self.frames.len() > 1 {
                    self.frames.pop();
                }
                None
            }
            token if token.is("CASE") => {
                let frame = self.frames.last()?;
                let broken = !frame.inline
                    && !frame.call
                    && plan.width(index, plan.case_end(index)) > INLINE_WIDTH;
                self.cases.push(Case {
                    broken,
                    indent: self.line_indent,
                    depth: self.frames.len(),
                });
                None
            }
            token if token.is("END") => {
                self.cases.pop();
                None
            }
            token if token.kind == TokenKind::LineComment => Some((index + 1, self.line_indent)),
            _ => self.frames.last_mut()?.clause_break(tokens, index),
        }
    }

    /// Opens the frame of the parenthesis at `tokens[index]`, breaking inside a long query.
    fn open_frame(
        &mut self,
        tokens: &[Token<'_>],
        index: usize,
        plan: &Plan,
    ) -> Option<(usize, usize)> {
        let frame = self.frames.last()?;
        let query = plan.opens_query(tokens, index);
        let broken = query && !frame.inline && plan.width(index, plan.close(index)) > INLINE_WIDTH;
        let base = self.line_indent + 1;
        let opened = Frame {
            query,
            broken,
            inline: frame.inline || (query && !broken),
            call: !query && (frame.call || calls(tokens, index)),
            base,
            close: self.line_indent,
            clause: Clause::Other,
            item_pending: false,
        };
        self.frames.push(opened);
        broken.then_some((index + 1, base))
    }
}

impl Frame {
    /// The line the select list's first item starts, after any `DISTINCT` or `DISTINCT ON (…)`.
    fn first_item_break(&mut self, tokens: &[Token<'_>], index: usize) -> Option<usize> {
        let token = &tokens[index];
        let modifier = SELECT_MODIFIERS.iter().any(|word| token.is(word))
            || (token.is_symbol("(") && tokens[index - 1].is("ON"));
        if modifier {
            return None;
        }
        self.item_pending = false;
        Some(self.base + 1)
    }

    /// The clause `tokens[index]` moves a broken query into, and the break its commas ask for.
    fn clause_break(&mut self, tokens: &[Token<'_>], index: usize) -> Option<(usize, usize)> {
        let token = &tokens[index];
        if !self.query || !self.broken {
            return None;
        }
        match token {
            token if token.is("SELECT") => {
                (self.clause, self.item_pending) = (Clause::Select, true);
                None
            }
            token if token.is("WITH") && self.clause == Clause::Other => {
                self.clause = Clause::With;
                None
            }
            token if token.is_symbol(",") => match self.clause {
                Clause::Select => Some((index + 1, self.base + 1)),
                Clause::With => Some((index + 1, self.base)),
                Clause::Other => None,
            },
            _ if is_set_operation(tokens, index) => {
                self.clause = Clause::Other;
                Some((after_modifiers(tokens, index), self.base))
            }
            _ if starts_clause(tokens, index) => {
                self.clause = Clause::Other;
                None
            }
            _ => None,
        }
    }
}

/// The level of the line `token` starts within `case`, when the `CASE` breaks.
fn case_break(token: &Token<'_>, case: &Case) -> Option<usize> {
    match case.broken {
        true if token.is("WHEN") || token.is("ELSE") => Some(case.indent + 1),
        true if token.is("END") => Some(case.indent),
        _ => None,
    }
}

/// The index of the first token after the set operation at `index` and its modifiers.
fn after_modifiers(tokens: &[Token<'_>], index: usize) -> usize {
    (index + 1..tokens.len())
        .find(|at| !is_set_operation_modifier(&tokens[*at]))
        .unwrap_or(tokens.len())
}

/// Whether `token` is `ALL`, `DISTINCT`, `BY` or `NAME` after a set operation.
fn is_set_operation_modifier(token: &Token<'_>) -> bool {
    SET_OPERATION_MODIFIERS.iter().any(|word| token.is(word))
}

/// Whether `token` can come before `JOIN` in a join.
fn is_join_modifier(token: &Token<'_>) -> bool {
    JOIN_MODIFIERS.iter().any(|word| token.is(word))
}

/// Whether `tokens[index]` starts a clause: a clause word, `GROUP BY`, `ORDER BY` or a join.
fn starts_clause(tokens: &[Token<'_>], index: usize) -> bool {
    let token = &tokens[index];
    let previous = index.checked_sub(1).map(|at| &tokens[at]);
    let next = tokens.get(index + 1);
    if token.is("FROM") && previous.is_some_and(|previous| previous.is("DISTINCT")) {
        return false;
    }
    let joins = (token.is("JOIN") || is_join_modifier(token))
        && !previous.is_some_and(is_join_modifier)
        && tokens[index..]
            .iter()
            .find(|token| !is_join_modifier(token))
            .is_some_and(|token| token.is("JOIN"));
    CLAUSE_WORDS.iter().any(|word| token.is(word))
        || (BY_CLAUSE_WORDS.iter().any(|word| token.is(word))
            && next.is_some_and(|next| next.is("BY")))
        || joins
}

/// Whether `tokens[index]` is a set operation, not the `EXCEPT` of a `* EXCEPT (…)` column list.
fn is_set_operation(tokens: &[Token<'_>], index: usize) -> bool {
    SET_OPERATIONS.iter().any(|word| tokens[index].is(word))
        && !(tokens[index].is("EXCEPT") && index > 0 && tokens[index - 1].is_symbol("*"))
}

/// Whether the parenthesis at `tokens[index]` calls a function rather than grouping.
fn calls(tokens: &[Token<'_>], index: usize) -> bool {
    let Some(previous) = index.checked_sub(1).map(|at| &tokens[at]) else {
        return false;
    };
    match previous.kind {
        TokenKind::Word => !NON_CALL_WORDS.iter().any(|word| previous.is(word)),
        TokenKind::Quoted => true,
        _ => previous.is_symbol("]") || previous.is_symbol(")"),
    }
}
