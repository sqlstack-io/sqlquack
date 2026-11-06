//! Lambda bodies in printed SQL, and the words they read, outside literals and quoted names.

use std::ops::Range;

use crate::checking::constants::LAMBDA_KEYWORD;

/// One lambda body: its byte range and the parameters it binds.
pub(crate) struct Body {
    pub(crate) range: Range<usize>,
    pub(crate) parameters: Vec<String>,
}

/// The lambda bodies in `sql` holding byte `at`.
pub(crate) fn bodies_around(bodies: &[Body], at: usize) -> Vec<&Body> {
    bodies
        .iter()
        .filter(|body| body.range.contains(&at))
        .collect()
}

/// Whether `name` stands as a whole word in `sql`, outside string literals and quoted identifiers.
pub(crate) fn reads_word(sql: &str, name: &str) -> bool {
    let quoted = quoted_ranges(sql);
    sql.match_indices(name)
        .any(|(start, _)| whole_word(sql, start..start + name.len(), &quoted))
}

/// Whether `range` of `sql` is a whole word outside every range in `quoted`.
fn whole_word(sql: &str, range: Range<usize>, quoted: &[Range<usize>]) -> bool {
    let word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let bytes = sql.as_bytes();
    !bytes[..range.start].last().copied().is_some_and(word)
        && !bytes.get(range.end).copied().is_some_and(word)
        && !quoted.iter().any(|span| span.contains(&range.start))
}

/// The byte ranges of string literals and quoted identifiers in `sql`.
fn quoted_ranges(sql: &str) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut open: Option<(u8, usize)> = None;
    for (index, byte) in sql.bytes().enumerate() {
        match (open, byte) {
            (Some((quote, start)), _) if byte == quote => {
                ranges.push(start..index + 1);
                open = None;
            }
            (None, b'\'' | b'"') => open = Some((byte, index)),
            _ => {}
        }
    }
    ranges
}

/// The lambda bodies in `sql`, outside string literals and quoted identifiers.
pub(crate) fn lambda_bodies(sql: &str) -> Vec<Body> {
    let bytes = sql.as_bytes();
    let mut scan = Scan::default();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        match (scan.quote, byte) {
            (Some(open), _) => scan.quote = (byte != open).then_some(open),
            (None, b'\'' | b'"') => scan.quote = Some(byte),
            (None, b'(' | b'[' | b'{') => scan.depth += 1,
            (None, b')' | b']' | b'}') => {
                scan.close(index);
                scan.depth = scan.depth.saturating_sub(1);
            }
            (None, b',') if scan.pending.is_none() => scan.close(index),
            (None, b':') if scan.pending.is_some_and(|(depth, _)| depth == scan.depth) => {
                scan.open_body(sql, index);
            }
            (None, _) if keyword_at(sql, index) => {
                index += LAMBDA_KEYWORD.len();
                scan.pending = Some((scan.depth, index));
                continue;
            }
            (None, _) => {}
        }
        index += 1;
    }
    scan.close_all(bytes.len());
    scan.bodies
}

/// Whether the lambda keyword starts at byte `at` of `sql`, as a word.
fn keyword_at(sql: &str, at: usize) -> bool {
    sql[at..].starts_with(LAMBDA_KEYWORD)
        && !sql[..at]
            .bytes()
            .next_back()
            .is_some_and(|before| before.is_ascii_alphanumeric() || before == b'_')
}

/// The state of a scan for lambda bodies.
#[derive(Default)]
struct Scan {
    /// The quote character of the literal or identifier the scan is in.
    quote: Option<u8>,
    /// How many brackets are open.
    depth: usize,
    /// The depth of a lambda whose parameters' colon is still to come, and where they start.
    pending: Option<(usize, usize)>,
    /// The depth, start and parameters of each body still open, innermost last.
    open: Vec<(usize, usize, Vec<String>)>,
    bodies: Vec<Body>,
}

impl Scan {
    /// Opens the body of the pending lambda after its colon at byte `colon` of `sql`.
    fn open_body(&mut self, sql: &str, colon: usize) {
        if let Some((depth, start)) = self.pending.take() {
            let parameters: Vec<String> = sql[start..colon]
                .split(',')
                .map(|name| name.trim().to_owned())
                .collect();
            self.open.push((depth, colon + 1, parameters));
        }
    }

    /// Ends the bodies open at the current depth, at byte `end`.
    fn close(&mut self, end: usize) {
        while self
            .open
            .last()
            .is_some_and(|(depth, _, _)| *depth == self.depth)
        {
            self.end_innermost(end);
        }
    }

    /// Ends every body still open, at byte `end`.
    fn close_all(&mut self, end: usize) {
        while !self.open.is_empty() {
            self.end_innermost(end);
        }
    }

    /// Ends the innermost open body at byte `end`.
    fn end_innermost(&mut self, end: usize) {
        if let Some((_, start, parameters)) = self.open.pop() {
            self.bodies.push(Body {
                range: start..end,
                parameters,
            });
        }
    }
}
