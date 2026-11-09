//! Collecting every refusal in a statement, on the current thread only: refused nodes become placeholders.

use std::cell::RefCell;
use std::collections::BTreeSet;

use sqlquack_diagnostics::refusals::models::{Refusal, RefusalCategory};
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::snowflake::conversion::_helpers::support::scope::{Emitted, Scope};
use sqlquack_semantics::schemas::constants::UNKNOWN_RELATION_PREFIX;

/// What collecting found so far in the statement being converted.
#[derive(Default)]
pub(crate) struct Collector {
    /// The refusals recorded, in the order found.
    pub(crate) refusals: Vec<Refusal>,
    /// Guessed types so far: a node failing after this grew may be a knock-on.
    pub(crate) events: usize,
    /// Whether a refusal was dropped as a possible knock-on, or conversion stopped early.
    pub(crate) incomplete: bool,
    /// Relations whose source was refused, whose columns resolve to placeholders.
    pub(crate) opaque: BTreeSet<String>,
    /// Whether part of the statement's structure was skipped, so later names may not resolve.
    pub(crate) skipped: bool,
}

thread_local! {
    static COLLECTOR: RefCell<Option<Collector>> = const { RefCell::new(None) };
}

/// Runs `convert` with collecting on, and returns its result with what was collected.
pub(crate) fn collected<T>(convert: impl FnOnce() -> T) -> (T, Collector) {
    COLLECTOR.with(|collector| *collector.borrow_mut() = Some(Collector::default()));
    let result = convert();
    let collector = COLLECTOR.with(|collector| collector.borrow_mut().take());
    (result, collector.unwrap_or_default())
}

/// Whether the current statement is converted to collect its refusals.
pub(crate) fn collecting() -> bool {
    COLLECTOR.with(|collector| collector.borrow().is_some())
}

/// The events so far, to compare against after converting a node.
pub(crate) fn events() -> usize {
    COLLECTOR.with(|collector| collector.borrow().as_ref().map_or(0, |found| found.events))
}

/// Records `refusal`, or drops it as a possible knock-on of a type guessed since `before`.
pub(crate) fn record(refusal: Refusal, before: usize) {
    COLLECTOR.with(|collector| {
        if let Some(found) = collector.borrow_mut().as_mut() {
            let knock_on = refusal.category != RefusalCategory::MissingRule
                && (found.events != before
                    || found.skipped
                        && refusal.category == RefusalCategory::UnresolvedName
                        && !refusal.reason.starts_with(UNKNOWN_RELATION_PREFIX));
            if !knock_on {
                found.refusals.push(refusal);
                return;
            }
            found.incomplete = true;
        }
    });
}

/// The number of refusals recorded so far.
pub(crate) fn recorded() -> usize {
    COLLECTOR.with(|collector| {
        collector
            .borrow()
            .as_ref()
            .map_or(0, |found| found.refusals.len())
    })
}

/// Whether a refusal recorded since `before` is an unresolved name.
pub(crate) fn unresolved_since(before: usize) -> bool {
    COLLECTOR.with(|collector| {
        collector.borrow().as_ref().is_some_and(|found| {
            found
                .refusals
                .iter()
                .skip(before)
                .any(|refusal| refusal.category == RefusalCategory::UnresolvedName)
        })
    })
}

/// Replaces the refusals recorded since `before` with `refusal`.
pub(crate) fn replace_since(before: usize, refusal: Refusal) {
    COLLECTOR.with(|collector| {
        if let Some(found) = collector.borrow_mut().as_mut() {
            found.refusals.truncate(before);
            found.refusals.push(refusal);
        }
    });
}

/// Runs `convert` with collecting off, then restores what was collected.
pub(crate) fn suspended<T>(convert: impl FnOnce() -> T) -> T {
    let saved = COLLECTOR.with(|collector| collector.borrow_mut().take());
    let result = convert();
    COLLECTOR.with(|collector| *collector.borrow_mut() = saved);
    result
}

/// Marks the statement possibly incomplete: conversion stopped inside a node.
pub(crate) fn stopped() {
    COLLECTOR.with(|collector| {
        if let Some(found) = collector.borrow_mut().as_mut() {
            found.incomplete = true;
        }
    });
}

/// Counts a guessed type as an event, so a node failing over it is not taken as genuine.
pub(crate) fn guessed() {
    COLLECTOR.with(|collector| {
        if let Some(found) = collector.borrow_mut().as_mut() {
            found.events += 1;
        }
    });
}

/// Marks the relation `alias` as one whose columns are unknown, and the structure as skipped.
pub(crate) fn opaque(alias: &str) {
    COLLECTOR.with(|collector| {
        if let Some(found) = collector.borrow_mut().as_mut() {
            found.opaque.insert(alias.to_ascii_uppercase());
            found.skipped = true;
        }
    });
}

/// Marks part of the statement's structure as skipped.
pub(crate) fn skipped() {
    COLLECTOR.with(|collector| {
        if let Some(found) = collector.borrow_mut().as_mut() {
            found.skipped = true;
        }
    });
}

/// Whether `qualifier`, or any name when there is none, may be a column of an opaque relation.
pub(crate) fn opaque_in_scope(scope: &Scope<'_>, qualifier: Option<&str>) -> bool {
    COLLECTOR.with(|collector| {
        let borrowed = collector.borrow();
        let Some(found) = borrowed.as_ref() else {
            return false;
        };
        let mut level = Some(scope);
        while let Some(current) = level {
            if current.relations.iter().any(|relation| {
                found.opaque.contains(&relation.alias.to_ascii_uppercase())
                    && qualifier
                        .is_none_or(|qualifier| relation.alias.eq_ignore_ascii_case(qualifier))
            }) {
                return true;
            }
            level = current.outer;
        }
        false
    })
}

/// The value standing for a refused or unknown node; an untyped one counts as a guess.
pub(crate) fn placeholder(data_type: Option<SnowflakeType>) -> Emitted {
    if data_type.is_none() {
        guessed();
    }
    Emitted::new("NULL".to_owned(), data_type.unwrap_or(SnowflakeType::Null))
}
