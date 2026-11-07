//! SQLCLEAN046 to SQLCLEAN053: report-only safety Rules for queries that cannot do what they appear to.

use std::borrow::Cow;
use std::collections::HashSet;

use crate::tree::main::is_aggregate::is_aggregate;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::queries::models::Join;
use sqlquack_syntax::expressions::queries::models::With;
use sqlquack_syntax::expressions::statements::models::Delete;
use sqlquack_syntax::expressions::statements::models::Insert;
use sqlquack_syntax::expressions::statements::models::Update;
use sqlquack_syntax::tokens::models::{Token, TokenType};
use sqlquack_syntax::{Dialect, Expression, ExpressionWalk};

use crate::lint::models::{LintDiagnostic, LintRuleMetadata};
use crate::quality::constants::{
    DUPLICATE_CTE_NAME, INSERT_WITHOUT_COLUMNS, LITERAL_ZERO_DIVISION, MISPLACED_AGGREGATE,
    NATURAL_JOIN, NON_SNOWFLAKE_AGGREGATE_CALLS, NULL_MATCH, REVERSED_BETWEEN, UNFILTERED_WRITE,
};
use crate::quality::models::QualityRequest;
use crate::quality::syntax::is_trivia;

const RENAME_REFUSAL: &str = "Renaming a CTE requires choosing which definition its readers mean";
const NULL_TEST_REFUSAL: &str = "Rewriting to IS NULL changes what the query returns";
const DIVISOR_REFUSAL: &str = "Choosing a divisor or a NULL guard requires the intended result";
const WRITE_SCOPE_REFUSAL: &str = "Only the author knows which rows the statement should change";
const COLUMN_LIST_REFUSAL: &str = "Listing the target columns requires the table's schema";
const JOIN_CONDITION_REFUSAL: &str = "Writing the join condition requires both relations' schemas";
const BOUNDS_REFUSAL: &str = "Swapping the bounds changes what the query returns";
const AGGREGATE_REFUSAL: &str =
    "Moving the expression to HAVING, QUALIFY or an outer query requires restructuring the query";
const ASYMMETRIC: &str = "ASYMMETRIC";
const MODULO_FUNCTION: &str = "MOD";
const SINGLE_ARGUMENT_AGGREGATES: [&str; 2] = ["MIN", "MAX"];
const ISO_DATE_LENGTH: usize = 10;

/// A finding's start and end character offsets.
type CharSpan = (usize, usize);

/// Significant tokens with their parenthesis depth.
struct Significant<'a> {
    tokens: Vec<&'a Token>,
    depths: Vec<usize>,
}

impl<'a> Significant<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        let tokens: Vec<&Token> = tokens.iter().filter(|token| !is_trivia(token)).collect();
        let mut depths: Vec<usize> = Vec::with_capacity(tokens.len());
        let mut depth: usize = 0;
        for token in &tokens {
            if token.token_type == TokenType::RParen {
                depth = depth.saturating_sub(1);
            }
            depths.push(depth);
            if token.token_type == TokenType::LParen {
                depth += 1;
            }
        }
        Self { tokens, depths }
    }

    fn kind(&self, index: usize) -> Option<TokenType> {
        self.tokens.get(index).map(|token| token.token_type)
    }

    fn is(&self, index: usize, kind: TokenType) -> bool {
        self.kind(index) == Some(kind)
    }

    fn text_is(&self, index: usize, text: &str) -> bool {
        self.tokens
            .get(index)
            .is_some_and(|token| token.text.eq_ignore_ascii_case(text))
    }

    fn close(&self, open: usize) -> Option<usize> {
        (open + 1..self.tokens.len()).find(|&index| {
            self.is(index, TokenType::RParen) && self.depths[index] == self.depths[open]
        })
    }

    fn span(&self, first: usize, last: usize) -> CharSpan {
        (self.tokens[first].span.start, self.tokens[last].span.end)
    }

    /// Top-level comma positions inside the parentheses that open at `open`.
    fn arguments_commas(&self, open: usize, close: usize) -> Vec<usize> {
        (open + 1..close)
            .filter(|&index| {
                self.is(index, TokenType::Comma) && self.depths[index] == self.depths[open] + 1
            })
            .collect()
    }

    /// Non-empty statements split at top-level semicolons, as index ranges.
    fn statements(&self) -> Vec<(usize, usize)> {
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        let mut start: usize = 0;
        for index in 0..=self.tokens.len() {
            let ends = index == self.tokens.len()
                || (self.is(index, TokenType::Semicolon) && self.depths[index] == 0);
            if ends {
                if index > start {
                    ranges.push((start, index));
                }
                start = index + 1;
            }
        }
        ranges
    }
}

