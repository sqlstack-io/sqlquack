//! Regular-expression functions of text, or of a constant Java pattern written in the RE2 that matches the same text.

use sqlquack_diagnostics::failures::constants::ARGUMENT_REJECTED;

use crate::databricks::conversion::_helpers::expressions::literals::{quoted_text, typed};
use crate::databricks::conversion::_helpers::functions::rewrites::Rewrite;
use crate::databricks::conversion::_helpers::functions::strings::strings;
use crate::databricks::conversion::constants::{
    GROUP_INDEX_ERROR, JAVA_DOT, JAVA_END, JAVA_SPACE, LAMBDA_ELEMENT, LINE_END_ERROR, MAX_GROUP,
    MAX_REPEAT, MAX_TREE_DEPTH, PATTERN_ERROR, REGEX_METACHARACTER_CLASS, REGEX_METACHARACTERS,
    REPLACEMENT_ERROR, SUPPLEMENTARY_CHARACTERS, SURROGATE_ERROR,
};
use crate::databricks::conversion::models::{DatabricksType, Element, Emitted, Literal};

/// The rewrite of the regular-expression function `name`, `None` when none handles it.
pub(crate) fn regex_rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "RLIKE" | "REGEXP" | "REGEXP_LIKE" => like,
        "REGEXP_REPLACE" => replace,
        "REGEXP_EXTRACT" => |values| extract(values, false),
        "REGEXP_EXTRACT_ALL" => |values| extract(values, true),
        "REGEXP_SUBSTR" => substr,
        "REGEXP_INSTR" => instr,
        "REGEXP_COUNT" => count,
        _ => return None,
    };
    Some(rewrite)
}

/// A pattern as RE2 reads it.
enum Pattern {
    /// Text without a metacharacter, which both engines find as it is.
    Text {
        /// The pattern's SQL.
        sql: String,
        /// Whether a constant is empty; `None` for a column, which raises for a metacharacter.
        empty: Option<bool>,
    },
    /// A constant pattern with metacharacters, written for RE2.
    Regex(Translated),
}

/// A constant Java pattern written for RE2, with what its uses depend on.
struct Translated {
    /// The RE2 pattern.
    re2: String,
    /// How many capturing groups it has.
    groups: usize,
    /// Whether it can match empty text, after which Java and RE2 step to the next match differently.
    nullable: bool,
    /// Whether it ends in `$`, whose RE2 form also takes the line terminator Java's leaves.
    anchored_end: bool,
}

/// A test that raises `error` before the function runs, where it holds.
type Raise = (String, String);

/// The pattern `value` as RE2 reads it, `None` for a constant outside the syntax shown to match alike.
fn pattern(value: &Emitted) -> Option<Pattern> {
    match &value.literal {
        Some(Literal::Text(literal))
            if !literal.contains(|character| REGEX_METACHARACTERS.contains(character)) =>
        {
            Some(Pattern::Text {
                sql: value.sql.clone(),
                empty: Some(literal.is_empty()),
            })
        }
        Some(Literal::Text(literal)) => translated(literal).map(Pattern::Regex),
        Some(Literal::Number(_)) => None,
        None => Some(Pattern::Text {
            sql: value.sql.clone(),
            empty: None,
        }),
    }
}

impl Pattern {
    /// The pattern's SQL.
    fn sql(&self) -> String {
        match self {
            Pattern::Text { sql, .. } => sql.clone(),
            Pattern::Regex(regex) => quoted_text(&regex.re2),
        }
    }

    /// Whether the pattern is found in `text`.
    fn found(&self, text: &str) -> String {
        match self {
            Pattern::Text { sql, .. } => format!("contains({text}, {sql})"),
            Pattern::Regex(regex) => {
                format!("regexp_matches({text}, {})", quoted_text(&regex.re2))
            }
        }
    }

