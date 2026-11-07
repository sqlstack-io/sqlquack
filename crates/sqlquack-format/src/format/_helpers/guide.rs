//! The layout guide: the line breaks a parse-tree pretty printer puts between the authored tokens.

use std::collections::HashMap;

use sqlquack_syntax::Dialect;
use sqlquack_syntax::tokens::models::TokenType;

use crate::format::_helpers::identifiers::ParsedNames;
use crate::format::_helpers::lexemes::{
    Lexeme, bracket_partners, ends_operand, is_keyword, is_unquoted_word,
};
use crate::format::constants::{
    ALIAS_FOLLOWERS, CASE_BRANCHES, CLAUSE_KEYWORDS, DUCKDB_INLINE_CALLS, GROUPING_CALLS,
    GUIDE_INDENT, GUIDE_WIDTH, JOIN_PREFIXES, PLACEHOLDER_WIDTH, POSTFIX_CAST_WIDTH,
    POSTGRES_INLINE_CALLS, SAMPLE_ROW_KEYWORD, SEMANTIC_VIEW_CALL, SEMANTIC_VIEW_CLAUSES,
    SET_OPERATOR_MODIFIERS, SET_OPERATORS, SNOWFLAKE_INLINE_CALLS, SPACED_BEFORE_PARENTHESIS,
    STRUCT_TYPE_WORDS, TYPE_CONTINUATIONS, TYPED_LITERALS,
};

/// The guide's text and its tokens, with byte ranges in that text.
pub(crate) struct GuideLayout {
    pub(crate) text: String,
    pub(crate) tokens: Vec<Lexeme>,
    /// Token ranges whose authored text is kept byte for byte, such as select items without an alias.
    pub(crate) verbatim: Vec<(usize, usize)>,
    /// The number of select lists laid out.
    pub(crate) selects: usize,
}

/// Lay out `lexemes` with what the parse tree says about them in `names`.
pub(crate) fn layout_guide(
    lexemes: &[Lexeme],
    names: &ParsedNames,
    dialect: Dialect,
) -> GuideLayout {
    let mut guide = Guide::new(lexemes, names, dialect);
    guide.statements();
    guide.render()
}

struct Guide<'a> {
    lexemes: &'a [Lexeme],
    identifiers: &'a [bool],
    /// Tokens the parse tree names as an alias.
    aliases: &'a [bool],
    /// Token ranges whose authored text is kept byte for byte.
    verbatim: Vec<(usize, usize)>,
    /// The number of select lists laid out.
    selects: usize,
    dialect: Dialect,
    partners: Vec<Option<usize>>,
    /// The innermost bracket each token is inside.
    enclosing: Vec<Option<usize>>,
    /// Tokens of a Snowflake stage reference such as `@stage/path/file.csv`.
    in_stage: Vec<bool>,
    /// The one-line spacing before each token.
    base: Vec<&'static str>,
    /// The laid-out whitespace before each token.
    gaps: Vec<String>,
    /// Tokens the pretty printer leaves out, such as the `{fn ...}` escape.
    dropped: Vec<bool>,
    /// The width each token adds to a one-line rendering with literals replaced by placeholders.
    widths: Vec<usize>,
    /// Tokens inside a placeholder after its first token, which add no width.
    covered: Vec<bool>,
    /// Tokens a placeholder stands in for, which keep their authored case.
    protected: Vec<bool>,
}

fn newline(indent: usize) -> String {
    format!("\n{}", " ".repeat(indent * GUIDE_INDENT))
}

fn sentinel_width(sequence: usize) -> usize {
    PLACEHOLDER_WIDTH + sequence.to_string().len()
}

impl<'a> Guide<'a> {
    fn new(lexemes: &'a [Lexeme], names: &'a ParsedNames, dialect: Dialect) -> Self {
        let count = lexemes.len();
        let mut enclosing: Vec<Option<usize>> = vec![None; count];
        let mut open: Vec<usize> = Vec::new();
        for (index, lexeme) in lexemes.iter().enumerate() {
            match lexeme.kind {
                TokenType::LParen | TokenType::LBracket | TokenType::LBrace => {
                    enclosing[index] = open.last().copied();
                    open.push(index);
                }
                TokenType::RParen | TokenType::RBracket | TokenType::RBrace => {
                    open.pop();
                    enclosing[index] = open.last().copied();
                }
                _ => enclosing[index] = open.last().copied(),
            }
        }
        let mut guide = Self {
            lexemes,
            identifiers: &names.identifiers,
            aliases: &names.aliases,
            verbatim: Vec::new(),
            selects: 0,
            dialect,
            partners: bracket_partners(lexemes),
            enclosing,
            in_stage: vec![false; count],
            base: vec![""; count],
            gaps: vec![String::new(); count],
            dropped: vec![false; count],
            widths: vec![0; count],
            covered: vec![false; count],
            protected: vec![false; count],
        };
        guide.mark_stages();
        for index in 1..count {
            guide.base[index] = guide.space(index);
            guide.gaps[index] = guide.base[index].to_string();
        }
        guide.measure();
        guide
    }

