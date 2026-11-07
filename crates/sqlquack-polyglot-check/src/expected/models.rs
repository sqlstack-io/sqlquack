//! Expected differences.

use crate::expected::_helpers::nodes::{
    differ_only_by_kept_all, differ_only_by_kept_field, differ_only_by_kept_node,
    differ_only_by_structured_calls, is_opaque,
};
use crate::expected::_helpers::quoting::{compact, differ_only_by_kept_quoting};
use crate::expected::constants::{ERROR_PREFIX, OK_PREFIX};
use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

/// The expected-differences file.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedFile {
    /// One entry per spelling.
    #[serde(default, rename = "spelling")]
    pub spellings: Vec<ExpectedSpelling>,
    /// One entry per reviewed parser fix.
    #[serde(default, rename = "fix")]
    pub fixes: Vec<ParserFix>,
    /// One entry per reviewed statement SQLQuack rejects where polyglot parses it.
    #[serde(default, rename = "rejection")]
    pub rejections: Vec<ParserRejection>,
    /// One entry per reviewed name whose quoting polyglot drops and SQLQuack keeps.
    #[serde(default, rename = "kept_quoting")]
    pub kept_quotings: Vec<KeptQuoting>,
    /// One entry per reviewed wrapper node SQLQuack keeps and polyglot drops.
    #[serde(default, rename = "kept_node")]
    pub kept_nodes: Vec<KeptNode>,
    /// One entry per reviewed field SQLQuack keeps as written where polyglot drops or infers it.
    #[serde(default, rename = "kept_field")]
    pub kept_fields: Vec<KeptField>,
    /// One entry per reviewed clause whose ALL SQLQuack keeps apart where polyglot reads it as a key.
    #[serde(default, rename = "kept_all")]
    pub kept_alls: Vec<KeptAll>,
    /// One entry per reviewed statement polyglot keeps as a command and SQLQuack structures.
    #[serde(default, rename = "structured")]
    pub structured: Vec<StructuredStatement>,
    /// One entry per reviewed call polyglot keeps as a function and SQLQuack parses into a node.
    #[serde(default, rename = "structured_call")]
    pub structured_calls: Vec<StructuredCall>,
}

/// A call polyglot keeps as a function, such as `ROLLUP(...)`, that SQLQuack parses into its own node.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredCall {
    /// A lower-case identifier.
    pub id: String,
    /// Text the statement contains, with whitespace removed and ignoring case, such as `ROLLUP(`.
    pub marker: String,
    /// SQLQuack's node, such as `rollup`; its only field is `expressions`, polyglot's arguments.
    pub sqlquack_node: String,
    /// The function name polyglot gives the call, such as `ROLLUP`.
    pub call: String,
    /// Why SQLQuack's node is right; a test in sqlquack-syntax pins its tree.
    pub note: String,
}

/// A statement polyglot keeps as an opaque command or raw SQL and SQLQuack parses into a structured node.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredStatement {
    /// A lower-case identifier.
    pub id: String,
    /// Text the statement contains, with whitespace removed and ignoring case, such as `BEGIN`.
    pub marker: String,
    /// The node SQLQuack's tree starts with, such as `scripting_block`.
    pub sqlquack_node: String,
    /// Why SQLQuack's structure is right; a test in sqlquack-syntax pins its tree.
    pub note: String,
}

/// A statement both parsers parse, where polyglot drops the quoting of names SQLQuack keeps quoted.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeptQuoting {
    /// A lower-case identifier.
    pub id: String,
    /// Text the statement contains, with whitespace removed and ignoring case, such as `EXCLUDE("`.
    pub marker: String,
    /// Why SQLQuack's quoting is right; the trees may differ in nothing else.
    pub note: String,
}

/// A statement both parsers parse, where SQLQuack keeps a clause's ALL apart and polyglot reads it as a key.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeptAll {
    /// A lower-case identifier.
    pub id: String,
    /// Text the statement contains, with whitespace removed and ignoring case, such as `ORDERBYALL`.
    pub marker: String,
    /// The clause holding the ALL, such as `order_by`.
    pub clause: String,
    /// Why SQLQuack keeps ALL apart; folded into polyglot's key the trees may differ in nothing.
    pub note: String,
}