    /// The tests that raise before a use over `text`, for `stepped` uses also an empty pattern over UTF-16 pairs.
    fn raises(&self, text: &str, stepped: bool) -> Vec<Raise> {
        let surrogate = format!("regexp_matches({text}, '{SUPPLEMENTARY_CHARACTERS}')");
        match self {
            Pattern::Regex(regex) => match regex.anchored_end {
                true => vec![(
                    format!("suffix({text}, chr(13) || chr(10))"),
                    LINE_END_ERROR.to_owned(),
                )],
                false => Vec::new(),
            },
            Pattern::Text { sql, empty } => {
                let mut tests: Vec<Raise> = Vec::new();
                if empty.is_none() {
                    tests.push((
                        format!("regexp_matches({sql}, '[{REGEX_METACHARACTER_CLASS}]')"),
                        PATTERN_ERROR.to_owned(),
                    ));
                }
                match (stepped, empty) {
                    (true, None) => tests.push((
                        format!("{sql} = '' AND {surrogate}"),
                        SURROGATE_ERROR.to_owned(),
                    )),
                    (true, Some(true)) => tests.push((surrogate, SURROGATE_ERROR.to_owned())),
                    _ => {}
                }
                tests
            }
        }
    }

    /// The capturing groups, none in text.
    fn groups(&self) -> usize {
        match self {
            Pattern::Text { .. } => 0,
            Pattern::Regex(regex) => regex.groups,
        }
    }

    /// Whether Java and RE2 step through the pattern's matches alike: text, or a regex that cannot match empty.
    fn steps_alike(&self) -> bool {
        match self {
            Pattern::Text { .. } => true,
            Pattern::Regex(regex) => !regex.nullable && !regex.anchored_end,
        }
    }

    /// Whether the first match is the same text in both engines.
    fn matches_alike(&self) -> bool {
        !matches!(
            self,
            Pattern::Regex(Translated {
                anchored_end: true,
                ..
            })
        )
    }
}

/// `WHEN ... THEN NULL ` for the values that are not literals, '' when all are.
fn null_when(values: &[&Emitted]) -> String {
    let mut nulls: Vec<String> = Vec::new();
    for value in values.iter().filter(|value| value.literal.is_none()) {
        let null = format!("{} IS NULL", value.sql);
        if !nulls.contains(&null) {
            nulls.push(null);
        }
    }
    match nulls.is_empty() {
        true => String::new(),
        false => format!("WHEN {} THEN NULL ", nulls.join(" OR ")),
    }
}

/// `sql`, NULL when any of `values` is and raising each test's error first where it holds.
fn guarded(values: &[&Emitted], tests: &[Raise], sql: String) -> String {
    if tests.is_empty() {
        return sql;
    }
    let raises: String = tests
        .iter()
        .map(|(test, error)| format!("WHEN {test} THEN error({}) ", quoted_text(error)))
        .collect();
    format!("(CASE {}{raises}ELSE {sql} END)", null_when(values))
}

/// `RLIKE(s, p)`: whether `p` matches anywhere in `s`.
fn like(values: &[Emitted]) -> Option<Emitted> {
    let [text, value] = strings::<2>(values)?;
    let pattern = pattern(value)?;
    let sql = guarded(
        &[text, value],
        &pattern.raises(&text.sql, false),
        pattern.found(&text.sql),
    );
    Some(typed(sql, DatabricksType::Boolean))
}

/// `REGEXP_REPLACE(s, p, r)`: every match replaced, `$n` in `r` naming a group.
fn replace(values: &[Emitted]) -> Option<Emitted> {
    let [text, value, with] = strings::<3>(values)?;
    let pattern = pattern(value)?;
    if !pattern.steps_alike() {
        return None;
    }
    let mut tests = pattern.raises(&text.sql, true);
    let s = &text.sql;
    let replaced =
        |replacement: &str| format!("regexp_replace({s}, {}, {replacement}, 'g')", pattern.sql());
    let sql = match &with.literal {
        Some(Literal::Text(literal)) => {
            replaced(&quoted_text(&replacement(literal, pattern.groups())?))
        }
        Some(Literal::Number(_)) => return None,
        None => {
            tests.push((
                format!(
                    "regexp_matches({}, '[$\\\\]') AND {}",
                    with.sql,
                    pattern.found(s)
                ),
                REPLACEMENT_ERROR.to_owned(),
            ));
            format!(
                "CASE WHEN regexp_matches({}, '[$\\\\]') THEN {s} ELSE {} END",
                with.sql,
                replaced(&with.sql)
            )
        }
    };
    Some(typed(
        guarded(&[text, value, with], &tests, sql),
        DatabricksType::String,
    ))
}

