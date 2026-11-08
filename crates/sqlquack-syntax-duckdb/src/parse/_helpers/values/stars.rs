//! A star's `EXCLUDE`, `REPLACE` and `RENAME` lists (expression.gram), with the transformer's checks.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Star, StarParts};
use sqlquack_syntax::expressions::operators::models::Alias;

use crate::keywords::main::classify::classify;
use crate::keywords::models::KeywordClasses;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_identifier};
use crate::parse::_helpers::tree::nodes::{joined, star};

/// A column name has at most `catalog.schema.table.column`.
const MAX_COLUMN_PARTS: usize = 4;
const EXCLUDE_LIST: &str = "EXCLUDE";
const REPLACE_LIST: &str = "REPLACE";
const RENAME_LIST: &str = "RENAME";

/// Whether two dotted names share a column, without case, and differ on no qualifier both give.
fn same_column(left: &[Identifier], right: &[Identifier]) -> bool {
    let pairs = left.iter().rev().zip(right.iter().rev());
    pairs.fold(true, |same, (a, b)| {
        same && a.name.eq_ignore_ascii_case(&b.name)
    })
}

/// A name part as the transformer prints it: bare when it reads back as the same identifier.
fn printed_part(name: &str) -> String {
    let bytes: &[u8] = name.as_bytes();
    let decimal_after_dot: bool = bytes.len() > 1 && bytes[0] == b'_' && bytes[1].is_ascii_digit();
    let mut plain: bool = !bytes.is_empty() && !decimal_after_dot;
    for (at, byte) in bytes.iter().enumerate() {
        plain &= byte.is_ascii_alphabetic() || *byte == b'_' || (at > 0 && byte.is_ascii_digit());
    }
    let classes: KeywordClasses = classify(name);
    let keyword: bool = classes.reserved
        || classes.unreserved
        || classes.column_name
        || classes.func_name
        || classes.type_name;
    match plain && !keyword {
        true => name.to_owned(),
        false => quoted(name),
    }
}

/// `name` in double quotes, its quotes doubled.
fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// A dotted name as the transformer prints it.
fn printed(parts: &[Identifier]) -> String {
    let printed: Vec<String> = parts.iter().map(|part| printed_part(&part.name)).collect();
    printed.join(".")
}

/// The refusal of a column both of two lists name.
fn overlap(column: &str, first: &str, second: &str) -> String {
    format!("Column \"{column}\" cannot occur in both {first} and {second} list")
}

