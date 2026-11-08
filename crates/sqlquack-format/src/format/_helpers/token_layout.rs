//! Print authored SQL tokens with whitespace taken from the layout guide, and check the result.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use sqlquack_syntax::tokens::models::TokenType;

use crate::format::_helpers::guide::GuideLayout;
use crate::format::_helpers::lexemes::{
    Comment, Lexeme, is_keyword, is_string_literal, is_unquoted_word,
};
use crate::format::constants::{
    ALIGNMENT_FAILURE, CLOSE_PARENTHESIS, COMMENT_ATTACHMENT_FAILURE, OPEN_PARENTHESIS,
    STRING_BREAK_FAILURE, TOKEN_PRESERVATION_FAILURE,
};

const RESYNC_WINDOW: usize = 48;
const RESYNC_CONFIRMATION: usize = 1;
const NO_SPACE_BEFORE: [&str; 7] = [",", ")", "]", ".", ";", "::", ":"];
const NO_SPACE_AFTER: [&str; 5] = ["(", "[", ".", "::", ":"];
const STRUCTURAL_PUNCTUATION: [&str; 6] = ["(", ")", "[", "]", ",", ";"];

/// One authored or guide token with its text and alignment key.
struct Keyed {
    start: usize,
    end: usize,
    raw: String,
    key: Key,
    /// An unquoted authored word outside the name positions that keep reserved words exact.
    recasable: bool,
    /// An unquoted word the tokenizer types as a keyword.
    keyword: bool,
}

/// How tokens compare: words ignoring ASCII case, punctuation by type, the rest by text.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    Word,
    Punctuation(TokenType),
    Raw,
}

impl Keyed {
    fn new(lexeme: &Lexeme, recasable: bool) -> Self {
        let raw = &lexeme.raw;
        let key = if is_unquoted_word(raw) {
            Key::Word
        } else if raw
            .chars()
            .all(|character| character.is_ascii_punctuation())
        {
            Key::Punctuation(lexeme.kind)
        } else {
            Key::Raw
        };
        Self {
            start: lexeme.start,
            end: lexeme.end,
            raw: raw.clone(),
            key,
            recasable,
            keyword: is_keyword(lexeme),
        }
    }

    fn same_key(&self, other: &Keyed) -> bool {
        self.key == other.key
            && match self.key {
                Key::Word => self.raw.eq_ignore_ascii_case(&other.raw),
                Key::Punctuation(_) => true,
                Key::Raw => self.raw == other.raw,
            }
    }
}

/// The guide's tokens and text, read once for every print against it.
pub(crate) struct Oracle {
    generated: Vec<Keyed>,
    text: String,
    /// Token ranges, in order and apart, whose authored text is printed byte for byte.
    pub(crate) verbatim: Vec<(usize, usize)>,
    /// Authored tokens that start a statement, which take keyword case even after `AS`.
    statement_starts: Vec<usize>,
}

impl Oracle {
    pub(crate) fn new(guide: GuideLayout) -> Self {
        let generated: Vec<Keyed> = guide
            .tokens
            .iter()
            .enumerate()
            .map(|(index, token)| {
                let mut keyed = Keyed::new(token, false);
                keyed.keyword |= is_called(&guide.tokens, index);
                keyed
            })
            .collect();
        Self {
            generated,
            text: guide.text,
            verbatim: guide.verbatim,
            statement_starts: guide.statement_starts,
        }
    }
}

/// Authored tokens printed against the guide, with what the token check needs about them.
pub(crate) struct AuthoredPrint {
    pub(crate) sql: String,
    /// The byte range and type of each printed token; comments and whitespace lie between them.
    pub(crate) tokens: Vec<(Range<usize>, TokenType)>,
    pub(crate) recases: BTreeMap<usize, String>,
}

/// Comments attached to the authored token they precede or follow.
#[derive(Default)]
struct Attached<'a> {
    leading: Vec<&'a Comment>,
    trailing: Vec<&'a Comment>,
}

/// Authored SQL with its tokens and the comments between them.
pub(crate) struct AuthoredSql<'a> {
    pub(crate) sql: &'a str,
    pub(crate) lexemes: &'a [Lexeme],
    pub(crate) comments: &'a [Comment],
}