/// A literal bound whose order is exact: a number, or an ISO `DATE` literal.
#[derive(PartialEq, PartialOrd)]
enum Bound {
    Number(f64),
    Date(String),
}

pub(crate) fn diagnostics(request: &QualityRequest<'_>) -> Vec<LintDiagnostic> {
    let significant = Significant::new(request.tokens);
    let enabled = |rule: &LintRuleMetadata| request.enabled.contains(rule.code);
    let mut found: Vec<LintDiagnostic> = Vec::new();
    let mut report =
        |rule: &'static LintRuleMetadata, reason: &'static str, spans: Vec<CharSpan>| {
            found.extend(spans.into_iter().map(|(start, end)| LintDiagnostic {
                code: rule.code,
                message: Cow::Borrowed(rule.message),
                remediation: Cow::Borrowed(rule.remediation),
                start,
                end,
                fix: None,
                fix_unavailable_reason: Some(reason),
            }));
        };
    if enabled(&DUPLICATE_CTE_NAME) {
        report(
            &DUPLICATE_CTE_NAME,
            RENAME_REFUSAL,
            duplicate_cte_spans(request.statements, request.dialect),
        );
    }
    if enabled(&NULL_MATCH) {
        report(
            &NULL_MATCH,
            NULL_TEST_REFUSAL,
            null_match_spans(&significant),
        );
    }
    if enabled(&LITERAL_ZERO_DIVISION) {
        report(
            &LITERAL_ZERO_DIVISION,
            DIVISOR_REFUSAL,
            zero_division_spans(&significant),
        );
    }
    if enabled(&UNFILTERED_WRITE) || enabled(&INSERT_WITHOUT_COLUMNS) {
        let (writes, inserts) = statement_spans(&significant, request.statements);
        if enabled(&UNFILTERED_WRITE) {
            report(&UNFILTERED_WRITE, WRITE_SCOPE_REFUSAL, writes);
        }
        if enabled(&INSERT_WITHOUT_COLUMNS) {
            report(&INSERT_WITHOUT_COLUMNS, COLUMN_LIST_REFUSAL, inserts);
        }
    }
    if enabled(&NATURAL_JOIN) {
        report(
            &NATURAL_JOIN,
            JOIN_CONDITION_REFUSAL,
            natural_join_spans(&significant),
        );
    }
    if enabled(&REVERSED_BETWEEN) {
        report(
            &REVERSED_BETWEEN,
            BOUNDS_REFUSAL,
            reversed_between_spans(&significant),
        );
    }
    if enabled(&MISPLACED_AGGREGATE) {
        report(
            &MISPLACED_AGGREGATE,
            AGGREGATE_REFUSAL,
            misplaced_aggregate_spans(
                &significant,
                &aggregate_names(request.statements, request.dialect),
            ),
        );
    }
    found
}

fn duplicate_cte_spans(statements: &[Expression], dialect: Dialect) -> Vec<CharSpan> {
    let mut spans: Vec<CharSpan> = Vec::new();
    for statement in statements {
        for with in statement.dfs().filter_map(with_clause) {
            let mut seen: HashSet<String> = HashSet::new();
            for cte in &with.ctes {
                if !seen.insert(cte_key(&cte.alias, dialect))
                    && let Some(span) = cte.alias.span
                {
                    spans.push((span.start, span.end));
                }
            }
        }
    }
    spans
}