    fn len(&self) -> usize {
        self.lexemes.len()
    }

    fn kind(&self, index: usize) -> Option<TokenType> {
        self.lexemes.get(index).map(|lexeme| lexeme.kind)
    }

    /// Whether token `index` is the keyword `word`, not an identifier spelled like it.
    fn is(&self, index: usize, word: &str) -> bool {
        self.lexemes
            .get(index)
            .is_some_and(|lexeme| lexeme.raw.eq_ignore_ascii_case(word))
            && !self.identifiers[index]
            && !self.name_position(index)
    }

    /// Whether token `index` is any of the keywords `words`.
    fn is_any(&self, index: usize, words: &[&str]) -> bool {
        words.iter().any(|word| self.is(index, word))
    }

    /// Whether token `index` follows `.`, `:`, `@` or `$`, where a keyword is a name or path key.
    fn name_position(&self, index: usize) -> bool {
        index > 0 && matches!(self.lexemes[index - 1].raw.as_str(), "." | ":" | "@" | "$")
    }

    fn upper(&self, index: usize) -> String {
        self.lexemes[index].raw.to_ascii_uppercase()
    }

    fn is_word(&self, index: usize) -> bool {
        is_unquoted_word(&self.lexemes[index].raw)
    }

    fn operand_ends(&self, index: usize) -> bool {
        self.identifiers[index] || ends_operand(&self.lexemes[index])
    }

    fn mark_stages(&mut self) {
        if self.dialect != Dialect::Snowflake {
            return;
        }
        let mut index = 0;
        while index < self.len() {
            let lexeme = &self.lexemes[index];
            let starts = lexeme.raw.starts_with('@')
                && matches!(lexeme.kind, TokenType::DAt | TokenType::Var);
            if !starts {
                index += 1;
                continue;
            }
            self.in_stage[index] = true;
            while index + 1 < self.len()
                && self.lexemes[index].end == self.lexemes[index + 1].start
                && !matches!(
                    self.lexemes[index + 1].kind,
                    TokenType::LParen | TokenType::RParen | TokenType::Comma | TokenType::Semicolon
                )
            {
                index += 1;
                self.in_stage[index] = true;
            }
            index += 1;
        }
    }

