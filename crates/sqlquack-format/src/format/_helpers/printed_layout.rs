//! Printed SQL as tokens and gaps, with clause layout rules and line-width wrapping.

use std::collections::HashMap;
use std::ops::Range;

use sqlquack_syntax::tokens::models::TokenType;

const OPEN_PARENTHESIS: &str = "(";

/// The indent step of the parse-tree layout every printed statement starts from.
const PARSED_INDENT: usize = 2;
const COMMA: &str = ",";
const SINGLE_SPACE: &str = " ";
const BETWEEN_KEYWORD: &str = "BETWEEN";
const BETWEEN_SEPARATOR: &str = "AND";
const VALUES_KEYWORD: &str = "VALUES";
const DECODE_FUNCTION: &str = "DECODE";
const CASE_KEYWORD: &str = "CASE";
const END_KEYWORD: &str = "END";
const WINDOW_KEYWORD: &str = "OVER";
const JOIN_KEYWORD: &str = "JOIN";
const SINGLE_ITEM_CLAUSES: [&str; 4] = ["SELECT", "WHERE", "QUALIFY", "HAVING"];
/// Words whose clause header is `<word> BY`, such as GROUP BY.
const BY_CLAUSE_WORDS: [&str; 6] = [
    "GROUP",
    "ORDER",
    "PARTITION",
    "CLUSTER",
    "DISTRIBUTE",
    "SORT",
];
/// Keywords that start a join's condition, which joins its line when both fit.
const JOIN_CONDITIONS: [&str; 2] = ["ON", "USING"];
/// Tokens at a condition's depth that show, before any JOIN, that it is not a join's condition.
const NOT_JOIN_CONDITION: [&str; 9] = [
    "FROM", "WHERE", "SELECT", "ON", "USING", "INTO", "INDEX", "TABLE", ",",
];
const SELECT_MODIFIERS: [&str; 10] = [
    "DISTINCT", "ALL", "TOP", "PERCENT", "WITH", "TIES", "ON", "AS", "STRUCT", "VALUE",
];
const LEADING_OPERATORS: [&str; 9] = ["+", "-", "*", "/", "//", "%", "||", "AND", "OR"];
const CHAIN_KEYWORDS: [&str; 2] = ["AND", "OR"];
const BETWEEN_BOUNDARIES: [&str; 12] = [
    "AND", "OR", "WHEN", "THEN", "ELSE", "WHERE", "ON", "HAVING", "QUALIFY", "SELECT", ",", "NOT",
];
/// Binary operators a long line breaks before, loosest-binding first: a line breaks at the first tier it holds.
const OPERATOR_TIERS: [&[&str]; 3] = [&["||"], &["+", "-"], &["*", "/", "//", "%"]];
const WINDOW_CLAUSE_KEYWORDS: [&str; 4] = ["ORDER", "ROWS", "RANGE", "GROUPS"];
const CASE_BRANCH_KEYWORDS: [&str; 3] = ["WHEN", "THEN", "ELSE"];
const TYPE_PARAMETER_WORDS: [&str; 1] = ["MAX"];
const TYPE_NAMES: [&str; 30] = [
    "BIGNUMERIC",
    "BINARY",
    "BIT",
    "CHAR",
    "CHARACTER",
    "DATETIME",
    "DATETIME2",
    "DATETIMEOFFSET",
    "DEC",
    "DECIMAL",
    "DOUBLE",
    "FLOAT",
    "INTERVAL",
    "NCHAR",
    "NUMBER",
    "NUMERIC",
    "NVARCHAR",
    "STRING",
    "TEXT",
    "TIME",
    "TIMESTAMP",
    "TIMESTAMPTZ",
    "TIMESTAMP_LTZ",
    "TIMESTAMP_NTZ",
    "TIMESTAMP_TZ",
    "TIMETZ",
    "VARBINARY",
    "VARCHAR",
    "BYTES",
    "BPCHAR",
];

/// Printed SQL whose layout passes only ever rewrite the gaps between tokens.
pub(crate) struct PrintedSql<'a> {
    tokens: Vec<String>,
    kinds: Vec<TokenType>,
    /// The text before each token, plus the text after the last token.
    gaps: Vec<String>,
    depths: Vec<usize>,
    partners: Vec<Option<usize>>,
    /// Gaps that line wrapping must keep: inside VALUES rows and type parameter lists.
    locked: Vec<bool>,
    token_widths: &'a HashMap<String, usize>,
    /// Spaces per indent step.
    indent: usize,
}

/// One printed line; end columns are absent when comments or multi-line tokens pin it.
struct Line {
    first: usize,
    last: usize,
    indentation: usize,
    end_columns: Option<Vec<usize>>,
}

impl<'a> PrintedSql<'a> {
    /// Split printed `sql` at its token byte ranges; `None` when it has no tokens.
    pub(crate) fn new(
        sql: &str,
        printed_tokens: &[(Range<usize>, TokenType)],
        token_widths: &'a HashMap<String, usize>,
    ) -> Option<Self> {
        if printed_tokens.is_empty() {
            return None;
        }
        let mut texts: Vec<String> = Vec::with_capacity(printed_tokens.len());
        let mut gaps: Vec<String> = Vec::with_capacity(printed_tokens.len() + 1);
        let mut previous_end = 0_usize;
        for (range, _) in printed_tokens {
            gaps.push(sql[previous_end..range.start].to_owned());
            texts.push(sql[range.clone()].to_owned());
            previous_end = range.end;
        }
        gaps.push(sql[previous_end..].to_owned());
        let (depths, partners) = bracket_structure(&texts);
        let locked = vec![false; texts.len() + 1];
        Some(Self {
            kinds: printed_tokens.iter().map(|(_, kind)| *kind).collect(),
            tokens: texts,
            gaps,
            depths,
            partners,
            locked,
            token_widths,
            indent: PARSED_INDENT,
        })
    }

