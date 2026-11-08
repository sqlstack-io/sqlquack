//! The layout guide: the line breaks a parse-tree pretty printer puts between the authored tokens.

use std::collections::HashMap;

use crate::format::models::FormatDialect;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax_bigquery::keywords::main::is_reserved::is_reserved as is_bigquery_reserved;
use sqlquack_syntax_sqlserver::is_reserved;

use crate::format::_helpers::identifiers::ParsedNames;
use crate::format::_helpers::lexemes::{
    Lexeme, bracket_partners, ends_operand, is_keyword, is_unquoted_word,
};
use crate::format::_helpers::sqlserver::StatementTokens;
use crate::format::constants::{
    ALIAS_FOLLOWERS, BIGQUERY_INDEX_KEYWORD, BIGQUERY_INLINE_CALLS, BIGQUERY_REFERENCES_KEYWORD,
    BIGQUERY_SET_OPERATOR_PREFIXES, BIGQUERY_VALUE_TABLE_KINDS, CALL_TARGET_KEYWORDS,
    CASE_BRANCHES, CLAUSE_KEYWORDS, COLUMNS_CALL, CTE_BODY_PREFIXES, DUCKDB_INLINE_CALLS,
    EXPRESSION_LEADERS, FROM_CONTINUED, GROUPING_CALLS, GUIDE_INDENT, GUIDE_WIDTH, INSERT_BODIES,
    INSERT_KEYWORD, JOIN_PREFIXES, LAMBDA_KEYWORD, PARAMETERISED_TYPES, PLACEHOLDER_NAME_PREFIX,
    PLACEHOLDER_WIDTH, POSTFIX_CAST_WIDTH, POSTGRES_CTE_CLAUSES, POSTGRES_INLINE_CALLS,
    POSTGRES_INLINE_SOURCES, POSTGRES_INTO_CLAUSE, POSTGRES_PATTERN_OPERATORS,
    POSTGRES_QUANTIFIERS, POSTGRES_SPACED_QUANTIFIER, SAMPLE_KEYWORDS, SAMPLE_ROW_KEYWORD,
    SEMANTIC_VIEW_CALL, SEMANTIC_VIEW_CLAUSES, SET_OPERATOR_MODIFIERS, SET_OPERATORS,
    SNOWFLAKE_INLINE_CALLS, SPACED_BEFORE_PARENTHESIS, SQLSERVER_APPLY_KEYWORD,
    SQLSERVER_APPLY_PREFIXES, SQLSERVER_BEGIN_STATEMENTS, SQLSERVER_BLOCK_WORDS,
    SQLSERVER_BRANCH_STATEMENTS, SQLSERVER_INLINE_CALLS, SQLSERVER_LIST_CLAUSES,
    SQLSERVER_OBJECT_PRECEDERS, SQLSERVER_SPACED_BEFORE_PARENTHESIS, STRUCT_TYPE_WORDS,
    TYPE_CONTINUATIONS, TYPED_LITERALS, VALUES_KEYWORD,
};

/// The guide's text and its tokens, with byte ranges in that text.
pub(crate) struct GuideLayout {
    pub(crate) text: String,
    pub(crate) tokens: Vec<Lexeme>,
    /// Token ranges whose authored text is kept byte for byte, such as select items without an alias.
    pub(crate) verbatim: Vec<(usize, usize)>,
    /// The number of select lists laid out.
    pub(crate) selects: usize,
    /// Tokens that start a statement the parser read, or the block of a routine's body: keywords, never names, even after `AS`.
    pub(crate) statement_starts: Vec<usize>,
}

/// Lay out `lexemes` with what the parse tree says about them in `names`.
pub(crate) fn layout_guide(
    lexemes: &[Lexeme],
    names: &ParsedNames,
    statement_ranges: &[StatementTokens],
    dialect: FormatDialect,
) -> GuideLayout {
    let mut guide = Guide::new(lexemes, names, dialect);
    guide.statement_ranges = statement_ranges;
    guide.statements();
    guide.render()
}