    /// The spacing a one-line rendering puts before token `index`.
    fn space(&self, index: usize) -> &'static str {
        let (previous, next) = (&self.lexemes[index - 1], &self.lexemes[index]);
        if self.in_stage[index] && self.in_stage[index - 1] {
            return "";
        }
        if self.in_stage[index - 1] && next.kind == TokenType::LParen {
            return " ";
        }
        if matches!(
            next.kind,
            TokenType::Comma
                | TokenType::RParen
                | TokenType::RBracket
                | TokenType::RBrace
                | TokenType::Dot
                | TokenType::Semicolon
                | TokenType::DColon
                | TokenType::Exclamation
                | TokenType::Colon
        ) {
            return "";
        }
        if matches!(
            previous.kind,
            TokenType::LParen
                | TokenType::LBracket
                | TokenType::LBrace
                | TokenType::Dot
                | TokenType::DColon
                | TokenType::Exclamation
                | TokenType::Dollar
        ) {
            return "";
        }
        if previous.kind == TokenType::Colon {
            return if self.path_colon(index - 1) { "" } else { " " };
        }
        if matches!(previous.kind, TokenType::Dash | TokenType::Plus)
            && index
                .checked_sub(2)
                .is_none_or(|before| !self.operand_ends(before))
        {
            return "";
        }
        if next.kind == TokenType::LParen {
            return if self.spaced_before_parenthesis(index - 1) || !self.calls(index - 1) {
                " "
            } else {
                ""
            };
        }
        if next.kind == TokenType::LBracket && self.operand_ends(index - 1) {
            return "";
        }
        " "
    }

    /// Whether a `(` right after token `index` opens its argument list.
    fn calls(&self, index: usize) -> bool {
        self.is_word(index)
            || matches!(
                self.lexemes[index].kind,
                TokenType::QuotedIdentifier | TokenType::RParen | TokenType::RBracket
            )
    }

    fn spaced_before_parenthesis(&self, index: usize) -> bool {
        if self.identifiers[index] || !self.is_word(index) {
            return false;
        }
        let upper = self.upper(index);
        let after_star = index > 0
            && matches!(
                self.lexemes[index - 1].kind,
                TokenType::Star | TokenType::RParen
            );
        let after_sample =
            index > 0 && (self.is(index - 1, "SAMPLE") || self.is(index - 1, "TABLESAMPLE"));
        SPACED_BEFORE_PARENTHESIS.contains(&upper.as_str())
            || (matches!(upper.as_str(), "EXCLUDE" | "REPLACE" | "RENAME" | "ILIKE") && after_star)
            || (upper == SAMPLE_ROW_KEYWORD && after_sample)
    }

    /// Whether the `:` at `index` is a Snowflake path accessor such as `v:field`.
    fn path_colon(&self, index: usize) -> bool {
        self.dialect == Dialect::Snowflake
            && index > 0
            && self.lexemes[index].kind == TokenType::Colon
            && !matches!(
                self.lexemes[index - 1].kind,
                TokenType::String | TokenType::DollarString
            )
            && self.lexemes.get(index + 1).is_some_and(|next| {
                is_unquoted_word(&next.raw)
                    || matches!(next.kind, TokenType::QuotedIdentifier | TokenType::String)
            })
    }

    /// Give every token the width it adds to a one-line rendering of protected SQL.
    fn measure(&mut self) {
        let mut cast_types: HashMap<usize, usize> = HashMap::new();
        for index in 0..self.len() {
            if !(self.is(index, "CAST") || self.is(index, "TRY_CAST"))
                || self.kind(index + 1) != Some(TokenType::LParen)
            {
                continue;
            }
            let Some(closer) = self.partners[index + 1] else {
                continue;
            };
            if let Some(alias) = self
                .tops(index + 2, closer - 1)
                .into_iter()
                .find(|&position| self.is(position, "AS"))
                && alias + 1 < closer
            {
                cast_types.insert(alias + 1, closer - 1);
            }
        }
        let mut sequence = 0_usize;
        let mut index = 0;
        while index < self.len() {
            let lexeme = &self.lexemes[index];
            let sentinel = sentinel_width(sequence);
            let (end, width) = if let Some(&end) = cast_types.get(&index) {
                (end, sentinel)
            } else if lexeme.kind == TokenType::DColon && index + 1 < self.len() {
                (
                    self.postfix_type_end(index + 1),
                    POSTFIX_CAST_WIDTH + sentinel,
                )
            } else if self.path_colon(index) {
                (index + 1, 1 + sentinel)
            } else if TYPED_LITERALS.contains(&lexeme.raw.to_ascii_uppercase().as_str())
                && matches!(
                    self.kind(index + 1),
                    Some(TokenType::String | TokenType::DollarString)
                )
            {
                (index + 1, sentinel)
            } else if (self.is(index, "MOD") || self.is(index, "POSITION"))
                && self.kind(index + 1) == Some(TokenType::LParen)
            {
                (index, sentinel)
            } else if matches!(lexeme.kind, TokenType::String | TokenType::DollarString) {
                (index, sentinel + 2)
            } else {
                self.widths[index] = lexeme.raw.chars().count();
                index += 1;
                continue;
            };
            self.widths[index] = width;
            for position in index..=end {
                self.protected[position] = true;
                self.covered[position] = position > index;
            }
            sequence += 1;
            index = end + 1;
        }
    }

    fn postfix_type_end(&self, start: usize) -> usize {
        let mut end = start;
        while end + 1 < self.len() && TYPE_CONTINUATIONS.contains(&self.upper(end + 1).as_str()) {
            end += 1;
        }
        if self.kind(end + 1) == Some(TokenType::LParen)
            && let Some(closer) = self.partners[end + 1]
        {
            return closer;
        }
        end
    }

    /// The width of tokens `first..=last` rendered on one line.
    fn compact(&self, first: usize, last: usize) -> usize {
        (first..=last)
            .filter(|&index| !self.covered[index] && !self.dropped[index])
            .map(|index| {
                self.widths[index]
                    + if index > first {
                        self.base[index].len()
                    } else {
                        0
                    }
            })
            .sum()
    }

    /// The tokens of `first..=last` outside its brackets, brackets included.
    fn tops(&self, first: usize, last: usize) -> Vec<usize> {
        let mut tops: Vec<usize> = Vec::new();
        let mut index = first;
        while index <= last && index < self.len() {
            tops.push(index);
            match self.partners[index] {
                Some(partner) if partner > index && partner <= last => {
                    tops.push(partner);
                    index = partner + 1;
                }
                _ => index += 1,
            }
        }
        tops
    }

    /// The non-empty ranges of `first..=last` between its top-level commas.
    fn split_commas(&self, first: usize, last: usize) -> Vec<(usize, usize)> {
        let mut items: Vec<(usize, usize)> = Vec::new();
        if first > last {
            return items;
        }
        let mut start = first;
        for index in self.tops(first, last) {
            if self.lexemes[index].kind == TokenType::Comma {
                if start < index {
                    items.push((start, index - 1));
                }
                start = index + 1;
            }
        }
        if start <= last {
            items.push((start, last));
        }
        items
    }

    fn starts_query(&self, index: usize) -> bool {
        self.is(index, "SELECT") || self.is(index, "WITH")
    }

    fn statements(&mut self) {
        let mut first = 0;
        for index in 0..self.len() {
            if self.lexemes[index].kind != TokenType::Semicolon || self.enclosing[index].is_some() {
                continue;
            }
            if first < index {
                self.statement(first, index - 1);
            }
            self.gaps[index] = String::new();
            first = index + 1;
        }
        if first < self.len() {
            self.statement(first, self.len() - 1);
        }
    }

    fn statement(&mut self, first: usize, last: usize) {
        self.gaps[first] = if first == 0 {
            String::new()
        } else {
            "\n".to_string()
        };
        self.query(first, last, 0);
    }

    fn query(&mut self, first: usize, last: usize, indent: usize) {
        if first > last {
            return;
        }
        let mut start = first;
        if self.is(first, "WITH") {
            match self.common_table_expressions(first, last, indent) {
                Some(main) if main <= last => {
                    self.gaps[main] = newline(indent);
                    start = main;
                }
                Some(_) => return,
                None => {
                    self.expr(first + 1, last, indent);
                    return;
                }
            }
        }
        let operators: Vec<usize> = self
            .tops(start, last)
            .into_iter()
            .filter(|&index| self.is_any(index, &SET_OPERATORS))
            .collect();
        let mut operand = start;
        for operator in operators {
            if operator > operand {
                self.operand(operand, operator - 1, indent);
            }
            self.gaps[operator] = newline(indent);
            let mut next = operator + 1;
            while next <= last && self.is_any(next, &SET_OPERATOR_MODIFIERS) {
                next += 1;
            }
            if next <= last {
                self.gaps[next] = newline(indent);
            }
            operand = next;
        }
        self.operand(operand, last, indent);
    }

    fn operand(&mut self, first: usize, last: usize, indent: usize) {
        if first > last {
            return;
        }
        if self.is(first, "SELECT") {
            self.select(first, last, indent);
        } else if self.is(first, "WITH") {
            self.query(first, last, indent);
        } else {
            self.expr(first, last, indent);
        }
    }

    /// Lay out the CTEs of the WITH at `with`; return the token after the last CTE.
    fn common_table_expressions(
        &mut self,
        with: usize,
        last: usize,
        indent: usize,
    ) -> Option<usize> {
        let mut item = with + 1;
        if self.is(item, "RECURSIVE") {
            item += 1;
        }
        loop {
            if item > last {
                return None;
            }
            let mut index = item + 1;
            if self.kind(index) == Some(TokenType::LParen)
                && let Some(closer) = self.partners[index]
                && self.is(closer + 1, "AS")
            {
                index = closer + 1;
            }
            if self.is(index, "AS") {
                index += 1;
                if self.is(index, "NOT") {
                    index += 1;
                }
                if self.is(index, "MATERIALIZED") {
                    index += 1;
                }
            }
            if self.kind(index) != Some(TokenType::LParen) {
                return None;
            }
            let closer = self.partners[index].filter(|&closer| closer <= last)?;
            self.gaps[index] = " ".to_string();
            self.subquery(index, closer, indent);
            if closer < last && self.kind(closer + 1) == Some(TokenType::Comma) {
                item = closer + 2;
                continue;
            }
            return Some(closer + 1);
        }
    }

    fn subquery(&mut self, opener: usize, closer: usize, indent: usize) {
        if closer <= opener + 1 {
            return;
        }
        self.gaps[opener + 1] = newline(indent + 1);
        self.query(opener + 1, closer - 1, indent + 1);
        self.gaps[closer] = newline(indent);
    }

    /// Whether a clause starts at `index`, not a keyword alias the parser records without a span.
    fn starts_clause(&self, index: usize) -> bool {
        let aliased = matches!(
            self.kind(index + 1),
            Some(TokenType::Comma | TokenType::RParen)
        ) || self.is_any(index + 1, &ALIAS_FOLLOWERS);
        (!aliased && self.is_any(index, &CLAUSE_KEYWORDS))
            || ((self.is(index, "GROUP") || self.is(index, "ORDER")) && self.is(index + 1, "BY"))
    }

    fn select(&mut self, select: usize, last: usize, indent: usize) {
        let clauses: Vec<usize> = self
            .tops(select + 1, last)
            .into_iter()
            .filter(|&index| self.starts_clause(index))
            .collect();
        let projection_end = clauses.first().map_or(last, |&clause| clause - 1);
        let mut item = select + 1;
        loop {
            if item > projection_end {
                break;
            }
            if self.is(item, "DISTINCT") || self.is(item, "ALL") {
                item += 1;
                if self.is(item - 1, "DISTINCT")
                    && self.is(item, "ON")
                    && let Some(closer) = self.partners.get(item + 1).copied().flatten()
                {
                    item = closer + 1;
                }
                continue;
            }
            if self.is(item, "TOP") {
                item += 1;
                item = match self.partners.get(item).copied().flatten() {
                    Some(closer) if closer > item => closer + 1,
                    _ => item + 1,
                };
                continue;
            }
            break;
        }
        self.selects += 1;
        for (first, end) in self.split_commas(item, projection_end) {
            self.gaps[first] = newline(indent + 1);
            if !self.aliases[end] && !self.plain_column(first, end) {
                self.verbatim.push((first, end));
            }
            self.expr(first, end, indent + 1);
        }
        for (position, &clause) in clauses.iter().enumerate() {
            let end = clauses.get(position + 1).map_or(last, |&next| next - 1);
            self.clause(clause, end, indent);
        }
    }

    /// Whether tokens `first..=last` are a column reference such as `t.a` or `*`, which names itself.
    fn plain_column(&self, first: usize, last: usize) -> bool {
        (first..=last).all(|index| {
            self.identifiers[index]
                || matches!(self.lexemes[index].kind, TokenType::Dot | TokenType::Star)
        })
    }

    fn clause(&mut self, clause: usize, end: usize, indent: usize) {
        self.gaps[clause] = newline(indent);
        match self.upper(clause).as_str() {
            "FROM" => self.from(clause + 1, end, indent),
            "WHERE" | "HAVING" | "QUALIFY" => {
                if clause < end {
                    self.gaps[clause + 1] = newline(indent + 1);
                    self.expr(clause + 1, end, indent + 1);
                }
            }
            "GROUP" | "ORDER" => {
                let body = clause + 2;
                if body > end || (self.is(body, "ALL") && body == end) {
                    return;
                }
                self.items(body, end, indent);
            }
            "WINDOW" => self.items(clause + 1, end, indent),
            _ => self.expr(clause + 1, end, indent),
        }
    }

    fn items(&mut self, first: usize, last: usize, indent: usize) {
        for (start, end) in self.split_commas(first, last) {
            self.gaps[start] = newline(indent + 1);
            self.expr(start, end, indent + 1);
        }
    }

    fn is_join_word(&self, index: usize) -> bool {
        self.is(index, "JOIN") || self.is_any(index, &JOIN_PREFIXES)
    }

    fn starts_join(&self, index: usize, first: usize) -> bool {
        if !self.is_join_word(index) || (index > first && self.is_join_word(index - 1)) {
            return false;
        }
        let mut position = index;
        while position < self.len() && self.is_join_word(position) && !self.is(position, "JOIN") {
            position += 1;
        }
        self.is(position, "JOIN")
    }

    fn from(&mut self, first: usize, last: usize, indent: usize) {
        if first > last {
            return;
        }
        let mut segments: Vec<(usize, usize, bool)> = Vec::new();
        let (mut start, mut joined) = (first, false);
        for index in self.tops(first, last) {
            if self.lexemes[index].kind == TokenType::Comma {
                if start < index {
                    segments.push((start, index - 1, joined));
                }
                (start, joined) = (index + 1, false);
            } else if index > start && self.starts_join(index, first) {
                segments.push((start, index - 1, joined));
                self.gaps[index] = newline(indent);
                (start, joined) = (index, true);
            }
        }
        if start <= last {
            segments.push((start, last, joined));
        }
        for (start, end, joined) in segments {
            if joined {
                self.join(start, end, indent);
            } else {
                self.source(start, end, indent);
            }
        }
    }

    fn join(&mut self, first: usize, last: usize, indent: usize) {
        let mut join = first;
        while join < last && !self.is(join, "JOIN") {
            join += 1;
        }
        let condition = self
            .tops(join + 1, last)
            .into_iter()
            .find(|&index| self.is(index, "ON") || self.is(index, "USING"));
        self.source(join + 1, condition.map_or(last, |at| at - 1), indent);
        if let Some(condition) = condition {
            self.gaps[condition] = newline(indent + 1);
            self.expr(condition + 1, last, indent + 1);
        }
    }

    fn source(&mut self, first: usize, last: usize, indent: usize) {
        if first > last {
            return;
        }
        let opener = if self.is(first, "LATERAL") {
            first + 1
        } else {
            first
        };
        if self.kind(opener) == Some(TokenType::LParen)
            && let Some(closer) = self.partners[opener].filter(|&closer| closer <= last)
        {
            if self.starts_query(opener + 1) {
                self.subquery(opener, closer, indent);
            } else {
                self.from(opener + 1, closer - 1, indent);
            }
            self.expr(closer + 1, last, indent);
            return;
        }
        self.expr(first, last, indent);
    }

    fn expr(&mut self, first: usize, last: usize, indent: usize) {
        let mut between = false;
        let mut index = first;
        while index <= last && index < self.len() {
            if self.is(index, "SELECT") {
                self.query(index, last, indent);
                return;
            }
            match self.lexemes[index].kind {
                TokenType::LParen | TokenType::LBracket | TokenType::LBrace => {
                    if let Some(closer) = self.partners[index].filter(|&closer| closer <= last) {
                        self.group(index, closer, indent);
                        index = closer + 1;
                        continue;
                    }
                }
                _ => {}
            }
            if self.is(index, "CASE")
                && let Some(end) = self.case_end(index, last)
            {
                self.case(index, end, indent);
                index = end + 1;
                continue;
            }
            if self.is(index, "BETWEEN") {
                between = true;
            } else if self.is(index, "AND") && between {
                between = false;
            } else if (self.is(index, "AND") || self.is(index, "OR"))
                && self.dialect == Dialect::Snowflake
                && !(index > 0 && self.is(index - 1, "CREATE"))
            {
                self.gaps[index] = newline(indent);
            }
            index += 1;
        }
    }

    fn group(&mut self, opener: usize, closer: usize, indent: usize) {
        match self.lexemes[opener].kind {
            TokenType::LBracket => {
                if self.base[opener].is_empty() && opener > 0 && self.operand_ends(opener - 1) {
                    self.expr(opener + 1, closer - 1, indent);
                } else {
                    let items = self.split_commas(opener + 1, closer - 1);
                    let split = !items.is_empty() && self.items_width(&items) > GUIDE_WIDTH;
                    self.list(closer, &items, split, indent);
                }
            }
            TokenType::LBrace => {
                if opener + 1 < closer && self.lexemes[opener + 1].raw.eq_ignore_ascii_case("fn") {
                    self.dropped[opener] = true;
                    self.dropped[opener + 1] = true;
                    self.dropped[closer] = true;
                    self.expr(opener + 2, closer - 1, indent);
                } else {
                    self.expr(opener + 1, closer - 1, indent);
                }
            }
            _ => self.parenthesis(opener, closer, indent),
        }
    }

    fn parenthesis(&mut self, opener: usize, closer: usize, indent: usize) {
        let previous = opener.checked_sub(1);
        if self.starts_query(opener + 1) {
            let pivot_list = previous.is_some_and(|at| self.is(at, "IN"))
                && self.enclosing[opener].is_some_and(|at| {
                    at > 0 && (self.is(at - 1, "PIVOT") || self.is(at - 1, "UNPIVOT"))
                });
            if pivot_list {
                self.query(opener + 1, closer - 1, indent);
            } else {
                self.subquery(opener, closer, indent);
            }
            return;
        }
        let Some(previous) = previous else {
            self.tuple(opener, closer, indent);
            return;
        };
        let pivot_values = self.is(previous, "IN")
            && self.enclosing[opener].is_some_and(|at| at > 0 && self.is(at - 1, "PIVOT"));
        if (self.is(previous, "PATTERN") || pivot_values) && opener + 1 < closer {
            self.verbatim.push((opener + 1, closer - 1));
            self.expr(opener + 1, closer - 1, indent);
        } else if self.is(previous, "PIVOT") || self.is(previous, "UNPIVOT") {
            self.expr(opener + 1, closer - 1, indent);
        } else if self.is_any(previous, &GROUPING_CALLS)
            || (self.base[opener].is_empty() && self.calls(previous) && !self.in_stage[previous])
        {
            self.call(previous, opener, closer, indent);
        } else if self.is_word(previous) || self.in_stage[previous] {
            self.expr(opener + 1, closer - 1, indent);
        } else {
            self.tuple(opener, closer, indent);
        }
    }

    fn tuple(&mut self, opener: usize, closer: usize, indent: usize) {
        let items = self.split_commas(opener + 1, closer - 1);
        let split = items.len() > 1 && self.items_width(&items) > GUIDE_WIDTH;
        self.list(closer, &items, split, indent);
    }

    fn call(&mut self, name: usize, opener: usize, closer: usize, indent: usize) {
        let upper = if self.is_word(name) {
            self.upper(name)
        } else {
            String::new()
        };
        if upper == SEMANTIC_VIEW_CALL {
            self.semantic_view(opener, closer, indent);
            return;
        }
        let mut first = opener + 1;
        if self.is(first, "DISTINCT") {
            first += 1;
        }
        let items = self.split_commas(first, closer - 1);
        let grouping = GROUPING_CALLS.contains(&upper.as_str());
        let inline = self.protected[opener] || self.inline_calls().contains(&upper.as_str());
        let split =
            !items.is_empty() && (grouping || (!inline && self.items_width(&items) > GUIDE_WIDTH));
        self.list(closer, &items, split, indent);
    }

    fn inline_calls(&self) -> &'static [&'static str] {
        match self.dialect {
            Dialect::Snowflake => &SNOWFLAKE_INLINE_CALLS,
            Dialect::DuckDB => &DUCKDB_INLINE_CALLS,
            Dialect::PostgreSQL | Dialect::SQLite => &POSTGRES_INLINE_CALLS,
        }
    }

    fn items_width(&self, items: &[(usize, usize)]) -> usize {
        items
            .iter()
            .map(|&(first, last)| self.compact(first, last))
            .sum()
    }

    /// Lay out the items of a bracket group closed at `closer`, one per line when `split`.
    fn list(&mut self, closer: usize, items: &[(usize, usize)], split: bool, indent: usize) {
        if !split {
            for &(first, last) in items {
                self.expr(first, last, indent);
            }
            return;
        }
        for &(first, last) in items {
            self.gaps[first] = newline(indent + 1);
            self.expr(first, last, indent + 1);
        }
        self.gaps[closer] = newline(indent);
    }

    fn semantic_view(&mut self, opener: usize, closer: usize, indent: usize) {
        if closer <= opener + 1 {
            return;
        }
        let clauses: Vec<usize> = self
            .tops(opener + 1, closer - 1)
            .into_iter()
            .filter(|&index| self.is_any(index, &SEMANTIC_VIEW_CLAUSES))
            .collect();
        let mut start = opener + 1;
        for clause in clauses.iter().copied().chain([closer]) {
            if start < clause {
                self.gaps[start] = newline(indent + 1);
                self.expr(start, clause - 1, indent + 1);
            }
            start = clause;
        }
        self.gaps[closer] = newline(indent);
    }

    fn case_end(&self, case: usize, last: usize) -> Option<usize> {
        let mut nested = 0_usize;
        for index in self.tops(case + 1, last) {
            if self.is(index, "CASE") {
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

    fn case(&mut self, case: usize, end: usize, indent: usize) {
        let mut nested = 0_usize;
        let mut branches: Vec<usize> = Vec::new();
        for index in self.tops(case + 1, end - 1) {
            if self.is(index, "CASE") {
                nested += 1;
            } else if self.is(index, "END") {
                nested = nested.saturating_sub(1);
            } else if nested == 0 && self.is_any(index, &CASE_BRANCHES) {
                branches.push(index);
            }
        }
        let width = self.compact(case, end).saturating_sub(branches.len() + 1);
        let multiline = width > GUIDE_WIDTH;
        let inner = if multiline { indent + 1 } else { indent };
        let first_branch = branches.first().copied().unwrap_or(end);
        self.expr(case + 1, first_branch - 1, indent);
        for (position, &branch) in branches.iter().enumerate() {
            if multiline {
                self.gaps[branch] = newline(indent + 1);
            }
            let next = branches.get(position + 1).copied().unwrap_or(end);
            self.expr(branch + 1, next - 1, inner);
        }
        if multiline {
            self.gaps[end] = newline(indent);
        }
    }

    /// The guide's spelling of token `index`: keywords in upper case, names as written.
    fn text(&self, index: usize) -> String {
        let lexeme = &self.lexemes[index];
        if self.protected[index]
            || self.in_stage[index]
            || self.identifiers[index]
            || self.name_position(index)
            || self.field_name(index)
            || !is_keyword(lexeme)
        {
            lexeme.raw.clone()
        } else {
            lexeme.raw.to_ascii_uppercase()
        }
    }

    /// Whether token `index` names a field of a struct type such as `STRUCT(type VARCHAR)` or `STRUCT<type: STRING>`; the tree keeps these names without spans, and a schema keeps their case.
    fn field_name(&self, index: usize) -> bool {
        let starts_field = index > 0
            && matches!(
                self.lexemes[index - 1].kind,
                TokenType::Comma | TokenType::LParen | TokenType::Lt
            );
        let typed = self
            .lexemes
            .get(index + 1)
            .is_some_and(|next| is_unquoted_word(&next.raw) || next.kind == TokenType::Colon);
        if !starts_field || !typed {
            return false;
        }
        let mut angles: usize = 0;
        let mut at = index;
        while at > 0 {
            at -= 1;
            match self.lexemes[at].kind {
                TokenType::Gt => angles += 1,
                TokenType::GtGt => angles += 2,
                TokenType::RParen | TokenType::RBracket => {
                    if let Some(opener) = self.partners[at].filter(|&opener| opener < at) {
                        at = opener;
                    }
                }
                TokenType::Lt if angles > 0 => angles -= 1,
                TokenType::Lt | TokenType::LParen | TokenType::LBracket => {
                    return angles == 0
                        && self.lexemes[at].kind != TokenType::LBracket
                        && at > 0
                        && STRUCT_TYPE_WORDS
                            .iter()
                            .any(|word| self.lexemes[at - 1].raw.eq_ignore_ascii_case(word));
                }
                _ => {}
            }
        }
        false
    }

    fn render(&self) -> GuideLayout {
        let mut text = String::new();
        let mut tokens: Vec<Lexeme> = Vec::with_capacity(self.len());
        let mut pending: Option<&str> = None;
        for index in 0..self.len() {
            if self.dropped[index] {
                pending.get_or_insert(&self.gaps[index]);
                continue;
            }
            if !tokens.is_empty() {
                let gap = match pending {
                    Some(gap) if gap.contains('\n') => gap,
                    _ => &self.gaps[index],
                };
                text.push_str(gap);
            }
            pending = None;
            let start = text.len();
            let raw = self.text(index);
            text.push_str(&raw);
            tokens.push(Lexeme {
                start,
                end: text.len(),
                kind: self.lexemes[index].kind,
                raw,
            });
        }
        let mut ranges = self.verbatim.clone();
        ranges.sort_unstable();
        let mut verbatim: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
        for range in ranges {
            if verbatim.last().is_none_or(|&(_, end)| range.0 > end) {
                verbatim.push(range);
            }
        }
        GuideLayout {
            text,
            tokens,
            verbatim,
            selects: self.selects,
        }
    }
}