/// Java's replacement `text` as DuckDB's, `None` for a group it lacks or a reference DuckDB cannot write.
fn replacement(text: &str, groups: usize) -> Option<String> {
    if groups > MAX_GROUP {
        return None;
    }
    let mut written = String::new();
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        match character {
            '\\' => match characters.next()? {
                '\\' => written.push_str(r"\\"),
                escaped => written.push(escaped),
            },
            '$' => {
                let group = characters.next()?.to_digit(10)? as usize;
                if group > groups {
                    return None;
                }
                written.push('\\');
                written.push_str(&group.to_string());
            }
            character => written.push(character),
        }
    }
    Some(written)
}

/// `REGEXP_EXTRACT(s, p[, idx])`, or with `all` `REGEXP_EXTRACT_ALL`: group `idx` (default 1) of the first or each match.
fn extract(values: &[Emitted], all: bool) -> Option<Emitted> {
    let (text, value, index) = match values {
        [text, value] => (text, value, 1),
        [text, value, index] if index.data_type == DatabricksType::Int => match &index.literal {
            Some(Literal::Number(number)) => match number.parse::<i32>() {
                Ok(index) => (text, value, i64::from(index)),
                Err(_) => return None,
            },
            _ => return None,
        },
        _ => return None,
    };
    if text.data_type != DatabricksType::String || value.data_type != DatabricksType::String {
        return None;
    }
    let pattern = pattern(value)?;
    if all && !pattern.steps_alike() || !pattern.matches_alike() {
        return None;
    }
    let mut tests = pattern.raises(&text.sql, all && index == 0);
    let data_type = match all {
        true => DatabricksType::Array(Element::String),
        false => DatabricksType::String,
    };
    let sql = match usize::try_from(index) {
        Ok(index) if index <= pattern.groups() => {
            if index > MAX_GROUP {
                return None;
            }
            let extracted = format!(
                "regexp_extract{}({}, {}, {index})",
                if all { "_all" } else { "" },
                text.sql,
                pattern.sql()
            );
            match all && index > 0 {
                true => format!(
                    "list_transform({extracted}, lambda {LAMBDA_ELEMENT}: coalesce({LAMBDA_ELEMENT}, ''))"
                ),
                false => extracted,
            }
        }
        _ => {
            tests.push((
                pattern.found(&text.sql),
                format!("{ARGUMENT_REJECTED}: {GROUP_INDEX_ERROR}"),
            ));
            let none = match all {
                true => "CAST([] AS VARCHAR[])",
                false => "''",
            };
            format!("(CASE WHEN {} IS NULL THEN NULL ELSE {none} END)", text.sql)
        }
    };
    Some(typed(guarded(&[text, value], &tests, sql), data_type))
}

/// `REGEXP_SUBSTR(s, p)`: the first match, NULL when there is none or it is empty.
fn substr(values: &[Emitted]) -> Option<Emitted> {
    let [text, value] = strings::<2>(values)?;
    let pattern = pattern(value)?;
    if !pattern.matches_alike() {
        return None;
    }
    let sql = guarded(
        &[text, value],
        &pattern.raises(&text.sql, false),
        format!(
            "nullif(regexp_extract({}, {}, 0), '')",
            text.sql,
            pattern.sql()
        ),
    );
    Some(typed(sql, DatabricksType::String))
}

/// `REGEXP_INSTR(s, p)`: the 1-based character position where the first match starts, 0 if none.
fn instr(values: &[Emitted]) -> Option<Emitted> {
    let [text, value] = strings::<2>(values)?;
    let pattern = pattern(value)?;
    if !pattern.matches_alike() {
        return None;
    }
    let s = &text.sql;
    let before = match &pattern {
        Pattern::Text { sql, .. } => format!("left({s}, instr({s}, {sql}) - 1)"),
        Pattern::Regex(regex) => format!(
            "regexp_extract({s}, {}, 1)",
            quoted_text(&format!("^((?s:.*?))(?:{})", regex.re2))
        ),
    };
    let position = format!(
        "CAST(CASE {nulls}WHEN {found} THEN length({before}) + 1 ELSE 0 END AS INTEGER)",
        nulls = null_when(&[text, value]),
        found = pattern.found(s)
    );
    Some(typed(
        guarded(&[text, value], &pattern.raises(s, false), position),
        DatabricksType::Int,
    ))
}