/// A statement both parsers parse, where SQLQuack keeps a wrapper node, such as unary plus, polyglot drops.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeptNode {
    /// A lower-case identifier.
    pub id: String,
    /// Text the statement contains, with whitespace removed and ignoring case, such as `+`.
    pub marker: String,
    /// The wrapper node SQLQuack keeps, such as `pos`; its only field is `this`.
    pub node: String,
    /// Why SQLQuack keeps it; without its wrappers the trees may differ in nothing.
    pub note: String,
}

/// A statement both parsers parse, where SQLQuack keeps a field as written, such as a list polyglot joins into one name.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeptField {
    /// A lower-case identifier.
    pub id: String,
    /// Text the statement contains, with whitespace removed and ignoring case, such as `SECONDARYROLES`.
    pub marker: String,
    /// The field, such as `roles`; without it in either tree the trees may differ in nothing.
    pub field: String,
    /// Why SQLQuack's field is right; a test in sqlquack-syntax pins its tree.
    pub note: String,
}

/// A statement SQLQuack rejects where polyglot parses it or fails differently, recognised by its exact text and dialect.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserRejection {
    /// A lower-case identifier.
    pub id: String,
    /// The exact statement.
    pub sql: String,
    /// The dialects it is rejected in, such as `DuckDB`.
    pub dialects: Vec<String>,
    /// Text SQLQuack's error contains.
    pub sqlquack_error: String,
    /// Why SQLQuack's error is right; a test in sqlquack-syntax pins it.
    pub note: String,
}

/// Valid Snowflake polyglot rejects and SQLQuack parses, recognised by its text and polyglot's error.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserFix {
    /// A lower-case identifier.
    pub id: String,
    /// Text the statement contains, with whitespace removed and ignoring case, such as `]:`.
    pub marker: String,
    /// Polyglot's errors on it; its error contains one of them.
    pub polyglot_errors: Vec<String>,
    /// Why SQLQuack's parse is right; a test in sqlquack-syntax pins its tree.
    pub note: String,
}

/// A call spelling SQLQuack keeps as written, and what polyglot makes of it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedSpelling {
    /// The upper-case call name.
    pub name: String,
    /// The nodes polyglot lowers it to, by arity; `error` where polyglot rejects the call.
    pub polyglot: Vec<String>,
    /// Why keeping the call changes spelling only.
    pub note: String,
}

/// The allowed polyglot nodes for each spelling.
#[derive(Clone, Debug, Default)]
pub struct ExpectedDifferences {
    /// Upper-case name to the polyglot nodes allowed for it.
    pub allowed: BTreeMap<String, BTreeSet<String>>,
    /// The reviewed parser fixes.
    pub fixes: Vec<ParserFix>,
    /// The reviewed rejections.
    pub rejections: Vec<ParserRejection>,
    /// The reviewed names whose quoting polyglot drops.
    pub kept_quotings: Vec<KeptQuoting>,
    /// The reviewed wrapper nodes SQLQuack keeps and polyglot drops.
    pub kept_nodes: Vec<KeptNode>,
    /// The reviewed fields SQLQuack keeps as written where polyglot drops or infers them.
    pub kept_fields: Vec<KeptField>,
    /// The reviewed clauses whose ALL SQLQuack keeps apart where polyglot reads it as a key.
    pub kept_alls: Vec<KeptAll>,
    /// The reviewed statements polyglot keeps as a command and SQLQuack structures.
    pub structured: Vec<StructuredStatement>,
    /// The reviewed calls polyglot keeps as functions and SQLQuack parses into nodes.
    pub structured_calls: Vec<StructuredCall>,
}

impl ParserFix {
    /// Whether polyglot's rendered error is one this fix lists.
    fn names_error(&self, polyglot: &str) -> bool {
        self.polyglot_errors
            .iter()
            .any(|error| polyglot.contains(error.as_str()))
    }
}

impl ExpectedDifferences {
    /// The fix explaining polyglot's error `polyglot` on `sql` where SQLQuack parsed it.
    pub fn fix_for(&self, sql: &str, polyglot: &str) -> Option<&ParserFix> {
        let compact = compact(sql);
        self.fixes.iter().find(|fix| {
            polyglot.starts_with(ERROR_PREFIX)
                && fix.names_error(polyglot)
                && compact.contains(&fix.marker.to_ascii_uppercase())
        })
    }