struct Guide<'a> {
    lexemes: &'a [Lexeme],
    /// Where the parser read each statement, nested ones included, when it reports that: SQL Server's.
    statement_ranges: &'a [StatementTokens],
    /// Tokens that start a statement or a routine body's block.
    statement_starts: Vec<usize>,
    identifiers: &'a [bool],
    /// Tokens the parse tree names as an alias.
    aliases: &'a [bool],
    /// Words matching a name the parse tree records without a span.
    unspanned: &'a [bool],
    /// Token ranges whose authored text is kept byte for byte.
    verbatim: Vec<(usize, usize)>,
    /// The number of select lists laid out.
    selects: usize,
    dialect: FormatDialect,
    partners: Vec<Option<usize>>,
    /// The innermost bracket each token is inside.
    enclosing: Vec<Option<usize>>,
    /// Tokens printed touching as written: a Snowflake stage reference such as `@stage/path/file.csv`, or a BigQuery dashed name such as `my-project.ds.t`.
    in_run: Vec<bool>,
    /// Operators the parse tree reads as binary, such as a comparison's `<`.
    binary_operators: &'a [bool],
    /// The `<` and `>` around the parameters of a type such as `ARRAY<INT>`.
    angles: Vec<bool>,
    /// Tokens inside a type's angle brackets, such as the comma of `STRUCT<a INT64, b STRING>`, which are not at the top of their list.
    in_angles: Vec<bool>,
    /// Tokens directly inside a struct type's field list, where a name keeps its case.
    field_lists: Vec<bool>,
    /// SQL Server commas that separate the items of a FROM list.
    from_list_commas: Vec<bool>,
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
    /// Openers of calls the guide keeps on one line wherever they stand, such as UNNEST in FROM.
    inline_openers: Vec<bool>,
}

fn newline(indent: usize) -> String {
    format!("\n{}", " ".repeat(indent * GUIDE_INDENT))
}

fn sentinel_width(sequence: usize) -> usize {
    PLACEHOLDER_WIDTH + sequence.to_string().len()
}

impl<'a> Guide<'a> {
    fn new(lexemes: &'a [Lexeme], names: &'a ParsedNames, dialect: FormatDialect) -> Self {
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
            statement_ranges: &[],
            statement_starts: Vec::new(),
            identifiers: &names.identifiers,
            aliases: &names.aliases,
            unspanned: &names.unspanned,
            verbatim: Vec::new(),
            selects: 0,
            dialect,
            partners: bracket_partners(lexemes),
            enclosing,
            in_run: vec![false; count],
            binary_operators: &names.binary_operators,
            angles: vec![false; count],
            in_angles: vec![false; count],
            field_lists: vec![false; count],
            from_list_commas: vec![false; count],
            base: vec![""; count],
            gaps: vec![String::new(); count],
            dropped: vec![false; count],
            widths: vec![0; count],
            covered: vec![false; count],
            protected: vec![false; count],
            inline_openers: vec![false; count],
        };
        guide.mark_runs();
        guide.mark_angles();
        guide.mark_field_lists();
        guide.mark_from_list_commas();
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