    fn len(&self) -> usize {
        self.tokens.len()
    }

    fn upper(&self, index: usize) -> String {
        self.tokens[index].to_ascii_uppercase()
    }

    fn is(&self, index: usize, text: &str) -> bool {
        self.tokens
            .get(index)
            .is_some_and(|token| token.eq_ignore_ascii_case(text))
    }

    fn line_ranges(&self, first: usize, last: usize) -> Vec<(usize, usize)> {
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        let mut start = first;
        for index in first + 1..=last {
            if self.gaps[index].contains('\n') {
                ranges.push((start, index - 1));
                start = index;
            }
        }
        ranges.push((start, last));
        ranges
    }

    /// The first and last token of the printed line holding `index`.
    fn line_of(&self, index: usize) -> (usize, usize) {
        let first = (1..=index)
            .rev()
            .find(|&position| self.gaps[position].contains('\n'))
            .unwrap_or(0);
        let last = (index + 1..self.len())
            .find(|&position| self.gaps[position].contains('\n'))
            .map_or(self.len() - 1, |position| position - 1);
        (first, last)
    }

    fn line(&self, first: usize, last: usize) -> Line {
        let indentation = self.indentation(first);
        let lead = self.gaps[first].rsplit('\n').next().unwrap_or_default();
        let trailing = self.gaps[last + 1].split('\n').next().unwrap_or_default();
        let commented = !lead.trim().is_empty()
            || !trailing.trim().is_empty()
            || (first + 1..=last).any(|index| !self.gaps[index].trim().is_empty());
        let multiline = (first..=last).any(|index| self.tokens[index].contains('\n'));
        let end_columns = (!commented && !multiline).then(|| {
            let mut column = indentation;
            let mut ends: Vec<usize> = Vec::with_capacity(last - first + 1);
            for index in first..=last {
                if index > first {
                    column += self.gaps[index].chars().count();
                }
                column += self.width(index);
                ends.push(column);
            }
            ends
        });
        Line {
            first,
            last,
            indentation,
            end_columns,
        }
    }

    /// Indentation of the line that starts at token `first`.
    fn indentation(&self, first: usize) -> usize {
        self.gaps[first]
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .chars()
            .count()
    }

    fn width(&self, index: usize) -> usize {
        let token = &self.tokens[index];
        self.token_widths
            .get(&token.to_ascii_uppercase())
            .copied()
            .unwrap_or_else(|| token.chars().count())
    }

    /// Whether any gap of the tokens after `first` up to `last` holds a line break.
    fn spans_lines(&self, first: usize, last: usize) -> bool {
        (first + 1..=last).any(|index| self.gaps[index].contains('\n'))
    }

    pub(crate) fn render(&self) -> String {
        let mut output = String::new();
        for (gap, token) in self.gaps.iter().zip(&self.tokens) {
            output.push_str(gap);
            output.push_str(token);
        }
        output.push_str(&self.gaps[self.tokens.len()]);
        output
    }
}

fn bracket_structure(tokens: &[String]) -> (Vec<usize>, Vec<Option<usize>>) {
    let mut depths: Vec<usize> = Vec::with_capacity(tokens.len());
    let mut partners: Vec<Option<usize>> = vec![None; tokens.len()];
    let mut open: Vec<usize> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.as_str() {
            "(" | "[" | "{" => {
                depths.push(open.len());
                open.push(index);
            }
            ")" | "]" | "}" => {
                let opener = open.pop();
                depths.push(open.len());
                if let Some(opener) = opener {
                    partners[opener] = Some(index);
                    partners[index] = Some(opener);
                }
            }
            _ => depths.push(open.len()),
        }
    }
    (depths, partners)
}

fn newline(indentation: usize) -> String {
    format!("\n{}", " ".repeat(indentation))
}