fn with_clause(node: &Expression) -> Option<&With> {
    match node {
        Expression::Select(select) => select.with.as_ref(),
        Expression::Union(operation) => operation.with.as_ref(),
        Expression::Intersect(operation) => operation.with.as_ref(),
        Expression::Except(operation) => operation.with.as_ref(),
        Expression::Insert(insert) => insert.with.as_ref(),
        Expression::Update(update) => update.with.as_ref(),
        Expression::Delete(delete) => delete.with.as_ref(),
        _ => None,
    }
}

/// A CTE name as the dialect resolves it, so only names the engine treats as equal collide.
fn cte_key(identifier: &Identifier, dialect: Dialect) -> String {
    let quoted_names_ignore_case = matches!(dialect, Dialect::DuckDB | Dialect::SQLite);
    match dialect {
        _ if identifier.quoted && !quoted_names_ignore_case => identifier.name.clone(),
        Dialect::Snowflake => identifier.name.to_uppercase(),
        _ => identifier.name.to_lowercase(),
    }
}

/// `CASE x WHEN NULL` and a singleton positive `IN (NULL)`; SQLCLEAN019 owns `NOT IN`.
fn null_match_spans(significant: &Significant<'_>) -> Vec<CharSpan> {
    let mut spans: Vec<CharSpan> = Vec::new();
    for index in 0..significant.tokens.len() {
        match significant.kind(index) {
            Some(TokenType::In)
                if !(index > 0 && significant.is(index - 1, TokenType::Not))
                    && significant.is(index + 1, TokenType::LParen)
                    && significant.is(index + 2, TokenType::Null)
                    && significant.is(index + 3, TokenType::RParen) =>
            {
                spans.push(significant.span(index, index + 3));
            }
            Some(TokenType::Case) if !significant.is(index + 1, TokenType::When) => {
                spans.extend(simple_case_null_spans(significant, index));
            }
            _ => {}
        }
    }
    spans
}

fn simple_case_null_spans(significant: &Significant<'_>, case: usize) -> Vec<CharSpan> {
    let depth: usize = significant.depths[case];
    let mut nested: usize = 0;
    let mut spans: Vec<CharSpan> = Vec::new();
    for index in case + 1..significant.tokens.len() {
        if significant.depths[index] < depth {
            break;
        }
        if significant.depths[index] > depth {
            continue;
        }
        match significant.kind(index) {
            Some(TokenType::Case) => nested += 1,
            Some(TokenType::End) if nested == 0 => break,
            Some(TokenType::End) => nested -= 1,
            Some(TokenType::When)
                if nested == 0
                    && significant.is(index + 1, TokenType::Null)
                    && significant.is(index + 2, TokenType::Then) =>
            {
                spans.push(significant.span(index, index + 1));
            }
            _ => {}
        }
    }
    spans
}

fn zero_division_spans(significant: &Significant<'_>) -> Vec<CharSpan> {
    let mut spans: Vec<CharSpan> = Vec::new();
    for index in 0..significant.tokens.len() {
        let operator = matches!(
            significant.kind(index),
            Some(TokenType::Slash | TokenType::Percent | TokenType::Div)
        );
        if operator
            && let Some((number, end)) = numeric_operand(significant, index + 1)
            && is_zero(&significant.tokens[number].text)
            && !binds_tighter(significant.kind(end + 1))
        {
            spans.push(significant.span(index, end));
        }
        if significant.text_is(index, MODULO_FUNCTION)
            && significant.is(index + 1, TokenType::LParen)
            && let Some(close) = significant.close(index + 1)
            && let [comma] = significant.arguments_commas(index + 1, close)[..]
            && let Some((number, end)) = numeric_operand(significant, comma + 1)
            && is_zero(&significant.tokens[number].text)
            && end + 1 == close
        {
            spans.push(significant.span(index, close));
        }
    }
    spans
}

/// Operators that bind tighter than division, so a following one changes the divisor.
fn binds_tighter(kind: Option<TokenType>) -> bool {
    matches!(
        kind,
        Some(
            TokenType::DColon
                | TokenType::DStar
                | TokenType::Caret
                | TokenType::LBracket
                | TokenType::Dot
        )
    )
}