    /// The rejection explaining SQLQuack's error `sqlquack` on `sql` in `dialect`, whatever polyglot made of it.
    pub fn rejection_for(
        &self,
        sql: &str,
        dialect: &str,
        sqlquack: &str,
    ) -> Option<&ParserRejection> {
        self.rejections.iter().find(|rejection| {
            sqlquack.starts_with(ERROR_PREFIX)
                && sqlquack.contains(&rejection.sqlquack_error)
                && rejection.sql == sql
                && rejection.dialects.contains(&dialect.to_owned())
        })
    }

    /// The entry explaining where polyglot drops the quoting of a name SQLQuack keeps quoted.
    pub fn kept_quoting_for(
        &self,
        sql: &str,
        polyglot: &str,
        sqlquack: &str,
    ) -> Option<&KeptQuoting> {
        let compact = compact(sql);
        self.kept_quotings
            .iter()
            .find(|entry| compact.contains(&entry.marker.to_ascii_uppercase()))
            .filter(|_| differ_only_by_kept_quoting(polyglot, sqlquack))
    }

    /// The entry explaining where SQLQuack keeps a wrapper node polyglot drops.
    pub fn kept_node_for(&self, sql: &str, polyglot: &str, sqlquack: &str) -> Option<&KeptNode> {
        let compact = compact(sql);
        self.kept_nodes.iter().find(|entry| {
            compact.contains(&entry.marker.to_ascii_uppercase())
                && differ_only_by_kept_node(polyglot, sqlquack, &entry.node)
        })
    }

    /// The entry explaining where SQLQuack keeps a field as written that polyglot drops or infers.
    pub fn kept_field_for(&self, sql: &str, polyglot: &str, sqlquack: &str) -> Option<&KeptField> {
        let compact = compact(sql);
        self.kept_fields.iter().find(|entry| {
            compact.contains(&entry.marker.to_ascii_uppercase())
                && differ_only_by_kept_field(polyglot, sqlquack, &entry.field)
        })
    }

    /// The entry explaining where SQLQuack keeps a clause's ALL apart that polyglot reads as a key.
    pub fn kept_all_for(&self, sql: &str, polyglot: &str, sqlquack: &str) -> Option<&KeptAll> {
        let compact = compact(sql);
        self.kept_alls.iter().find(|entry| {
            compact.contains(&entry.marker.to_ascii_uppercase())
                && differ_only_by_kept_all(polyglot, sqlquack, &entry.clause)
        })
    }

    /// The entry explaining where SQLQuack structures a statement polyglot keeps as a command.
    pub fn structured_for(
        &self,
        sql: &str,
        polyglot: &str,
        sqlquack: &str,
    ) -> Option<&StructuredStatement> {
        let compact = compact(sql);
        self.structured.iter().find(|entry| {
            is_opaque(polyglot)
                && sqlquack.starts_with(&format!("{OK_PREFIX}[{{\"{}\":", entry.sqlquack_node))
                && compact.contains(&entry.marker.to_ascii_uppercase())
        })
    }

    /// The first entry explaining a tree that differs only by calls SQLQuack parses into nodes.
    pub fn structured_call_for(
        &self,
        sql: &str,
        polyglot: &str,
        sqlquack: &str,
    ) -> Option<&StructuredCall> {
        let compact = compact(sql);
        let present: Vec<&StructuredCall> = self
            .structured_calls
            .iter()
            .filter(|entry| compact.contains(&entry.marker.to_ascii_uppercase()))
            .collect();
        let calls: Vec<(&str, &str)> = present
            .iter()
            .map(|entry| (entry.sqlquack_node.as_str(), entry.call.as_str()))
            .collect();
        present
            .first()
            .copied()
            .filter(|_| differ_only_by_structured_calls(polyglot, sqlquack, &calls))
    }

    /// Whether keeping `name` where polyglot makes `polyglot` of it is listed.
    pub fn allows(&self, name: &str, polyglot: &str) -> bool {
        self.allowed
            .get(name)
            .is_some_and(|nodes| nodes.contains(polyglot))
    }
}