/// The authored tokens, each word recasable unless it follows an accessor, or an `AS` without starting a statement.
fn keyed_authored(lexemes: &[Lexeme], statement_starts: &[usize]) -> Vec<Keyed> {
    lexemes
        .iter()
        .enumerate()
        .map(|(index, lexeme)| {
            let name = index > 0
                && (matches!(lexemes[index - 1].raw.as_str(), "." | ":" | "@" | "$")
                    || (lexemes[index - 1].raw.eq_ignore_ascii_case("AS")
                        && !statement_starts.contains(&index)));
            Keyed::new(lexeme, !name && is_unquoted_word(&lexeme.raw))
        })
        .collect()
}

/// Print every authored token in order, taking only whitespace and keyword case from `oracle`.
pub(crate) fn print_authored_tokens(
    source: &AuthoredSql<'_>,
    oracle: &Oracle,
) -> Result<AuthoredPrint, String> {
    let AuthoredSql {
        sql,
        lexemes,
        comments,
    } = *source;
    let authored = keyed_authored(lexemes, &oracle.statement_starts);
    let generated = &oracle.generated;
    let alignment = align(&authored, generated)?;
    let separators = separators(&authored, generated, &alignment, &oracle.text)?;
    let attached = attach_comments(lexemes, comments);
    let mut output = String::with_capacity(sql.len() + sql.len() / 4);
    let string_breaks = string_breaks(sql, lexemes);
    let mut recases: BTreeMap<usize, String> = BTreeMap::new();
    let mut printed_tokens: Vec<(Range<usize>, TokenType)> = Vec::with_capacity(lexemes.len());
    for (index, lexeme) in authored.iter().enumerate() {
        let forced_break = string_breaks.contains(&index) && !separators[index].contains('\n');
        let forced: String;
        let separator: &str = if forced_break {
            forced = format!("\n{}", current_indentation(&output));
            &forced
        } else {
            &separators[index]
        };
        let leading = &attached[index].leading;
        if leading.is_empty() {
            output.push_str(separator);
        } else if index == 0 || separator.contains('\n') {
            output.push_str(separator);
            let indentation = current_indentation(&output);
            for comment in leading {
                output.push_str(&comment.text);
                output.push('\n');
                output.push_str(&indentation);
            }
        } else {
            let indentation = current_indentation(&output);
            for comment in leading {
                output.push('\n');
                output.push_str(&indentation);
                output.push_str(&comment.text);
            }
            output.push('\n');
            output.push_str(&indentation);
        }
        let guide = alignment[index].map(|at| &generated[at]);
        let start = output.len();
        match guide.filter(|guide| confirms_keyword(lexeme, guide)) {
            Some(guide) => {
                recases.insert(index, guide.raw.clone());
                output.push_str(&guide.raw);
            }
            None => output.push_str(&lexeme.raw),
        }
        printed_tokens.push((start..output.len(), lexemes[index].kind));
        let trailing = &attached[index].trailing;
        for (position, comment) in trailing.iter().enumerate() {
            if position == 0 || !trailing[position - 1].line {
                output.push(' ');
            }
            output.push_str(&comment.text);
            if !comment.line {
                continue;
            }
            let next_starts_line = if position + 1 < trailing.len() {
                false
            } else {
                separators
                    .get(index + 1)
                    .is_none_or(|next| next.starts_with('\n'))
            };
            if !next_starts_line {
                output.push('\n');
                if position + 1 < trailing.len() {
                    output.push_str(&current_indentation(&output));
                }
            }
        }
    }
    Ok(AuthoredPrint {
        sql: output,
        tokens: printed_tokens,
        recases,
    })
}