impl PrintedSql<'_> {
    /// Re-indent every line from the parse-tree indent step to `indent` spaces per step.
    pub(crate) fn set_indent(&mut self, indent: usize) {
        if indent == self.indent {
            return;
        }
        let step = self.indent;
        let scale = |spaces: usize| spaces / step * indent + spaces % step;
        for gap in &mut self.gaps {
            if !gap.contains('\n') {
                continue;
            }
            let comment_start = if gap.contains("/*") {
                gap.rfind('\n').unwrap_or(0)
            } else {
                0
            };
            let (kept, scaled) = gap.split_at(comment_start);
            let mut rebuilt = kept.to_string();
            for (position, segment) in scaled.split('\n').enumerate() {
                if position > 0 {
                    rebuilt.push('\n');
                    let spaces = segment.len() - segment.trim_start_matches(' ').len();
                    rebuilt.push_str(&" ".repeat(scale(spaces)));
                    rebuilt.push_str(&segment[spaces..]);
                } else {
                    rebuilt.push_str(segment);
                }
            }
            *gap = rebuilt;
        }
        self.indent = indent;
    }

    /// Rejoin each multi-line expression group, innermost first, whose joined line fits.
    pub(crate) fn join_fitting_groups(&mut self, line_width: usize) {
        let mut openers: Vec<usize> = (0..self.len())
            .filter(|&index| self.tokens[index] == OPEN_PARENTHESIS)
            .collect();
        openers.sort_by_key(|&index| std::cmp::Reverse(self.depths[index]));
        for opener in openers {
            let Some(closer) = self.partners[opener] else {
                continue;
            };
            if !self.spans_lines(opener, closer) || !self.joinable_group(opener, closer) {
                continue;
            }
            let (first, _) = self.line_of(opener);
            let (_, last) = self.line_of(closer);
            let indentation = self.indentation(first);
            let fits = |start: usize| {
                self.joined_width(start, last, indentation)
                    .is_some_and(|width| width <= line_width)
            };
            let start = if fits(first) {
                first
            } else if let Some(operator) = self.leading_operator_before(first, opener)
                && fits(operator)
            {
                operator
            } else {
                continue;
            };
            for index in opener + 1..=closer {
                if self.gaps[index].contains('\n') {
                    self.gaps[index] = self.joined_gap(index).to_string();
                }
            }
            if start != first {
                self.gaps[start] = newline(indentation);
            }
        }
    }

    /// The last binary operator on the line from `first` that applies to the group at `opener`.
    fn leading_operator_before(&self, first: usize, opener: usize) -> Option<usize> {
        (first + 1..opener).rev().find(|&index| {
            self.depths[index] == self.depths[opener]
                && LEADING_OPERATORS.contains(&self.upper(index).as_str())
                && self.gaps[index] == SINGLE_SPACE
                && self.ends_operand(index - 1)
        })
    }

    /// Move a comma that follows a line comment to the end of the commented line.
    pub(crate) fn attach_commas_before_comments(&mut self) {
        for index in 1..self.len().saturating_sub(1) {
            if self.tokens[index] != COMMA {
                continue;
            }
            let Some((comment, rest)) = self.gaps[index].split_once('\n') else {
                continue;
            };
            if !comment.trim_start().starts_with("--") || !rest.trim().is_empty() {
                continue;
            }
            let (comment, rest) = (comment.to_string(), rest.to_string());
            let next = self.gaps[index + 1].trim_start_matches(' ').to_string();
            self.gaps[index] = String::new();
            self.gaps[index + 1] = if next.starts_with('\n') {
                comment + &next
            } else {
                format!("{comment}\n{rest}{next}")
            };
        }
    }

    fn joinable_group(&self, opener: usize, closer: usize) -> bool {
        (opener + 1..closer).all(|index| {
            !(self.kinds[index] == TokenType::Select
                || self.is(index, VALUES_KEYWORD)
                || (self.is(index, CASE_KEYWORD)
                    && self
                        .case_end(index)
                        .is_none_or(|end| self.spans_lines(index, end))))
        })
    }

    /// The width of tokens `first..=last` on one line at `indentation`; `None` when comments pin.
    fn joined_width(&self, first: usize, last: usize, indentation: usize) -> Option<usize> {
        let trailing = self.gaps[last + 1].split('\n').next().unwrap_or_default();
        let pinned = !trailing.trim().is_empty()
            || (first + 1..=last).any(|index| !self.gaps[index].trim().is_empty())
            || (first..=last).any(|index| self.tokens[index].contains('\n'));
        if pinned {
            return None;
        }
        let mut width = indentation;
        for index in first..=last {
            if index > first {
                width += if self.gaps[index].contains('\n') {
                    self.joined_gap(index).len()
                } else {
                    self.gaps[index].chars().count()
                };
            }
            width += self.width(index);
        }
        Some(width)
    }

    /// The gap before `index` once its line break is removed.
    fn joined_gap(&self, index: usize) -> &'static str {
        if self.tokens[index - 1] == OPEN_PARENTHESIS
            || matches!(self.tokens[index].as_str(), ")" | ",")
        {
            ""
        } else {
            SINGLE_SPACE
        }
    }

    /// Apply the layout rules that only move existing line breaks before wrapping.
    pub(crate) fn arrange_clauses(&mut self, line_width: usize) {
        for index in 0..self.len() {
            if self.is(index, DECODE_FUNCTION) {
                self.pair_decode_arguments(index);
            }
        }
        for index in 1..self.len() {
            if CHAIN_KEYWORDS.contains(&self.upper(index).as_str()) {
                self.join_chain_break_in_group(index);
            }
        }
        for index in 0..self.len() {
            if self.is(index, VALUES_KEYWORD) {
                self.break_values_rows_unless_fitting(index, line_width);
            }
        }
        for index in 0..self.len() {
            if self.kinds[index] == TokenType::With {
                self.separate_common_table_expressions(index);
            }
        }
        for index in 0..self.len() {
            let word = self.upper(index);
            if SINGLE_ITEM_CLAUSES.contains(&word.as_str())
                || (BY_CLAUSE_WORDS.contains(&word.as_str()) && self.is(index + 1, "BY"))
            {
                self.join_single_line_body(index, line_width);
            }
        }
        for index in 1..self.len() {
            if JOIN_CONDITIONS.contains(&self.upper(index).as_str()) {
                self.join_condition_line(index, line_width);
            }
        }
    }

    /// Start the inside of each parenthesised group that spans lines on its own line and close it on its own line, so a group never opens or closes on the line of another; true when a gap changed.
    pub(crate) fn separate_multiline_groups(&mut self) -> bool {
        let rows: Vec<usize> = (0..self.len())
            .filter(|&index| self.is(index, VALUES_KEYWORD))
            .flat_map(|values| self.values_rows(values))
            .map(|(opener, _)| opener)
            .collect();
        let mut changed = false;
        for opener in 0..self.len() {
            if self.tokens[opener] != OPEN_PARENTHESIS || rows.contains(&opener) {
                continue;
            }
            let Some(closer) = self.partners[opener] else {
                continue;
            };
            if closer <= opener + 1
                || self.locked[opener + 1]
                || !self.spans_lines(opener, closer)
                || !self.gaps[opener + 1].trim().is_empty()
                || !self.gaps[closer].trim().is_empty()
            {
                continue;
            }
            let opens = !self.gaps[opener + 1].contains('\n');
            let closes = !self.gaps[closer].contains('\n');
            if !opens && !closes {
                continue;
            }
            let (first, _) = self.line_of(opener);
            let indentation = self.indentation(first);
            if opens {
                self.indent_lines(opener + 2, closer - 1, self.indent);
                let inner = newline(indentation + self.indent);
                self.gaps[opener + 1] = inner.clone();
                let separates_arguments = opener > 0
                    && !self.is(opener - 1, DECODE_FUNCTION)
                    && !self.is(opener - 1, WINDOW_KEYWORD);
                for index in opener + 2..closer {
                    if separates_arguments
                        && self.depths[index] == self.depths[opener + 1]
                        && self.tokens[index - 1] == COMMA
                        && !self.gaps[index].contains('\n')
                    {
                        self.set_break(index, &inner);
                    }
                }
            }
            if closes {
                self.gaps[closer] = newline(indentation);
            }
            changed = true;
        }
        changed
    }

    /// Start the next line with any binary operator whose operand spans several lines.
    pub(crate) fn lead_operators_of_multiline_operands(&mut self) {
        for index in 1..self.len().saturating_sub(1) {
            if !LEADING_OPERATORS.contains(&self.upper(index).as_str())
                || self.gaps[index] != SINGLE_SPACE
                || self.gaps[index + 1].contains('\n')
                || !self.ends_operand(index - 1)
                || (self.is(index, "AND") && self.follows_between(index))
            {
                continue;
            }
            let multiline = self
                .left_operand_start(index - 1)
                .is_some_and(|start| self.spans_lines(start, index - 1))
                || self
                    .right_operand_end(index + 1)
                    .is_some_and(|end| self.spans_lines(index + 1, end));
            if multiline {
                let (first, _) = self.line_of(index);
                self.gaps[index] = newline(self.indentation(first));
            }
        }
    }

    /// Put the first DECODE argument on its own line and each search/result pair on one line.
    fn pair_decode_arguments(&mut self, name: usize) {
        let opener = name + 1;
        if !self.is(opener, OPEN_PARENTHESIS) || (name > 0 && self.is(name - 1, ".")) {
            return;
        }
        let Some(closer) = self.partners[opener] else {
            return;
        };
        if !self.gaps[opener + 1].contains('\n') {
            return;
        }
        let mut argument = 0_usize;
        for index in opener + 2..closer {
            if self.depths[index] != self.depths[opener] + 1 || self.tokens[index - 1] != COMMA {
                continue;
            }
            argument += 1;
            if argument.is_multiple_of(2) && self.gaps[index].trim().is_empty() {
                self.gaps[index] = SINGLE_SPACE.to_string();
            }
        }
    }

    /// Join an AND or OR line break inside a bracket group whose arguments share its line.
    fn join_chain_break_in_group(&mut self, index: usize) {
        if !self.gaps[index].contains('\n') || !self.gaps[index].trim().is_empty() {
            return;
        }
        let depth = self.depths[index];
        let Some(opener) = (0..index)
            .rev()
            .find(|&previous| self.depths[previous] < depth)
        else {
            return;
        };
        if self.tokens[opener] == OPEN_PARENTHESIS && !self.spans_lines(opener, index - 1) {
            self.gaps[index] = SINGLE_SPACE.to_string();
        }
    }

    /// Start every row of a VALUES list on its own line unless the whole list fits on its line.
    fn break_values_rows_unless_fitting(&mut self, values: usize, line_width: usize) {
        let rows = self.values_rows(values);
        let Some(&(_, last_closer)) = rows.last() else {
            return;
        };
        let (first, last) = self.line_of(values);
        let fits = !self.spans_lines(values, last_closer)
            && self
                .line(first, last)
                .end_columns
                .is_some_and(|ends| ends.last().is_some_and(|end| *end <= line_width));
        if fits {
            return;
        }
        let mut indentation = self.indentation(first);
        if self.gaps[values].contains('\n') {
            indentation = self.indentation(values);
        } else if values > 0
            && self.tokens[values - 1] == OPEN_PARENTHESIS
            && let Some(closer) = self.partners[values - 1]
        {
            self.set_break(values, &newline(indentation + self.indent));
            self.set_break(closer, &newline(indentation));
            indentation += self.indent;
        }
        let row_line = newline(indentation + self.indent);
        for (opener, _) in rows {
            self.set_break(opener, &row_line);
        }
    }

    /// Separate the CTEs of the WITH at `with`, and the statement after them, by one blank line.
    fn separate_common_table_expressions(&mut self, with: usize) {
        let (first, _) = self.line_of(with);
        let indentation = self.indentation(first);
        let mut item = with + 1;
        if self.is(item, "RECURSIVE") {
            item += 1;
        }
        while let Some(end) = self.common_table_expression_end(item) {
            let next = end + 1;
            if self.is(next, ",") {
                self.blank_line_before(next + 1, indentation);
                item = next + 1;
                continue;
            }
            if next < self.len() && !self.is(next, ";") && !self.is(next, ")") {
                self.blank_line_before(next, indentation);
            }
            break;
        }
    }

    /// The last token of the CTE that starts at `item`.
    fn common_table_expression_end(&self, item: usize) -> Option<usize> {
        let name = self.tokens.get(item)?;
        if !matches!(
            self.kinds[item],
            TokenType::Var | TokenType::Identifier | TokenType::QuotedIdentifier
        ) && !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            return None;
        }
        let mut index = item + 1;
        if self.is(index, OPEN_PARENTHESIS) {
            index = self.partners[index]? + 1;
        }
        if !self.is(index, "AS") {
            return None;
        }
        index += 1;
        if self.is(index, "NOT") {
            index += 1;
        }
        if self.is(index, "MATERIALIZED") {
            index += 1;
        }
        if !self.is(index, OPEN_PARENTHESIS) {
            return None;
        }
        self.partners[index]
    }

    /// Replace the line break before `index` with exactly one blank line, keeping comments.
    fn blank_line_before(&mut self, index: usize, indentation: usize) {
        let gap = &self.gaps[index];
        let (trailing, rest) = gap.split_once('\n').unwrap_or((gap.as_str(), ""));
        let rest = rest.trim_start();
        let margin = " ".repeat(indentation);
        self.gaps[index] = if rest.is_empty() {
            format!("{}\n\n{margin}", trailing.trim_end())
        } else {
            format!("{}\n\n{margin}{rest}", trailing.trim_end())
        };
    }

    /// Move a clause body that is one line under its clause keyword onto the keyword line.
    fn join_single_line_body(&mut self, clause: usize, line_width: usize) {
        let (first, last) = self.line_of(clause);
        let body = last + 1;
        let is_select = self.is(clause, "SELECT");
        let header_last = if self.is(clause + 1, "BY") {
            clause + 1
        } else {
            clause
        };
        let modifiers_only = (clause + 1..=last).all(|index| {
            SELECT_MODIFIERS.contains(&self.upper(index).as_str())
                || self.kinds[index] == TokenType::Number
                || matches!(self.tokens[index].as_str(), "(" | ")" | ",")
                || self.depths[index] > self.depths[clause]
        });
        if body >= self.len()
            || clause != first
            || (!is_select && header_last != last)
            || (is_select && !modifiers_only)
            || !self.gaps[body].contains('\n')
            || !self.gaps[body].trim().is_empty()
        {
            return;
        }
        let clause_indentation = self.indentation(first);
        if self.indentation(body) <= clause_indentation {
            return;
        }
        let (_, body_last) = self.line_of(body);
        if body_last + 1 < self.len() && self.indentation(body_last + 1) > clause_indentation {
            return;
        }
        let pinned = (body + 1..=body_last).any(|index| !self.gaps[index].trim().is_empty())
            || (first..=body_last).any(|index| self.tokens[index].contains('\n'));
        if pinned || (!is_select && self.has_top_level_chain(body, body_last)) {
            return;
        }
        let mut width = clause_indentation;
        for index in first..=body_last {
            if index > first {
                width += if index == body {
                    1
                } else {
                    self.gaps[index].chars().count()
                };
            }
            width += self.width(index);
        }
        width += self.gaps[body_last + 1]
            .split('\n')
            .next()
            .unwrap_or_default()
            .chars()
            .count();
        if width <= line_width {
            self.gaps[body] = SINGLE_SPACE.to_string();
        }
    }

    /// Move a join's one-line ON or USING condition onto the line before it when both fit.
    fn join_condition_line(&mut self, condition: usize, line_width: usize) {
        let (first, last) = self.line_of(condition);
        if condition != first
            || !self.gaps[condition].contains('\n')
            || !self.gaps[condition].trim().is_empty()
            || !self.conditions_join(condition)
        {
            return;
        }
        let indentation = self.indentation(condition);
        if last + 1 < self.len() && self.indentation(last + 1) >= indentation {
            return;
        }
        let (previous, _) = self.line_of(condition - 1);
        let previous_indentation = self.indentation(previous);
        if previous_indentation >= indentation || self.has_top_level_chain(condition + 1, last) {
            return;
        }
        if self
            .joined_width(previous, last, previous_indentation)
            .is_some_and(|width| width <= line_width)
        {
            self.gaps[condition] = SINGLE_SPACE.to_string();
        }
    }

    /// Whether the ON or USING at `condition` belongs to a JOIN rather than another statement.
    fn conditions_join(&self, condition: usize) -> bool {
        let depth = self.depths[condition];
        for index in (0..condition).rev() {
            if self.depths[index] < depth {
                return false;
            }
            if self.depths[index] > depth {
                continue;
            }
            let word = self.upper(index);
            if word == JOIN_KEYWORD {
                return true;
            }
            if NOT_JOIN_CONDITION.contains(&word.as_str()) {
                return false;
            }
        }
        false
    }

    fn has_top_level_chain(&self, first: usize, last: usize) -> bool {
        let depth = (first..=last).map(|index| self.depths[index]).min();
        (first..=last).any(|index| {
            Some(self.depths[index]) == depth
                && CHAIN_KEYWORDS.contains(&self.upper(index).as_str())
                && !(self.is(index, "AND") && self.follows_between(index))
        })
    }

    /// Whether the AND at `index` separates the bounds of a BETWEEN.
    fn follows_between(&self, index: usize) -> bool {
        let depth = self.depths[index];
        for previous in (0..index).rev() {
            if self.depths[previous] < depth {
                return false;
            }
            if self.depths[previous] > depth {
                continue;
            }
            let upper = self.upper(previous);
            if upper == BETWEEN_KEYWORD {
                return true;
            }
            if BETWEEN_BOUNDARIES.contains(&upper.as_str()) {
                return false;
            }
        }
        false
    }

    fn ends_operand(&self, index: usize) -> bool {
        matches!(self.tokens[index].as_str(), ")" | "]")
            || self.is(index, "END")
            || matches!(
                self.kinds[index],
                TokenType::Var
                    | TokenType::Identifier
                    | TokenType::QuotedIdentifier
                    | TokenType::Number
                    | TokenType::String
            )
    }

    /// The first token of a bracket group or CASE expression that ends at `last`.
    fn left_operand_start(&self, last: usize) -> Option<usize> {
        if matches!(self.tokens[last].as_str(), ")" | "]") {
            return self.partners[last];
        }
        if !self.is(last, "END") {
            return None;
        }
        let depth = self.depths[last];
        let mut nested = 0_usize;
        for previous in (0..last).rev() {
            if self.depths[previous] != depth {
                continue;
            }
            if self.is(previous, "END") {
                nested += 1;
            } else if self.is(previous, "CASE") {
                if nested == 0 {
                    return Some(previous);
                }
                nested -= 1;
            }
        }
        None
    }

    /// The last token of a bracket group, call, or CASE expression that starts at `first`.
    fn right_operand_end(&self, first: usize) -> Option<usize> {
        if self.tokens[first] == OPEN_PARENTHESIS {
            return self.partners[first];
        }
        let word = self.tokens[first]
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_');
        if word && self.is(first + 1, OPEN_PARENTHESIS) {
            return self.partners[first + 1];
        }
        if !self.is(first, CASE_KEYWORD) {
            return None;
        }
        self.case_end(first)
    }
}

