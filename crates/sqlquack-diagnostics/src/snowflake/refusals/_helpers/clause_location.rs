//! Where a rejected node's offending clause is: a set operator with ALL, or a QUALIFY clause.

use sqlquack_syntax::Expression;

use sqlquack_syntax::tokens::main::node_location::node_location;
use sqlquack_syntax::tokens::models::{SourceLines, SourceLocation};

/// The keyword that starts a QUALIFY clause.
const QUALIFY_KEYWORD: &str = "QUALIFY";
/// The set operators Snowflake rejects with ALL.
const SET_OPERATORS: &[&str] = &["INTERSECT", "EXCEPT", "MINUS"];
/// The keyword after a set operator that keeps duplicates.
const ALL_KEYWORD: &str = "ALL";

/// The offending clause of a rejected node: a set operation's keywords, or a whole QUALIFY clause.
pub(crate) fn clause_location(
    node: &Expression,
    text: &[char],
    lines: &SourceLines,
) -> Option<SourceLocation> {
    match node {
        Expression::Intersect(operation) => {
            operator_location(node, (&operation.left, &operation.right), (text, lines))
        }
        Expression::Except(operation) => {
            operator_location(node, (&operation.left, &operation.right), (text, lines))
        }
        Expression::Select(select) => {
            let from = node_location(node, lines)?.start;
            let keyword = select_keyword(text, from, QUALIFY_KEYWORD)?;
            let end = node_location(&select.qualify.as_ref()?.this, lines)
                .map_or(keyword.1, |predicate| predicate.end.max(keyword.1));
            Some(lines.source_location(keyword.0, end))
        }
        _ => None,
    }
}

/// The range of `keyword` in the query from `from`, outside quotes and parentheses, before `;`.
fn select_keyword(text: &[char], from: usize, keyword: &str) -> Option<(usize, usize)> {
    let mut depth: usize = 0;
    let mut quote: Option<char> = None;
    let mut offset = from;
    while offset < text.len() {
        if let Some(open) = quote {
            quote = (text[offset] != open).then_some(open);
            offset += 1;
            continue;
        }
        match text[offset] {
            '\'' | '"' => quote = Some(text[offset]),
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            ';' if depth == 0 => return None,
            _ if depth == 0
                && (offset == 0 || !text[offset - 1].is_alphanumeric())
                && text.len() >= offset + keyword.len() =>
            {
                let end = offset + keyword.len();
                let word: String = text[offset..end].iter().collect();
                if word.eq_ignore_ascii_case(keyword)
                    && text
                        .get(end)
                        .is_none_or(|next| !next.is_alphanumeric() && *next != '_')
                {
                    return Some((offset, end));
                }
            }
            _ => {}
        }
        offset += 1;
    }
    None
}

/// The set operator with ALL between the operands, or within the operation where they have no place.
fn operator_location(
    node: &Expression,
    (left, right): (&Expression, &Expression),
    (text, lines): (&[char], &SourceLines),
) -> Option<SourceLocation> {
    let whole = node_location(node, lines);
    let (left, right) = (node_location(left, lines), node_location(right, lines));
    let from = match left {
        Some(left) => left.end,
        None => whole?.start,
    };
    let to = right
        .map_or(whole?.end, |right| right.start)
        .min(text.len());
    let found: Vec<(usize, usize)> = words(text.get(from..to)?)
        .windows(2)
        .filter_map(operator_with_all)
        .collect();
    let (start, end) = match (left, right) {
        (_, Some(_)) => found.last(),
        (Some(_), None) => found.first(),
        (None, None) => found.first().filter(|_| found.len() == 1),
    }?;
    Some(lines.source_location(from + start, from + end))
}

/// The range of a set operator and the ALL after it, when the pair of words is one.
fn operator_with_all(pair: &[(String, usize, usize)]) -> Option<(usize, usize)> {
    let [(operator, start, _), (all, _, end)] = pair else {
        return None;
    };
    let is_operator = SET_OPERATORS
        .iter()
        .any(|keyword| operator.eq_ignore_ascii_case(keyword));
    (is_operator && all.eq_ignore_ascii_case(ALL_KEYWORD)).then_some((*start, *end))
}

/// The words of `text` between spaces and parentheses, each with its character range.
fn words(text: &[char]) -> Vec<(String, usize, usize)> {
    let mut found: Vec<(String, usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (offset, character) in text.iter().chain(std::iter::once(&' ')).enumerate() {
        let separator = character.is_whitespace() || matches!(character, '(' | ')');
        match (separator, start) {
            (false, None) => start = Some(offset),
            (true, Some(begun)) => {
                found.push((text[begun..offset].iter().collect(), begun, offset));
                start = None;
            }
            _ => {}
        }
    }
    found
}