    /// Whether token `index` directly follows a `*`, as BigQuery's `* EXCEPT (...)` does.
    fn after_star(&self, index: usize) -> bool {
        index > 0 && self.lexemes[index - 1].kind == TokenType::Star
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

    fn mark_runs(&mut self) {
        if self.dialect == FormatDialect::BigQuery {
            self.mark_dashed_names();
        }
        if self.dialect != FormatDialect::Snowflake {
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
            self.in_run[index] = true;
            while index + 1 < self.len()
                && self.lexemes[index].end == self.lexemes[index + 1].start
                && !matches!(
                    self.lexemes[index + 1].kind,
                    TokenType::LParen | TokenType::RParen | TokenType::Comma | TokenType::Semicolon
                )
            {
                index += 1;
                self.in_run[index] = true;
            }
            index += 1;
        }
    }

    /// Mark each run of touching tokens around a `-` the parse tree does not read as subtraction, such as `my-project.ds.t`.
    fn mark_dashed_names(&mut self) {
        let touching = |left: &Lexeme, right: &Lexeme| left.end == right.start;
        let piece = |lexeme: &Lexeme| {
            is_unquoted_word(&lexeme.raw)
                || matches!(
                    lexeme.kind,
                    TokenType::Number
                        | TokenType::QuotedIdentifier
                        | TokenType::Dot
                        | TokenType::Dash
                )
        };
        for index in 1..self.len().saturating_sub(1) {
            let lexemes = self.lexemes;
            let dash = lexemes[index].kind == TokenType::Dash
                && !self.in_run[index]
                && !self.binary_operators[index]
                && touching(&lexemes[index - 1], &lexemes[index])
                && touching(&lexemes[index], &lexemes[index + 1]);
            if !dash {
                continue;
            }
            let mut first = index;
            while first > 0
                && touching(&lexemes[first - 1], &lexemes[first])
                && piece(&lexemes[first - 1])
            {
                first -= 1;
            }
            let mut last = index;
            while last + 1 < self.len()
                && touching(&lexemes[last], &lexemes[last + 1])
                && piece(&lexemes[last + 1])
            {
                last += 1;
            }
            for at in first..=last {
                self.in_run[at] = true;
            }
        }
    }

    /// Mark the `<` after a parameterised type name such as `ARRAY`, and the `>` that closes it.
    fn mark_angles(&mut self) {
        let mut open: usize = 0;
        for index in 1..self.len() {
            let kind = self.lexemes[index].kind;
            if kind == TokenType::Lt
                && !self.binary_operators[index]
                && self.is_any(index - 1, &PARAMETERISED_TYPES)
            {
                self.angles[index] = true;
                open += 1;
            } else if kind == TokenType::Gt && open > 0 && !self.binary_operators[index] {
                self.angles[index] = true;
                open -= 1;
            }
            self.in_angles[index] = open > 0 && !self.angles[index];
        }
    }

    /// The spacing a one-line rendering puts before token `index`.
    fn space(&self, index: usize) -> &'static str {
        let (previous, next) = (&self.lexemes[index - 1], &self.lexemes[index]);
        if (self.in_run[index] && self.in_run[index - 1])
            || (self.angles[index - 1] && self.lexemes[index - 1].kind == TokenType::Lt)
            || self.angles[index]
        {
            return "";
        }
        if self.in_run[index - 1] && next.kind == TokenType::LParen {
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
            return if self.tight_colon(index - 1) { "" } else { " " };
        }
        if (previous.kind == TokenType::DAt && !self.in_run[index - 1])
            || (previous.kind == TokenType::Star && self.is(index, COLUMNS_CALL))
        {
            return "";
        }
        if next.kind == TokenType::Percent && self.postfix_percent(index) {
            return "";
        }
        if matches!(previous.kind, TokenType::Dash | TokenType::Plus)
            && index
                .checked_sub(2)
                .is_none_or(|before| !self.operand_ends(before))
        {
            return "";
        }
        if next.kind == TokenType::LParen {
            return if self.spaced_before_parenthesis(index - 1)
                || self.sqlserver_spaced_before_parenthesis(index - 1)
                || !self.calls(index - 1)
            {
                " "
            } else {
                ""
            };
        }
        if next.kind == TokenType::LBracket
            && (self.operand_ends(index - 1) || self.angles[index - 1])
        {
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
        if self.dialect == FormatDialect::PostgreSQL
            && POSTGRES_QUANTIFIERS.contains(&upper.as_str())
        {
            return upper == POSTGRES_SPACED_QUANTIFIER && self.after_pattern_operator(index);
        }
        let after_star = index > 0
            && matches!(
                self.lexemes[index - 1].kind,
                TokenType::Star | TokenType::RParen
            );
        let after_sample =
            index > 0 && (self.is(index - 1, "SAMPLE") || self.is(index - 1, "TABLESAMPLE"));
        SPACED_BEFORE_PARENTHESIS.contains(&upper.as_str())
            || (matches!(
                upper.as_str(),
                "EXCLUDE" | "EXCEPT" | "REPLACE" | "RENAME" | "ILIKE"
            ) && after_star)
            || (upper == SAMPLE_ROW_KEYWORD && after_sample)
    }

    /// Lay out a SQL Server `TOP (SELECT ...)` count as a subquery; any other `TOP (...)` stays as written.
    fn sqlserver_top_query(&mut self, opener: usize, closer: usize, indent: usize) {
        if self.dialect == FormatDialect::SqlServer && self.starts_query(opener + 1) {
            self.subquery(opener, closer, indent);
        }
    }

    /// Whether the word at `index` names a SQL Server object before `(`, whose name resolves under the database collation, which may be case-sensitive: an unreserved word after a keyword that names a table, view, routine or index target, or after a comma of a FROM list.
    fn sqlserver_object_name(&self, index: usize) -> bool {
        self.dialect == FormatDialect::SqlServer
            && index > 0
            && self.kind(index + 1) == Some(TokenType::LParen)
            && !is_reserved(&self.lexemes[index].raw)
            && (self.is_any(index - 1, &SQLSERVER_OBJECT_PRECEDERS)
                || self.from_list_commas[index - 1])
    }

    /// Whether the word at `index` names a BigQuery table before `(`, whose name is case-sensitive and which the tree keeps without a span: the table of a REFERENCES or of an index's ON. Unqualified table functions are built-ins, which resolve in any case.
    fn bigquery_table_name(&self, index: usize) -> bool {
        let after_references = index
            .checked_sub(1)
            .is_some_and(|before| self.is(before, BIGQUERY_REFERENCES_KEYWORD));
        let after_index_on = index.checked_sub(3).is_some_and(|keyword| {
            self.is(keyword, BIGQUERY_INDEX_KEYWORD) && self.is(keyword + 2, "ON")
        });
        self.dialect == FormatDialect::BigQuery
            && self.kind(index + 1) == Some(TokenType::LParen)
            && !is_bigquery_reserved(&self.lexemes[index].raw)
            && (after_references || after_index_on)
    }

    /// Mark the SQL Server commas whose nearest list clause at their depth is FROM, in one pass that keeps each depth's last list clause.
    fn mark_from_list_commas(&mut self) {
        if self.dialect != FormatDialect::SqlServer {
            return;
        }
        let mut in_from: HashMap<Option<usize>, bool> = HashMap::new();
        for index in 0..self.len() {
            if self.is_any(index, &SQLSERVER_LIST_CLAUSES) {
                in_from.insert(self.enclosing[index], self.is(index, "FROM"));
            } else if self.lexemes[index].kind == TokenType::Comma {
                self.from_list_commas[index] = in_from
                    .get(&self.enclosing[index])
                    .copied()
                    .unwrap_or(false);
            }
        }
    }

    /// Whether token `index` is the APPLY of a SQL Server `CROSS APPLY` or `OUTER APPLY`.
    fn sqlserver_apply(&self, index: usize) -> bool {
        self.dialect == FormatDialect::SqlServer
            && index > 0
            && self.is(index, SQLSERVER_APPLY_KEYWORD)
            && self.is_any(index - 1, &SQLSERVER_APPLY_PREFIXES)
    }

    /// Whether a SQL Server `CROSS APPLY` or `OUTER APPLY`, which the guide lays out as a join, starts at token `index`.
    fn sqlserver_starts_apply(&self, index: usize) -> bool {
        self.is_any(index, &SQLSERVER_APPLY_PREFIXES) && self.sqlserver_apply(index + 1)
    }

    /// Whether token `index` is a SQL Server keyword the guide separates from a following `(`.
    fn sqlserver_spaced_before_parenthesis(&self, index: usize) -> bool {
        self.dialect == FormatDialect::SqlServer
            && self.is_word(index)
            && self.is_any(index, &SQLSERVER_SPACED_BEFORE_PARENTHESIS)
    }

    /// Whether a `:` at `index` binds tightly: a Snowflake path, a slice, or a DuckDB prefix alias.
    fn tight_colon(&self, index: usize) -> bool {
        let enclosing = self.enclosing[index].map(|opener| self.lexemes[opener].kind);
        self.path_colon(index)
            || self.in_subscript(index)
            || (self.dialect == FormatDialect::DuckDB
                && enclosing != Some(TokenType::LBrace)
                && !self.lambda_colon(index))
    }

    /// Whether token `index` is inside a subscript such as `x[1:2]`.
    fn in_subscript(&self, index: usize) -> bool {
        self.enclosing[index].is_some_and(|opener| {
            self.lexemes[opener].kind == TokenType::LBracket
                && opener > 0
                && self.lexemes[opener - 1].end == self.lexemes[opener].start
                && self.operand_ends(opener - 1)
        })
    }

    /// Whether the `:` at `index` ends the parameters of a DuckDB `LAMBDA x, y :`.
    fn lambda_colon(&self, index: usize) -> bool {
        let parameters = (0..index).rev().find(|&at| {
            !matches!(
                self.lexemes[at].kind,
                TokenType::Var | TokenType::Identifier | TokenType::Comma
            ) || self.is(at, LAMBDA_KEYWORD)
        });
        parameters.is_some_and(|at| self.is(at, LAMBDA_KEYWORD))
    }

    /// Whether the `%` at `index` is DuckDB's percentage suffix, as in `LIMIT 10%`, not modulo.
    fn postfix_percent(&self, index: usize) -> bool {
        let after_number = self.kind(index - 1) == Some(TokenType::Number);
        let sampled = index > 1 && self.is_any(index - 2, &SAMPLE_KEYWORDS);
        let operand_follows = match self.kind(index + 1) {
            Some(TokenType::LParen) => !sampled,
            Some(kind) => matches!(
                kind,
                TokenType::Number
                    | TokenType::Var
                    | TokenType::Identifier
                    | TokenType::QuotedIdentifier
                    | TokenType::Dash
            ),
            None => false,
        };
        self.dialect == FormatDialect::DuckDB && after_number && !operand_follows
    }

    /// Whether token `index` follows LIKE or ILIKE, as the quantifier of `x LIKE ANY (...)` does.
    fn after_pattern_operator(&self, index: usize) -> bool {
        index > 0 && self.is_any(index - 1, &POSTGRES_PATTERN_OPERATORS)
    }

    /// Whether the PostgreSQL `(` at `opener` holds the operand of a quantified pattern match.
    fn quantified_pattern(&self, opener: usize) -> bool {
        self.dialect == FormatDialect::PostgreSQL
            && opener > 0
            && self.is_any(opener - 1, &POSTGRES_QUANTIFIERS)
            && self.after_pattern_operator(opener - 1)
    }

    /// Whether the `:` at `index` is a Snowflake path accessor such as `v:field`.
    fn path_colon(&self, index: usize) -> bool {
        self.dialect == FormatDialect::Snowflake
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
            if self.in_angles[index] {
                index += 1;
                continue;
            }
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
        self.is(index, "SELECT") || self.is(index, "WITH") || self.starts_from_first(index)
    }

    /// Whether a DuckDB FROM-first query, such as `FROM t WHERE x`, starts at `index`.
    fn starts_from_first(&self, index: usize) -> bool {
        self.dialect == FormatDialect::DuckDB && self.is(index, "FROM")
    }

    fn statements(&mut self) {
        if !self.statement_ranges.is_empty() {
            let tops: Vec<StatementTokens> = self.inner_statements(0, self.len() - 1, 0);
            for top in tops {
                self.gaps[top.first] = match top.first {
                    0 => String::new(),
                    _ => "\n".to_string(),
                };
                self.ranged_statement(top, 0);
            }
            return;
        }
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

    /// The statements at `depth` the parser read within tokens `first` to `last`.
    fn inner_statements(&self, first: usize, last: usize, depth: usize) -> Vec<StatementTokens> {
        self.statement_ranges
            .iter()
            .copied()
            .filter(|range| range.depth == depth && first <= range.first && range.last <= last)
            .collect()
    }

    /// Whether a block, `BEGIN ... END` or `BEGIN TRY`, starts at `index`, not a `BEGIN TRANSACTION`.
    fn starts_block(&self, index: usize) -> bool {
        self.is(index, "BEGIN") && !self.is_any(index + 1, &SQLSERVER_BEGIN_STATEMENTS)
    }

    /// Lay out a statement the parser read, at `indent`: the statements inside a block, branch, loop or procedure each on their own line, a block's indented one step, and its other words around them.
    fn ranged_statement(&mut self, statement: StatementTokens, indent: usize) {
        let first = statement.first;
        let mut last = statement.last;
        while last > first && self.kind(last) == Some(TokenType::Semicolon) {
            self.gaps[last] = String::new();
            last -= 1;
        }
        self.statement_starts.push(first);
        let inner = self.inner_statements(first, statement.last, statement.depth + 1);
        let Some(head_end) = inner.first().map(|inner| inner.first - 1) else {
            self.query(first, last, indent);
            return;
        };
        let block = self.starts_block(first);
        let branches = self.is_any(first, &SQLSERVER_BRANCH_STATEMENTS);
        if branches {
            self.expr(first + 1, head_end, indent);
        }
        let opens_block = !block && !branches && head_end > first && self.is(head_end, "BEGIN");
        if opens_block {
            self.statement_starts.push(head_end);
        }
        if !block && !branches {
            let mut head_last = head_end;
            for word in ["BEGIN", "AS"] {
                if head_last > first && self.is(head_last, word) {
                    self.gaps[head_last] = newline(indent);
                    head_last -= 1;
                }
            }
            self.expr(first, head_last, indent);
        }
        let mut after = head_end + 1;
        for statement in inner {
            self.frame(after, statement.first.saturating_sub(1), indent);
            let after_else = statement.first > 0 && self.is(statement.first - 1, "ELSE");
            let inner_indent =
                if block || opens_block || (branches && !self.starts_block(statement.first)) {
                    indent + 1
                } else {
                    indent
                };
            if after_else && self.is(statement.first, "IF") {
                self.gaps[statement.first] = " ".to_string();
                self.ranged_statement(statement, indent);
            } else {
                self.gaps[statement.first] = newline(inner_indent);
                self.ranged_statement(statement, inner_indent);
            }
            after = statement.last + 1;
        }
        self.frame(after, last, indent);
    }

    /// Put each BEGIN, END and ELSE among tokens `first` to `last`, the words around a block's statements, on its own line at `indent`.
    fn frame(&mut self, first: usize, last: usize, indent: usize) {
        if first > last {
            return;
        }
        for index in first..=last {
            if self.is_any(index, &SQLSERVER_BLOCK_WORDS) {
                self.gaps[index] = newline(indent);
            } else if self.kind(index) == Some(TokenType::Semicolon) {
                self.gaps[index] = String::new();
            }
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
            .filter(|&index| self.is_any(index, &SET_OPERATORS) && !self.after_star(index))
            .collect();
        let mut operand = start;
        for operator in operators {
            let mut line = operator;
            while self.dialect == FormatDialect::BigQuery
                && line > operand
                && self.is_any(line - 1, &BIGQUERY_SET_OPERATOR_PREFIXES)
            {
                line -= 1;
            }
            if line > operand {
                self.operand(operand, line - 1, indent);
            }
            self.gaps[line] = newline(indent);
            let mut next = operator + 1;
            while next <= last && self.is_any(next, &SET_OPERATOR_MODIFIERS) {
                next += 1;
                if (self.is(next - 1, "BY") || self.is(next - 1, "ON"))
                    && let Some(closer) = self.partners.get(next).copied().flatten()
                    && self.kind(next) == Some(TokenType::LParen)
                {
                    next = closer + 1;
                }
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
        } else if self.starts_from_first(first) {
            self.lay_out_from_first(first, last, indent);
        } else if self.is(first, "WITH") {
            self.query(first, last, indent);
        } else if self.is(first, INSERT_KEYWORD) {
            self.insert(first, last, indent);
        } else {
            self.expr(first, last, indent);
        }
    }

    /// Lay out an INSERT: its target on the first line, then its VALUES or query from a new line.
    fn insert(&mut self, insert: usize, last: usize, indent: usize) {
        let body = self
            .tops(insert + 1, last)
            .into_iter()
            .find(|&index| self.is_any(index, &INSERT_BODIES));
        let Some(body) = body else {
            self.expr(insert, last, indent);
            return;
        };
        self.expr(insert, body - 1, indent);
        self.gaps[body] = newline(indent);
        if self.is(body, VALUES_KEYWORD) {
            self.expr(body, last, indent);
        } else {
            self.query(body, last, indent);
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
            {
                index = closer + 1;
            }
            if self.is(index, "USING")
                && self.is(index + 1, "KEY")
                && let Some(closer) = self.partners.get(index + 2).copied().flatten()
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
            let end = self.postgres_cte_clauses_end(closer, last, indent);
            if end < last && self.kind(end + 1) == Some(TokenType::Comma) {
                item = end + 2;
                continue;
            }
            return Some(end + 1);
        }
    }

    /// The last token of the PostgreSQL SEARCH and CYCLE clauses after the CTE body ending at `closer`, each on its own line; `closer` when there are none.
    fn postgres_cte_clauses_end(&mut self, closer: usize, last: usize, indent: usize) -> usize {
        let mut end = closer;
        while self.dialect == FormatDialect::PostgreSQL {
            let Some(&(_, last_word)) = POSTGRES_CTE_CLAUSES
                .iter()
                .find(|(clause, _)| self.is(end + 1, clause))
            else {
                break;
            };
            let Some(word) = (end + 2..=last).find(|&index| self.is(index, last_word)) else {
                break;
            };
            self.gaps[end + 1] = newline(indent);
            end = word + 1;
        }
        end
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
        let continues = index > 0
            && ((self.is(index, "FROM") && self.is_any(index - 1, &FROM_CONTINUED))
                || (self.is(index, "OFFSET") && self.is(index - 1, "WITH")));
        (!aliased && !continues && self.is_any(index, &CLAUSE_KEYWORDS))
            || (self.is(index, "USING") && self.is(index + 1, "SAMPLE"))
            || ((self.is(index, "GROUP") || self.is(index, "ORDER")) && self.is(index + 1, "BY"))
            || (self.dialect == FormatDialect::PostgreSQL && self.is(index, POSTGRES_INTO_CLAUSE))
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
            if self.is(item, "AS") && self.is_any(item + 1, &BIGQUERY_VALUE_TABLE_KINDS) {
                item += 2;
                continue;
            }
            if self.is(item, "TOP") {
                item += 1;
                item = match self.partners.get(item).copied().flatten() {
                    Some(closer) if closer > item => {
                        self.sqlserver_top_query(item, closer, indent);
                        closer + 1
                    }
                    _ => item + 1,
                };
                continue;
            }
            break;
        }
        self.selects += 1;
        for (first, end) in self.split_commas(item, projection_end) {
            self.gaps[first] = newline(indent + 1);
            if self.dialect == FormatDialect::Snowflake
                && !self.aliases[end]
                && !self.plain_column(first, end)
            {
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

    /// Lay out a FROM-first query: its clauses, then any SELECT with the clauses after it.
    fn lay_out_from_first(&mut self, from: usize, last: usize, indent: usize) {
        let tops = self.tops(from + 1, last);
        let select = tops.iter().copied().find(|&index| self.is(index, "SELECT"));
        let end = select.map_or(last, |select| select - 1);
        let clauses: Vec<usize> = std::iter::once(from)
            .chain(
                tops.into_iter()
                    .filter(|&index| index <= end && self.starts_clause(index)),
            )
            .collect();
        for (position, &clause) in clauses.iter().enumerate() {
            let clause_end = clauses.get(position + 1).map_or(end, |&next| next - 1);
            self.clause(clause, clause_end, indent);
        }
        if let Some(select) = select {
            self.gaps[select] = newline(indent);
            self.select(select, last, indent);
        }
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
            "USING" => {
                for index in clause + 2..=end {
                    self.protected[index] = true;
                }
                self.expr(clause + 2, end, indent);
            }
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
            } else if index > start
                && (self.starts_join(index, first) || self.sqlserver_starts_apply(index))
            {
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
        while join < last && !self.is(join, "JOIN") && !self.sqlserver_apply(join) {
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
        if self.dialect == FormatDialect::PostgreSQL
            && self.is_any(opener, &POSTGRES_INLINE_SOURCES)
            && self.kind(opener + 1) == Some(TokenType::LParen)
        {
            self.inline_openers[opener + 1] = true;
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
                && self.dialect == FormatDialect::Snowflake
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
        if self.is(opener + 1, "VALUES") {
            self.expr(opener + 1, closer - 1, indent);
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
        } else if self.quantified_pattern(opener) {
            self.list(closer, &[(opener + 1, closer - 1)], true, indent);
        } else if self.is(previous, "PIVOT")
            || self.is(previous, "UNPIVOT")
            || self.enclosing[opener]
                .is_some_and(|at| at > 0 && self.is(at - 1, "UNPIVOT") && at + 1 == opener)
        {
            self.expr(opener + 1, closer - 1, indent);
        } else if self.is_any(previous, &GROUPING_CALLS)
            || self.angles[previous]
            || (self.base[opener].is_empty() && self.calls(previous) && !self.in_run[previous])
        {
            self.call(previous, opener, closer, indent);
        } else if (self.is_word(previous) && !self.is_any(previous, &EXPRESSION_LEADERS))
            || self.in_run[previous]
        {
            self.expr(opener + 1, closer - 1, indent);
        } else {
            self.tuple(opener, closer, indent);
        }
    }

    /// Lay out a tuple, split past the guide width, or a parenthesised expression, always split.
    fn tuple(&mut self, opener: usize, closer: usize, indent: usize) {
        let items = self.split_commas(opener + 1, closer - 1);
        let split = match items.len() {
            0 => false,
            1 => true,
            _ => self.items_width(&items) > GUIDE_WIDTH,
        };
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
        let inline = self.protected[opener]
            || self.inline_openers[opener]
            || self.inline_calls().contains(&upper.as_str());
        let split =
            !items.is_empty() && (grouping || (!inline && self.items_width(&items) > GUIDE_WIDTH));
        self.list(closer, &items, split, indent);
    }

    fn inline_calls(&self) -> &'static [&'static str] {
        match self.dialect {
            FormatDialect::Snowflake => &SNOWFLAKE_INLINE_CALLS,
            FormatDialect::DuckDB => &DUCKDB_INLINE_CALLS,
            FormatDialect::PostgreSQL | FormatDialect::Databricks => &POSTGRES_INLINE_CALLS,
            FormatDialect::BigQuery => &BIGQUERY_INLINE_CALLS,
            FormatDialect::SqlServer => &SQLSERVER_INLINE_CALLS,
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

    /// The guide's spelling of token `index`: keywords and called names in upper case, other names as written.
    fn text(&self, index: usize) -> String {
        let lexeme = &self.lexemes[index];
        if self.protected[index]
            || self.in_run[index]
            || self.identifiers[index]
            || self.unspanned[index]
            || self.name_position(index)
            || self.field_name(index)
            || self.sqlserver_object_name(index)
            || self.bigquery_table_name(index)
            || !(is_keyword(lexeme) || self.called_name(index))
        {
            lexeme.raw.clone()
        } else {
            lexeme.raw.to_ascii_uppercase()
        }
    }

    /// Whether token `index` is an unquoted name called with `(`, which folds to upper case as SQLBuild's query fingerprint folds it: not a placeholder, an INSERT or CREATE TABLE target, or a CTE name with a column list.
    fn called_name(&self, index: usize) -> bool {
        let raw = &self.lexemes[index].raw;
        let Some(closer) = self.partners.get(index + 1).copied().flatten() else {
            return false;
        };
        self.kind(index + 1) == Some(TokenType::LParen)
            && is_unquoted_word(raw)
            && !raw.starts_with(PLACEHOLDER_NAME_PREFIX)
            && !(index > 0 && self.is_any(index - 1, &CALL_TARGET_KEYWORDS))
            && !(self.is(closer + 1, "AS")
                && (self.kind(closer + 2) == Some(TokenType::LParen)
                    || self.is_any(closer + 2, &CTE_BODY_PREFIXES)))
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
        self.field_lists[index]
    }

    /// Mark every token whose innermost open bracket is a struct type's field list, such as the `(` of `STRUCT(a INT)` or the `<` of `STRUCT<a INT>`, in one pass.
    fn mark_field_lists(&mut self) {
        let mut open: Vec<(TokenType, bool)> = Vec::new();
        for index in 0..self.len() {
            self.field_lists[index] = open.last().is_some_and(|&(_, fields)| fields);
            let kind = self.lexemes[index].kind;
            let after_struct = index > 0
                && STRUCT_TYPE_WORDS
                    .iter()
                    .any(|word| self.lexemes[index - 1].raw.eq_ignore_ascii_case(word));
            match kind {
                TokenType::LParen | TokenType::LBracket | TokenType::LBrace => {
                    open.push((kind, after_struct && kind == TokenType::LParen));
                }
                TokenType::Lt if after_struct || self.angles[index] => {
                    open.push((kind, after_struct));
                }
                TokenType::RParen | TokenType::RBracket | TokenType::RBrace => {
                    while open
                        .pop()
                        .is_some_and(|(opener, _)| opener == TokenType::Lt)
                    {}
                }
                TokenType::Gt | TokenType::GtGt => {
                    let closes = if kind == TokenType::GtGt { 2 } else { 1 };
                    for _ in 0..closes {
                        if open
                            .last()
                            .is_some_and(|&(opener, _)| opener == TokenType::Lt)
                        {
                            open.pop();
                        }
                    }
                }
                _ => {}
            }
        }
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
            statement_starts: self.statement_starts.clone(),
        }
    }
}