/// Whether a number literal's digits are all zero, whatever its exponent.
fn is_zero(text: &str) -> bool {
    let mantissa: &str = text.split(['e', 'E']).next().unwrap_or(text);
    mantissa.chars().any(|character| character.is_ascii_digit())
        && mantissa
            .chars()
            .all(|character| matches!(character, '0' | '.' | '_'))
}

/// An optionally signed and parenthesised number literal at `start`: its number token and last index.
fn numeric_operand(significant: &Significant<'_>, start: usize) -> Option<(usize, usize)> {
    let mut index: usize = start;
    let mut opens: usize = 0;
    while significant.is(index, TokenType::LParen) {
        opens += 1;
        index += 1;
    }
    if matches!(
        significant.kind(index),
        Some(TokenType::Dash | TokenType::Plus)
    ) {
        index += 1;
    }
    if !significant.is(index, TokenType::Number) {
        return None;
    }
    let mut end: usize = index;
    for _ in 0..opens {
        if !significant.is(end + 1, TokenType::RParen) {
            return None;
        }
        end += 1;
    }
    Some((index, end))
}

/// A number or an ISO `DATE 'YYYY-MM-DD'` literal starting at `start`, and its last index.
fn bound(significant: &Significant<'_>, start: usize) -> Option<(Bound, usize)> {
    if significant.is(start, TokenType::Date) && significant.is(start + 1, TokenType::String) {
        let text: &str = &significant.tokens[start + 1].text;
        let iso: bool = text.len() == ISO_DATE_LENGTH
            && text
                .char_indices()
                .all(|(position, character)| match position {
                    4 | 7 => character == '-',
                    _ => character.is_ascii_digit(),
                });
        return iso.then(|| (Bound::Date(text.to_owned()), start + 1));
    }
    let (number, end) = numeric_operand(significant, start)?;
    let Ok(magnitude) = significant.tokens[number]
        .text
        .replace('_', "")
        .parse::<f64>()
    else {
        return None;
    };
    let negative: bool = significant.is(number - 1, TokenType::Dash);
    Some((
        Bound::Number(if negative { -magnitude } else { magnitude }),
        end,
    ))
}

fn reversed_between_spans(significant: &Significant<'_>) -> Vec<CharSpan> {
    let mut spans: Vec<CharSpan> = Vec::new();
    for index in 0..significant.tokens.len() {
        if !significant.is(index, TokenType::Between) {
            continue;
        }
        let low_start: usize = index + 1 + usize::from(significant.text_is(index + 1, ASYMMETRIC));
        if let Some((low, low_end)) = bound(significant, low_start)
            && significant.is(low_end + 1, TokenType::And)
            && let Some((high, high_end)) = bound(significant, low_end + 2)
            && ends_bound(significant.kind(high_end + 1))
            && matches!(
                (&low, &high),
                (Bound::Number(_), Bound::Number(_)) | (Bound::Date(_), Bound::Date(_))
            )
            && low > high
        {
            spans.push(significant.span(index, high_end));
        }
    }
    spans
}

/// Tokens that bind looser than BETWEEN, so the upper bound ends before them.
fn ends_bound(kind: Option<TokenType>) -> bool {
    match kind {
        None => true,
        Some(kind) => matches!(
            kind,
            TokenType::RParen
                | TokenType::Comma
                | TokenType::Semicolon
                | TokenType::And
                | TokenType::Or
                | TokenType::Then
                | TokenType::When
                | TokenType::Else
                | TokenType::End
                | TokenType::As
                | TokenType::From
                | TokenType::Where
                | TokenType::Group
                | TokenType::Having
                | TokenType::Qualify
                | TokenType::Window
                | TokenType::Order
                | TokenType::Limit
                | TokenType::Union
                | TokenType::Except
                | TokenType::Intersect
        ),
    }
}