/// Refuse output that changes anything but layout and the recorded keyword recases.
pub(crate) fn verify_printed_tokens(
    before: &AuthoredSql<'_>,
    after: &AuthoredSql<'_>,
    recases: &BTreeMap<usize, String>,
) -> Result<(), String> {
    if before.lexemes.len() != after.lexemes.len() {
        return Err(TOKEN_PRESERVATION_FAILURE.to_string());
    }
    for (index, (authored, printed)) in before.lexemes.iter().zip(after.lexemes).enumerate() {
        if authored.raw == printed.raw {
            continue;
        }
        let allowed = recases.get(&index).is_some_and(|recase| {
            *recase == printed.raw
                && (is_keyword(printed) || is_called(after.lexemes, index))
                && printed.raw == printed.raw.to_ascii_uppercase()
                && authored.raw.eq_ignore_ascii_case(&printed.raw)
        });
        if !allowed {
            return Err(TOKEN_PRESERVATION_FAILURE.to_string());
        }
    }
    let attached = comments_stay_attached(
        &comment_positions(before.comments, before.lexemes),
        &comment_positions(after.comments, after.lexemes),
        after.lexemes,
    );
    if !attached {
        return Err(COMMENT_ATTACHMENT_FAILURE.to_string());
    }
    if !string_breaks(before.sql, before.lexemes)
        .is_subset(&string_breaks(after.sql, after.lexemes))
    {
        return Err(STRING_BREAK_FAILURE.to_string());
    }
    Ok(())
}

/// Whether every comment keeps its place, allowing a line comment to move just past a comma.
fn comments_stay_attached(
    before: &[(&str, usize)],
    after: &[(&str, usize)],
    after_lexemes: &[Lexeme],
) -> bool {
    before.len() == after.len()
        && before.iter().zip(after).all(
            |((before_text, before_position), (after_text, after_position))| {
                before_text == after_text
                    && (before_position == after_position
                        || (*after_position == before_position + 1
                            && before_text.starts_with("--")
                            && after_lexemes
                                .get(*before_position)
                                .is_some_and(|lexeme| lexeme.kind == TokenType::Comma)))
            },
        )
}

fn comment_positions<'a>(comments: &'a [Comment], lexemes: &[Lexeme]) -> Vec<(&'a str, usize)> {
    comments
        .iter()
        .map(|comment| {
            let preceding = lexemes.partition_point(|lexeme| lexeme.end <= comment.start);
            (comment.text.as_str(), preceding)
        })
        .collect()
}

/// Indices of string literals that follow another string literal across a line break.
fn string_breaks(sql: &str, lexemes: &[Lexeme]) -> BTreeSet<usize> {
    (1..lexemes.len())
        .filter(|&index| {
            is_string_literal(lexemes[index - 1].kind)
                && is_string_literal(lexemes[index].kind)
                && sql[lexemes[index - 1].end..lexemes[index].start].contains('\n')
        })
        .collect()
}

/// Map authored tokens to matching guide tokens; a closer only matches its opener's partner.
fn align(authored: &[Keyed], generated: &[Keyed]) -> Result<Vec<Option<usize>>, String> {
    let mut state = Alignment {
        authored,
        generated,
        authored_partners: partners(authored),
        generated_partners: partners(generated),
        forward: vec![None; authored.len()],
        backward: vec![None; generated.len()],
    };
    let (mut left, mut right) = (0_usize, 0_usize);
    while left < authored.len() && right < generated.len() {
        if state.matches(left, right) {
            state.forward[left] = Some(right);
            state.backward[right] = Some(left);
            left += 1;
            right += 1;
            continue;
        }
        let (skip_left, skip_right) = state
            .resync(left, right)
            .ok_or_else(|| ALIGNMENT_FAILURE.to_string())?;
        left += skip_left;
        right += skip_right;
    }
    Ok(state.forward)
}

struct Alignment<'a> {
    authored: &'a [Keyed],
    generated: &'a [Keyed],
    authored_partners: Vec<Option<usize>>,
    generated_partners: Vec<Option<usize>>,
    forward: Vec<Option<usize>>,
    backward: Vec<Option<usize>>,
}