impl Parser<'_> {
    /// `ExcludeName`: a dotted name or a string, as its parts.
    fn exclude_name(&mut self) -> Parsing<Vec<Identifier>> {
        if self.string_at(0) {
            return Ok(vec![self.col_id_or_string()?]);
        }
        let start: usize = self.offset();
        let mut parts: Vec<Identifier> = vec![self.name(is_identifier)?];
        while self.is_char('.') && self.is_at(1, is_col_label) {
            self.at += 1;
            parts.push(self.name(is_col_label)?);
        }
        if parts.len() > MAX_COLUMN_PARTS {
            let names: Vec<&str> = parts.iter().map(|part| part.name.as_str()).collect();
            let message: String = format!(
                "Expected at most {MAX_COLUMN_PARTS} entries (catalog.schema.table.column), but found {} entries (input: {})",
                parts.len(),
                names.join(".")
            );
            self.refuse_later(&message, start);
        }
        Ok(parts)
    }

    /// One entry, or a parenthesized list of them.
    fn entries<T>(&mut self, entry: impl FnMut(&mut Self) -> Parsing<T>) -> Parsing<Vec<T>> {
        let mut entry = entry;
        match self.is_char('(') {
            true => self.parenthesized(entry),
            false => Ok(vec![entry(self)?]),
        }
    }

    /// `* [EXCLUDE ...] [REPLACE ...] [RENAME ...]` after the star.
    pub(crate) fn star_modifiers(&mut self, table: Option<Identifier>) -> Parsing<Expression> {
        let start: usize = self.offset();
        let mut node: Star = star(table, None);
        let mut excludes: Vec<Vec<Identifier>> = Vec::new();
        let has_excludes: bool = self.kw_in(0, &["EXCLUDE", "EXCEPT"])
            && (self.char_at(1, '(') || self.is_at(1, is_identifier) || self.string_at(1));
        if has_excludes {
            self.at += 1;
            excludes = self.entries(|parser| parser.exclude_name())?;
            self.refuse_duplicate_excludes(&excludes, start);
            node.except = Some(excludes.iter().cloned().map(joined).collect());
            if excludes.iter().any(|parts| parts.len() > 1) {
                node.qualified.get_or_insert_with(star_parts).except = excludes.clone();
            }
        }
        let mut replaces: Vec<Alias> = Vec::new();
        if self.kw("REPLACE") && (self.char_at(1, '(') || !self.terminal_at(1)) {
            self.at += 1;
            replaces = self.entries(|parser| {
                let this: Expression = parser.expression()?;
                parser.expect_kw("AS")?;
                let name: Identifier = parser.name(is_col_label)?;
                Ok(Alias::new(this, name))
            })?;
            self.refuse_duplicate_replaces(&replaces, start);
            node.replace = Some(replaces.clone());
        }
        let mut renames: Vec<Vec<Identifier>> = Vec::new();
        if self.kw("RENAME") && (self.char_at(1, '(') || !self.terminal_at(1)) {
            self.at += 1;
            let pairs: Vec<(Vec<Identifier>, Identifier)> = self.entries(|parser| {
                let from: Vec<Identifier> = parser.exclude_name()?;
                parser.expect_kw("AS")?;
                let to: Identifier = parser.name(is_identifier)?;
                Ok((from, to))
            })?;
            renames = pairs.iter().map(|(from, _)| from.clone()).collect();
            if renames.iter().any(|parts| parts.len() > 1) {
                node.qualified.get_or_insert_with(star_parts).rename = renames.clone();
            }
            node.rename = Some(
                pairs
                    .into_iter()
                    .map(|(from, to)| (joined(from), to))
                    .collect(),
            );
        }
        self.refuse_overlaps(&excludes, &replaces, &renames, start);
        Ok(Expression::Star(node))
    }

    /// `Duplicate entry` of a later exclude naming an earlier one's column.
    fn refuse_duplicate_excludes(&mut self, excludes: &[Vec<Identifier>], start: usize) {
        for (at, later) in excludes.iter().enumerate() {
            if excludes[..at]
                .iter()
                .any(|earlier| same_column(earlier, later))
            {
                let message: String = format!(
                    "Duplicate entry \"{}\" in {EXCLUDE_LIST} list",
                    printed(later)
                );
                self.refuse_later(&message, start);
                return;
            }
        }
    }

    /// `Duplicate entry` of a later replacement reusing an earlier one's name.
    fn refuse_duplicate_replaces(&mut self, replaces: &[Alias], start: usize) {
        for (at, later) in replaces.iter().enumerate() {
            let name: &str = later.alias.name.as_str();
            if replaces[..at]
                .iter()
                .any(|earlier| earlier.alias.name.eq_ignore_ascii_case(name))
            {
                let message: String = format!("Duplicate entry \"{name}\" in {REPLACE_LIST} list");
                self.refuse_later(&message, start);
                return;
            }
        }
    }

    /// The transformer's refusals of a column two of the lists name, in its order.
    fn refuse_overlaps(
        &mut self,
        excludes: &[Vec<Identifier>],
        replaces: &[Alias],
        renames: &[Vec<Identifier>],
        start: usize,
    ) {
        for replace in replaces {
            let name: &[Identifier] = std::slice::from_ref(&replace.alias);
            if excludes.iter().any(|exclude| same_column(exclude, name)) {
                let column: String = quoted(&replace.alias.name);
                self.refuse_later(&overlap(&column, EXCLUDE_LIST, REPLACE_LIST), start);
                return;
            }
        }
        for rename in renames {
            if excludes.iter().any(|exclude| same_column(exclude, rename)) {
                let message: String = overlap(&printed(rename), EXCLUDE_LIST, RENAME_LIST);
                self.refuse_later(&message, start);
                return;
            }
            let column: &[Identifier] = &rename[rename.len() - 1..];
            if replaces
                .iter()
                .any(|replace| same_column(std::slice::from_ref(&replace.alias), column))
            {
                let message: String = overlap(&printed(rename), REPLACE_LIST, RENAME_LIST);
                self.refuse_later(&message, start);
                return;
            }
        }
    }
}

/// An empty set of qualified star entries.
fn star_parts() -> Box<StarParts> {
    Box::new(StarParts {
        except: Vec::new(),
        rename: Vec::new(),
    })
}