/// UPDATE or DELETE without a limiting predicate, and INSERT without a column list.
fn statement_spans(
    significant: &Significant<'_>,
    statements: &[Expression],
) -> (Vec<CharSpan>, Vec<CharSpan>) {
    let mut writes: Vec<CharSpan> = Vec::new();
    let mut inserts: Vec<CharSpan> = Vec::new();
    let ranges: Vec<(usize, usize)> = significant.statements();
    if ranges.len() != statements.len() {
        return (writes, inserts);
    }
    for (statement, (start, end)) in statements.iter().zip(ranges) {
        let keyword: usize = (start..end)
            .find(|&index| {
                significant.depths[index] == 0
                    && matches!(
                        significant.kind(index),
                        Some(TokenType::Update | TokenType::Delete | TokenType::Insert)
                    )
            })
            .unwrap_or(start);
        let span = significant.span(keyword, keyword);
        match statement {
            Expression::Update(update) if !update_is_limited(update) => writes.push(span),
            Expression::Delete(delete) if !delete_is_limited(delete) => writes.push(span),
            Expression::Insert(insert) if lacks_column_list(insert) => inserts.push(span),
            _ => {}
        }
    }
    (writes, inserts)
}

fn update_is_limited(update: &Update) -> bool {
    update
        .where_clause
        .as_ref()
        .is_some_and(|clause| reads_rows(&clause.this))
        || update.limit.is_some()
        || update
            .table_joins
            .iter()
            .chain(&update.from_joins)
            .any(join_is_limiting)
}

fn delete_is_limited(delete: &Delete) -> bool {
    delete
        .where_clause
        .as_ref()
        .is_some_and(|clause| reads_rows(&clause.this))
        || delete.limit.is_some()
        || delete.joins.iter().any(join_is_limiting)
}

/// Whether a predicate depends on the rows: it reads a column or a subquery, unlike `WHERE TRUE`.
fn reads_rows(predicate: &Expression) -> bool {
    predicate.dfs().any(|node| {
        matches!(
            node,
            Expression::Column(_) | Expression::Select(_) | Expression::Subquery(_)
        )
    })
}

fn join_is_limiting(join: &Join) -> bool {
    join.on.as_ref().is_some_and(reads_rows) || !join.using.is_empty()
}

fn lacks_column_list(insert: &Insert) -> bool {
    insert.columns.is_empty()
        && !insert.default_values
        && !insert.by_name
        && insert.directory.is_none()
        && (insert.query.is_some() || !insert.values.is_empty())
}

fn natural_join_spans(significant: &Significant<'_>) -> Vec<CharSpan> {
    let mut spans: Vec<CharSpan> = Vec::new();
    for index in 0..significant.tokens.len() {
        if !significant.is(index, TokenType::Natural) {
            continue;
        }
        let mut join: usize = index + 1;
        while matches!(
            significant.kind(join),
            Some(
                TokenType::Left
                    | TokenType::Right
                    | TokenType::Full
                    | TokenType::Inner
                    | TokenType::Outer
            )
        ) {
            join += 1;
        }
        if significant.is(join, TokenType::Join) {
            spans.push(significant.span(index, join));
        }
    }
    spans
}

/// Function names the parser classified as aggregates, upper-cased without underscores.
fn aggregate_names(statements: &[Expression], dialect: Dialect) -> HashSet<String> {
    statements
        .iter()
        .flat_map(|statement| statement.dfs())
        .filter(|node| is_aggregate(node))
        .filter_map(|node| match node {
            Expression::AggregateFunction(function)
                if dialect == Dialect::Snowflake && is_non_snowflake_aggregate(&function.name) =>
            {
                None
            }
            Expression::AggregateFunction(function) => Some(normalized_name(&function.name)),
            other => Some(normalized_name(other.variant_name())),
        })
        .collect()
}

/// Whether `name` is another dialect's aggregate that Snowflake does not have.
fn is_non_snowflake_aggregate(name: &str) -> bool {
    NON_SNOWFLAKE_AGGREGATE_CALLS
        .iter()
        .any(|aggregate| aggregate.eq_ignore_ascii_case(name))
}

fn normalized_name(name: &str) -> String {
    name.chars()
        .filter(|character| *character != '_')
        .flat_map(char::to_uppercase)
        .collect()
}