/// `REGEXP_COUNT(s, p)`: how many matches Java finds, one more than the UTF-16 units for an empty pattern.
fn count(values: &[Emitted]) -> Option<Emitted> {
    let [text, value] = strings::<2>(values)?;
    let pattern = pattern(value)?;
    if !pattern.steps_alike() {
        return None;
    }
    let s = &text.sql;
    let counted = format!("len(regexp_extract_all({s}, {}, 0))", pattern.sql());
    let empty = format!("length({s}) + {} + 1", supplementary(s));
    let sql = match &pattern {
        Pattern::Text {
            empty: Some(true), ..
        } => empty,
        Pattern::Text { sql, empty: None } => {
            format!("CASE WHEN {sql} = '' THEN {empty} ELSE {counted} END")
        }
        _ => counted,
    };
    Some(typed(
        guarded(
            &[text, value],
            &pattern.raises(s, false),
            format!("CAST({sql} AS INTEGER)"),
        ),
        DatabricksType::Int,
    ))
}

/// How many characters of `text` lie above U+FFFF, the extra UTF-16 unit each takes.
fn supplementary(text: &str) -> String {
    format!(
        "length(regexp_replace({text}, '[^{}]', '', 'g'))",
        &SUPPLEMENTARY_CHARACTERS[1..SUPPLEMENTARY_CHARACTERS.len() - 1]
    )
}

/// The Java pattern `pattern` in RE2, `None` outside the syntax shown to match the same text.
fn translated(pattern: &str) -> Option<Translated> {
    let mut parser = Parser {
        characters: pattern.chars().collect(),
        at: 0,
        groups: 0,
        depth: 0,
        anchored_end: false,
    };
    let (re2, nullable) = parser.alternation()?;
    (parser.at == parser.characters.len()).then_some(Translated {
        re2,
        groups: parser.groups,
        nullable,
        anchored_end: parser.anchored_end,
    })
}

/// A reader of a Java pattern, writing RE2 as it goes.
struct Parser {
    /// The pattern's characters.
    characters: Vec<char>,
    /// The next character's place.
    at: usize,
    /// The capturing groups read so far.
    groups: usize,
    /// How deep the groups nest here.
    depth: usize,
    /// Whether the pattern ended in `$`.
    anchored_end: bool,
}

impl Parser {
    /// The character `offset` places ahead.
    fn peek(&self, offset: usize) -> Option<char> {
        self.characters.get(self.at + offset).copied()
    }

    /// The next character, consumed.
    fn next(&mut self) -> Option<char> {
        let character = self.peek(0)?;
        self.at += 1;
        Some(character)
    }

    /// Branches separated by `|`, and whether any can match empty text.
    fn alternation(&mut self) -> Option<(String, bool)> {
        let (mut written, mut nullable) = self.sequence()?;
        while self.peek(0) == Some('|') {
            self.at += 1;
            let (branch, empty) = self.sequence()?;
            written = format!("{written}|{branch}");
            nullable |= empty;
        }
        Some((written, nullable))
    }

    /// Quantified atoms up to `|`, `)` or the end, and whether they can all match empty text.
    fn sequence(&mut self) -> Option<(String, bool)> {
        let mut written = String::new();
        let mut nullable = true;
        while let Some(character) = self.peek(0) {
            match character {
                '|' | ')' => break,
                '$' if self.depth == 0 && self.at + 1 == self.characters.len() => {
                    self.at += 1;
                    self.anchored_end = true;
                    written.push_str(JAVA_END);
                }
                _ => {
                    let (atom, empty) = self.atom()?;
                    let (quantifier, empty) = self.quantifier(empty)?;
                    written.push_str(&atom);
                    written.push_str(&quantifier);
                    nullable &= empty;
                }
            }
        }
        Some((written, nullable))
    }

    /// One atom, and whether it can match empty text.
    fn atom(&mut self) -> Option<(String, bool)> {
        match self.next()? {
            '(' => self.group(),
            '[' => Some((self.class()?, false)),
            '.' => Some((JAVA_DOT.to_owned(), false)),
            '^' => Some(("^".to_owned(), true)),
            '\\' => Some((self.escape(false)?, false)),
            character if REGEX_METACHARACTERS.contains(character) => None,
            character => Some((character.to_string(), false)),
        }
    }