/// Re-break every line of `printed` that is longer than the line width and can be shortened.
pub(crate) fn wrap_lines(printed: &mut PrintedSql<'_>, line_width: usize) {
    printed.lock_unbreakable_groups();
    let mut pending: Vec<(usize, usize)> = printed.line_ranges(0, printed.len() - 1);
    pending.reverse();
    while let Some((first, last)) = pending.pop() {
        let line = printed.line(first, last);
        if !overflows(&line, line_width) || !printed.break_line(&line) {
            continue;
        }
        let mut lines = printed.line_ranges(first, last);
        lines.reverse();
        pending.extend(lines);
    }
}

fn overflows(line: &Line, line_width: usize) -> bool {
    line.end_columns
        .as_ref()
        .is_some_and(|ends| ends.last().is_some_and(|end| *end > line_width))
}

impl PrintedSql<'_> {
    /// Lock the inside of VALUES rows and of type parameter lists such as `NUMBER(38, 2)`.
    fn lock_unbreakable_groups(&mut self) {
        for index in 0..self.len() {
            if self.is(index, VALUES_KEYWORD) {
                for (opener, closer) in self.values_rows(index) {
                    self.lock(opener, closer);
                }
            } else if let Some(closer) = self.type_parameters_closer(index) {
                self.lock(index - 1, closer);
            }
        }
    }

    fn lock(&mut self, first: usize, last: usize) {
        for gap in first + 1..=last {
            self.locked[gap] = true;
        }
    }

    /// The bracketed rows that directly follow the VALUES keyword at `values`.
    fn values_rows(&self, values: usize) -> Vec<(usize, usize)> {
        let mut rows: Vec<(usize, usize)> = Vec::new();
        let mut opener = values + 1;
        while opener < self.len() && self.tokens[opener] == OPEN_PARENTHESIS {
            let Some(closer) = self.partners[opener] else {
                break;
            };
            rows.push((opener, closer));
            if !(self.is(closer + 1, COMMA) && self.is(closer + 2, OPEN_PARENTHESIS)) {
                break;
            }
            opener = closer + 2;
        }
        rows
    }

    /// The closer of a type parameter list opened at `opener`, such as `(38, 2)` of `NUMBER`.
    fn type_parameters_closer(&self, opener: usize) -> Option<usize> {
        if opener < 1 || self.tokens[opener] != OPEN_PARENTHESIS {
            return None;
        }
        let closer = self.partners[opener]?;
        let name = &self.tokens[opener - 1];
        if closer == opener + 1 || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        let type_position = opener > 1 && (self.is(opener - 2, "::") || self.is(opener - 2, "AS"));
        if !type_position && !TYPE_NAMES.contains(&name.to_ascii_uppercase().as_str()) {
            return None;
        }
        let parameters = (opener + 1..closer).all(|index| {
            let token = &self.tokens[index];
            token == COMMA
                || is_digits(token)
                || TYPE_PARAMETER_WORDS.contains(&token.to_ascii_uppercase().as_str())
        });
        parameters.then_some(closer)
    }

    /// Apply the outermost applicable break: chains, VALUES rows, bracket groups, then commas.
    fn break_line(&mut self, line: &Line) -> bool {
        let values_row = self.tokens[line.first] == OPEN_PARENTHESIS && self.locked[line.first + 1];
        let operator_indent = if self.sole_operand_of_group(line) {
            0
        } else {
            self.indent
        };
        !values_row
            && (self.break_before(line, &CHAIN_KEYWORDS, 0)
                || OPERATOR_TIERS
                    .iter()
                    .any(|operators| self.break_before(line, operators, operator_indent))
                || self.break_values_rows(line)
                || self.explode_group(line)
                || self.break_after_commas(line))
    }

    /// Whether `line` starts or continues with an operator the one operand of a parenthesised group whose inside starts on its own line, so an operator broken onto a new line cannot read as the start of another item.
    fn sole_operand_of_group(&self, line: &Line) -> bool {
        let Some(opener) = (0..line.first).rev().find(|&index| {
            self.tokens[index] == OPEN_PARENTHESIS
                && self.partners[index].is_some_and(|closer| closer > line.last)
        }) else {
            return false;
        };
        let Some(closer) = self.partners[opener] else {
            return false;
        };
        let inner = self.depths[opener + 1];
        let continues_operand = line.first == opener + 1
            || OPERATOR_TIERS
                .iter()
                .any(|operators| operators.contains(&self.upper(line.first).as_str()));
        continues_operand
            && self.gaps[opener + 1].contains('\n')
            && self.line_depth(line) == inner
            && (opener + 1..closer)
                .all(|index| self.depths[index] != inner || self.tokens[index] != COMMA)
    }

    /// Put each row of a VALUES list that starts this line on its own line.
    fn break_values_rows(&mut self, line: &Line) -> bool {
        let depth = self.line_depth(line);
        let Some(values) = (line.first..=line.last).find(|&index| {
            self.is(index, VALUES_KEYWORD)
                && (self.depths[index] == depth || index == line.first)
                && !self.locked[index + 1]
        }) else {
            return false;
        };
        let rows = self.values_rows(values);
        if rows.is_empty()
            || rows
                .iter()
                .all(|(opener, _)| self.gaps[*opener].contains('\n'))
        {
            return false;
        }
        let row_line = newline(line.indentation + self.indent);
        for (opener, _) in rows {
            self.gaps[opener] = row_line.clone();
        }
        true
    }

    /// Put the arguments or CASE branches of the outermost, widest group on their own lines.
    fn explode_group(&mut self, line: &Line) -> bool {
        let Some(ends) = line.end_columns.as_ref() else {
            return false;
        };
        let start_column = |index: usize| {
            if index == line.first {
                line.indentation
            } else {
                ends[index - 1 - line.first]
            }
        };
        let line_end = ends[ends.len() - 1];
        let mut chosen: Option<(usize, usize, usize)> = None;
        for opener in line.first..=line.last {
            let Some(closer) = self.group_closer(opener, line) else {
                continue;
            };
            let end_column = if closer > line.last {
                line_end
            } else {
                ends[closer - line.first]
            };
            let width = end_column - start_column(opener);
            let better = chosen.is_none_or(|(best, _, best_width)| {
                self.depths[opener] < self.depths[best]
                    || (self.depths[opener] == self.depths[best] && width >= best_width)
            });
            if better {
                chosen = Some((opener, closer, width));
            }
        }
        let Some((opener, closer, _)) = chosen else {
            return false;
        };
        if closer > line.last {
            self.indent_lines(opener + 1, closer - 1, self.indent);
        }
        let inner = newline(line.indentation + self.indent);
        if self.is(opener, CASE_KEYWORD) {
            self.explode_case(opener, closer, line.indentation);
            return true;
        }
        self.set_break(opener + 1, &inner);
        self.set_break(closer, &newline(line.indentation));
        if self.is(opener + 1, VALUES_KEYWORD) {
            return true;
        }
        let window = opener > 0 && self.is(opener - 1, "OVER");
        let decode = opener > 0 && self.is(opener - 1, DECODE_FUNCTION);
        let mut argument = 0_usize;
        for index in opener + 2..closer {
            if self.depths[index] != self.depths[opener] + 1 {
                continue;
            }
            if window {
                if WINDOW_CLAUSE_KEYWORDS.contains(&self.upper(index).as_str()) {
                    self.set_break(index, &inner);
                }
            } else if self.tokens[index - 1] == COMMA {
                argument += 1;
                if !decode || !argument.is_multiple_of(2) {
                    self.set_break(index, &inner);
                }
            }
        }
        true
    }

    /// The closer of an explodable group opened at `opener` on `line`.
    fn group_closer(&self, opener: usize, line: &Line) -> Option<usize> {
        if self.is(opener, CASE_KEYWORD) {
            let end = self.case_end(opener)?;
            return (end <= line.last && end > opener + 2).then_some(end);
        }
        let closer = self.partners[opener]?;
        let explodable = closer > opener + 2
            && matches!(self.tokens[opener].as_str(), "(" | "[" | "{")
            && !self.locked[opener + 1]
            && (closer <= line.last || opener < line.last);
        explodable.then_some(closer)
    }

    /// The END of the CASE expression at `case`.
    fn case_end(&self, case: usize) -> Option<usize> {
        let depth = self.depths[case];
        let mut nested = 0_usize;
        for index in case + 1..self.len() {
            if self.depths[index] < depth {
                return None;
            }
            if self.depths[index] != depth {
                continue;
            }
            if self.is(index, CASE_KEYWORD) {
                nested += 1;
            } else if self.is(index, "END") {
                if nested == 0 {
                    return Some(index);
                }
                nested -= 1;
            }
        }
        None
    }

    /// Put each WHEN, THEN and ELSE of a one-line CASE on its own line and END under CASE.
    fn explode_case(&mut self, case: usize, end: usize, indentation: usize) {
        let inner = newline(indentation + self.indent);
        let depth = self.depths[case];
        let mut nested = 0_usize;
        for index in case + 1..end {
            if self.depths[index] != depth {
                continue;
            }
            if self.is(index, CASE_KEYWORD) {
                nested += 1;
            } else if self.is(index, "END") {
                nested -= 1;
            } else if nested == 0 && CASE_BRANCH_KEYWORDS.contains(&self.upper(index).as_str()) {
                self.set_break(index, &inner);
            }
        }
        self.set_break(end, &newline(indentation));
    }

    /// Replace a whitespace-only gap; comments keep their gaps.
    fn set_break(&mut self, index: usize, gap: &str) {
        if self.gaps[index].trim().is_empty() {
            self.gaps[index] = gap.to_string();
        }
    }

    /// Indent lines starting within `first..=last` by `extra`, except gaps with block comments.
    fn indent_lines(&mut self, first: usize, last: usize, extra: usize) {
        let margin = format!("\n{}", " ".repeat(extra));
        for index in first..=last {
            if self.gaps[index].contains('\n') && !self.gaps[index].contains("/*") {
                self.gaps[index] = self.gaps[index].replace('\n', &margin);
            }
        }
    }

    /// Break before every operator of `operators` at the line's outermost depth, indenting a group it leaves open.
    fn break_before(&mut self, line: &Line, operators: &[&str], extra_indent: usize) -> bool {
        let depth = self.line_depth(line);
        let mut between = false;
        let mut open_cases = 0_usize;
        let mut breaks: Vec<usize> = Vec::new();
        for index in line.first..=line.last {
            if self.depths[index] != depth || self.locked[index] {
                continue;
            }
            let upper = self.upper(index);
            if upper == CASE_KEYWORD {
                open_cases += 1;
                continue;
            }
            if upper == END_KEYWORD {
                open_cases = open_cases.saturating_sub(1);
                continue;
            }
            if open_cases > 0 {
                continue;
            }
            if upper == BETWEEN_KEYWORD {
                between = true;
                continue;
            }
            if upper == BETWEEN_SEPARATOR && between {
                between = false;
                continue;
            }
            if index > line.first
                && operators.contains(&upper.as_str())
                && self.gaps[index] == SINGLE_SPACE
                && ends_operand(&self.tokens[index - 1])
                && !self.is(index - 1, "CREATE")
            {
                breaks.push(index);
            }
        }
        let indentation = newline(line.indentation + extra_indent);
        for index in &breaks {
            self.gaps[*index] = indentation.clone();
        }
        if extra_indent > 0
            && let Some(&first) = breaks.first()
        {
            let mut continued: Option<usize> = None;
            for index in first..=line.last {
                if let Some(closer) = self.partners[index]
                    && closer > line.last
                {
                    continued = continued.max(Some(closer));
                }
            }
            if let Some(closer) = continued {
                self.indent_lines(line.last + 1, closer, extra_indent);
            }
        }
        !breaks.is_empty()
    }

    /// Break after every comma at the outermost comma depth of the line.
    fn break_after_commas(&mut self, line: &Line) -> bool {
        let Some(depth) = (line.first..line.last)
            .filter(|&index| self.tokens[index] == COMMA && !self.locked[index + 1])
            .map(|index| self.depths[index])
            .min()
        else {
            return false;
        };
        let indentation = newline(line.indentation + self.indent);
        let mut changed = false;
        for index in line.first..line.last {
            if self.depths[index] == depth && self.tokens[index] == COMMA && !self.locked[index + 1]
            {
                self.gaps[index + 1] = indentation.clone();
                changed = true;
            }
        }
        changed
    }

    fn line_depth(&self, line: &Line) -> usize {
        (line.first..=line.last)
            .map(|index| self.depths[index])
            .min()
            .unwrap_or_default()
    }
}

fn is_digits(token: &str) -> bool {
    token.chars().all(|character| character.is_ascii_digit())
}

fn ends_operand(token: &str) -> bool {
    token.chars().last().is_some_and(|character| {
        character.is_alphanumeric() || matches!(character, '_' | '\'' | '"' | '`' | ')' | ']')
    })
}