impl Alignment<'_> {
    fn matches(&self, left: usize, right: usize) -> bool {
        if !self.authored[left].same_key(&self.generated[right]) {
            return false;
        }
        if !is_closer(&self.authored[left].raw) {
            return true;
        }
        match (self.authored_partners[left], self.generated_partners[right]) {
            (Some(opener), Some(generated_opener)) => match self.forward[opener] {
                Some(aligned) => aligned == generated_opener,
                None => self.backward[generated_opener].is_none(),
            },
            (None, None) => true,
            _ => false,
        }
    }

    fn resync(&self, left: usize, right: usize) -> Option<(usize, usize)> {
        let remaining_left = self.authored.len() - left;
        let remaining_right = self.generated.len() - right;
        for cost in 1..=(2 * RESYNC_WINDOW) {
            for skip_left in 0..=cost.min(RESYNC_WINDOW) {
                let skip_right = cost - skip_left;
                if skip_right > RESYNC_WINDOW {
                    continue;
                }
                if skip_left >= remaining_left || skip_right >= remaining_right {
                    if skip_left >= remaining_left && skip_right >= remaining_right {
                        return Some((remaining_left, remaining_right));
                    }
                    continue;
                }
                if !self.matches(left + skip_left, right + skip_right) {
                    continue;
                }
                let anchor = &self.authored[left + skip_left];
                let confirmations = if matches!(anchor.key, Key::Punctuation(_)) {
                    RESYNC_CONFIRMATION
                } else {
                    0
                };
                let confirmed = (1..=confirmations).all(|offset| {
                    match (
                        self.authored.get(left + skip_left + offset),
                        self.generated.get(right + skip_right + offset),
                    ) {
                        (Some(before), Some(after)) => before.same_key(after),
                        _ => true,
                    }
                });
                if confirmed {
                    return Some((skip_left, skip_right));
                }
            }
        }
        None
    }
}

fn is_closer(raw: &str) -> bool {
    matches!(raw, ")" | "]" | "}")
}

fn partners(tokens: &[Keyed]) -> Vec<Option<usize>> {
    let mut partners: Vec<Option<usize>> = vec![None; tokens.len()];
    let mut open: Vec<usize> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.raw.as_str() {
            "(" | "[" | "{" => open.push(index),
            ")" | "]" | "}" => {
                if let Some(opener) = open.pop() {
                    partners[opener] = Some(index);
                    partners[index] = Some(opener);
                }
            }
            _ => {}
        }
    }
    partners
}

fn separators(
    authored: &[Keyed],
    generated: &[Keyed],
    alignment: &[Option<usize>],
    oracle: &str,
) -> Result<Vec<String>, String> {
    let mut separators: Vec<String> = vec![String::new(); authored.len()];
    let anchors: Vec<usize> = (0..authored.len())
        .filter(|&index| alignment[index].is_some())
        .collect();
    if anchors.is_empty() {
        return Err(ALIGNMENT_FAILURE.to_string());
    }
    let gap = |left: usize, right: usize| -> String {
        oracle[generated[left].end..generated[right].start].to_string()
    };
    for index in 1..anchors[0] {
        separators[index] = default_space(&authored[index - 1], &authored[index]).to_string();
    }
    if anchors[0] > 0 {
        separators[anchors[0]] =
            default_space(&authored[anchors[0] - 1], &authored[anchors[0]]).to_string();
    }
    for pair in anchors.windows(2) {
        let (previous, next) = (pair[0], pair[1]);
        let (previous_at, next_at) = (
            alignment[previous].ok_or(ALIGNMENT_FAILURE)?,
            alignment[next].ok_or(ALIGNMENT_FAILURE)?,
        );
        let merged = merged_gap(
            &(previous_at..next_at)
                .map(|at| gap(at, at + 1))
                .collect::<Vec<_>>(),
        );
        if previous + 1 == next {
            separators[next] = merged;
            continue;
        }
        for index in previous + 2..next {
            separators[index] = default_space(&authored[index - 1], &authored[index]).to_string();
        }
        let first = previous + 1;
        let last = next - 1;
        let substitution = next_at > previous_at + 1;
        separators[first] = if default_space(&authored[previous], &authored[first]).is_empty() {
            String::new()
        } else if substitution {
            gap(previous_at, previous_at + 1)
        } else {
            " ".to_string()
        };
        separators[next] = if substitution {
            let oracle_gap = (previous_at + 1..next_at)
                .rev()
                .find(|&at| generated[at].same_key(&authored[last]))
                .map_or_else(|| gap(next_at - 1, next_at), |at| gap(at, at + 1));
            let unspaced = default_space(&authored[last], &authored[next]).is_empty();
            if unspaced && !oracle_gap.contains('\n') {
                String::new()
            } else {
                oracle_gap
            }
        } else if merged.contains('\n') {
            merged
        } else {
            default_space(&authored[last], &authored[next]).to_string()
        };
    }
    let last_anchor = anchors[anchors.len() - 1];
    for index in last_anchor + 1..authored.len() {
        separators[index] = default_space(&authored[index - 1], &authored[index]).to_string();
    }
    let last_at = alignment[last_anchor].ok_or(ALIGNMENT_FAILURE)?;
    if last_anchor + 1 < authored.len()
        && last_at + 1 < generated.len()
        && !separators[last_anchor + 1].is_empty()
    {
        separators[last_anchor + 1] = gap(last_at, last_at + 1);
    }
    for index in 1..authored.len() {
        let adjacent_in_source = authored[index - 1].end == authored[index].start;
        let oracle_pair = matches!(
            (alignment[index - 1], alignment[index]),
            (Some(previous), Some(next)) if previous + 1 == next
        );
        let structural = [&authored[index - 1], &authored[index]]
            .iter()
            .any(|token| STRUCTURAL_PUNCTUATION.contains(&token.raw.as_str()));
        let call =
            is_unquoted_word(&authored[index - 1].raw) && authored[index].raw == OPEN_PARENTHESIS;
        let inside_parentheses =
            authored[index - 1].raw == OPEN_PARENTHESIS || authored[index].raw == CLOSE_PARENTHESIS;
        if (adjacent_in_source && !oracle_pair && (!structural || call))
            || (inside_parentheses && !separators[index].contains('\n'))
        {
            separators[index].clear();
        } else if separators[index].is_empty()
            && !adjacent_in_source
            && would_fuse(&authored[index - 1], &authored[index])
        {
            separators[index] = " ".to_string();
        }
    }
    Ok(separators)
}