    /// A group after its `(`: capturing, or non-capturing with `?:`; other `(?` constructs are refused.
    fn group(&mut self) -> Option<(String, bool)> {
        let capturing = match self.peek(0) {
            Some('?') => {
                self.at += 1;
                (self.next()? == ':').then_some(false)?
            }
            _ => {
                self.groups += 1;
                true
            }
        };
        self.depth += 1;
        if self.depth > MAX_TREE_DEPTH {
            return None;
        }
        let (inner, nullable) = self.alternation()?;
        self.depth -= 1;
        (self.next()? == ')').then_some(())?;
        let opening = if capturing { "(" } else { "(?:" };
        Some((format!("{opening}{inner})"), nullable))
    }

    /// A greedy or lazy quantifier after an atom and whether the result can match empty; possessive ones are refused.
    fn quantifier(&mut self, atom_nullable: bool) -> Option<(String, bool)> {
        let (mut written, nullable) = match self.peek(0) {
            Some(character @ ('*' | '+' | '?')) => {
                self.at += 1;
                (character.to_string(), character != '+')
            }
            Some('{') => {
                self.at += 1;
                let (written, least) = self.bounds()?;
                (written, least == 0)
            }
            _ => return Some((String::new(), atom_nullable)),
        };
        if atom_nullable {
            return None;
        }
        match self.peek(0) {
            Some('?') => {
                self.at += 1;
                written.push('?');
            }
            Some('+') => return None,
            _ => {}
        }
        Some((written, nullable))
    }

    /// `{n}`, `{n,}` or `{n,m}` after its `{`, with its least count.
    fn bounds(&mut self) -> Option<(String, u32)> {
        let least = self.number()?;
        match self.next()? {
            '}' => Some((format!("{{{least}}}"), least)),
            ',' if self.peek(0) == Some('}') => {
                self.at += 1;
                Some((format!("{{{least},}}"), least))
            }
            ',' => {
                let most = self.number()?;
                (self.next()? == '}' && most >= least)
                    .then(|| (format!("{{{least},{most}}}"), least))
            }
            _ => None,
        }
    }

    /// A decimal count of at most [`MAX_REPEAT`].
    fn number(&mut self) -> Option<u32> {
        let start = self.at;
        let mut number: u32 = 0;
        while let Some(digit) = self.peek(0).and_then(|character| character.to_digit(10)) {
            self.at += 1;
            number = number * 10 + digit;
            if number > MAX_REPEAT {
                return None;
            }
        }
        (self.at > start).then_some(number)
    }

    /// A class after its `[`: characters, ranges and `\d`, `\w`, `\s`, without nesting or intersection.
    fn class(&mut self) -> Option<String> {
        let negated = self.peek(0) == Some('^');
        if negated {
            self.at += 1;
        }
        let mut body = String::new();
        let mut first = true;
        loop {
            match self.next()? {
                ']' if !first => break,
                '[' | ']' => return None,
                '&' if self.peek(0) == Some('&') => return None,
                '-' if first || self.peek(0) == Some(']') => body.push_str(r"\-"),
                '-' => return None,
                '\\' => body.push_str(&self.escape(true)?),
                low if self.peek(0) == Some('-') && self.peek(1) != Some(']') => {
                    self.at += 1;
                    let high = self.next()?;
                    if matches!(high, '\\' | '[' | ']') || high < low {
                        return None;
                    }
                    body.push_str(&format!("{}-{}", member(low), member(high)));
                }
                character => body.push_str(&member(character)),
            }
            first = false;
        }
        Some(format!("[{}{body}]", if negated { "^" } else { "" }))
    }

    /// An escape after its `\`, inside a class when `in_class`; `None` for one whose Java meaning RE2 lacks.
    fn escape(&mut self, in_class: bool) -> Option<String> {
        let escaped = match self.next()? {
            character @ ('d' | 'w' | 't' | 'n' | 'r' | 'f') => format!("\\{character}"),
            character @ ('D' | 'W') if !in_class => format!("\\{character}"),
            's' if in_class => JAVA_SPACE.to_owned(),
            's' => format!("[{JAVA_SPACE}]"),
            'S' if !in_class => format!("[^{JAVA_SPACE}]"),
            character if character.is_ascii_punctuation() && in_class => member(character),
            character if REGEX_METACHARACTERS.contains(character) => format!("\\{character}"),
            character if character.is_ascii_punctuation() => character.to_string(),
            _ => return None,
        };
        Some(escaped)
    }
}

/// `character` matched as itself inside a class.
fn member(character: char) -> String {
    match character {
        '\\' | ']' | '[' | '^' | '-' => format!("\\{character}"),
        character => character.to_string(),
    }
}