/// Aggregate and window calls directly in WHERE, ON or GROUP BY, outside nested queries.
fn misplaced_aggregate_spans(
    significant: &Significant<'_>,
    aggregates: &HashSet<String>,
) -> Vec<CharSpan> {
    let mut spans: Vec<CharSpan> = Vec::new();
    for index in 0..significant.tokens.len() {
        let first: usize = match significant.kind(index) {
            Some(TokenType::Where | TokenType::On) => index + 1,
            Some(TokenType::Group) if significant.is(index + 1, TokenType::By) => index + 2,
            _ => continue,
        };
        spans.extend(clause_call_spans(
            significant,
            (first, significant.depths[index]),
            significant.is(index, TokenType::On),
            aggregates,
        ));
    }
    spans
}

fn clause_call_spans(
    significant: &Significant<'_>,
    (first, depth): (usize, usize),
    comma_ends: bool,
    aggregates: &HashSet<String>,
) -> Vec<CharSpan> {
    let mut spans: Vec<CharSpan> = Vec::new();
    let mut index: usize = first;
    while index < significant.tokens.len() {
        if significant.depths[index] < depth
            || (significant.depths[index] == depth && ends_clause(significant, index, comma_ends))
        {
            break;
        }
        if significant.is(index, TokenType::LParen)
            && matches!(
                significant.kind(index + 1),
                Some(TokenType::Select | TokenType::With | TokenType::Values)
            )
        {
            index = significant
                .close(index)
                .map_or(significant.tokens.len(), |close| close + 1);
            continue;
        }
        if let Some((close, end)) = call(significant, index) {
            if significant.is(close + 1, TokenType::Over) {
                spans.push(significant.span(index, end));
                index = end + 1;
                continue;
            }
            if is_aggregate_call(significant, index, close, aggregates) {
                spans.push(significant.span(index, close));
                index = close + 1;
                continue;
            }
        }
        index += 1;
    }
    spans
}

/// A function call at `name`: its closing parenthesis and the end of any OVER specification.
fn call(significant: &Significant<'_>, name: usize) -> Option<(usize, usize)> {
    let text: &str = &significant.tokens[name].text;
    let is_name: bool = text
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    if !is_name
        || !significant.is(name + 1, TokenType::LParen)
        || (name > 0 && significant.is(name - 1, TokenType::Dot))
    {
        return None;
    }
    let close: usize = significant.close(name + 1)?;
    let end: usize = if significant.is(close + 1, TokenType::Over) {
        if significant.is(close + 2, TokenType::LParen) {
            significant.close(close + 2)?
        } else {
            close + 2
        }
    } else {
        close
    };
    (end < significant.tokens.len()).then_some((close, end))
}

fn is_aggregate_call(
    significant: &Significant<'_>,
    name: usize,
    close: usize,
    aggregates: &HashSet<String>,
) -> bool {
    let normalized: String = normalized_name(&significant.tokens[name].text);
    aggregates.contains(&normalized)
        && (!SINGLE_ARGUMENT_AGGREGATES.contains(&normalized.as_str())
            || significant.arguments_commas(name + 1, close).is_empty())
}

/// Whether `index` ends the clause; only an ON condition ends at a comma, before the next relation.
fn ends_clause(significant: &Significant<'_>, index: usize, comma_ends: bool) -> bool {
    match significant.kind(index) {
        Some(TokenType::Group) => significant.is(index + 1, TokenType::By),
        Some(TokenType::Comma) => comma_ends,
        Some(kind) => matches!(
            kind,
            TokenType::Where
                | TokenType::Having
                | TokenType::Qualify
                | TokenType::Window
                | TokenType::Order
                | TokenType::Limit
                | TokenType::Union
                | TokenType::Except
                | TokenType::Intersect
                | TokenType::Returning
                | TokenType::Fetch
                | TokenType::Offset
                | TokenType::Semicolon
                | TokenType::Join
                | TokenType::Inner
                | TokenType::Left
                | TokenType::Right
                | TokenType::Full
                | TokenType::Cross
                | TokenType::Natural
                | TokenType::Set
                | TokenType::Using
        ),
        None => true,
    }
}