fn merged_gap(gaps: &[String]) -> String {
    if let [only] = gaps {
        return only.clone();
    }
    if let Some(newline) = gaps.iter().rev().find(|gap| gap.contains('\n')) {
        return newline.clone();
    }
    if gaps.iter().any(|gap| !gap.is_empty()) {
        " ".to_string()
    } else {
        String::new()
    }
}

fn default_space(previous: &Keyed, next: &Keyed) -> &'static str {
    if NO_SPACE_BEFORE.contains(&next.raw.as_str())
        || NO_SPACE_AFTER.contains(&previous.raw.as_str())
    {
        ""
    } else {
        " "
    }
}

/// Whether printing two separated tokens without a space could lex them differently.
fn would_fuse(previous: &Keyed, next: &Keyed) -> bool {
    let (Some(left), Some(right)) = (previous.raw.chars().last(), next.raw.chars().next()) else {
        return false;
    };
    let word = |character: char| {
        character.is_alphanumeric() || matches!(character, '_' | '$' | '\'' | '"' | '`' | '@')
    };
    let operator = |character: char| "+-*/<>=!|&^%~:#?".contains(character);
    (word(left) && word(right)) || (operator(left) && operator(right))
}

/// Whether token `index` is an unquoted word followed by `(`, as a called name is.
fn is_called(lexemes: &[Lexeme], index: usize) -> bool {
    is_unquoted_word(&lexemes[index].raw)
        && lexemes
            .get(index + 1)
            .is_some_and(|next| next.kind == TokenType::LParen)
}

/// Whether the guide prints the aligned authored word as an upper-case keyword.
fn confirms_keyword(authored: &Keyed, generated: &Keyed) -> bool {
    authored.recasable
        && generated.keyword
        && generated.raw != authored.raw
        && generated.raw == generated.raw.to_ascii_uppercase()
        && generated.raw.eq_ignore_ascii_case(&authored.raw)
}

fn attach_comments<'a>(lexemes: &[Lexeme], comments: &'a [Comment]) -> Vec<Attached<'a>> {
    let mut attached: Vec<Attached<'a>> = (0..lexemes.len()).map(|_| Attached::default()).collect();
    for comment in comments {
        let next = lexemes.partition_point(|lexeme| lexeme.start < comment.end);
        let preceding = lexemes.partition_point(|lexeme| lexeme.end <= comment.start);
        let next = (next < lexemes.len()).then_some(next);
        let previous = preceding.checked_sub(1);
        match (comment.leading, next, previous) {
            (true, Some(next), _) | (false, Some(next), None) => {
                attached[next].leading.push(comment);
            }
            (_, _, Some(previous)) => attached[previous].trailing.push(comment),
            (_, None, None) => {}
        }
    }
    attached
}

fn current_indentation(output: &str) -> String {
    let line = output.rsplit('\n').next().unwrap_or_default();
    line.chars()
        .take_while(|character| *character == ' ' || *character == '\t')
        .collect()
}
