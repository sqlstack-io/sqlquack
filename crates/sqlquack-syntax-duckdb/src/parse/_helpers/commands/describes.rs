//! `DescribeStatement` (describe.gram): `DESCRIBE`, `SUMMARIZE` and `SHOW` as a query's atom.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Null};
use sqlquack_syntax::expressions::statements::models::{Describe, Show};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_identifier};
use crate::parse::_helpers::tree::nodes::table;

/// The statement names the transformer's refusals use for each form.
const DESCRIBE: &str = "DESCRIBE";
const SUMMARY: &str = "SUMMARY";
const SHOW: &str = "SHOW";
const SHOW_FROM: &str = "SHOW_FROM";
const SHOW_SPECIAL: &str = "SHOW_SPECIAL";
/// `SUMMARIZE` as the tree's describe style.
const SUMMARIZE: &str = "SUMMARIZE";
/// Names a one-part target turns into a listing, as `SHOW TABLES` does.
const SPECIAL_NAMES: [&str; 4] = ["databases", "schemas", "tables", "variables"];
/// `TABLES FROM` names a database, a schema, or both.
const SHOW_FROM_MAX_PARTS: usize = 2;
const NO_SUMMARIZE_TARGET: &str = "Expected table name with SUMMARIZE";
/// The transformer's refusal of `SHOW` of an empty string.
const SHOW_EMPTY_NAME: &str = "Expected a table name, but got an empty string";
/// The `Show` node's `this` for each listing and lookup.
const ALL_TABLES: &str = "ALL TABLES";
const TABLES: &str = "TABLES";
const SETTING: &str = "SETTING";
const TABLE: &str = "TABLE";
const QUERY: &str = "QUERY";
const SHOW_FROM_PARTS: &str = "Expected \"SHOW TABLES FROM database\", \"SHOW TABLES FROM schema\", or \"SHOW TABLES FROM database.schema\"";

impl Parser<'_> {
    /// `DescribeStatement`: the atom, and the statement name its refusals use.
    pub(crate) fn describe_statement(&mut self) -> Parsing<(Expression, &'static str)> {
        let start: usize = self.offset();
        let word: String = self.word_at(0).unwrap_or_default();
        self.at += 1;
        let (label, style): (&'static str, Option<String>) = match word.as_str() {
            "SUMMARIZE" => (SUMMARY, Some(SUMMARIZE.to_owned())),
            "SHOW" => (SHOW, None),
            _ => (DESCRIBE, None),
        };
        if label != SUMMARY && self.kw("TABLES") && self.kw_at(1, "FROM") {
            self.at += 2;
            let parts: Vec<Identifier> = self.qualified_name()?;
            if parts.len() > SHOW_FROM_MAX_PARTS {
                self.refuse_later(SHOW_FROM_PARTS, start);
            }
            let mut node: Show = Show::new(TABLES);
            node.from = Some(Expression::Table(Box::new(table(parts))));
            return Ok((show(node), SHOW_FROM));
        }
        if self.starts_query_at(0) {
            let query: Expression = self.query()?;
            return Ok(match label {
                SHOW => (show_of(QUERY, query), SHOW),
                _ => (describe(query, style), label),
            });
        }
        if label != SUMMARY && self.kw("ALL") {
            self.at += 1;
            self.eat_kw("TABLES");
            return Ok((show(Show::new(ALL_TABLES)), SHOW_SPECIAL));
        }
        let target: Option<(Expression, Option<String>)> = self.describe_target(label == SHOW)?;
        match target {
            None if label == SUMMARY => {
                self.refuse_later(NO_SUMMARIZE_TARGET, start);
                Ok((Expression::Null(Null), SUMMARY))
            }
            None => Ok((show(Show::new(ALL_TABLES)), SHOW_SPECIAL)),
            Some((_, Some(special))) => Ok((show(Show::new(special)), SHOW_SPECIAL)),
            Some((target, None)) if label == SHOW => {
                Ok((show_of(show_kind(&target), target), SHOW))
            }
            Some((target, None)) => Ok((describe(target, style), label)),
        }
    }

    /// `DescribeTarget` and the listing it names; after `SHOW` a string never names a listing.
    fn describe_target(&mut self, show: bool) -> Parsing<Option<(Expression, Option<String>)>> {
        let literal: bool = show && self.string_at(0);
        let position: usize = self.offset();
        let parts: Vec<Identifier> = if self.string_at(0) {
            let name: Identifier = self.col_id_or_string()?;
            if literal && name.name.is_empty() {
                self.refuse_later(SHOW_EMPTY_NAME, position);
            }
            vec![name]
        } else if self.is(is_identifier) {
            let mut parts: Vec<Identifier> = vec![self.name(is_identifier)?];
            while self.is_char('.') && self.is_at(1, is_col_label) {
                self.at += 1;
                parts.push(self.name(is_col_label)?);
            }
            parts.extend(self.quoted_table_part()?);
            parts
        } else {
            return Ok(None);
        };
        let special: Option<String> = match parts.as_slice() {
            [name]
                if !literal && SPECIAL_NAMES.contains(&name.name.to_ascii_lowercase().as_str()) =>
            {
                Some(name.name.to_ascii_uppercase())
            }
            _ => None,
        };
        Ok(Some((Expression::Table(Box::new(table(parts))), special)))
    }
}

/// `SHOW name` looks a one-part name up as a setting first; a qualified name is a table.
fn show_kind(target: &Expression) -> &'static str {
    match target {
        Expression::Table(table) if table.schema.is_none() && table.catalog.is_none() => SETTING,
        _ => TABLE,
    }
}

/// `SHOW` of a setting, table or query, held in `target`.
fn show_of(kind: &str, target: Expression) -> Expression {
    let mut node: Show = Show::new(kind);
    node.target = Some(target);
    show(node)
}

fn show(node: Show) -> Expression {
    Expression::Show(Box::new(node))
}

/// `DESCRIBE target`, `SUMMARIZE` when `style` says so.
fn describe(target: Expression, style: Option<String>) -> Expression {
    let mut node: Describe = Describe::new(target);
    node.style = style;
    Expression::Describe(Box::new(node))
}
